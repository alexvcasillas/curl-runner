//! Environments (`.env.{name}` files) and where each `${VAR}` gets its value.

use crate::model::secrets::{SecretStore, service_name};
use curl_runner_core::config::{self, CliOptions};
use curl_runner_core::discover::find_yaml_files;
use curl_runner_core::env;
use curl_runner_core::interpolate::{self, Store, extract_variables, resolve_dynamic_variable};
use curl_runner_core::model::GlobalConfig;
use curl_runner_core::parser;
use curl_runner_core::parser::parse_raw_stream;
use indexmap::IndexMap;
use serde_json::Value as Json;
use std::collections::{HashMap, HashSet};
use std::io;
use std::path::{Path, PathBuf};

/// The environment that loads no `.env.{name}` file.
pub const DEFAULT_ENVIRONMENT: &str = "Default";

/// `Default`, then one entry per `.env.{name}` file in `root` (sorted,
/// `*.local` overrides skipped).
pub fn list_environments(root: &Path) -> Vec<String> {
    let mut names: Vec<String> = std::fs::read_dir(root)
        .into_iter()
        .flatten()
        .flatten()
        .filter(|e| e.file_type().is_ok_and(|t| t.is_file()))
        .filter_map(|e| {
            let file = e.file_name().into_string().ok()?;
            let name = file.strip_prefix(".env.")?;
            (!name.is_empty() && name != "local" && !name.ends_with(".local"))
                .then(|| name.to_string())
        })
        .collect();
    names.sort();
    names.dedup();
    names.insert(0, DEFAULT_ENVIRONMENT.to_string());
    names
}

/// A `SECRET_*` name, or one the collection marked as secret.
pub fn is_secret(name: &str, marked: &[String]) -> bool {
    name.starts_with("SECRET_") || marked.iter().any(|m| m == name)
}

/// Where a variable's value comes from. A plain name resolves Keychain,
/// then process env, then `.env` files; `store.NAME` only from Captured.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum VarSource {
    /// The response store of the file's last run (`store.NAME` only).
    Captured,
    Keychain,
    Process,
    /// The `.env` file that sets it (the last one loaded wins).
    EnvFile(PathBuf),
}

/// Its `Debug` hides the value (it may be a Keychain secret).
#[derive(Clone, PartialEq, Eq)]
pub struct ResolvedVar {
    pub name: String,
    pub value: Option<String>,
    pub source: Option<VarSource>,
    pub secret: bool,
}

impl std::fmt::Debug for ResolvedVar {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ResolvedVar")
            .field("name", &self.name)
            .field("value", &self.value.as_ref().map(|_| ".."))
            .field("source", &self.source)
            .field("secret", &self.secret)
            .finish()
    }
}

/// The collection and environment a run or a lookup is for.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EnvCtx {
    /// Absolute collection root (`.env` files, `curl-runner.yaml`).
    pub root: PathBuf,
    /// Collection name, for the Keychain service.
    pub collection: String,
    /// The collection's `CollectionMeta::id`, for the Keychain service.
    pub collection_id: String,
    /// [`DEFAULT_ENVIRONMENT`] or a `.env.{name}` name.
    pub environment: String,
    /// The collection's `secret_names`.
    pub marked_secret: Vec<String>,
}

impl EnvCtx {
    /// The `--env` value: `None` for [`DEFAULT_ENVIRONMENT`].
    pub fn env_name(&self) -> Option<&str> {
        Some(self.environment.as_str()).filter(|e| *e != DEFAULT_ENVIRONMENT && !e.is_empty())
    }

    /// The CLI options a run of this context uses (the picked environment
    /// as `--env`).
    pub fn cli_options(&self) -> CliOptions {
        CliOptions {
            env: self.env_name().map(String::from),
            ..Default::default()
        }
    }

    /// The Keychain service of this collection and environment.
    pub fn service(&self) -> String {
        service_name(&self.collection, &self.environment, &self.collection_id)
    }
}

/// The environment whose `.env` files a run of `files` (relative to the
/// root) loads: the picked one, else what `load_run` resolves from the
/// files' `global.env.environment` and `curl-runner.yaml` (`env` is the
/// lookup `config::resolve` reads `CURL_RUNNER_*` overrides through).
pub fn effective_environment(
    ctx: &EnvCtx,
    files: &[PathBuf],
    env: &dyn Fn(&str) -> Option<String>,
) -> Option<String> {
    if let Some(e) = ctx.env_name() {
        return Some(e.to_string());
    }
    let mut folded = GlobalConfig::default();
    for f in files {
        let global =
            parser::load_raw(&ctx.root.join(f)).and_then(|raw| parser::global_from_raw(&raw));
        if let Ok(g) = global {
            folded = config::merge_global_configs(&folded, &g);
        }
    }
    let project = config::load_config_file(&ctx.root).config;
    config::resolve(folded, Some(project), env, &ctx.cli_options())
        .env
        .and_then(|e| e.environment)
}

/// Resolve `names` (referenced by `files`, relative to the root) the way a
/// run of `files` does. `store.NAME` reads the captured store
/// only. A plain `NAME` never does: Keychain (secret names only), then the
/// process env, then the environment's `.env` files (core's fallback, as in
/// the CLI).
pub fn resolve_vars(
    names: &[String],
    captured: &Store,
    secrets: &dyn SecretStore,
    ctx: &EnvCtx,
    files: &[PathBuf],
) -> Vec<ResolvedVar> {
    resolve_vars_with(
        names,
        captured,
        secrets,
        ctx,
        files,
        &interpolate::process_env,
    )
}

/// [`resolve_vars`] with an injectable process env (tests).
fn resolve_vars_with(
    names: &[String],
    captured: &Store,
    secrets: &dyn SecretStore,
    ctx: &EnvCtx,
    files: &[PathBuf],
    process_env: &dyn Fn(&str) -> Option<String>,
) -> Vec<ResolvedVar> {
    let environment = effective_environment(ctx, files, process_env);
    // Later files win, as in `env::load_env_files`.
    let files: Vec<_> = env::env_file_paths(&ctx.root, environment.as_deref())
        .into_iter()
        .rev()
        .filter_map(|p| {
            let vars = env::parse_env_content(&std::fs::read_to_string(&p).ok()?);
            Some((p, vars))
        })
        .collect();
    let service = ctx.service();
    names
        .iter()
        .map(|name| {
            if let Some(key) = name.strip_prefix("store.") {
                let value = captured.get(key).cloned();
                return ResolvedVar {
                    name: name.clone(),
                    source: value.as_ref().map(|_| VarSource::Captured),
                    value,
                    secret: false,
                };
            }
            let secret = is_secret(name, &ctx.marked_secret);
            let found = secret
                .then(|| secrets.get(&service, name))
                .flatten()
                .map(|v| (v, VarSource::Keychain))
                .or_else(|| process_env(name).map(|v| (v, VarSource::Process)))
                .or_else(|| {
                    files.iter().find_map(|(p, vars)| {
                        vars.get(name)
                            .map(|v| (v.clone(), VarSource::EnvFile(p.clone())))
                    })
                });
            let (value, source) = found.unzip();
            ResolvedVar {
                name: name.clone(),
                value,
                source,
                secret,
            }
        })
        .collect()
}

