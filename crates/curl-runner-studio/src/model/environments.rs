//! Environments (`.env.{name}` files) and where each `${VAR}` gets its value.

use crate::model::secrets::{SecretStore, service_name};
use curl_runner_core::env;
use curl_runner_core::interpolate::{self, Store, extract_variables, resolve_dynamic_variable};
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

/// Where a variable's value comes from, highest precedence first.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum VarSource {
    /// The response store of the file's last run.
    Captured,
    Keychain,
    /// The `.env` file that sets it (the last one loaded wins).
    EnvFile(PathBuf),
    Process,
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

    /// The Keychain service of this collection and environment.
    pub fn service(&self) -> String {
        service_name(&self.collection, &self.environment)
    }
}

/// Resolve `names`: captured store, then Keychain (secrets only), then the
/// environment's `.env` files, then the process env.
pub fn resolve_vars(
    names: &[String],
    captured: &Store,
    secrets: &dyn SecretStore,
    ctx: &EnvCtx,
) -> Vec<ResolvedVar> {
    // Later files win, as in `env::load_env_files`.
    let files: Vec<_> = env::env_file_paths(&ctx.root, ctx.env_name())
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
            let secret = is_secret(name, &ctx.marked_secret);
            let found = captured
                .get(name)
                .map(|v| (v.clone(), VarSource::Captured))
                .or_else(|| {
                    secret
                        .then(|| secrets.get(&service, name))
                        .flatten()
                        .map(|v| (v, VarSource::Keychain))
                })
                .or_else(|| {
                    files.iter().find_map(|(p, vars)| {
                        vars.get(name)
                            .map(|v| (v.clone(), VarSource::EnvFile(p.clone())))
                    })
                })
                .or_else(|| interpolate::process_env(name).map(|v| (v, VarSource::Process)));
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

/// The plain `${NAME}` references of `yaml`, deduplicated in order: no
/// `store.*` or dynamic variables, and no reference with a `:default`.
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
            || name.starts_with("store.")
            || resolve_dynamic_variable(name).is_some()
            || names.iter().any(|n| n == name)
        {
            continue;
        }
        names.push(name.to_string());
    }
    names
}

/// The references of `yaml` with no value in `resolved` nor in the file's
/// own `variables`. The run still goes: the CLI sends them literally.
pub fn missing_vars(yaml: &str, resolved: &[ResolvedVar]) -> Vec<String> {
    let own = file_variables(yaml);
    referenced_vars(yaml)
        .into_iter()
        .filter(|name| {
            !own.contains(name)
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
    fn precedence_captured_over_keychain_over_env_file() {
        let d = tempfile::tempdir().unwrap();
        let env_file = d.path().join(".env.staging");
        fs::write(&env_file, "SECRET_TOKEN=from-file\n").unwrap();
        let ctx = ctx(d.path(), "staging");
        let secrets = MemorySecrets::default();
        secrets
            .set(&ctx.service(), "SECRET_TOKEN", "from-keychain")
            .unwrap();
        let mut store = Store::new();
        store.insert("SECRET_TOKEN".into(), "captured".into());
        let names = vec!["SECRET_TOKEN".to_string()];
        let one = |store: &Store| resolve_vars(&names, store, &secrets, &ctx).remove(0);

        let v = one(&store);
        assert_eq!(v.value.as_deref(), Some("captured"));
        assert_eq!(v.source, Some(VarSource::Captured));
        assert!(v.secret);

        store.clear();
        let v = one(&store);
        assert_eq!(v.value.as_deref(), Some("from-keychain"));
        assert_eq!(v.source, Some(VarSource::Keychain));

        secrets.delete(&ctx.service(), "SECRET_TOKEN").unwrap();
        let v = one(&store);
        assert_eq!(v.value.as_deref(), Some("from-file"));
        assert_eq!(v.source, Some(VarSource::EnvFile(env_file)));
    }

    #[test]
    fn keychain_only_for_secret_names() {
        let d = tempfile::tempdir().unwrap();
        let ctx = ctx(d.path(), DEFAULT_ENVIRONMENT);
        let secrets = MemorySecrets::default();
        secrets.set(&ctx.service(), "BASE_URL", "x").unwrap();
        let v = resolve_vars(&["BASE_URL".into()], &Store::new(), &secrets, &ctx);
        assert_eq!(v[0].source, None);
        assert_eq!(v[0].value, None);
    }

    #[test]
    fn missing_vars_lists_unset_names_only() {
        let d = tempfile::tempdir().unwrap();
        fs::write(d.path().join(".env"), "BASE_URL=https://api.example.com\n").unwrap();
        let yaml = "${BASE_URL}/x ${API_TOKEN} ${store.ID} ${DATE:YYYY}";
        let names = referenced_vars(yaml);
        assert_eq!(names, ["BASE_URL", "API_TOKEN"]);
        let resolved = resolve_vars(
            &names,
            &Store::new(),
            &MemorySecrets::default(),
            &ctx(d.path(), DEFAULT_ENVIRONMENT),
        );
        assert_eq!(missing_vars(yaml, &resolved), ["API_TOKEN"]);
    }

    #[test]
    fn file_variables_and_defaults_are_not_missing() {
        let yaml = "global:\n  variables:\n    HOST: api.example.com\nrequest:\n  url: https://${HOST}/${PATH:v1}/${UUID}/${ID:upper}\n";
        assert_eq!(missing_vars(yaml, &[]), ["ID"]);
    }
}
