//! What the Import dialog writes: request files from curl commands, and
//! the YAML files of another folder copied into the collection. Nothing
//! here overwrites a file; a name that is taken gets `-2`, `-3`, ….

use super::templates::{FromCurl, curl_to_ir, file_from_ir, file_name_for, request_name_for};
use curl_runner_core::convert::{Auth, CurlRunnerIr, JsMap};
use curl_runner_core::security::redact::is_sensitive_header;
use curl_runner_core::yaml_text::scalar_text;
use std::collections::HashSet;
use std::io::{self, Write as _};
use std::path::{Path, PathBuf};

/// The comment an imported request file starts with.
pub const IMPORTED_HEADER: &str = "# Imported from curl\n";

/// One pasted command and what it converts to.
#[derive(Debug, Clone, PartialEq)]
pub struct CurlImport {
    pub command: String,
    /// The request's name, from its method and path.
    pub name: String,
    pub converted: Result<FromCurl, String>,
    /// The secret values taken out of the file, by variable.
    pub extracted: Vec<Extracted>,
    /// The URL's origin became `${BASE_URL}`.
    pub base_url: bool,
}

/// A secret value moved out of an imported request into a variable. Its
/// `Debug` never shows the value.
#[derive(Clone, PartialEq, Eq)]
pub struct Extracted {
    pub var: String,
    pub value: String,
    /// Where it was: `header Authorization`, `query token`, ….
    pub place: String,
}

impl std::fmt::Debug for Extracted {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Extracted")
            .field("var", &self.var)
            .field("place", &self.place)
            .finish_non_exhaustive()
    }
}

/// What an import can look up: the environment's `BASE_URL`, and the value
/// a variable already has (Keychain, process env, `.env`), so an extracted
/// value never takes over a name that holds something else.
pub struct ImportEnv<'a> {
    pub base_url: Option<&'a str>,
    pub known: &'a dyn Fn(&str) -> Option<String>,
}

/// Converts each command in `text` (see
/// [`split_curl_commands`](super::templates::split_curl_commands)), with
/// its secrets moved into variables and its origin into `${BASE_URL}`
/// when that is where `BASE_URL` points.
pub fn convert_commands(text: &str, env: &ImportEnv) -> Vec<CurlImport> {
    // Values extracted so far: the same value keeps its variable across
    // commands, a different one gets the next name.
    let mut taken: Vec<Extracted> = Vec::new();
    super::templates::split_curl_commands(text)
        .into_iter()
        .map(|command| {
            let ir = match curl_to_ir(&command) {
                Ok(ir) => ir,
                Err(e) => {
                    return CurlImport {
                        command,
                        name: String::new(),
                        converted: Err(e),
                        extracted: Vec::new(),
                        base_url: false,
                    };
                }
            };
            let known = |name: &str| {
                taken
                    .iter()
                    .find(|e| e.var == name)
                    .map(|e| e.value.clone())
                    .or_else(|| (env.known)(name))
            };
            let (mut ir, extracted) = scrub_secrets(ir, &known);
            let base_url = env.base_url.is_some_and(|b| use_base_url(&mut ir, b));
            let method = if ir.method.is_empty() {
                "GET"
            } else {
                &ir.method
            };
            let name = request_name_for(method, &ir.url);
            let mut converted = file_from_ir(ir, &name);
            // Belt and braces: `scrub_secrets` already cleaned the warnings
            // the `# Warning:` comments come from; only the file-body note's
            // comment, so a URL, a body or another note that happens to
            // contain the value is never touched.
            converted.yaml = scrub_comments(&converted.yaml, &extracted);
            converted.yaml = unquote_references(&converted.yaml);
            converted.yaml.insert_str(0, IMPORTED_HEADER);
            for e in &extracted {
                if !taken.iter().any(|t| t.var == e.var) {
                    taken.push(e.clone());
                }
            }
            CurlImport {
                command,
                name,
                converted: Ok(converted),
                extracted,
                base_url,
            }
        })
        .collect()
}

/// The file-body note's comment line in `yaml` with each extracted value
/// as its `${VAR}`.
fn scrub_comments(yaml: &str, extracted: &[Extracted]) -> String {
    yaml.split_inclusive('\n')
        .map(|line| {
            let comment = line.trim_start();
            if comment.starts_with('#') && comment.contains(BODY_FILE_NOTE) {
                scrub_text(line, extracted)
            } else {
                line.to_string()
            }
        })
        .collect()
}

/// `text` with every extracted value as its `${VAR}`, longest first; only
/// whole tokens (between the start or end, whitespace, quotes, `=`, `:`,
/// `@` or `/`), never inside a `${…}` already there. A backstop for the one
/// warning that quotes the command (the body file's name, see
/// [`safe_warning`]): elsewhere it would turn words into references.
fn scrub_text(text: &str, extracted: &[Extracted]) -> String {
    let boundary = |c: char| c.is_whitespace() || "\"'=:@/".contains(c);
    let mut by_len: Vec<&Extracted> = extracted.iter().filter(|e| !e.value.is_empty()).collect();
    by_len.sort_by_key(|e| std::cmp::Reverse(e.value.len()));
    let mut s = text.to_string();
    for e in by_len {
        let var = format!("${{{}}}", e.var);
        let mut out = String::with_capacity(s.len());
        let mut at = 0;
        while let Some(found) = s[at..].find(&e.value) {
            let (start, end) = (at + found, at + found + e.value.len());
            let whole = s[..start].chars().next_back().is_none_or(boundary)
                && s[end..].chars().next().is_none_or(boundary);
            let in_reference = s[..start]
                .rfind("${")
                .is_some_and(|open| !s[open..start].contains('}'));
            out.push_str(&s[at..start]);
            out.push_str(if whole && !in_reference {
                &var
            } else {
                &e.value
            });
            at = end;
        }
        out.push_str(&s[at..]);
        s = out;
    }
    s
}

const UNSUPPORTED: &str = "Unsupported curl flag: ";

/// The only warning that quotes the command: the body file it names.
const BODY_FILE_NOTE: &str = "Body references a file (";

/// The converter's warnings that quote nothing from the command, kept as
/// they are.
const STATIC_WARNINGS: [&str; 4] = [
    "Flag --compressed has no YAML equivalent; curl handles decompression natively",
    "--retry-delay 0 means curl's default backoff (1s, 2s, 4s...); converted as such",
    "--retry-delay has no effect without --retry; ignored",
    CERT_PASSWORD_WARNING,
];