/// The `${NAME}` and `${store.NAME}` references of `yaml`, deduplicated in
/// order: no dynamic variables, and no reference with a `:default`.
pub fn referenced_vars(yaml: &str) -> Vec<String> {
    let mut names: Vec<String> = Vec::new();
    for r in extract_variables(yaml) {
        let name = match r.name.split_once(':') {
            // `${NAME:upper}` / `${NAME:lower}` read NAME.
            Some((base, "upper" | "lower")) if !base.is_empty() => base,
            // `${DATE:...}` and friends, or `${NAME:default}`.
            Some(_) => continue,
            None => r.name.as_str(),
        };
        if name.is_empty()
            || resolve_dynamic_variable(name).is_some()
            || names.iter().any(|n| n == name)
        {
            continue;
        }
        names.push(name.to_string());
    }
    names
}

/// The plain references of `yaml` with no value in `resolved` nor in the
/// file's own `variables`. The run still goes: the CLI sends them literally.
/// `store.*` references are never missing: the run fills them.
pub fn missing_vars(yaml: &str, resolved: &[ResolvedVar]) -> Vec<String> {
    let own = file_variables(yaml);
    referenced_vars(yaml)
        .into_iter()
        .filter(|name| {
            !name.starts_with("store.")
                && !own.contains(name)
                && !resolved
                    .iter()
                    .any(|r| &r.name == name && r.value.is_some())
        })
        .collect()
}

/// Names set by the file's `global.variables`, `collection.variables` or a
/// request's `variables` (any document).
fn file_variables(yaml: &str) -> Vec<String> {
    fn collect(v: &Json, out: &mut Vec<String>) {
        match v {
            Json::Object(map) => {
                for (k, v) in map {
                    if k == "variables"
                        && let Json::Object(vars) = v
                    {
                        out.extend(vars.keys().cloned());
                    } else {
                        collect(v, out);
                    }
                }
            }
            Json::Array(items) => items.iter().for_each(|v| collect(v, out)),
            _ => {}
        }
    }
    let mut out = Vec::new();
    if let Ok(raw) = parse_raw_stream(yaml) {
        collect(&raw, &mut out);
    }
    out
}

// Editing environments: `.env` files and the Keychain.

/// The file environment `env`'s values are written to: `.env` for
/// [`DEFAULT_ENVIRONMENT`], else `.env.{env}`. `*.local` files are the user's
/// own overrides: never written.
pub fn env_file(root: &Path, env: &str) -> io::Result<PathBuf> {
    if env == DEFAULT_ENVIRONMENT {
        return Ok(root.join(".env"));
    }
    check_environment_name(env)?;
    Ok(root.join(format!(".env.{env}")))
}

/// A name `.env.{name}` can carry: not empty, no `/`, `\`, `.` or spaces,
/// not `local` (the `.env.local` override) nor `Default`.
fn check_environment_name(name: &str) -> io::Result<()> {
    let bad = name.is_empty()
        || name.contains(['/', '\\', '.'])
        || name.chars().any(|c| c.is_whitespace() || c.is_control())
        || name == "local"
        || name.eq_ignore_ascii_case(DEFAULT_ENVIRONMENT);
    if bad {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("“{name}” can't name an environment. Use letters, digits, - or _."),
        ));
    }
    Ok(())
}

/// A name a `.env` line can set and core's parser reads back.
fn check_var_name(name: &str) -> io::Result<()> {
    let bad = name.is_empty()
        || name.starts_with('#')
        || name.contains('=')
        || name.chars().any(|c| c.is_whitespace() || c.is_control());
    if bad {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("“{name}” can't name a variable."),
        ));
    }
    Ok(())
}

/// Creates the empty `.env.{name}` of a new environment.
pub fn create_environment(root: &Path, name: &str) -> io::Result<()> {
    check_environment_name(name)?;
    std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(root.join(format!(".env.{name}")))
        .map(drop)
}

/// Sets `name` in environment `env`'s file. An existing line is replaced in
/// place, keeping its quoting; a new one goes at the end. Comments, blank
/// lines and the order of the others are kept.
pub fn write_env_var(root: &Path, env: &str, name: &str, value: &str) -> io::Result<()> {
    check_var_name(name)?;
    edit_env_file(root, env, |lines| set_line(lines, name, value))
}

/// Removes every line setting `name` from environment `env`'s file.
pub fn remove_env_var(root: &Path, env: &str, name: &str) -> io::Result<()> {
    edit_env_file(root, env, |lines| {
        remove_lines(lines, name);
        Ok(())
    })
}

/// Removes `name` from `file`, a `.env` or `.env.{env}` file in `root` (a
/// file another environment owns). `*.local` overrides are refused.
pub fn remove_env_var_at(root: &Path, file: &Path, name: &str) -> io::Result<()> {
    let refuse = || {
        io::Error::new(
            io::ErrorKind::InvalidInput,
            format!(
                "{} is not an environment file the studio writes.",
                file.display()
            ),
        )
    };
    if file.parent() != Some(root) {
        return Err(refuse());
    }
    let file_name = file.file_name().ok_or_else(refuse)?.to_string_lossy();
    let env = match file_name.as_ref() {
        ".env" => DEFAULT_ENVIRONMENT,
        n => n.strip_prefix(".env.").ok_or_else(refuse)?,
    };
    // `env_file` refuses `local` and `x.local`.
    remove_env_var(root, env, name)
}

/// Which environments read `file_name` (a `.env*` file): `Some("")` for
/// `.env` and `.env.local` (every one), `Some(env)` for `.env.{env}[.local]`.
pub fn file_readers(file_name: &str) -> Option<String> {
    let rest = file_name.strip_prefix(".env")?;
    if rest.is_empty() || rest == ".local" {
        return Some(String::new());
    }
    let env = rest.strip_prefix('.')?;
    let env = env.strip_suffix(".local").unwrap_or(env);
    (!env.is_empty()).then(|| env.to_string())
}

