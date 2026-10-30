//! Environments (`.env.{name}` files) and where each `${VAR}` gets its value.

use crate::model::secrets::{SecretStore, service_name};
use curl_runner_core::config::{self, CliOptions};
use curl_runner_core::env;
use curl_runner_core::interpolate::{self, Store, extract_variables, resolve_dynamic_variable};
use curl_runner_core::model::GlobalConfig;
use curl_runner_core::parser;
use curl_runner_core::parser::parse_raw_stream;
use serde_json::Value as Json;
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedVar {
    pub name: String,
    pub value: Option<String>,
    pub source: Option<VarSource>,
    pub secret: bool,
}

/// The collection and environment a run or a lookup is for.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EnvCtx {
    /// Absolute collection root (`.env` files, `curl-runner.yaml`).
    pub root: PathBuf,
    /// Collection name, for the Keychain service.
    pub collection: String,
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
        service_name(&self.collection, &self.environment)
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::secrets::MemorySecrets;
    use std::fs;

    fn ctx(root: &Path, environment: &str) -> EnvCtx {
        EnvCtx {
            root: root.to_path_buf(),
            collection: "Payments API".into(),
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
}