const CERT_PASSWORD_WARNING: &str =
    "The --cert password was left out of the file; set it up outside the request.";

/// The flag of a token as typed: `--proxy-user`, and for an attached short
/// form (`-Uuser:pw`) its first two characters.
fn flag_name(token: &str) -> &str {
    if token.starts_with("--") {
        token.split('=').next().unwrap_or(token)
    } else if token.starts_with('-') && token.chars().count() > 2 {
        let end = token.char_indices().nth(2).map_or(token.len(), |(i, _)| i);
        &token[..end]
    } else {
        token
    }
}

/// A converter warning that names only what it must: an unsupported flag
/// keeps its name, never its value; the cookie warnings drop the cookie;
/// the file-body note names the file (it is the body's text anyway).
/// Anything else that may quote the command is replaced by a generic note
/// (default deny). `None`: drop it (`--oauth2-bearer`, carried over).
fn safe_warning(w: &str, oauth_carried: bool) -> Option<String> {
    if let Some(rest) = w.strip_prefix(UNSUPPORTED) {
        let token = rest.split(' ').next().unwrap_or("");
        let flag = flag_name(token);
        if oauth_carried && flag == "--oauth2-bearer" {
            return None;
        }
        let valued = rest.len() > token.len() || flag.len() < token.len();
        let hint = [
            "(expected a whole number); ignored",
            "(expected whole seconds); ignored",
        ]
        .into_iter()
        .find(|h| rest.ends_with(h));
        return Some(match (valued, hint) {
            (_, Some(hint)) => format!("{UNSUPPORTED}{flag} (value omitted) {hint}"),
            (true, None) => format!("{UNSUPPORTED}{flag} (value omitted)"),
            (false, None) => format!("{UNSUPPORTED}{flag}"),
        });
    }
    if w.starts_with("Cookie flag (-b ") {
        return Some("Cookie flag (-b) stored as header; manual review recommended".into());
    }
    if w.starts_with("Cookie jar flag (-c ") {
        return Some("Cookie jar flag (-c) has no YAML equivalent".into());
    }
    if w.starts_with(BODY_FILE_NOTE) || STATIC_WARNINGS.contains(&w) {
        return Some(w.to_string());
    }
    Some("Part of the command was not converted (details omitted).".into())
}

/// `scheme://user:pass@host…` split into the part before the userinfo,
/// the user, the password and the rest (`host…`). Without a scheme the
/// userinfo may start the text (`user:pass@proxy:8080`).
fn split_userinfo(url: &str) -> Option<(&str, &str, Option<&str>, &str)> {
    let start = url.find("://").map_or(0, |i| i + 3);
    let authority_end = url[start..]
        .find(['/', '?', '#'])
        .map_or(url.len(), |i| start + i);
    let at = url[start..authority_end].rfind('@')? + start;
    let userinfo = &url[start..at];
    let (user, pass) = match userinfo.split_once(':') {
        Some((u, p)) => (u, Some(p)),
        None => (userinfo, None),
    };
    Some((&url[..start], user, pass, &url[at + 1..]))
}

/// Where `--cert file[:password]` splits: the last `:` that isn't escaped
/// (`\:`), not a drive letter's, and never in a `pkcs11:` URI.
fn cert_password_at(cert: &str) -> Option<usize> {
    if cert
        .get(..7)
        .is_some_and(|p| p.eq_ignore_ascii_case("pkcs11:"))
    {
        return None;
    }
    cert.char_indices()
        .filter(|&(i, c)| c == ':' && i != 1 && !cert[..i].ends_with('\\'))
        .map(|(i, _)| i)
        .next_back()
}

/// Query parameters whose values are secrets.
const SECRET_PARAMS: [&str; 9] = [
    "token",
    "access_token",
    "api_key",
    "apikey",
    "key",
    "secret",
    "password",
    "sig",
    "signature",
];