/// Every `.env` file environment `env` loads that sets `name`, the one that
/// wins first (`*.local` overrides included).
pub fn plain_copies(root: &Path, env: &str, name: &str) -> Vec<PathBuf> {
    let env = Some(env).filter(|e| *e != DEFAULT_ENVIRONMENT);
    let mut files: Vec<PathBuf> = env::env_file_paths(root, env)
        .into_iter()
        .filter(|p| {
            std::fs::read_to_string(p).is_ok_and(|t| env::parse_env_content(&t).contains_key(name))
        })
        .collect();
    files.reverse();
    files
}

/// Whether any `.env` file environment `env` loads sets `name`.
pub fn name_in_env_files(root: &Path, env: &str, name: &str) -> bool {
    let env = Some(env).filter(|e| *e != DEFAULT_ENVIRONMENT);
    env::env_file_paths(root, env).iter().any(|p| {
        std::fs::read_to_string(p).is_ok_and(|t| env::parse_env_content(&t).contains_key(name))
    })
}

/// Imports `.env` text into environment `env`: secret names (see
/// [`is_secret`]) go to `secrets` under `service` and out of the file,
/// the rest into the file. Returns how many variables were imported.
pub fn import_dotenv(
    root: &Path,
    env: &str,
    content: &str,
    secrets: &dyn SecretStore,
    service: &str,
    marked: &[String],
) -> io::Result<usize> {
    let vars: Vec<(String, String)> = env::parse_env_content(content)
        .into_iter()
        .filter(|(k, _)| check_var_name(k).is_ok())
        .collect();
    for (k, v) in vars.iter().filter(|(k, _)| is_secret(k, marked)) {
        secrets.set(service, k, v).map_err(io::Error::other)?;
    }
    edit_env_file(root, env, |lines| {
        for (k, v) in &vars {
            if is_secret(k, marked) {
                remove_lines(lines, k);
            } else {
                set_line(lines, k, v)?;
            }
        }
        Ok(())
    })?;
    Ok(vars.len())
}

/// Makes `name` a Keychain value: `value` goes to `secrets`, then its line
/// leaves environment `env`'s file.
pub fn move_to_keychain(
    root: &Path,
    env: &str,
    name: &str,
    value: &str,
    secrets: &dyn SecretStore,
    service: &str,
) -> io::Result<()> {
    secrets
        .set(service, name, value)
        .map_err(io::Error::other)?;
    remove_env_var(root, env, name)
}

/// Makes `name` a file value again: `value` (else the Keychain's) is written
/// to environment `env`'s file, then the Keychain item is deleted.
pub fn move_to_file(
    root: &Path,
    env: &str,
    name: &str,
    value: Option<&str>,
    secrets: &dyn SecretStore,
    service: &str,
) -> io::Result<()> {
    let value = match value {
        Some(v) => v.to_string(),
        None => secrets
            .try_get(service, name)
            .map_err(io::Error::other)?
            .unwrap_or_default(),
    };
    write_env_var(root, env, name, &value)?;
    secrets.delete(service, name).map_err(io::Error::other)
}

/// Rewrites environment `env`'s file with `edit` applied to its lines, through
/// a temp file and a rename; an unchanged file is not written. A missing file
/// starts empty.
fn edit_env_file(
    root: &Path,
    env: &str,
    edit: impl FnOnce(&mut Vec<String>) -> io::Result<()>,
) -> io::Result<()> {
    let path = env_file(root, env)?;
    // Through a symlink, to the file it points at.
    let path = path.canonicalize().unwrap_or(path);
    let content = match std::fs::read_to_string(&path) {
        Ok(c) => c,
        Err(e) if e.kind() == io::ErrorKind::NotFound => String::new(),
        Err(e) => return Err(e),
    };
    let mut lines: Vec<String> = content.split('\n').map(String::from).collect();
    // A symlink to a `*.local` override is still the user's own file.
    if path
        .file_name()
        .is_some_and(|n| n.to_string_lossy().ends_with(".local"))
    {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!(
                "{} is a local override: the studio never writes it.",
                path.display()
            ),
        ));
    }
    edit(&mut lines)?;
    let out = lines.join("\n");
    if out == content {
        return Ok(());
    }
    crate::model::write_atomic(&path, &out)
}

/// The key a `.env` line sets, as core's parser reads it.
fn line_key(line: &str) -> Option<&str> {
    let t = line.trim().trim_start_matches('\u{feff}');
    if t.is_empty() || t.starts_with('#') {
        return None;
    }
    let eq = t.find('=')?;
    Some(t[..eq].trim()).filter(|k| !k.is_empty())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Quote {
    None,
    Single,
    Double,
}

/// `value` as a `.env` value in `quote` style, or the closest style that
/// reads back as `value`; `None` when no style does (core's parser can't
/// unescape a backslash before `n`, `r` or `t` in a multi-line value).
fn encode_value(value: &str, quote: Quote) -> Option<String> {
    let multi_line = value.contains(['\n', '\r']);
    let needs_quotes =
        value.contains('\t') || value != value.trim() || value.starts_with(['"', '\'']);
    // Only double quotes unescape, and only they hold a line break.
    let quote = match quote {
        _ if multi_line => Quote::Double,
        Quote::None if needs_quotes => Quote::Single,
        Quote::Double if value.contains('\\') => Quote::Single,
        q => q,
    };
    let encoded = match quote {
        Quote::None => value.to_string(),
        Quote::Single => format!("'{value}'"),
        Quote::Double => {
            let escaped = value
                .replace('\\', "\\\\")
                .replace('\n', "\\n")
                .replace('\r', "\\r")
                .replace('\t', "\\t");
            format!("\"{escaped}\"")
        }
    };
    let reads_back = env::parse_env_content(&format!("K={encoded}"))
        .get("K")
        .is_some_and(|v| v == value);
    reads_back.then_some(encoded)
}

/// The error for a value a `.env` file can't hold.
fn unencodable(name: &str) -> io::Error {
    io::Error::new(
        io::ErrorKind::InvalidInput,
        format!("The value of {name} can't be written to a .env file."),
    )
}

/// Sets `name` on its last line (the one that wins), else appends it.
fn set_line(lines: &mut Vec<String>, name: &str, value: &str) -> io::Result<()> {
    if let Some(at) = lines.iter().rposition(|l| line_key(l) == Some(name)) {
        let line = &lines[at];
        let cr = if line.ends_with('\r') { "\r" } else { "" };
        let body = line.strip_suffix('\r').unwrap_or(line);
        let eq = body.find('=').unwrap_or(body.len());
        let rest = &body[(eq + 1).min(body.len())..];
        let space = &rest[..rest.len() - rest.trim_start().len()];
        let quote = match rest.trim_start().chars().next() {
            Some('"') => Quote::Double,
            Some('\'') => Quote::Single,
            _ => Quote::None,
        };
        let encoded = encode_value(value, quote).ok_or_else(|| unencodable(name))?;
        lines[at] = format!("{}={space}{encoded}{cr}", &body[..eq]);
        return Ok(());
    }
    let crlf = lines.iter().any(|l| l.ends_with('\r'));
    let encoded = encode_value(value, Quote::None).ok_or_else(|| unencodable(name))?;
    let line = format!("{name}={encoded}{}", if crlf { "\r" } else { "" });
    // Before the empty piece a trailing newline leaves.
    match lines.last().map(String::as_str) {
        Some("") if lines.len() == 1 => {
            lines.insert(0, line);
        }
        Some("") => {
            let at = lines.len() - 1;
            lines.insert(at, line);
        }
        _ => {
            // The last line had no line break: it gets the file's.
            if crlf && let Some(last) = lines.last_mut() {
                last.push('\r');
            }
            lines.push(line);
            lines.push(String::new());
        }
    }
    Ok(())
}

fn remove_lines(lines: &mut Vec<String>, name: &str) {
    lines.retain(|l| line_key(l) != Some(name));
}

/// Where `${NAME}` is used in the collection's requests, by top folder.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct UsageIndex {
    /// Name → (top folder, or the file name for a top-level file; how many
    /// requests there use it), in folder order.
    by_name: HashMap<String, Vec<(String, usize)>>,
    /// Every top folder (or top-level file) holding a request.
    groups: Vec<String>,
    /// Names the files set themselves (`variables:`).
    defined: HashSet<String>,
}

