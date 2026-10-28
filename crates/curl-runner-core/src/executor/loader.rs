//! Run pipeline loader (TS `executeMain` + `processYamlFile` + the `.env`
//! part of `resolveConfig`). Entry point for the CLI: [`load_run`], then one
//! `Executor::run` per file group (the multi-file orchestration lives in the
//! CLI's `RunCtx::execute`).

use super::PreparedRequest;
use crate::config::{self, CliOptions};
use crate::env;
use crate::interpolate::EnvLookup;
use crate::model::{GlobalConfig, Json};
use crate::parser::{self, ParseError};
use indexmap::IndexMap;
use std::path::{Path, PathBuf};

/// Every request of a run plus the effective config.
#[derive(Debug, Clone, PartialEq)]
pub struct LoadedRun {
    /// `config::resolve(folded file globals, project, env, cli)` with the
    /// `.env` variables merged under `variables` (TS resolver).
    pub global: GlobalConfig,
    /// All files' requests in file order (merged raw JSON, see
    /// `parser::prepare_raw`).
    pub requests: Vec<PreparedRequest>,
    /// Variables from the `.env` files (`env::secret_keys` picks the
    /// `SECRET_*` ones for the redactor).
    pub dotenv: IndexMap<String, String>,
}

/// A test file that could not be loaded (the CLI exits 1 with this message).
#[derive(Debug, thiserror::Error)]
#[error("{}: {source}", file.display())]
pub struct LoadError {
    pub file: PathBuf,
    #[source]
    pub source: ParseError,
}

/// Load `files` (already discovered, in run order).
///
/// - `project`: `curl-runner.yaml`'s `global` (`config::load_config_file`).
/// - `env`: the process environment (`CURL_RUNNER_*` and `${VAR}` lookups).
/// - `cwd`: absolute working directory (`.env` files, relative paths).
///
/// Steps: read every file's `global`, fold them in order and resolve the
/// final config; load `.env` files with the final `env.environment`; then
/// per file, layer defaults with `config::defaults_layers` (project + the
/// `global`s of the files before it, as TS passes `mergedConfig.defaults`),
/// interpolate each request with the file's variables, where `${VAR}` falls
/// back to the process env and then to `.env` values, and merge.
pub fn load_run(
    files: &[PathBuf],
    project: Option<&GlobalConfig>,
    cli: &CliOptions,
    env: EnvLookup,
    cwd: &Path,
) -> Result<LoadedRun, LoadError> {
    let err = |file: &Path| {
        let file = file.to_path_buf();
        move |source| LoadError { file, source }
    };
    let mut raws: Vec<(Json, GlobalConfig)> = Vec::with_capacity(files.len());
    for f in files {
        let raw = parser::load_raw(f).map_err(err(f))?;
        let g = parser::global_from_raw(&raw).map_err(err(f))?;
        raws.push((raw, g));
    }

    let mut folded = GlobalConfig::default();
    let mut before: Vec<GlobalConfig> = Vec::with_capacity(files.len());
    for (_, g) in &raws {
        before.push(folded.clone());
        folded = config::merge_global_configs(&folded, g);
    }
    let mut global = config::resolve(folded, project.cloned(), env, cli);

    let environment = global.env.as_ref().and_then(|e| e.environment.clone());
    let dotenv = env::load_env_files(cwd, environment.as_deref());
    let mut vars = dotenv.clone();
    vars.extend(global.variables.take().unwrap_or_default());
    global.variables = Some(vars);

    let lookup = |k: &str| env(k).or_else(|| dotenv.get(k).cloned());
    let mut requests = Vec::new();
    for ((f, (raw, _)), prev) in files.iter().zip(&raws).zip(&before) {
        let layers = config::defaults_layers(project, prev, env, cli);
        let parsed = parser::prepare_raw(raw, f, &layers, Some(&lookup)).map_err(err(f))?;
        requests.extend(
            parsed
                .requests
                .into_iter()
                .map(|raw| PreparedRequest { raw }),
        );
    }
    Ok(LoadedRun {
        global,
        requests,
        dotenv,
    })
}