/// `ir` with its secret values (bearer token, basic credentials, sensitive
/// headers as the CLI's redactor judges them, secret query parameters)
/// replaced by `${VAR}` references, and what was taken out. A bearer token
/// becomes an `Authorization: Bearer ${API_TOKEN}` header. Values that
/// already reference a variable are left alone. `known` gives a name's
/// current value: a name holding another value is skipped for `_2`, `_3`.
pub fn scrub_secrets(
    mut ir: CurlRunnerIr,
    known: &dyn Fn(&str) -> Option<String>,
) -> (CurlRunnerIr, Vec<Extracted>) {
    let mut out: Vec<Extracted> = Vec::new();
    let mut take = |base: &str, value: &str, place: String| -> String {
        let var = pick_var(base, value, known, &out);
        if !out.iter().any(|e| e.var == var) {
            out.push(Extracted {
                var: var.clone(),
                value: value.to_string(),
                place,
            });
        }
        format!("${{{var}}}")
    };
    // `--oauth2-bearer`: a bearer token the converter leaves out; carried
    // over as the `Authorization` header unless the command has one.
    let oauth = ir
        .unsupported_flags
        .iter()
        .find(|f| f.flag == "--oauth2-bearer")
        .and_then(|f| f.value.clone())
        .filter(|v| !v.is_empty());
    let mut has_auth = ir.auth.is_some()
        || ir
            .headers
            .entries()
            .iter()
            .any(|(k, _)| k.eq_ignore_ascii_case("authorization"));
    // `https://user:pass@host`: curl sends it as basic auth, as `-u` does
    // (which wins when both are given); the URL keeps no userinfo.
    if let Some((before, user, pass, rest)) = split_userinfo(&ir.url) {
        let (user, pass) = (user.to_string(), pass.map(str::to_string));
        ir.url = format!("{before}{rest}");
        if !has_auth && !user.is_empty() {
            ir.auth = Some(Auth {
                kind: "basic".into(),
                username: Some(user),
                password: pass,
                token: None,
            });
            has_auth = true;
        }
    }
    let oauth_carried = oauth.is_some() && !has_auth;
    let mut warnings: Vec<String> = std::mem::take(&mut ir.warnings)
        .iter()
        .filter_map(|w| safe_warning(w, oauth_carried))
        .collect();
    for f in &mut ir.unsupported_flags {
        f.value = None;
    }
    // `--cert file:password`: the file stays, the password doesn't (YAML
    // has no field for it).
    if let Some(cert) = ir.ssl.as_mut().and_then(|s| s.cert.as_mut())
        && let Some(at) = cert_password_at(cert)
    {
        cert.truncate(at);
        warnings.push(CERT_PASSWORD_WARNING.into());
    }
    // `-x http://user:pass@proxy/?token=…`: the credentials and secret
    // query values become variables.
    if let Some(proxy) = ir.proxy.take() {
        ir.proxy = Some(scrub_url(&proxy, "PROXY", true, &mut take));
    }
    let mut headers = JsMap::new();
    if let Some(token) = oauth.filter(|_| oauth_carried) {
        let value = if is_reference(&token) {
            token
        } else {
            take("API_TOKEN", &token, "--oauth2-bearer".into())
        };
        headers.insert("Authorization", format!("Bearer {value}"));
    }
    if let Some(auth) = ir.auth.take() {
        match auth.kind.as_str() {
            "bearer" => {
                let token = auth.token.unwrap_or_default();
                let value = if token.is_empty() || is_reference(&token) {
                    token
                } else {
                    take("API_TOKEN", &token, "bearer token".into())
                };
                headers.insert("Authorization", format!("Bearer {value}"));
            }
            _ => {
                let mut auth = auth;
                for (field, base) in [
                    (&mut auth.username, "API_USER"),
                    (&mut auth.password, "API_PASSWORD"),
                ] {
                    if let Some(v) = field.as_mut().filter(|v| !v.is_empty() && !is_reference(v)) {
                        *v = take(base, v, format!("basic auth {}", base[4..].to_lowercase()));
                    }
                }
                ir.auth = Some(auth);
            }
        }
    }
    for (name, value) in ir.headers.entries() {
        let value = if name.eq_ignore_ascii_case("referer") {
            // A page address: its userinfo and secret query values.
            scrub_url(value, "REFERER", false, &mut take)
        } else if value.is_empty() || is_reference(value) || !secret_header(name) {
            value.clone()
        } else if name.eq_ignore_ascii_case("authorization") {
            // Keep the scheme: `Token abc` → `Token ${API_TOKEN}`.
            match value.split_once(' ') {
                Some((scheme, cred)) => format!(
                    "{scheme} {}",
                    take("API_TOKEN", cred.trim(), format!("header {name}"))
                ),
                None => take("API_TOKEN", value, format!("header {name}")),
            }
        } else {
            take(&header_var(name), value, format!("header {name}"))
        };
        headers.insert(name.clone(), value);
    }
    ir.headers = headers;
    if let Some(params) = ir.params.take() {
        let mut kept = JsMap::new();
        for (name, value) in params.entries() {
            let secret = SECRET_PARAMS.iter().any(|p| p.eq_ignore_ascii_case(name));
            let value = if secret && !value.is_empty() && !is_reference(value) {
                take(&param_var(name), value, format!("query {name}"))
            } else {
                value.clone()
            };
            kept.insert(name.clone(), value);
        }
        ir.params = Some(kept);
    }
    // Backstop for the one warning that still quotes the command.
    ir.warnings = warnings
        .into_iter()
        .map(|w| {
            if w.starts_with(BODY_FILE_NOTE) {
                scrub_text(&w, &out)
            } else {
                w
            }
        })
        .collect();
    (ir, out)
}

/// `url` with its userinfo (`{what}_USER`, `{what}_PASSWORD`) and secret
/// query values ([`SECRET_PARAMS`]) as variables through `take`; parts
/// that already reference one stay. `bare_userinfo`: `user:pass@host`
/// without a scheme counts (a proxy); else only after `scheme://`.
fn scrub_url(
    url: &str,
    what: &str,
    bare_userinfo: bool,
    take: &mut dyn FnMut(&str, &str, String) -> String,
) -> String {
    let label = what.to_lowercase();
    let mut url = url.to_string();
    if (bare_userinfo || url.contains("://"))
        && let Some((before, user, pass, rest)) = split_userinfo(&url)
    {
        let user = if user.is_empty() || is_reference(user) {
            user.to_string()
        } else {
            take(&format!("{what}_USER"), user, format!("{label} user"))
        };
        let pass = pass.map(|p| {
            if p.is_empty() || is_reference(p) {
                p.to_string()
            } else {
                take(&format!("{what}_PASSWORD"), p, format!("{label} password"))
            }
        });
        url = match pass {
            Some(p) => format!("{before}{user}:{p}@{rest}"),
            None => format!("{before}{user}@{rest}"),
        };
    }
    let Some(q) = url.find('?') else { return url };
    let end = url[q..].find('#').map_or(url.len(), |i| q + i);
    let query: Vec<String> = url[q + 1..end]
        .split('&')
        .map(|pair| match pair.split_once('=') {
            Some((k, v))
                if SECRET_PARAMS.iter().any(|p| p.eq_ignore_ascii_case(k))
                    && !v.is_empty()
                    && !is_reference(v) =>
            {
                format!(
                    "{k}={}",
                    take(&param_var(k), v, format!("{label} query {k}"))
                )
            }
            _ => pair.to_string(),
        })
        .collect();
    format!("{}?{}{}", &url[..q], query.join("&"), &url[end..])
}

/// `${BASE_URL}` in place of the URL's start when `base_url` (the
/// environment's value) is that start: the same origin, and the same path
/// prefix if it has one. Returns whether it replaced it.
pub fn use_base_url(ir: &mut CurlRunnerIr, base_url: &str) -> bool {
    let base = base_url.trim().trim_end_matches('/');
    if !base.contains("://") || is_reference(&ir.url) {
        return false;
    }
    let Some(rest) = ir.url.strip_prefix(base) else {
        return false;
    };
    if !(rest.is_empty() || rest.starts_with(['/', '?', '#'])) {
        return false;
    }
    ir.url = format!("${{BASE_URL}}{rest}");
    true
}

/// The note under the preview: which variables took the secrets and the
/// host, in the board's words.
pub fn replaced_note(vars: &[&str], base_url: bool) -> Option<String> {
    let list = |vars: &[&str]| {
        let refs: Vec<String> = vars.iter().map(|v| format!("${{{v}}}")).collect();
        match refs.as_slice() {
            [] => String::new(),
            [one] => one.clone(),
            [rest @ .., last] => format!("{} and {last}", rest.join(", ")),
        }
    };
    let host = " the host with ${BASE_URL}";
    match (vars.len(), base_url) {
        (0, false) => None,
        (0, true) => Some("The host was replaced with ${BASE_URL}.".to_string()),
        (n, _) => Some(format!(
            "The live {} replaced with {}{}. Nothing secret is saved in the file.",
            if n == 1 { "key was" } else { "keys were" },
            list(vars),
            if base_url {
                format!(" and{host}")
            } else {
                String::new()
            },
        )),
    }
}