impl UsageIndex {
    /// Reads every request file under `root` once.
    pub fn scan(root: &Path) -> UsageIndex {
        let mut index = UsageIndex::default();
        let files = find_yaml_files(&[], true, root)
            .into_iter()
            .map(|f| PathBuf::from(f.strip_prefix("./").unwrap_or(&f)))
            .filter(|p| p != Path::new("curl-runner.yaml"));
        for rel in files {
            let Ok(text) = std::fs::read_to_string(root.join(&rel)) else {
                continue;
            };
            // The top folder; a top-level file is its own group.
            let first = rel.components().next();
            let Some(group) = first.map(|c| c.as_os_str().to_string_lossy().into_owned()) else {
                continue;
            };
            let counts = request_counts(&text);
            if counts.requests == 0 {
                continue;
            }
            if !index.groups.contains(&group) {
                index.groups.push(group.clone());
            }
            index.defined.extend(file_variables(&text));
            for (name, n) in counts.by_name {
                let entry = index.by_name.entry(name).or_default();
                match entry.iter_mut().find(|(g, _)| *g == group) {
                    Some((_, total)) => *total += n,
                    None => entry.push((group.clone(), n)),
                }
            }
        }
        index.groups.sort();
        for v in index.by_name.values_mut() {
            v.sort();
        }
        index
    }

    /// Requests using `name`, per top folder.
    pub fn of(&self, name: &str) -> &[(String, usize)] {
        self.by_name.get(name).map_or(&[], Vec::as_slice)
    }

    /// How many requests use `name`.
    pub fn total(&self, name: &str) -> usize {
        self.of(name).iter().map(|(_, n)| n).sum()
    }

    /// The board's Scope: `All requests` when every top folder uses `name`,
    /// else `charges/*` (a folder) or `smoke.yaml` (a top-level file), joined;
    /// `None` when nothing uses it.
    pub fn scope(&self, root: &Path, name: &str) -> Option<String> {
        let used = self.of(name);
        if used.is_empty() {
            return None;
        }
        if used.len() == self.groups.len() {
            return Some("All requests".into());
        }
        let label = |g: &str| {
            if root.join(g).is_dir() {
                format!("{g}/*")
            } else {
                g.to_string()
            }
        };
        Some(
            used.iter()
                .map(|(g, _)| label(g))
                .collect::<Vec<_>>()
                .join(", "),
        )
    }

    /// The plain names requests use that no file sets itself, sorted.
    pub fn referenced(&self) -> Vec<String> {
        let mut names: Vec<String> = self
            .by_name
            .keys()
            .filter(|n| !n.starts_with("store.") && !self.defined.contains(*n))
            .cloned()
            .collect();
        names.sort();
        names
    }
}

/// [`UsageIndex::of`] for one name (reads every request file). The screen
/// scans once with [`UsageIndex::scan`] instead.
#[cfg(test)]
pub fn usage(root: &Path, name: &str) -> Vec<(String, usize)> {
    UsageIndex::scan(root).of(name).to_vec()
}

/// A file's request count and, per referenced name, how many of its
/// requests use it (a reference outside the requests counts for them all).
struct RequestCounts {
    requests: usize,
    by_name: Vec<(String, usize)>,
}

fn request_counts(text: &str) -> RequestCounts {
    let Ok(raw) = parse_raw_stream(text) else {
        // Unparsed: one request, using whatever the text names.
        return RequestCounts {
            requests: 1,
            by_name: referenced_vars(text).into_iter().map(|n| (n, 1)).collect(),
        };
    };
    let docs = match raw {
        Json::Array(docs) => docs,
        doc => vec![doc],
    };
    let mut requests = 0;
    let mut by_name: Vec<(String, usize)> = Vec::new();
    let mut add = |name: String, n: usize| match by_name.iter_mut().find(|(k, _)| *k == name) {
        Some((_, total)) => *total += n,
        None => by_name.push((name, n)),
    };
    for mut doc in docs {
        let Some(map) = doc.as_object_mut() else {
            continue;
        };
        let mut items: Vec<Json> = Vec::new();
        if let Some(r) = map.remove("request").filter(Json::is_object) {
            items.push(r);
        }
        if let Some(Json::Array(rs)) = map.remove("requests") {
            items.extend(rs);
        }
        if let Some(Json::Object(c)) = map.get_mut("collection")
            && let Some(Json::Array(rs)) = c.remove("requests")
        {
            items.extend(rs);
        }
        if items.is_empty() {
            continue;
        }
        requests += items.len();
        // What is left (`global`, `collection.defaults`…) applies to all.
        let shared = referenced_vars(&doc.to_string());
        for name in &shared {
            add(name.clone(), items.len());
        }
        for r in &items {
            for name in referenced_vars(&r.to_string()) {
                if !shared.contains(&name) {
                    add(name, 1);
                }
            }
        }
    }
    RequestCounts { requests, by_name }
}

/// The value names `text`'s requests save with `store:`: name → (the
/// request's `name`, if it has one; the response path).
pub fn store_paths(text: &str) -> HashMap<String, (Option<String>, String)> {
    fn walk(v: &Json, out: &mut HashMap<String, (Option<String>, String)>) {
        match v {
            Json::Object(map) => {
                if let Some(Json::Object(store)) = map.get("store") {
                    let request = map.get("name").and_then(Json::as_str).map(String::from);
                    for (k, path) in store {
                        if let Some(path) = path.as_str() {
                            out.insert(k.clone(), (request.clone(), path.to_string()));
                        }
                    }
                }
                map.values().for_each(|v| walk(v, out));
            }
            Json::Array(items) => items.iter().for_each(|v| walk(v, out)),
            _ => {}
        }
    }
    let mut out = HashMap::new();
    if let Ok(raw) = parse_raw_stream(text) {
        walk(&raw, &mut out);
    }
    out
}

/// A value the last run saved: shown by name and origin, never by value.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CapturedVar {
    pub name: String,
    /// The request that saved it.
    pub request: Option<String>,
    /// The response path it came from (`body.id`).
    pub path: Option<String>,
}

/// Where an Environments row's value comes from.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RowSource {
    /// A `.env` file (`.env` itself is the collection default).
    File(PathBuf),
    Keychain,
    Process,
    Captured {
        request: Option<String>,
        path: Option<String>,
    },
    /// A received copy's default, set by the owner `org`: read only.
    Shared(String),
    NotSet,
}

/// One row of the Environments table.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EnvRow {
    pub name: String,
    /// The value, for non-secret names only: a secret's is read from the
    /// Keychain when its drawer opens, never for the table.
    pub value: Option<String>,
    pub source: RowSource,
    pub secret: bool,
    /// A Keychain value that still has a plain-text copy in this file.
    pub plain_copy: Option<PathBuf>,
}