/// `key: "${BASE_URL}/v1/x"` written plain (`key: ${BASE_URL}/v1/x`), as
/// the board shows it: only double-quoted values with a `${…}` reference
/// whose plain form reads back as the same string (core's `scalar_text`).
fn unquote_references(yaml: &str) -> String {
    let plain =
        |v: &str| v.contains("${") && !v.contains(['\\', '"']) && scalar_text(v, false, false) == v;
    let mut out = String::with_capacity(yaml.len());
    for line in yaml.split_inclusive('\n') {
        let body = line.trim_end_matches('\n');
        let unquoted = body.split_once(": \"").and_then(|(key, rest)| {
            let value = rest.strip_suffix('"')?;
            (!key.trim_start().starts_with('#') && plain(value)).then(|| format!("{key}: {value}"))
        });
        match unquoted {
            Some(l) => {
                out.push_str(&l);
                out.push_str(&line[body.len()..]);
            }
            None => out.push_str(line),
        }
    }
    out
}

/// `${NAME}` somewhere in the value: it already points at a variable.
fn is_reference(value: &str) -> bool {
    value.contains("${")
}

/// A secret header: the CLI redactor's names, plus `X-…-Key`.
fn secret_header(name: &str) -> bool {
    let lower = name.to_ascii_lowercase();
    is_sensitive_header(name) || lower.ends_with("-key") || lower.ends_with("_key")
}

/// `X-Auth-Token` → `AUTH_TOKEN`, `x-api-key` → `API_KEY`.
fn header_var(name: &str) -> String {
    let lower = name.to_ascii_lowercase();
    screaming(lower.strip_prefix("x-").unwrap_or(&lower))
}

/// `access_token` → `ACCESS_TOKEN`; a bare key is `API_KEY`.
fn param_var(name: &str) -> String {
    match name.to_ascii_lowercase().as_str() {
        "key" | "apikey" | "api_key" => "API_KEY".into(),
        other => screaming(other),
    }
}

fn screaming(s: &str) -> String {
    let mut out = String::new();
    for c in s.chars() {
        if c.is_ascii_alphanumeric() {
            out.push(c.to_ascii_uppercase());
        } else if !out.is_empty() && !out.ends_with('_') {
            out.push('_');
        }
    }
    let out = out.trim_end_matches('_');
    if out.is_empty() {
        "SECRET".into()
    } else {
        out.to_string()
    }
}

/// `base`, or `base_2`, `base_3`, …: the first name that is free or
/// already holds `value`.
fn pick_var(
    base: &str,
    value: &str,
    known: &dyn Fn(&str) -> Option<String>,
    out: &[Extracted],
) -> String {
    (1..)
        .map(|n| {
            if n == 1 {
                base.to_string()
            } else {
                format!("{base}_{n}")
            }
        })
        .find(|var| {
            let held = out
                .iter()
                .find(|e| &e.var == var)
                .map(|e| e.value.clone())
                .or_else(|| known(var));
            held.is_none_or(|v| v == value)
        })
        .expect("an unbounded range finds a free name")
}

/// `path` if `taken` says it is free, else `stem-2.ext`, `stem-3.ext`, ….
pub fn unique_path(path: &Path, taken: impl Fn(&Path) -> bool) -> PathBuf {
    if !taken(path) {
        return path.to_path_buf();
    }
    let stem = path
        .file_stem()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_default();
    let ext = path
        .extension()
        .map(|e| format!(".{}", e.to_string_lossy()))
        .unwrap_or_default();
    (2..)
        .map(|n| path.with_file_name(format!("{stem}-{n}{ext}")))
        .find(|p| !taken(p))
        .expect("an unbounded range finds a free name")
}

/// Where each convertible command's file goes, relative to `root`: in
/// `folder`, named after the request, never over an existing file or
/// another command's (`root: None`: nothing on disk yet). `None` for a
/// command that failed to convert.
pub fn curl_targets(
    root: Option<&Path>,
    folder: &Path,
    imports: &[CurlImport],
) -> Vec<Option<PathBuf>> {
    let mut planned = HashSet::new();
    imports
        .iter()
        .map(|i| {
            i.converted.as_ref().ok()?;
            let rel = unique_path(&folder.join(file_name_for(&i.name)), |p| {
                planned.contains(p) || on_disk(root, p)
            });
            planned.insert(rel.clone());
            Some(rel)
        })
        .collect()
}

/// `rel` exists under `root` (`None`: no folder yet, nothing to clash with).
fn on_disk(root: Option<&Path>, rel: &Path) -> bool {
    root.is_some_and(|r| r.join(rel).symlink_metadata().is_ok())
}

/// One file the Folder of YAML import copies.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CopyPlan {
    /// The file, absolute.
    pub from: PathBuf,
    /// Its path inside the picked folder.
    pub rel: PathBuf,
    /// Where it goes, relative to the collection root.
    pub to: PathBuf,
}

impl CopyPlan {
    /// Copied under another name, because the name was taken.
    pub fn renamed(&self, folder: &Path) -> bool {
        self.to != folder.join(&self.rel)
    }
}

/// The `*.yaml` / `*.yml` files under `source`, depth first in name order,
/// skipping hidden entries, symlinks and a top-level `curl-runner.yaml`, and
/// where each goes under `root/folder`, keeping subfolders.
pub fn plan_yaml_copy(
    source: &Path,
    root: Option<&Path>,
    folder: &Path,
) -> io::Result<Vec<CopyPlan>> {
    let mut files = Vec::new();
    collect_yaml(source, Path::new(""), &mut files)?;
    let mut planned = HashSet::new();
    Ok(files
        .into_iter()
        .map(|rel| {
            let to = unique_path(&folder.join(&rel), |p| {
                planned.contains(p) || on_disk(root, p)
            });
            planned.insert(to.clone());
            CopyPlan {
                from: source.join(&rel),
                rel,
                to,
            }
        })
        .collect())
}

fn collect_yaml(dir: &Path, rel: &Path, out: &mut Vec<PathBuf>) -> io::Result<()> {
    let mut entries: Vec<_> = std::fs::read_dir(dir)?.collect::<io::Result<_>>()?;
    entries.sort_by_key(|e| e.file_name());
    for entry in entries {
        let name = entry.file_name();
        let name_str = name.to_string_lossy();
        if name_str.starts_with('.') {
            continue;
        }
        let kind = entry.file_type()?;
        let path = rel.join(&name);
        if kind.is_dir() {
            collect_yaml(&entry.path(), &path, out)?;
        } else if kind.is_file()
            && (name_str.ends_with(".yaml") || name_str.ends_with(".yml"))
            && !(rel.as_os_str().is_empty()
                && matches!(name_str.as_ref(), "curl-runner.yaml" | "curl-runner.yml"))
        {
            out.push(path);
        }
    }
    Ok(())
}