/// Environment `env`'s variables as the Environments screen lists them:
/// the names its `.env` files set, its Keychain items (`keychain`, secret
/// names only, as resolution reads them), the names marked secret, the
/// plain names requests use (`referenced`) and the last run's captured
/// values. A plain name's source follows resolution: Keychain, then the
/// process env, then the files (later wins). No Keychain value is read.
pub fn environment_rows(
    root: &Path,
    environment: &str,
    marked: &[String],
    keychain: &[String],
    referenced: &[String],
    captured: &[CapturedVar],
    process_env: &dyn Fn(&str) -> Option<String>,
) -> Vec<EnvRow> {
    let env_name = Some(environment).filter(|e| *e != DEFAULT_ENVIRONMENT);
    // Later files win, as in `env::load_env_files`.
    let files: Vec<(PathBuf, IndexMap<String, String>)> = env::env_file_paths(root, env_name)
        .into_iter()
        .filter_map(|p| {
            let vars = env::parse_env_content(&std::fs::read_to_string(&p).ok()?);
            Some((p, vars))
        })
        .collect();
    let mut names: Vec<String> = Vec::new();
    let mut push = |n: &str| {
        if !names.iter().any(|m| m == n) {
            names.push(n.to_string());
        }
    };
    files
        .iter()
        .flat_map(|(_, v)| v.keys())
        .for_each(|n| push(n));
    keychain
        .iter()
        .filter(|n| is_secret(n, marked))
        .for_each(|n| push(n));
    marked.iter().for_each(|n| push(n));
    referenced.iter().for_each(|n| push(n));
    let mut rows: Vec<EnvRow> = names
        .into_iter()
        .map(|name| {
            let secret = is_secret(&name, marked);
            let (value, source) = if secret && keychain.contains(&name) {
                (None, RowSource::Keychain)
            } else if let Some(v) = process_env(&name) {
                (Some(v), RowSource::Process)
            } else if let Some((p, v)) = files
                .iter()
                .rev()
                .find_map(|(p, vars)| Some((p, vars.get(&name)?)))
            {
                (Some(v.clone()), RowSource::File(p.clone()))
            } else {
                (None, RowSource::NotSet)
            };
            let plain_copy = (source == RowSource::Keychain)
                .then(|| {
                    files
                        .iter()
                        .rev()
                        .find(|(_, vars)| vars.contains_key(&name))
                        .map(|(p, _)| p.clone())
                })
                .flatten();
            EnvRow {
                value: value.filter(|_| !secret),
                name,
                source,
                secret,
                plain_copy,
            }
        })
        .collect();
    rows.extend(captured.iter().map(|c| EnvRow {
        name: c.name.clone(),
        value: None,
        source: RowSource::Captured {
            request: c.request.clone(),
            path: c.path.clone(),
        },
        secret: false,
        plain_copy: None,
    }));
    rows
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::secrets::MemorySecrets;
    use std::fs;

    fn ctx(root: &Path, environment: &str) -> EnvCtx {
        EnvCtx {
            root: root.to_path_buf(),
            collection: "Payments API".into(),
            collection_id: "0a1b2c3d".into(),
            environment: environment.into(),
            marked_secret: vec![],
        }
    }

    #[test]
    fn environments_are_default_plus_env_files() {
        let d = tempfile::tempdir().unwrap();
        for f in [
            ".env",
            ".env.staging",
            ".env.production",
            ".env.staging.local",
        ] {
            fs::write(d.path().join(f), "A=1\n").unwrap();
        }
        assert_eq!(
            list_environments(d.path()),
            ["Default", "production", "staging"]
        );
    }

    #[test]
    fn secret_by_prefix_or_mark() {
        assert!(is_secret("SECRET_KEY", &[]));
        assert!(is_secret("API_TOKEN", &["API_TOKEN".into()]));
        assert!(!is_secret("BASE_URL", &[]));
    }

    #[test]
    fn precedence_keychain_over_process_over_env_file() {
        let d = tempfile::tempdir().unwrap();
        let env_file = d.path().join(".env.staging");
        fs::write(&env_file, "SECRET_TOKEN=from-file\n").unwrap();
        let ctx = ctx(d.path(), "staging");
        let secrets = MemorySecrets::default();
        secrets
            .set(&ctx.service(), "SECRET_TOKEN", "from-keychain")
            .unwrap();
        // A captured value never fills a plain `${NAME}`.
        let mut store = Store::new();
        store.insert("SECRET_TOKEN".into(), "captured".into());
        let names = vec!["SECRET_TOKEN".to_string()];
        let process = |k: &str| (k == "SECRET_TOKEN").then(|| "from-process".to_string());
        let none = |_: &str| None;
        let one = |env: &dyn Fn(&str) -> Option<String>| {
            resolve_vars_with(&names, &store, &secrets, &ctx, &[], env).remove(0)
        };

        let v = one(&process);
        assert_eq!(v.value.as_deref(), Some("from-keychain"));
        assert_eq!(v.source, Some(VarSource::Keychain));
        assert!(v.secret);

        secrets.delete(&ctx.service(), "SECRET_TOKEN").unwrap();
        let v = one(&process);
        assert_eq!(v.value.as_deref(), Some("from-process"));
        assert_eq!(v.source, Some(VarSource::Process));

        let v = one(&none);
        assert_eq!(v.value.as_deref(), Some("from-file"));
        assert_eq!(v.source, Some(VarSource::EnvFile(env_file)));
    }

    #[test]
    fn store_refs_resolve_from_captured_only() {
        let d = tempfile::tempdir().unwrap();
        let ctx = ctx(d.path(), DEFAULT_ENVIRONMENT);
        let secrets = MemorySecrets::default();
        secrets.set(&ctx.service(), "SECRET_ID", "kc").unwrap();
        let mut store = Store::new();
        store.insert("ID".into(), "abc".into());
        let yaml = "${store.ID} ${store.SECRET_ID} ${ID}";
        let names = referenced_vars(yaml);
        assert_eq!(names, ["store.ID", "store.SECRET_ID", "ID"]);
        let none = |_: &str| None;
        let v = resolve_vars_with(&names, &store, &secrets, &ctx, &[], &none);
        assert_eq!(v[0].value.as_deref(), Some("abc"));
        assert_eq!(v[0].source, Some(VarSource::Captured));
        assert_eq!(v[1].value, None);
        assert_eq!(v[1].source, None);
        assert_eq!(v[2].value, None);
        // Unfilled store refs are filled at run time, never missing.
        assert_eq!(missing_vars(yaml, &v), ["ID"]);
    }

    #[test]
    fn resolved_var_debug_hides_the_value() {
        let v = ResolvedVar {
            name: "SECRET_TOKEN".into(),
            value: Some("kc-value-31".into()),
            source: Some(VarSource::Keychain),
            secret: true,
        };
        let dbg = format!("{v:?}");
        assert!(
            dbg.contains("SECRET_TOKEN") && !dbg.contains("kc-value-31"),
            "{dbg}"
        );
    }

    #[test]
    fn keychain_only_for_secret_names() {
        let d = tempfile::tempdir().unwrap();
        let ctx = ctx(d.path(), DEFAULT_ENVIRONMENT);
        let secrets = MemorySecrets::default();
        secrets.set(&ctx.service(), "BASE_URL", "x").unwrap();
        let v = resolve_vars(&["BASE_URL".into()], &Store::new(), &secrets, &ctx, &[]);
        assert_eq!(v[0].source, None);
        assert_eq!(v[0].value, None);
    }

    #[test]
    fn missing_vars_lists_unset_names_only() {
        let d = tempfile::tempdir().unwrap();
        fs::write(d.path().join(".env"), "BASE_URL=https://api.example.com\n").unwrap();
        let yaml = "${BASE_URL}/x ${API_TOKEN} ${store.ID} ${DATE:YYYY}";
        let names = referenced_vars(yaml);
        assert_eq!(names, ["BASE_URL", "API_TOKEN", "store.ID"]);
        let resolved = resolve_vars(
            &names,
            &Store::new(),
            &MemorySecrets::default(),
            &ctx(d.path(), DEFAULT_ENVIRONMENT),
            &[],
        );
        assert_eq!(missing_vars(yaml, &resolved), ["API_TOKEN"]);
    }

    #[test]
    fn default_uses_the_environment_the_yaml_picks() {
        let d = tempfile::tempdir().unwrap();
        fs::write(d.path().join(".env"), "HOST=default.example.com\n").unwrap();
        fs::write(d.path().join(".env.staging"), "HOST=staging.example.com\n").unwrap();
        fs::write(
            d.path().join("a.yaml"),
            "global:\n  env:\n    environment: staging\nrequest:\n  url: https://${HOST}\n",
        )
        .unwrap();
        let ctx = ctx(d.path(), DEFAULT_ENVIRONMENT);
        let none = |_: &str| None;
        let files = [PathBuf::from("a.yaml")];
        assert_eq!(
            effective_environment(&ctx, &files, &none).as_deref(),
            Some("staging")
        );
        let v = resolve_vars_with(
            &["HOST".into()],
            &Store::new(),
            &MemorySecrets::default(),
            &ctx,
            &files,
            &none,
        );
        assert_eq!(v[0].value.as_deref(), Some("staging.example.com"));
        assert_eq!(
            v[0].source,
            Some(VarSource::EnvFile(d.path().join(".env.staging")))
        );
        // The picked environment wins (it is `--env`).
        let picked = EnvCtx {
            environment: "qa".into(),
            ..ctx.clone()
        };
        assert_eq!(
            effective_environment(&picked, &files, &none).as_deref(),
            Some("qa")
        );
    }

    #[test]
    fn file_variables_and_defaults_are_not_missing() {
        let yaml = "global:\n  variables:\n    HOST: api.example.com\nrequest:\n  url: https://${HOST}/${PATH:v1}/${UUID}/${ID:upper}\n";
        assert_eq!(missing_vars(yaml, &[]), ["ID"]);
    }

    #[test]
    fn write_env_var_preserves_other_lines() {
        let d = tempfile::tempdir().unwrap();
        let file = d.path().join(".env.staging");
        let before = "# Staging\nBASE_URL=https://staging.api.example.com\n\n# keys\nREGION='eu'\nTOKEN = \"a b\"\n";
        fs::write(&file, before).unwrap();
        write_env_var(d.path(), "staging", "REGION", "us").unwrap();
        write_env_var(d.path(), "staging", "TOKEN", "c\"d\\e").unwrap();
        write_env_var(d.path(), "staging", "NEW", "x y").unwrap();
        let after = fs::read_to_string(&file).unwrap();
        assert_eq!(
            after,
            "# Staging\nBASE_URL=https://staging.api.example.com\n\n# keys\nREGION='us'\nTOKEN = 'c\"d\\e'\nNEW=x y\n"
        );
        let vars = env::parse_env_content(&after);
        assert_eq!(vars["REGION"], "us");
        assert_eq!(vars["TOKEN"], "c\"d\\e");
        assert_eq!(vars["NEW"], "x y");
        // Default writes `.env`, created when missing.
        write_env_var(d.path(), DEFAULT_ENVIRONMENT, "A", "1").unwrap();
        assert_eq!(fs::read_to_string(d.path().join(".env")).unwrap(), "A=1\n");
        // CRLF files stay CRLF; a file without a final newline gets one.
        fs::write(d.path().join(".env.crlf"), "A=1\r\nB=2").unwrap();
        write_env_var(d.path(), "crlf", "A", "3").unwrap();
        write_env_var(d.path(), "crlf", "C", "4").unwrap();
        assert_eq!(
            fs::read_to_string(d.path().join(".env.crlf")).unwrap(),
            "A=3\r\nB=2\r\nC=4\r\n"
        );
        remove_env_var(d.path(), "staging", "REGION").unwrap();
        let after = fs::read_to_string(&file).unwrap();
        assert!(!after.contains("REGION") && after.starts_with("# Staging\n"));
        assert!(write_env_var(d.path(), "staging", "BAD NAME", "x").is_err());
    }

    #[test]
    fn written_values_read_back() {
        let values = [
            "",
            "plain",
            "x y",
            " padded ",
            "\"quoted",
            "'single",
            "a\nb",
            "tab\there",
            "back\\slash",
            "a\\\nb",
        ];
        for value in values {
            for quote in [Quote::None, Quote::Single, Quote::Double] {
                let line = format!("K={}", encode_value(value, quote).unwrap());
                assert_eq!(env::parse_env_content(&line)["K"], value, "{line}");
            }
        }
        // core reads `\\n` in double quotes as a line break.
        assert_eq!(encode_value("a\\nb\nc", Quote::None), None);
        let d = tempfile::tempdir().unwrap();
        let err = write_env_var(d.path(), "qa", "K", "a\\nb\nc").unwrap_err();
        assert_eq!(err.kind(), io::ErrorKind::InvalidInput);
    }

    #[test]
    fn never_writes_local_files() {
        let d = tempfile::tempdir().unwrap();
        fs::write(d.path().join(".env.local"), "A=mine\n").unwrap();
        fs::write(d.path().join(".env.staging.local"), "A=mine\n").unwrap();
        assert!(write_env_var(d.path(), "local", "A", "x").is_err());
        assert!(write_env_var(d.path(), "staging.local", "A", "x").is_err());
        assert!(create_environment(d.path(), "local").is_err());
        write_env_var(d.path(), DEFAULT_ENVIRONMENT, "A", "x").unwrap();
        write_env_var(d.path(), "staging", "A", "x").unwrap();
        remove_env_var(d.path(), "staging", "A").unwrap();
        let secrets = MemorySecrets::default();
        import_dotenv(d.path(), "staging", "A=y\n", &secrets, "s", &[]).unwrap();
        assert_eq!(
            fs::read_to_string(d.path().join(".env.local")).unwrap(),
            "A=mine\n"
        );
        assert_eq!(
            fs::read_to_string(d.path().join(".env.staging.local")).unwrap(),
            "A=mine\n"
        );
    }

    #[test]
    fn create_environment_validates_name() {
        let d = tempfile::tempdir().unwrap();
        for bad in [
            "",
            "a/b",
            "a.b",
            "..",
            "with space",
            "local",
            "Default",
            "default",
        ] {
            assert_eq!(
                create_environment(d.path(), bad).unwrap_err().kind(),
                io::ErrorKind::InvalidInput,
                "{bad:?}"
            );
        }
        create_environment(d.path(), "qa").unwrap();
        assert_eq!(fs::read_to_string(d.path().join(".env.qa")).unwrap(), "");
        assert_eq!(list_environments(d.path()), ["Default", "qa"]);
        assert_eq!(
            create_environment(d.path(), "qa").unwrap_err().kind(),
            io::ErrorKind::AlreadyExists
        );
    }

    #[test]
    fn import_dotenv_routes_secrets_to_keychain() {
        let d = tempfile::tempdir().unwrap();
        let file = d.path().join(".env.staging");
        fs::write(&file, "# keep\nSECRET_X=old-plain\nBASE_URL=old\n").unwrap();
        let secrets = MemorySecrets::default();
        let service = service_name("Payments API", "staging", "0a1b2c3d");
        let marked = vec!["API_TOKEN".to_string()];
        let content = "BASE_URL=https://staging.api.example.com\nSECRET_X=s3cr3t\nAPI_TOKEN=\"tok en\"\nexport BAD=1\n";
        let n = import_dotenv(d.path(), "staging", content, &secrets, &service, &marked).unwrap();
        assert_eq!(n, 3);
        assert_eq!(secrets.get(&service, "SECRET_X").as_deref(), Some("s3cr3t"));
        assert_eq!(
            secrets.get(&service, "API_TOKEN").as_deref(),
            Some("tok en")
        );
        let after = fs::read_to_string(&file).unwrap();
        assert_eq!(after, "# keep\nBASE_URL=https://staging.api.example.com\n");
        assert!(!after.contains("s3cr3t") && !after.contains("tok en"));
    }

    #[test]
    fn toggle_secret_moves_value() {
        let d = tempfile::tempdir().unwrap();
        let file = d.path().join(".env.staging");
        fs::write(&file, "BASE_URL=x\nAPI_TOKEN=abc\n# end\n").unwrap();
        let secrets = MemorySecrets::default();
        let service = service_name("Payments API", "staging", "0a1b2c3d");
        move_to_keychain(d.path(), "staging", "API_TOKEN", "abc", &secrets, &service).unwrap();
        assert_eq!(fs::read_to_string(&file).unwrap(), "BASE_URL=x\n# end\n");
        assert_eq!(secrets.get(&service, "API_TOKEN").as_deref(), Some("abc"));
        move_to_file(d.path(), "staging", "API_TOKEN", None, &secrets, &service).unwrap();
        assert_eq!(
            fs::read_to_string(&file).unwrap(),
            "BASE_URL=x\n# end\nAPI_TOKEN=abc\n"
        );
        assert_eq!(secrets.get(&service, "API_TOKEN"), None);
        // An edited value goes to the file instead of the Keychain's.
        secrets.set(&service, "API_TOKEN", "kc").unwrap();
        move_to_file(
            d.path(),
            "staging",
            "API_TOKEN",
            Some("new"),
            &secrets,
            &service,
        )
        .unwrap();
        assert_eq!(
            env::parse_env_content(&fs::read_to_string(&file).unwrap())["API_TOKEN"],
            "new"
        );
        assert_eq!(secrets.get(&service, "API_TOKEN"), None);
    }

    #[test]
    fn usage_counts_by_folder() {
        let d = tempfile::tempdir().unwrap();
        let files = [
            (
                "auth/login.yaml",
                "request:\n  url: ${BASE_URL}/login\n  headers:\n    Authorization: Bearer ${API_TOKEN}\n",
            ),
            (
                "charges/list.yaml",
                "requests:\n  - url: ${BASE_URL}/charges\n  - url: ${BASE_URL}/charges/${store.ID}\n    headers:\n      X: ${API_TOKEN}\n",
            ),
            (
                "charges/deep/one.yaml",
                "global:\n  variables:\n    OWN: x\n  defaults:\n    headers:\n      Authorization: ${API_TOKEN}\nrequests:\n  - url: ${BASE_URL}/a/${OWN}\n  - url: ${BASE_URL}/b\n",
            ),
            ("smoke.yaml", "request:\n  url: ${BASE_URL}/health\n"),
            (
                "curl-runner.yaml",
                "global:\n  variables:\n    X: ${BASE_URL}\n",
            ),
        ];
        for (rel, text) in files {
            let p = d.path().join(rel);
            fs::create_dir_all(p.parent().unwrap()).unwrap();
            fs::write(p, text).unwrap();
        }
        assert_eq!(
            usage(d.path(), "API_TOKEN"),
            [("auth".to_string(), 1), ("charges".to_string(), 3)]
        );
        let index = UsageIndex::scan(d.path());
        assert_eq!(index.total("BASE_URL"), 6);
        assert_eq!(
            index.scope(d.path(), "BASE_URL").as_deref(),
            Some("All requests")
        );
        assert_eq!(
            index.scope(d.path(), "API_TOKEN").as_deref(),
            Some("auth/*, charges/*")
        );
        assert_eq!(index.scope(d.path(), "NOPE"), None);
        assert_eq!(
            index.referenced(),
            ["API_TOKEN", "BASE_URL"],
            "no store.* nor own"
        );
    }

    #[test]
    fn store_paths_name_request_and_path() {
        let yaml = "requests:\n  - name: create-charge\n    url: https://api.example.com\n    store:\n      CHARGE_ID: body.id\n";
        let got = store_paths(yaml);
        assert_eq!(
            got["CHARGE_ID"],
            (Some("create-charge".to_string()), "body.id".to_string())
        );
    }

    #[test]
    fn environment_rows_follow_resolution_without_reading_secrets() {
        let d = tempfile::tempdir().unwrap();
        fs::write(
            d.path().join(".env"),
            "BASE_URL=https://api.example.com\nCUSTOMER_EMAIL=test@example.com\n",
        )
        .unwrap();
        fs::write(
            d.path().join(".env.staging"),
            "BASE_URL=https://staging.api.example.com\nSECRET_PLAIN=oops\n",
        )
        .unwrap();
        let marked = vec!["API_TOKEN".to_string()];
        let keychain = vec!["API_TOKEN".to_string(), "NOT_SECRET".to_string()];
        let referenced = vec![
            "ACCOUNT_ID".to_string(),
            "BASE_URL".to_string(),
            "SHELL_ONLY".to_string(),
        ];
        let captured = vec![CapturedVar {
            name: "CHARGE_ID".into(),
            request: Some("create-charge".into()),
            path: Some("body.id".into()),
        }];
        let process = |k: &str| (k == "SHELL_ONLY").then(|| "sh".to_string());
        let rows = environment_rows(
            d.path(),
            "staging",
            &marked,
            &keychain,
            &referenced,
            &captured,
            &process,
        );
        let got: Vec<(&str, Option<&str>, &RowSource)> = rows
            .iter()
            .map(|r| (r.name.as_str(), r.value.as_deref(), &r.source))
            .collect();
        let staging = RowSource::File(d.path().join(".env.staging"));
        let default = RowSource::File(d.path().join(".env"));
        assert_eq!(
            got,
            [
                (
                    "BASE_URL",
                    Some("https://staging.api.example.com"),
                    &staging
                ),
                ("CUSTOMER_EMAIL", Some("test@example.com"), &default),
                ("SECRET_PLAIN", None, &staging),
                ("API_TOKEN", None, &RowSource::Keychain),
                ("ACCOUNT_ID", None, &RowSource::NotSet),
                ("SHELL_ONLY", Some("sh"), &RowSource::Process),
                (
                    "CHARGE_ID",
                    None,
                    &RowSource::Captured {
                        request: Some("create-charge".into()),
                        path: Some("body.id".into())
                    }
                ),
            ]
        );
        assert!(rows[2].secret && rows[3].secret && !rows[0].secret);
    }

    #[test]
    fn remove_env_var_at_owned_files_only() {
        let d = tempfile::tempdir().unwrap();
        for f in [".env", ".env.local", ".env.staging.local"] {
            fs::write(d.path().join(f), "A=1\nB=2\n").unwrap();
        }
        remove_env_var_at(d.path(), &d.path().join(".env"), "A").unwrap();
        assert_eq!(fs::read_to_string(d.path().join(".env")).unwrap(), "B=2\n");
        for f in [".env.local", ".env.staging.local"] {
            assert!(remove_env_var_at(d.path(), &d.path().join(f), "A").is_err());
            assert_eq!(fs::read_to_string(d.path().join(f)).unwrap(), "A=1\nB=2\n");
        }
        let elsewhere = tempfile::tempdir().unwrap();
        assert!(remove_env_var_at(d.path(), &elsewhere.path().join(".env"), "B").is_err());
        assert_eq!(file_readers(".env").as_deref(), Some(""));
        assert_eq!(file_readers(".env.local").as_deref(), Some(""));
        assert_eq!(file_readers(".env.staging").as_deref(), Some("staging"));
        assert_eq!(
            file_readers(".env.staging.local").as_deref(),
            Some("staging")
        );
        assert!(name_in_env_files(d.path(), "Default", "B"));
        assert!(!name_in_env_files(d.path(), "Default", "C"));
    }

    #[cfg(unix)]
    #[test]
    fn symlinked_local_override_and_permissions() {
        use std::os::unix::fs::PermissionsExt;
        let d = tempfile::tempdir().unwrap();
        let local = d.path().join(".env.staging.local");
        fs::write(&local, "A=1\n").unwrap();
        std::os::unix::fs::symlink(&local, d.path().join(".env.staging")).unwrap();
        assert!(write_env_var(d.path(), "staging", "A", "2").is_err());
        assert_eq!(fs::read_to_string(&local).unwrap(), "A=1\n");
        let own = d.path().join(".env.qa");
        fs::write(&own, "A=1\n").unwrap();
        fs::set_permissions(&own, fs::Permissions::from_mode(0o600)).unwrap();
        write_env_var(d.path(), "qa", "A", "2").unwrap();
        assert_eq!(
            fs::metadata(&own).unwrap().permissions().mode() & 0o777,
            0o600
        );
    }

    #[test]
    fn keychain_rows_flag_a_plain_copy() {
        let d = tempfile::tempdir().unwrap();
        fs::write(d.path().join(".env"), "API_KEY=plain\n").unwrap();
        let marked = vec!["API_KEY".to_string()];
        let rows = environment_rows(d.path(), "staging", &marked, &marked, &[], &[], &|_| None);
        assert_eq!(rows[0].source, RowSource::Keychain);
        assert_eq!(rows[0].plain_copy, Some(d.path().join(".env")));
    }
}