/// Whether `yaml` parses as the runner reads it; an import never writes
/// a file that doesn't.
pub fn reads_as_yaml(yaml: &str) -> bool {
    curl_runner_core::parser::parse_raw_stream(yaml).is_ok()
}

/// Copies the planned files into `root`, never over an existing file, in
/// order: how many were copied, and the failure that stopped the rest.
pub fn copy_planned(root: &Path, plan: &[CopyPlan]) -> (usize, Option<io::Error>) {
    for (n, file) in plan.iter().enumerate() {
        let copied =
            std::fs::read(&file.from).and_then(|text| write_new(&root.join(&file.to), &text));
        if let Err(e) = copied {
            let e = io::Error::new(e.kind(), format!("{}: {e}", file.to.display()));
            return (n, Some(e));
        }
    }
    (plan.len(), None)
}

/// Writes a new file, creating its folders; an existing one fails with
/// `AlreadyExists`.
pub fn write_new(path: &Path, bytes: &[u8]) -> io::Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)?
        .write_all(bytes)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::diagnostics::diagnose;

    fn write(root: &Path, rel: &str, text: &str) {
        let p = root.join(rel);
        std::fs::create_dir_all(p.parent().unwrap()).unwrap();
        std::fs::write(p, text).unwrap();
    }

    #[test]
    fn copy_folder_of_yaml_skips_hidden_and_non_yaml() {
        let src = tempfile::tempdir().unwrap();
        let s = src.path();
        for f in [
            "curl-runner.yaml",
            "a.yaml",
            "b.yml",
            "notes.md",
            ".hidden.yaml",
            ".git/config.yaml",
            "nested/c.yaml",
            "nested/curl-runner.yaml",
            "nested/.d.yaml",
        ] {
            write(s, f, "request:\n  url: https://api.example.com\n");
        }
        #[cfg(unix)]
        std::os::unix::fs::symlink(s.join("a.yaml"), s.join("link.yaml")).unwrap();

        let dst = tempfile::tempdir().unwrap();
        let d = dst.path();
        write(d, "imported/a.yaml", "kept\n");
        let plan = plan_yaml_copy(s, Some(d), Path::new("imported")).unwrap();
        // Paths, not their text: Windows separators differ.
        let summary: Vec<(PathBuf, PathBuf)> =
            plan.iter().map(|p| (p.rel.clone(), p.to.clone())).collect();
        let pair = |rel: &str, to: &str| -> (PathBuf, PathBuf) {
            (rel.split('/').collect(), to.split('/').collect())
        };
        assert_eq!(
            summary,
            [
                pair("a.yaml", "imported/a-2.yaml"),
                pair("b.yml", "imported/b.yml"),
                pair("nested/c.yaml", "imported/nested/c.yaml"),
                pair(
                    "nested/curl-runner.yaml",
                    "imported/nested/curl-runner.yaml"
                ),
            ]
        );
        assert!(plan[0].renamed(Path::new("imported")));
        assert!(!plan[1].renamed(Path::new("imported")));

        let (copied, error) = copy_planned(d, &plan);
        assert_eq!((copied, error.map(|e| e.kind())), (4, None));
        assert_eq!(
            std::fs::read_to_string(d.join("imported/a.yaml")).unwrap(),
            "kept\n"
        );
        assert!(d.join("imported/a-2.yaml").is_file());
        assert!(d.join("imported/nested/c.yaml").is_file());
        // Copying the same plan again never overwrites.
        let (copied, again) = copy_planned(d, &plan);
        assert_eq!(copied, 0);
        assert_eq!(again.unwrap().kind(), io::ErrorKind::AlreadyExists);
    }

    #[test]
    fn unique_paths_count_up() {
        let taken = [PathBuf::from("x/a.yaml"), PathBuf::from("x/a-2.yaml")];
        let free = |p: &Path| taken.iter().any(|t| t == p);
        assert_eq!(
            unique_path(Path::new("x/a.yaml"), free),
            Path::new("x/a-3.yaml")
        );
        assert_eq!(
            unique_path(Path::new("x/b.yaml"), free),
            Path::new("x/b.yaml")
        );
    }

    #[test]
    fn commands_convert_with_derived_names_and_unique_files() {
        let text = "curl -X POST https://api.example.com/v1/customers -d name=Ada\n\n\
                    curl https://api.example.com/v1/customers/cus_8f2k1Abc\n\
                    curl -X POST https://api.example.com/v1/customers -d name=Bob\n\n\
                    hello world";
        let imports = convert_commands(text, &NO_ENV);
        assert_eq!(imports.len(), 4);
        let names: Vec<&str> = imports.iter().map(|i| i.name.as_str()).collect();
        assert_eq!(
            names,
            ["create-customer", "get-customer", "create-customer", ""]
        );
        let first = imports[0].converted.as_ref().unwrap();
        assert!(first.yaml.starts_with(IMPORTED_HEADER), "{}", first.yaml);
        assert!(
            first.yaml.contains("name: create-customer"),
            "{}",
            first.yaml
        );
        assert_eq!(diagnose(&first.yaml), []);
        assert!(imports[3].converted.is_err());

        let root = tempfile::tempdir().unwrap();
        write(root.path(), "customers/get-customer.yaml", "x\n");
        let targets = curl_targets(Some(root.path()), Path::new("customers"), &imports);
        assert_eq!(
            targets,
            [
                Some(PathBuf::from("customers/create-customer.yaml")),
                Some(PathBuf::from("customers/get-customer-2.yaml")),
                Some(PathBuf::from("customers/create-customer-2.yaml")),
                None,
            ]
        );
    }

    /// No errors: the file reads back (style hints are fine).
    fn assert_parses(yaml: &str) {
        let errors: Vec<_> = diagnose(yaml)
            .into_iter()
            .filter(|d| d.severity == curl_runner_core::lint::Severity::Error)
            .collect();
        assert!(errors.is_empty(), "{errors:?}\n{yaml}");
    }

    const NO_ENV: ImportEnv = ImportEnv {
        base_url: None,
        known: &|_| None,
    };

    const LIVE_KEY: &str = concat!("sk_live", "_51HxAbCdEfGhIjKlMnOpQrSt");

    fn one(command: &str, env: &ImportEnv) -> (String, Vec<Extracted>, bool) {
        let mut imports = convert_commands(command, env);
        assert_eq!(imports.len(), 1);
        let i = imports.remove(0);
        (i.converted.unwrap().yaml, i.extracted, i.base_url)
    }

    #[test]
    fn the_board_command_keeps_no_secret() {
        let command = format!(
            "curl -X POST https://api.example.com/v1/customers \\\n  -H \"Authorization: Bearer {LIVE_KEY}\" \\\n  -H \"Content-Type: application/json\" \\\n  -d @customer.json \\\n  --retry 3 \\\n  --max-time 10"
        );
        let env = ImportEnv {
            base_url: Some("https://api.example.com/"),
            known: &|_| None,
        };
        let (yaml, extracted, base_url) = one(&command, &env);
        assert!(base_url);
        for line in [
            "  name: create-customer",
            "  url: ${BASE_URL}/v1/customers",
            "  headers:",
            "    Authorization: Bearer ${API_TOKEN}",
            "    Content-Type: application/json",
        ] {
            assert!(yaml.lines().any(|l| l == line), "{line:?} in\n{yaml}");
        }
        assert!(!yaml.contains(LIVE_KEY), "{yaml}");
        assert!(!yaml.contains("sk_live"), "{yaml}");
        assert!(!yaml.contains("auth:"), "{yaml}");
        assert_eq!(extracted.len(), 1);
        assert_eq!(extracted[0].var, "API_TOKEN");
        assert_eq!(extracted[0].value, LIVE_KEY);
        // Debug never prints the value.
        assert!(!format!("{extracted:?}").contains(LIVE_KEY));
        assert_parses(&yaml);
        assert_eq!(
            replaced_note(&["API_TOKEN"], true).unwrap(),
            "The live key was replaced with ${API_TOKEN} and the host with ${BASE_URL}. \
             Nothing secret is saved in the file."
        );
    }

    #[test]
    fn another_host_keeps_its_url() {
        let env = ImportEnv {
            base_url: Some("https://staging.example.com"),
            known: &|_| None,
        };
        let (yaml, _, base_url) = one("curl https://api.example.com/v1/charges", &env);
        assert!(!base_url);
        assert!(
            yaml.contains("url: https://api.example.com/v1/charges"),
            "{yaml}"
        );
        // A prefix that isn't a whole origin doesn't match either.
        let env = ImportEnv {
            base_url: Some("https://api.example.co"),
            known: &|_| None,
        };
        assert!(!one("curl https://api.example.com/x", &env).2);
        assert_eq!(replaced_note(&[], false), None);
    }

    #[test]
    fn query_params_and_basic_auth_move_into_variables() {
        let (yaml, extracted, _) = one(
            "curl -u admin:hunter2-long 'https://api.example.com/v1/x?access_token=tok_abcdef12&page=2&key=k_99999999'",
            &NO_ENV,
        );
        let vars: Vec<(&str, &str)> = extracted
            .iter()
            .map(|e| (e.var.as_str(), e.value.as_str()))
            .collect();
        assert_eq!(
            vars,
            [
                ("API_USER", "admin"),
                ("API_PASSWORD", "hunter2-long"),
                ("ACCESS_TOKEN", "tok_abcdef12"),
                ("API_KEY", "k_99999999"),
            ]
        );
        for secret in ["hunter2-long", "tok_abcdef12", "k_99999999"] {
            assert!(!yaml.contains(secret), "{secret} in {yaml}");
        }
        assert!(yaml.contains("${ACCESS_TOKEN}"), "{yaml}");
        assert!(yaml.contains("page"), "{yaml}");
        assert_parses(&yaml);
    }

    #[test]
    fn named_headers_and_references() {
        let (yaml, extracted, _) = one(
            "curl https://api.example.com -H 'X-Auth-Token: abcdef123456' -H 'X-API-Key: ${MY_KEY}' -H 'X-Client-Key: zzzzzzzz' -H 'Accept: text/plain'",
            &NO_ENV,
        );
        let vars: Vec<&str> = extracted.iter().map(|e| e.var.as_str()).collect();
        assert_eq!(vars, ["AUTH_TOKEN", "CLIENT_KEY"]);
        assert!(yaml.contains("    X-Api-Key: ${MY_KEY}\n"), "{yaml}");
        assert!(yaml.contains("    X-Auth-Token: ${AUTH_TOKEN}\n"), "{yaml}");
        assert_parses(&yaml);
        assert!(yaml.contains("Accept: text/plain"), "{yaml}");
        assert!(
            !yaml.contains("abcdef123456") && !yaml.contains("zzzzzzzz"),
            "{yaml}"
        );
        // An already-referenced bearer token stays as it is.
        let (yaml, extracted, _) = one(
            "curl https://api.example.com -H 'Authorization: Bearer ${API_TOKEN}'",
            &NO_ENV,
        );
        assert!(extracted.is_empty());
        assert!(
            yaml.contains("Authorization: Bearer ${API_TOKEN}"),
            "{yaml}"
        );
    }

    #[test]
    fn a_name_holding_another_value_is_not_reused() {
        let env = ImportEnv {
            base_url: None,
            known: &|n| (n == "API_TOKEN").then(|| "other-token".to_string()),
        };
        let text = "curl https://a.example.com -H 'Authorization: Bearer tok_one_111'\n\n\
                    curl https://b.example.com -H 'Authorization: Bearer tok_one_111'\n\n\
                    curl https://c.example.com -H 'Authorization: Bearer tok_two_222'";
        let vars: Vec<Vec<String>> = convert_commands(text, &env)
            .iter()
            .map(|i| i.extracted.iter().map(|e| e.var.clone()).collect())
            .collect();
        assert_eq!(vars, [["API_TOKEN_2"], ["API_TOKEN_2"], ["API_TOKEN_3"]]);
        // The value the environment already holds keeps its name.
        let env = ImportEnv {
            base_url: None,
            known: &|n| (n == "API_TOKEN").then(|| "tok_one_111".to_string()),
        };
        assert_eq!(
            convert_commands(text, &env)[0].extracted[0].var,
            "API_TOKEN"
        );
    }

    #[test]
    fn a_username_in_the_host_stays_in_the_url() {
        let (yaml, extracted, _) = one(
            "curl -u github:tok_123456789 https://github.example.com/repos/github/x",
            &NO_ENV,
        );
        assert_eq!(extracted[0].value, "github");
        assert!(
            yaml.contains("url: https://github.example.com/repos/github/x"),
            "{yaml}"
        );
        assert!(!yaml.contains("tok_123456789"), "{yaml}");
    }

    #[test]
    fn credential_flags_never_reach_the_file() {
        let cases = [
            ("--oauth2-bearer tok_oauth_9f8e7d", "tok_oauth_9f8e7d"),
            ("--proxy-user proxyuser:pw_proxy_42", "pw_proxy_42"),
            ("-U proxyuser:pw_proxy_43", "pw_proxy_43"),
            ("-E client.pem:pw_cert_44", "pw_cert_44"),
            ("--cert client.pem:pw_cert_45", "pw_cert_45"),
            ("-b session=sess_cookie_46", "sess_cookie_46"),
            ("--cookie session=sess_cookie_47", "sess_cookie_47"),
            ("--aws-sigv4 aws:amz:us-east-1:s3", "aws:amz:us-east-1:s3"),
        ];
        for (flag, secret) in cases {
            let command = format!("curl {flag} https://api.example.com/v1/x");
            let imports = convert_commands(&command, &NO_ENV);
            let i = &imports[0];
            let c = i.converted.as_ref().unwrap();
            assert!(!c.yaml.contains(secret), "{flag}:\n{}", c.yaml);
            assert!(
                c.warnings.iter().all(|w| !w.contains(secret)),
                "{flag}: {:?}",
                c.warnings
            );
            let vars: Vec<&str> = i.extracted.iter().map(|e| e.var.as_str()).collect();
            let note = replaced_note(&vars, false).unwrap_or_default();
            assert!(!note.contains(secret), "{flag}: {note}");
            assert_parses(&c.yaml);
        }
        // `--oauth2-bearer` becomes the Authorization header.
        let (yaml, extracted, _) = one(
            "curl --oauth2-bearer tok_oauth_9f8e7d https://api.example.com",
            &NO_ENV,
        );
        assert!(
            yaml.contains("Authorization: Bearer ${API_TOKEN}"),
            "{yaml}"
        );
        assert!(!yaml.contains("--oauth2-bearer"), "{yaml}");
        assert_eq!(extracted[0].value, "tok_oauth_9f8e7d");
        // The redacted flags still say what was left out.
        let (yaml, _, _) = one(
            "curl -U proxyuser:pw_proxy_43 https://api.example.com",
            &NO_ENV,
        );
        assert!(
            yaml.contains("Unsupported curl flag: -U (value omitted)"),
            "{yaml}"
        );
        let (yaml, _, _) = one(
            "curl --cert client.pem:pw_cert_45 https://api.example.com",
            &NO_ENV,
        );
        assert!(yaml.contains("cert: client.pem"), "{yaml}");
    }

    #[test]
    fn only_values_that_read_back_are_unquoted() {
        for kept in [
            r#"k: "{${A}}""#,
            r#"k: "&${A}""#,
            r#"k: "@${F}""#,
            r#"k: "- ${A}""#,
            r#"k: "${A}:""#,
            r#"k: "${A} # x""#,
        ] {
            assert_eq!(unquote_references(kept), kept);
        }
        assert_eq!(
            unquote_references(r#"  url: "${BASE_URL}/v1""#),
            "  url: ${BASE_URL}/v1"
        );
        assert!(reads_as_yaml("request:\n  url: ${BASE_URL}/v1\n"));
        assert!(!reads_as_yaml("request: [\n"));
    }

    /// The secret is nowhere: not in the file (the preview's text), the
    /// warnings or the note.
    fn assert_nowhere(command: &str, secret: &str) {
        let imports = convert_commands(command, &NO_ENV);
        let i = &imports[0];
        let c = i
            .converted
            .as_ref()
            .unwrap_or_else(|e| panic!("{command}: {e}"));
        assert!(!c.yaml.contains(secret), "{command}:\n{}", c.yaml);
        assert!(
            c.warnings.iter().all(|w| !w.contains(secret)),
            "{command}: {:?}",
            c.warnings
        );
        let vars: Vec<&str> = i.extracted.iter().map(|e| e.var.as_str()).collect();
        assert!(
            !replaced_note(&vars, i.base_url)
                .unwrap_or_default()
                .contains(secret)
        );
        assert_parses(&c.yaml);
    }

    #[test]
    fn unsupported_flags_never_show_their_values() {
        for (args, secret) in [
            ("-Uuser:pw_u_secret1", "pw_u_secret1"),
            ("-Eclient.pem:pw_e_secret2", "pw_e_secret2"),
            (
                "--proxy-header 'Proxy-Authorization: Basic abc_secret3'",
                "abc_secret3",
            ),
            ("--url-query token=abc_secret4", "abc_secret4"),
            ("--mail-auth mail_secret5", "mail_secret5"),
            ("--socks5 user:pw_socks6@h.example.com", "pw_socks6"),
        ] {
            assert_nowhere(&format!("curl https://api.example.com {args}"), secret);
        }
        let (yaml, _, _) = one(
            "curl https://api.example.com -Uuser:pw_u_secret1 --proxy-header 'X: y'",
            &NO_ENV,
        );
        assert!(
            yaml.contains("Unsupported curl flag: -U (value omitted)"),
            "{yaml}"
        );
        assert!(
            yaml.contains("Unsupported curl flag: --proxy-header (value omitted)"),
            "{yaml}"
        );
        assert_eq!(flag_name("-Eclient.pem:pw"), "-E");
        assert_eq!(flag_name("--socks5"), "--socks5");
        assert_eq!(flag_name("-k"), "-k");
        // Anything unrecognised that may quote the command says so, nothing more.
        assert_eq!(
            safe_warning("Something about sk_live_x", false).as_deref(),
            Some("Part of the command was not converted (details omitted).")
        );
        assert_eq!(
            safe_warning(
                "Unsupported curl flag: --retry abc (expected a whole number); ignored",
                false
            )
            .as_deref(),
            Some(
                "Unsupported curl flag: --retry (value omitted) (expected a whole number); ignored"
            )
        );
    }

    fn import(cmd: &str) -> (String, Vec<Extracted>) {
        let env = ImportEnv {
            base_url: None,
            known: &|_| None,
        };
        let i = convert_commands(cmd, &env).remove(0);
        (i.converted.unwrap().yaml, i.extracted)
    }

    fn vars(extracted: &[Extracted]) -> Vec<&str> {
        extracted.iter().map(|e| e.var.as_str()).collect()
    }

    #[test]
    fn a_dash_value_after_a_credential_flag_stays_its_value() {
        let (yaml, extracted) =
            import("curl https://api.example.com/x --oauth2-bearer --tok_s3cr3t1");
        assert!(!yaml.contains("tok_s3cr3t1"), "{yaml}");
        assert!(
            yaml.contains("Authorization: Bearer ${API_TOKEN}"),
            "{yaml}"
        );
        assert_eq!(extracted[0].value, "--tok_s3cr3t1");
        let (yaml, _) = import("curl --proxy-user --pw_s3cr3t2 https://api.example.com/x");
        assert!(!yaml.contains("s3cr3t2"), "{yaml}");
        assert!(yaml.contains("url: https://api.example.com/x"), "{yaml}");
        assert!(
            yaml.contains("Unsupported curl flag: --proxy-user (value omitted)"),
            "{yaml}"
        );
    }

    #[test]
    fn referer_and_proxy_urls_lose_their_credentials() {
        let (yaml, extracted) = import(
            "curl -H 'Referer: https://u:pw_ref_1@site.example.com/a?token=tok_ref_2&page=3' \
             -x 'http://proxy.example.com:8080/?key=k_px_3' https://api.example.com/x",
        );
        for secret in ["pw_ref_1", "tok_ref_2", "k_px_3"] {
            assert!(!yaml.contains(secret), "{secret}: {yaml}");
        }
        assert!(
            yaml.contains(
                "https://${REFERER_USER}:${REFERER_PASSWORD}@site.example.com/a?token=${TOKEN}&page=3"
            ),
            "{yaml}"
        );
        assert!(
            yaml.contains("http://proxy.example.com:8080/?key=${API_KEY}"),
            "{yaml}"
        );
        assert_eq!(
            vars(&extracted),
            ["API_KEY", "REFERER_USER", "REFERER_PASSWORD", "TOKEN"]
        );
        // A plain page address stays as it is.
        let (yaml, extracted) =
            import("curl -e 'https://site.example.com/a@b?page=2' https://api.example.com/x");
        assert!(extracted.is_empty(), "{yaml}");
    }

    #[test]
    fn values_that_are_words_leave_the_notes_alone() {
        let (yaml, _) =
            import("curl -u curl:flag https://api.example.com/x --compressed --foo bar");
        assert!(
            yaml.contains("# Warning: Unsupported curl flag: --foo (value omitted)"),
            "{yaml}"
        );
        assert!(
            yaml.contains("curl handles decompression natively"),
            "{yaml}"
        );
        assert!(yaml.contains("password: ${API_PASSWORD}"), "{yaml}");
    }

    #[test]
    fn proxy_and_url_userinfo_move_into_variables() {
        let (yaml, extracted, _) = one(
            "curl -x http://puser:ppass_123456@proxy.example.com:8080 https://api.example.com/x",
            &NO_ENV,
        );
        let vars: Vec<&str> = extracted.iter().map(|e| e.var.as_str()).collect();
        assert_eq!(vars, ["PROXY_USER", "PROXY_PASSWORD"]);
        assert!(
            yaml.contains("${PROXY_USER}:${PROXY_PASSWORD}@proxy.example.com:8080"),
            "{yaml}"
        );
        assert!(
            !yaml.contains("ppass_123456") && !yaml.contains("puser"),
            "{yaml}"
        );
        assert_parses(&yaml);

        let (yaml, extracted, _) = one(
            "curl https://uuser:upass_123456@api.example.com/v1/x",
            &NO_ENV,
        );
        let vars: Vec<(&str, &str)> = extracted
            .iter()
            .map(|e| (e.var.as_str(), e.value.as_str()))
            .collect();
        assert_eq!(
            vars,
            [("API_USER", "uuser"), ("API_PASSWORD", "upass_123456")]
        );
        assert!(yaml.contains("url: https://api.example.com/v1/x"), "{yaml}");
        assert!(yaml.contains("type: basic"), "{yaml}");
        assert!(
            !yaml.contains("upass_123456") && !yaml.contains("uuser@"),
            "{yaml}"
        );
        assert_parses(&yaml);
        // `-u` wins over the URL's userinfo, which is dropped either way.
        let (yaml, extracted, _) = one(
            "curl -u real:real_pw_1 https://other:other_pw_2@api.example.com/x",
            &NO_ENV,
        );
        assert_eq!(extracted[1].value, "real_pw_1");
        assert!(
            !yaml.contains("other_pw_2") && !yaml.contains("other@"),
            "{yaml}"
        );
    }

    #[test]
    fn short_values_only_replace_whole_tokens() {
        let (yaml, extracted, _) = one("curl -u a:b --compressed https://api.example.com", &NO_ENV);
        assert_eq!(extracted.len(), 2);
        assert!(
            yaml.contains(
                "# Warning: Flag --compressed has no YAML equivalent; curl handles decompression natively"
            ),
            "{yaml}"
        );
        let e = |var: &str, value: &str| Extracted {
            var: var.into(),
            value: value.into(),
            place: String::new(),
        };
        let both = [e("API_USER", "a"), e("API_PASSWORD", "b")];
        assert_eq!(
            scrub_text("flag a b='b' x/a:a ${API_USER}", &both),
            "flag ${API_USER} ${API_PASSWORD}='${API_PASSWORD}' x/${API_USER}:${API_USER} ${API_USER}"
        );
        // Never inside an inserted reference.
        let nested = [e("TOKEN", "TOKEN"), e("API", "API")];
        assert_eq!(scrub_text("TOKEN API", &nested), "${TOKEN} ${API}");
    }

    #[test]
    fn cert_passwords_split_on_the_last_plain_colon() {
        assert_eq!(cert_password_at("client.pem:pw"), Some(10));
        assert_eq!(cert_password_at("C:\\certs\\c.pem"), None);
        assert_eq!(cert_password_at("C:\\c.pem:pw"), Some(8));
        assert_eq!(cert_password_at("a:b.pem:pw"), Some(7));
        assert_eq!(cert_password_at("my\\:cert.pem"), None);
        assert_eq!(cert_password_at("my\\:cert.pem:pw"), Some(12));
        assert_eq!(cert_password_at("pkcs11:token=x;object=y"), None);
        assert_eq!(cert_password_at("PKCS11:token=x;object=y"), None);
        let (yaml, _, _) = one(
            "curl --cert 'pkcs11:token=x;object=y' https://api.example.com",
            &NO_ENV,
        );
        assert!(yaml.contains("pkcs11:token=x;object=y"), "{yaml}");
        assert!(!yaml.contains("password was left out"), "{yaml}");
    }
}
