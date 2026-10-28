//! Config resolution: `core/config/{resolver,defaults,file-loader}.ts`.
//!
//! # Precedence (deliberate fix of a TS bug)
//!
//! Lowest to highest: `curl-runner.yaml` `global` < the test files' `global`
//! blocks < `CURL_RUNNER_*` env vars < CLI flags. TS instead merged each test
//! file's `global` AFTER the CLI layer (`cli.ts` `executeMain`), so a test file
//! beat CLI flags and env vars, contradicting the docs; see
//! `crates/CHANGES-FROM-TS.md`. Per-group merge semantics are those of
//! `mergeGlobalConfigs` / `applyCliOptionsToConfig`.
//!
//! `project_cfg` is `curl-runner.yaml`'s `global`; `file_cfg` is the test
//! files' globals folded in order with [`merge_global_configs`].
//! `CONFIG_DEFAULTS` (`defaults.ts`) is exported as [`defaults`] but, as in TS,
//! not applied by the resolver; consumers use them as `?? default` fallbacks.
//! The only default applied here is `security.allowedProtocols`.

use crate::env::apply_env_overrides;
use crate::model::{
    CiExitConfig, ConnectionPoolConfig, EnvConfig, GlobalConfig, GlobalDiffConfig,
    GlobalSnapshotConfig, OutputConfig, ProfileConfig, RequestConfig, SecurityConfig, WatchConfig,
};
use crate::parser;
use std::path::Path;

/// All options of the TS `CLIOptions`.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct CliOptions {
    pub files: Vec<String>,
    pub all: Option<bool>,
    pub env: Option<String>,
    pub no_redact: Option<bool>,
    /// `sequential` | `parallel`
    pub execution: Option<String>,
    pub max_concurrency: Option<u64>,
    pub continue_on_error: Option<bool>,
    pub dry_run: Option<bool>,
    pub http2: Option<bool>,
    pub connection_pool: Option<bool>,
    pub max_streams: Option<u64>,
    pub keepalive_time: Option<u64>,
    pub connect_timeout: Option<u64>,
    pub verbose: Option<bool>,
    pub quiet: Option<bool>,
    pub output: Option<String>,
    /// `json` | `pretty` | `raw`
    pub output_format: Option<String>,
    /// `minimal` | `standard` | `detailed`
    pub pretty_level: Option<String>,
    pub show_headers: Option<bool>,
    pub show_body: Option<bool>,
    pub show_metrics: Option<bool>,
    /// Milliseconds.
    pub timeout: Option<u64>,
    pub retries: Option<u64>,
    /// Milliseconds.
    pub retry_delay: Option<u64>,
    pub no_retry: Option<bool>,
    pub watch: Option<bool>,
    pub watch_debounce: Option<u64>,
    pub watch_clear: Option<bool>,
    pub profile: Option<u64>,
    pub profile_warmup: Option<u64>,
    pub profile_concurrency: Option<u64>,
    pub profile_histogram: Option<bool>,
    pub profile_export: Option<String>,
    pub snapshot: Option<bool>,
    /// `none` | `all` | `failing`
    pub snapshot_update: Option<String>,
    pub snapshot_dir: Option<String>,
    pub snapshot_ci: Option<bool>,
    pub diff: Option<bool>,
    pub diff_save: Option<bool>,
    pub diff_label: Option<String>,
    pub diff_compare: Option<String>,
    pub diff_dir: Option<String>,
    /// `terminal` | `json` | `markdown`
    pub diff_output: Option<String>,
    pub strict_exit: Option<bool>,
    pub fail_on: Option<u64>,
    pub fail_on_percentage: Option<f64>,
    pub allow_protocols: Option<Vec<String>>,
    pub allow_path: Option<bool>,
    pub help: Option<bool>,
    pub version: Option<bool>,
}

/// `DEFAULT_ALLOWED_PROTOCOLS` from `core/security/url-guard.ts`.
pub const DEFAULT_ALLOWED_PROTOCOLS: [&str; 2] = ["http", "https"];

/// `CONFIG_FILE_PATHS`, searched in order.
pub const CONFIG_FILE_PATHS: [&str; 4] = [
    "curl-runner.yaml",
    "curl-runner.yml",
    ".curl-runner.yaml",
    ".curl-runner.yml",
];

/// Result of [`load_config_file`]. TS reports through callbacks; here the
/// messages are returned in order of occurrence (`messages`).
#[derive(Debug, Clone, PartialEq, Default)]
pub struct ConfigFileResult {
    pub config: GlobalConfig,
    /// File name as searched (relative), `None` when nothing was loaded.
    pub path: Option<String>,
    pub messages: Vec<ConfigMessage>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ConfigMessage {
    Info(String),
    Warning(String),
}

/// Port of `loadConfigFile`: the first existing file of [`CONFIG_FILE_PATHS`]
/// in `cwd` wins; its `global` block (or empty) is returned; other sections are
/// ignored, as in TS. A file whose YAML or `global` types are invalid yields a warning and the search continues. No interpolation
/// is applied (TS parses the raw YAML).
pub fn load_config_file(cwd: &Path) -> ConfigFileResult {
    let mut messages = Vec::new();
    for name in CONFIG_FILE_PATHS {
        let path = cwd.join(name);
        if !path.is_file() {
            continue;
        }
        match parser::load_raw(&path).and_then(|raw| parser::global_from_raw(&raw)) {
            Ok(global) => {
                messages.push(ConfigMessage::Info(format!(
                    "Loaded configuration from {name}"
                )));
                return ConfigFileResult {
                    config: global,
                    path: Some(name.to_string()),
                    messages,
                };
            }
            Err(e) => messages.push(ConfigMessage::Warning(format!(
                "Failed to load configuration from {name}: {e}"
            ))),
        }
    }
    ConfigFileResult {
        messages,
        ..Default::default()
    }
}

fn merge_opt<T: Clone>(
    base: &Option<T>,
    over: &Option<T>,
    f: impl FnOnce(&T, &T) -> T,
) -> Option<T> {
    match (base, over) {
        (None, None) => None,
        (Some(b), None) => Some(b.clone()),
        (None, Some(o)) => Some(o.clone()),
        (Some(b), Some(o)) => Some(f(b, o)),
    }
}

/// `{...base, ...over}` for a group struct: a field set in `over` wins.
macro_rules! spread {
    ($ty:ident, $b:expr, $o:expr, [$($f:ident),*]) => {
        $ty { $($f: $o.$f.clone().or_else(|| $b.$f.clone())),* }
    };
}
pub(crate) use spread;

/// Distinct default layers handed to the parser (see
/// [`parser::from_raw_with_defaults`]); build with [`defaults_layers`].
#[derive(Debug, Clone, PartialEq, Default)]
pub struct DefaultsLayers {
    /// Project config + test-file `global.defaults` (lowest layer).
    pub base: RequestConfig,
    /// Defaults derived from env vars and CLI flags only (timeout, retry
    /// count/delay, http2). Beats the file's `defaults`, loses to the request.
    pub overrides: RequestConfig,
    /// Output settings derived from env vars and CLI flags only; beats the
    /// file's `global.output` in each request's `sourceOutputConfig`.
    pub output_overrides: OutputConfig,
}

/// Build the [`DefaultsLayers`] for parsing test files.
pub fn defaults_layers(
    project_cfg: Option<&GlobalConfig>,
    file_cfg: &GlobalConfig,
    env: &dyn Fn(&str) -> Option<String>,
    cli: &CliOptions,
) -> DefaultsLayers {
    let base = merge_global_configs(&project_cfg.cloned().unwrap_or_default(), file_cfg);
    let mut derived = GlobalConfig::default();
    apply_env_overrides(&mut derived, &env);
    apply_cli_options(&mut derived, cli);
    // `global.http2` is the lowest-priority source of `defaults.http2`.
    let mut base_defaults = base.defaults.unwrap_or_default();
    base_defaults.http2 = base_defaults.http2.or(base.http2);
    DefaultsLayers {
        base: base_defaults,
        overrides: derived.defaults.unwrap_or_default(),
        output_overrides: derived.output.unwrap_or_default(),
    }
}

/// Port of `mergeGlobalConfigs`: top-level fields of `over` replace `base`'s;
/// the groups `env`, `variables`, `output`, `defaults` (with `retry` one level
/// deeper), `ci`, `watch`, `snapshot`, `diff`, `connectionPool` and `security`
/// merge field-wise. `ssl` and `profile` are replaced whole. Absent groups
/// stay `None` (TS yields `{}`; no observable difference).
pub fn merge_global_configs(base: &GlobalConfig, over: &GlobalConfig) -> GlobalConfig {
    GlobalConfig {
        execution: over.execution.clone().or_else(|| base.execution.clone()),
        max_concurrency: over.max_concurrency.or(base.max_concurrency),
        continue_on_error: over.continue_on_error.or(base.continue_on_error),
        dry_run: over.dry_run.or(base.dry_run),
        http2: over.http2.or(base.http2),
        ssl: over.ssl.clone().or_else(|| base.ssl.clone()),
        profile: over.profile.clone().or_else(|| base.profile.clone()),
        env: merge_opt(&base.env, &over.env, |b, o| {
            spread!(EnvConfig, b, o, [environment, redact_secrets])
        }),
        variables: merge_opt(&base.variables, &over.variables, |b, o| {
            let mut m = b.clone();
            m.extend(o.iter().map(|(k, v)| (k.clone(), v.clone())));
            m
        }),
        output: merge_opt(&base.output, &over.output, |b, o| {
            spread!(
                OutputConfig,
                b,
                o,
                [
                    verbose,
                    quiet,
                    show_headers,
                    show_body,
                    show_metrics,
                    format,
                    pretty_level,
                    save_to_file,
                    dry_run
                ]
            )
        }),
        defaults: merge_opt(&base.defaults, &over.defaults, |b, o| {
            parser::spread_defaults(b, o)
        }),
        ci: merge_opt(&base.ci, &over.ci, |b, o| {
            spread!(
                CiExitConfig,
                b,
                o,
                [strict_exit, fail_on, fail_on_percentage]
            )
        }),
        watch: merge_opt(&base.watch, &over.watch, |b, o| {
            spread!(WatchConfig, b, o, [enabled, debounce, clear])
        }),
        snapshot: merge_opt(&base.snapshot, &over.snapshot, |b, o| {
            spread!(
                GlobalSnapshotConfig,
                b,
                o,
                [
                    enabled,
                    name,
                    include,
                    exclude,
                    match_rules,
                    dir,
                    update_mode,
                    ci
                ]
            )
        }),
        diff: merge_opt(&base.diff, &over.diff, |b, o| {
            spread!(
                GlobalDiffConfig,
                b,
                o,
                [
                    enabled,
                    exclude,
                    match_rules,
                    include_timings,
                    dir,
                    label,
                    compare_with,
                    save,
                    output_format
                ]
            )
        }),
        connection_pool: merge_opt(&base.connection_pool, &over.connection_pool, |b, o| {
            spread!(
                ConnectionPoolConfig,
                b,
                o,
                [
                    enabled,
                    max_streams_per_host,
                    keepalive_time,
                    connect_timeout
                ]
            )
        }),
        security: merge_opt(&base.security, &over.security, |b, o| {
            spread!(SecurityConfig, b, o, [allowed_protocols, allow_paths])
        }),
    }
}

/// Port of `applyCliOptionsToConfig` (CLI wins). JS truthiness is preserved
/// where TS uses it: `--timeout 0`, `--retries 0`, `--retry-delay 0` and an
/// empty `--env`/`--output` change nothing.
pub fn apply_cli_options(cfg: &mut GlobalConfig, cli: &CliOptions) {
    if let Some(e) = cli.env.as_ref().filter(|e| !e.is_empty()) {
        cfg.env.get_or_insert_with(Default::default).environment = Some(e.clone());
    }
    if cli.no_redact == Some(true) {
        cfg.env.get_or_insert_with(Default::default).redact_secrets = Some(false);
    }

    if let Some(e) = &cli.execution {
        cfg.execution = Some(e.clone());
    }
    if cli.max_concurrency.is_some() {
        cfg.max_concurrency = cli.max_concurrency;
    }
    if cli.continue_on_error.is_some() {
        cfg.continue_on_error = cli.continue_on_error;
    }
    if let Some(d) = cli.dry_run {
        cfg.dry_run = Some(d);
        cfg.output.get_or_insert_with(Default::default).dry_run = Some(d);
    }
    if let Some(h) = cli.http2 {
        cfg.http2 = Some(h);
        cfg.defaults.get_or_insert_with(Default::default).http2 = Some(h);
    }

    // Connection pooling: tuning flags imply `enabled: true`.
    if let Some(e) = cli.connection_pool {
        cfg.connection_pool
            .get_or_insert_with(Default::default)
            .enabled = Some(e);
    }
    if cli.max_streams.is_some() || cli.keepalive_time.is_some() || cli.connect_timeout.is_some() {
        let p = cfg.connection_pool.get_or_insert_with(Default::default);
        if cli.max_streams.is_some() {
            p.enabled = Some(true);
            p.max_streams_per_host = cli.max_streams;
        }
        if cli.keepalive_time.is_some() {
            p.enabled = Some(true);
            p.keepalive_time = cli.keepalive_time;
        }
        if cli.connect_timeout.is_some() {
            p.enabled = Some(true);
            p.connect_timeout = cli.connect_timeout;
        }
    }

    // Output
    if cli.verbose.is_some() {
        cfg.output.get_or_insert_with(Default::default).verbose = cli.verbose;
    }
    if cli.quiet == Some(true) {
        let o = cfg.output.get_or_insert_with(Default::default);
        o.verbose = Some(false);
        o.quiet = Some(true);
    }
    if let Some(f) = cli.output.as_ref().filter(|f| !f.is_empty()) {
        cfg.output.get_or_insert_with(Default::default).save_to_file = Some(f.clone());
    }
    if let Some(f) = &cli.output_format {
        cfg.output.get_or_insert_with(Default::default).format = Some(f.clone());
    }
    if let Some(l) = &cli.pretty_level {
        cfg.output.get_or_insert_with(Default::default).pretty_level = Some(l.clone());
    }
    if cli.show_headers.is_some() {
        cfg.output.get_or_insert_with(Default::default).show_headers = cli.show_headers;
    }
    if cli.show_body.is_some() {
        cfg.output.get_or_insert_with(Default::default).show_body = cli.show_body;
    }
    if cli.show_metrics.is_some() {
        cfg.output.get_or_insert_with(Default::default).show_metrics = cli.show_metrics;
    }

    // Timeouts (milliseconds) & retries
    if let Some(t) = cli.timeout.filter(|t| *t != 0) {
        cfg.defaults.get_or_insert_with(Default::default).timeout = Some(t);
    }
    let no_retry = cli.no_retry == Some(true);
    if no_retry || cli.retries.is_some_and(|r| r != 0) {
        let count = if no_retry {
            0
        } else {
            cli.retries.unwrap_or(0)
        };
        let d = cfg.defaults.get_or_insert_with(Default::default);
        d.retry.get_or_insert_with(Default::default).count = Some(count);
    }
    if let Some(delay) = cli.retry_delay.filter(|d| *d != 0) {
        let d = cfg.defaults.get_or_insert_with(Default::default);
        d.retry.get_or_insert_with(Default::default).delay = Some(delay);
    }

    // CI
    if cli.strict_exit.is_some() {
        cfg.ci.get_or_insert_with(Default::default).strict_exit = cli.strict_exit;
    }
    if cli.fail_on.is_some() {
        cfg.ci.get_or_insert_with(Default::default).fail_on = cli.fail_on;
    }
    if cli.fail_on_percentage.is_some() {
        cfg.ci
            .get_or_insert_with(Default::default)
            .fail_on_percentage = cli.fail_on_percentage;
    }

    // Snapshot
    if cli.snapshot.is_some() {
        cfg.snapshot.get_or_insert_with(Default::default).enabled = cli.snapshot;
    }
    if let Some(m) = &cli.snapshot_update {
        let s = cfg.snapshot.get_or_insert_with(Default::default);
        s.enabled = Some(true);
        s.update_mode = Some(m.clone());
    }
    if let Some(d) = &cli.snapshot_dir {
        cfg.snapshot.get_or_insert_with(Default::default).dir = Some(d.clone());
    }
    if cli.snapshot_ci.is_some() {
        cfg.snapshot.get_or_insert_with(Default::default).ci = cli.snapshot_ci;
    }

    // Diff
    if cli.diff.is_some() {
        cfg.diff.get_or_insert_with(Default::default).enabled = cli.diff;
    }
    if cli.diff_save.is_some() {
        let d = cfg.diff.get_or_insert_with(Default::default);
        d.enabled = Some(true);
        d.save = cli.diff_save;
    }
    if let Some(l) = &cli.diff_label {
        cfg.diff.get_or_insert_with(Default::default).label = Some(l.clone());
    }
    if let Some(c) = &cli.diff_compare {
        let d = cfg.diff.get_or_insert_with(Default::default);
        d.enabled = Some(true);
        d.compare_with = Some(c.clone());
    }
    if let Some(d) = &cli.diff_dir {
        cfg.diff.get_or_insert_with(Default::default).dir = Some(d.clone());
    }
    if let Some(f) = &cli.diff_output {
        cfg.diff.get_or_insert_with(Default::default).output_format = Some(f.clone());
    }

    // Security
    if let Some(p) = cli.allow_protocols.as_ref().filter(|p| !p.is_empty()) {
        cfg.security
            .get_or_insert_with(Default::default)
            .allowed_protocols = Some(p.clone());
    }
    if cli.allow_path == Some(true) {
        cfg.security
            .get_or_insert_with(Default::default)
            .allow_paths = Some(true);
    }
    // `security.allowedProtocols` always has a default.
    let sec = cfg.security.get_or_insert_with(Default::default);
    if sec.allowed_protocols.as_ref().is_none_or(|p| p.is_empty()) {
        sec.allowed_protocols = Some(DEFAULT_ALLOWED_PROTOCOLS.map(String::from).to_vec());
    }
}

/// Resolve the effective global config: project < file < env < CLI (see the
/// module docs). `project_cfg` is `curl-runner.yaml`'s `global` ([`load_config_file`]);
/// `file_cfg` is the YAML test files' `global` blocks folded in order with
/// [`merge_global_configs`] (pass `GlobalConfig::default()` when none).
/// Do NOT pass the result's `defaults` to the parser as a default layer: it
/// would let file-level `defaults` re-win over CLI per request. Use
/// [`defaults_layers`] for parsing test files. `.env` file variables are merged
/// by the run pipeline (`loadEnvFiles` needs the final `env.environment`).
pub fn resolve(
    file_cfg: GlobalConfig,
    project_cfg: Option<GlobalConfig>,
    env: &dyn Fn(&str) -> Option<String>,
    cli: &CliOptions,
) -> GlobalConfig {
    let mut cfg = merge_global_configs(&project_cfg.unwrap_or_default(), &file_cfg);
    apply_env_overrides(&mut cfg, &env);
    apply_cli_options(&mut cfg, cli);
    cfg
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExecutionMode {
    Normal,
    Watch,
    Profile,
}

/// Port of `determineExecutionMode`: profile (iterations > 0) beats watch.
pub fn determine_execution_mode(cli: &CliOptions, cfg: &GlobalConfig) -> ExecutionMode {
    let watch = cli.watch == Some(true) || cfg.watch.as_ref().and_then(|w| w.enabled) == Some(true);
    let iterations = cli.profile.or(cfg.profile.as_ref().map(|p| p.iterations));
    if iterations.is_some_and(|n| n > 0) {
        ExecutionMode::Profile
    } else if watch {
        ExecutionMode::Watch
    } else {
        ExecutionMode::Normal
    }
}

/// Port of `buildProfileConfig` (all numeric fields filled).
pub fn build_profile_config(cli: &CliOptions, cfg: &GlobalConfig) -> ProfileConfig {
    let p = cfg.profile.as_ref();
    ProfileConfig {
        iterations: cli
            .profile
            .or(p.map(|p| p.iterations))
            .unwrap_or(defaults::PROFILE_ITERATIONS),
        warmup: Some(
            cli.profile_warmup
                .or(p.and_then(|p| p.warmup))
                .unwrap_or(defaults::PROFILE_WARMUP),
        ),
        concurrency: Some(
            cli.profile_concurrency
                .or(p.and_then(|p| p.concurrency))
                .unwrap_or(defaults::PROFILE_CONCURRENCY),
        ),
        histogram: Some(
            cli.profile_histogram
                .or(p.and_then(|p| p.histogram))
                .unwrap_or(defaults::PROFILE_HISTOGRAM),
        ),
        export_file: cli
            .profile_export
            .clone()
            .or(p.and_then(|p| p.export_file.clone())),
    }
}

/// Port of `buildWatchConfig`.
pub fn build_watch_config(cli: &CliOptions, cfg: &GlobalConfig) -> WatchConfig {
    let w = cfg.watch.as_ref();
    WatchConfig {
        enabled: Some(true),
        debounce: Some(
            cli.watch_debounce
                .or(w.and_then(|w| w.debounce))
                .unwrap_or(defaults::WATCH_DEBOUNCE),
        ),
        clear: Some(
            cli.watch_clear
                .or(w.and_then(|w| w.clear))
                .unwrap_or(defaults::WATCH_CLEAR),
        ),
    }
}

/// `CONFIG_DEFAULTS` from `defaults.ts`.
pub mod defaults {
    pub const EXECUTION: &str = "sequential";
    pub const CONTINUE_ON_ERROR: bool = false;
    pub const DRY_RUN: bool = false;
    pub const HTTP2: bool = false;
    /// Milliseconds.
    pub const TIMEOUT: u64 = 30000;
    pub const RETRY_COUNT: u64 = 0;
    /// Milliseconds.
    pub const RETRY_DELAY: u64 = 1000;
    pub const RETRY_BACKOFF: f64 = 1.0;
    pub const VERBOSE: bool = false;
    pub const OUTPUT_FORMAT: &str = "pretty";
    pub const PRETTY_LEVEL: &str = "standard";
    pub const SHOW_HEADERS: bool = false;
    pub const SHOW_BODY: bool = true;
    pub const SHOW_METRICS: bool = false;
    pub const WATCH_DEBOUNCE: u64 = 300;
    pub const WATCH_CLEAR: bool = true;
    pub const PROFILE_ITERATIONS: u64 = 10;
    pub const PROFILE_WARMUP: u64 = 1;
    pub const PROFILE_CONCURRENCY: u64 = 1;
    pub const PROFILE_HISTOGRAM: bool = false;
    pub const SNAPSHOT_DIR: &str = "__snapshots__";
    pub const SNAPSHOT_UPDATE_MODE: &str = "none";
    pub const SNAPSHOT_CI: bool = false;
    pub const BASELINE_DIR: &str = "__baselines__";
    pub const DIFF_OUTPUT_FORMAT: &str = "terminal";
    pub const CONNECTION_POOL_ENABLED: bool = false;
    pub const MAX_STREAMS_PER_HOST: u64 = 10;
    /// Seconds (TS quirk: unit differs from the millisecond timeouts).
    pub const KEEPALIVE_TIME: u64 = 60;
    /// Seconds.
    pub const CONNECT_TIMEOUT: u64 = 30;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{RequestConfig, RetryConfig};
    use std::collections::HashMap;

    fn no_env(_: &str) -> Option<String> {
        None
    }
    fn env_of(pairs: &[(&str, &str)]) -> impl Fn(&str) -> Option<String> {
        let m: HashMap<String, String> = pairs
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect();
        move |k| m.get(k).cloned()
    }
    fn cli(f: impl FnOnce(&mut CliOptions)) -> CliOptions {
        let mut c = CliOptions::default();
        f(&mut c);
        c
    }
    fn out(c: &GlobalConfig) -> OutputConfig {
        c.output.clone().unwrap_or_default()
    }

    // --- resolver-quiet.test.ts ---
    #[test]
    fn quiet_sets_output_quiet_and_verbose_false() {
        let r = resolve(
            GlobalConfig::default(),
            None,
            &no_env,
            &cli(|c| c.quiet = Some(true)),
        );
        assert_eq!(out(&r).quiet, Some(true));
        assert_eq!(out(&r).verbose, Some(false));
    }

    #[test]
    fn quiet_overrides_project_verbose() {
        let project = GlobalConfig {
            output: Some(OutputConfig {
                verbose: Some(true),
                ..Default::default()
            }),
            ..Default::default()
        };
        let r = resolve(
            GlobalConfig::default(),
            Some(project),
            &no_env,
            &cli(|c| c.quiet = Some(true)),
        );
        assert_eq!(out(&r).verbose, Some(false));
    }

    fn gc(f: impl FnOnce(&mut GlobalConfig)) -> GlobalConfig {
        let mut g = GlobalConfig::default();
        f(&mut g);
        g
    }

    #[test]
    fn file_global_beats_project_config() {
        let project = gc(|g| g.execution = Some("sequential".into()));
        let file = gc(|g| g.execution = Some("parallel".into()));
        let r = resolve(file, Some(project), &no_env, &CliOptions::default());
        assert_eq!(r.execution.as_deref(), Some("parallel"));
    }

    #[test]
    fn env_beats_file_global() {
        let file = gc(|g| g.max_concurrency = Some(2));
        let env = env_of(&[("CURL_RUNNER_MAX_CONCURRENCY", "4")]);
        let r = resolve(file, None, &env, &CliOptions::default());
        assert_eq!(r.max_concurrency, Some(4));
    }

    #[test]
    fn cli_beats_env() {
        let env = env_of(&[("CURL_RUNNER_MAX_CONCURRENCY", "4")]);
        let r = resolve(
            GlobalConfig::default(),
            None,
            &env,
            &cli(|c| c.max_concurrency = Some(9)),
        );
        assert_eq!(r.max_concurrency, Some(9));
    }

    #[test]
    fn cli_beats_file_global() {
        let file = gc(|g| g.continue_on_error = Some(false));
        let r = resolve(
            file,
            None,
            &no_env,
            &cli(|c| c.continue_on_error = Some(true)),
        );
        assert_eq!(r.continue_on_error, Some(true));
        let file = gc(|g| {
            g.output = Some(OutputConfig {
                verbose: Some(true),
                show_body: Some(false),
                ..Default::default()
            })
        });
        let r = resolve(file, None, &no_env, &cli(|c| c.quiet = Some(true)));
        // CLI wins per field; untouched file fields survive.
        assert_eq!(out(&r).verbose, Some(false));
        assert_eq!(out(&r).quiet, Some(true));
        assert_eq!(out(&r).show_body, Some(false));
    }

    #[test]
    fn full_chain_project_file_env_cli() {
        let project = gc(|g| {
            g.execution = Some("sequential".into());
            g.http2 = Some(false);
            g.dry_run = Some(false);
        });
        let file = gc(|g| {
            g.execution = Some("parallel".into());
            g.http2 = Some(true);
        });
        let env = env_of(&[("CURL_RUNNER_HTTP2", "false")]);
        let r = resolve(file, Some(project), &env, &cli(|c| c.dry_run = Some(true)));
        assert_eq!(r.execution.as_deref(), Some("parallel")); // file > project
        assert_eq!(r.http2, Some(false)); // env > file
        assert_eq!(r.dry_run, Some(true)); // cli > project
    }

    #[test]
    fn default_allowed_protocols_always_present() {
        let r = resolve(
            GlobalConfig::default(),
            None,
            &no_env,
            &CliOptions::default(),
        );
        assert_eq!(
            r.security.unwrap().allowed_protocols,
            Some(vec!["http".to_string(), "https".to_string()])
        );
        let r = resolve(
            GlobalConfig::default(),
            None,
            &no_env,
            &cli(|c| c.allow_protocols = Some(vec!["ftp".into()])),
        );
        assert_eq!(
            r.security.unwrap().allowed_protocols,
            Some(vec!["ftp".to_string()])
        );
        // Empty list in project config falls back to the default.
        let project = GlobalConfig {
            security: Some(SecurityConfig {
                allowed_protocols: Some(vec![]),
                allow_paths: Some(true),
            }),
            ..Default::default()
        };
        let r = resolve(
            GlobalConfig::default(),
            Some(project),
            &no_env,
            &CliOptions::default(),
        );
        let s = r.security.unwrap();
        assert_eq!(s.allowed_protocols.unwrap().len(), 2);
        assert_eq!(s.allow_paths, Some(true));
    }

    #[test]
    fn cli_scalars_and_output() {
        let r = resolve(
            GlobalConfig::default(),
            None,
            &no_env,
            &cli(|c| {
                c.execution = Some("parallel".into());
                c.continue_on_error = Some(true);
                c.output = Some("res.json".into());
                c.output_format = Some("json".into());
                c.pretty_level = Some("minimal".into());
                c.show_headers = Some(true);
                c.show_body = Some(false);
                c.show_metrics = Some(true);
                c.verbose = Some(true);
                c.env = Some("staging".into());
                c.no_redact = Some(true);
            }),
        );
        assert_eq!(r.execution.as_deref(), Some("parallel"));
        assert_eq!(r.continue_on_error, Some(true));
        let o = out(&r);
        assert_eq!(o.save_to_file.as_deref(), Some("res.json"));
        assert_eq!(o.format.as_deref(), Some("json"));
        assert_eq!(o.pretty_level.as_deref(), Some("minimal"));
        assert_eq!(o.show_headers, Some(true));
        assert_eq!(o.show_body, Some(false));
        assert_eq!(o.show_metrics, Some(true));
        assert_eq!(o.verbose, Some(true));
        let e = r.env.unwrap();
        assert_eq!(e.environment.as_deref(), Some("staging"));
        assert_eq!(e.redact_secrets, Some(false));
    }

    #[test]
    fn dry_run_and_http2_mirror_into_output_and_defaults() {
        let r = resolve(
            GlobalConfig::default(),
            None,
            &no_env,
            &cli(|c| {
                c.dry_run = Some(true);
                c.http2 = Some(true);
            }),
        );
        assert_eq!(r.dry_run, Some(true));
        assert_eq!(out(&r).dry_run, Some(true));
        assert_eq!(r.http2, Some(true));
        assert_eq!(r.defaults.unwrap().http2, Some(true));
    }

    #[test]
    fn timeout_and_retries_are_milliseconds_and_truthy_only() {
        let r = resolve(
            GlobalConfig::default(),
            None,
            &no_env,
            &cli(|c| {
                c.timeout = Some(30000);
                c.retries = Some(3);
                c.retry_delay = Some(5000);
            }),
        );
        let d = r.defaults.unwrap();
        assert_eq!(d.timeout, Some(30000));
        let retry = d.retry.unwrap();
        assert_eq!((retry.count, retry.delay), (Some(3), Some(5000)));

        // Zero values are falsy in TS and change nothing.
        let project = GlobalConfig {
            defaults: Some(RequestConfig {
                timeout: Some(1000),
                retry: Some(RetryConfig {
                    count: Some(5),
                    ..Default::default()
                }),
                ..Default::default()
            }),
            ..Default::default()
        };
        let r = resolve(
            GlobalConfig::default(),
            Some(project.clone()),
            &no_env,
            &cli(|c| {
                c.timeout = Some(0);
                c.retries = Some(0);
                c.retry_delay = Some(0);
            }),
        );
        assert_eq!(r.defaults, project.defaults);

        // --no-retry forces count 0 and keeps the rest of the retry block.
        let r = resolve(
            GlobalConfig::default(),
            Some(GlobalConfig {
                defaults: Some(RequestConfig {
                    retry: Some(RetryConfig {
                        count: Some(5),
                        delay: Some(7),
                        ..Default::default()
                    }),
                    ..Default::default()
                }),
                ..Default::default()
            }),
            &no_env,
            &cli(|c| c.no_retry = Some(true)),
        );
        let retry = r.defaults.unwrap().retry.unwrap();
        assert_eq!((retry.count, retry.delay), (Some(0), Some(7)));
    }

    #[test]
    fn connection_pool_flags_enable_pool() {
        let r = resolve(
            GlobalConfig::default(),
            None,
            &no_env,
            &cli(|c| {
                c.max_streams = Some(20);
                c.keepalive_time = Some(30);
                c.connect_timeout = Some(5);
            }),
        );
        let p = r.connection_pool.unwrap();
        assert_eq!(p.enabled, Some(true));
        assert_eq!(p.max_streams_per_host, Some(20));
        assert_eq!(p.keepalive_time, Some(30));
        assert_eq!(p.connect_timeout, Some(5));
        let r = resolve(
            GlobalConfig::default(),
            None,
            &no_env,
            &cli(|c| c.connection_pool = Some(true)),
        );
        assert_eq!(r.connection_pool.unwrap().enabled, Some(true));
    }

    #[test]
    fn ci_snapshot_and_diff_flags() {
        let r = resolve(
            GlobalConfig::default(),
            None,
            &no_env,
            &cli(|c| {
                c.strict_exit = Some(true);
                c.fail_on = Some(2);
                c.fail_on_percentage = Some(10.0);
                c.snapshot_update = Some("failing".into());
                c.snapshot_dir = Some("snaps".into());
                c.snapshot_ci = Some(true);
                c.diff_save = Some(true);
                c.diff_label = Some("staging".into());
                c.diff_compare = Some("prod".into());
                c.diff_dir = Some("bl".into());
                c.diff_output = Some("json".into());
            }),
        );
        let ci = r.ci.unwrap();
        assert_eq!(
            (ci.strict_exit, ci.fail_on, ci.fail_on_percentage),
            (Some(true), Some(2), Some(10.0))
        );
        let s = r.snapshot.unwrap();
        assert_eq!(s.enabled, Some(true));
        assert_eq!(s.update_mode.as_deref(), Some("failing"));
        assert_eq!(s.dir.as_deref(), Some("snaps"));
        assert_eq!(s.ci, Some(true));
        let d = r.diff.unwrap();
        assert_eq!(d.enabled, Some(true));
        assert_eq!(d.save, Some(true));
        assert_eq!(d.label.as_deref(), Some("staging"));
        assert_eq!(d.compare_with.as_deref(), Some("prod"));
        assert_eq!(d.dir.as_deref(), Some("bl"));
        assert_eq!(d.output_format.as_deref(), Some("json"));
    }

    #[test]
    fn merge_is_field_wise_per_group_and_retry_merges() {
        let base = GlobalConfig {
            output: Some(OutputConfig {
                verbose: Some(true),
                show_body: Some(true),
                ..Default::default()
            }),
            variables: Some([("A".to_string(), "1".to_string())].into_iter().collect()),
            defaults: Some(RequestConfig {
                timeout: Some(1),
                retry: Some(RetryConfig {
                    count: Some(2),
                    ..Default::default()
                }),
                ..Default::default()
            }),
            ..Default::default()
        };
        let over = GlobalConfig {
            output: Some(OutputConfig {
                verbose: Some(false),
                ..Default::default()
            }),
            variables: Some([("B".to_string(), "2".to_string())].into_iter().collect()),
            defaults: Some(RequestConfig {
                retry: Some(RetryConfig {
                    delay: Some(9),
                    ..Default::default()
                }),
                ..Default::default()
            }),
            ..Default::default()
        };
        let m = merge_global_configs(&base, &over);
        let o = out(&m);
        assert_eq!((o.verbose, o.show_body), (Some(false), Some(true)));
        assert_eq!(m.variables.unwrap().len(), 2);
        let d = m.defaults.unwrap();
        assert_eq!(d.timeout, Some(1));
        let r = d.retry.unwrap();
        assert_eq!((r.count, r.delay), (Some(2), Some(9)));
    }

    #[test]
    fn merge_replaces_profile_and_ssl_whole() {
        let base = GlobalConfig {
            profile: Some(ProfileConfig {
                iterations: 5,
                warmup: Some(2),
                ..Default::default()
            }),
            ..Default::default()
        };
        let over = GlobalConfig {
            profile: Some(ProfileConfig {
                iterations: 7,
                ..Default::default()
            }),
            ..Default::default()
        };
        let m = merge_global_configs(&base, &over);
        assert_eq!(m.profile.unwrap().warmup, None);
    }

    // --- mode / profile / watch ---
    #[test]
    fn execution_mode() {
        let none = GlobalConfig::default();
        assert_eq!(
            determine_execution_mode(&CliOptions::default(), &none),
            ExecutionMode::Normal
        );
        assert_eq!(
            determine_execution_mode(&cli(|c| c.watch = Some(true)), &none),
            ExecutionMode::Watch
        );
        assert_eq!(
            determine_execution_mode(&cli(|c| c.profile = Some(5)), &none),
            ExecutionMode::Profile
        );
        // profile wins over watch
        assert_eq!(
            determine_execution_mode(
                &cli(|c| {
                    c.profile = Some(5);
                    c.watch = Some(true)
                }),
                &none
            ),
            ExecutionMode::Profile
        );
        let cfg = GlobalConfig {
            watch: Some(WatchConfig {
                enabled: Some(true),
                ..Default::default()
            }),
            profile: Some(ProfileConfig {
                iterations: 0,
                ..Default::default()
            }),
            ..Default::default()
        };
        assert_eq!(
            determine_execution_mode(&CliOptions::default(), &cfg),
            ExecutionMode::Watch
        );
        // `--profile 0` is `0 ?? cfg` = 0: not profile.
        let cfg = GlobalConfig {
            profile: Some(ProfileConfig {
                iterations: 9,
                ..Default::default()
            }),
            ..Default::default()
        };
        assert_eq!(
            determine_execution_mode(&cli(|c| c.profile = Some(0)), &cfg),
            ExecutionMode::Normal
        );
    }

    #[test]
    fn profile_and_watch_config_defaults_and_overrides() {
        let p = build_profile_config(&CliOptions::default(), &GlobalConfig::default());
        assert_eq!(
            (
                p.iterations,
                p.warmup,
                p.concurrency,
                p.histogram,
                p.export_file
            ),
            (10, Some(1), Some(1), Some(false), None)
        );
        let p = build_profile_config(
            &cli(|c| {
                c.profile = Some(50);
                c.profile_export = Some("r.csv".into());
                c.profile_histogram = Some(true)
            }),
            &GlobalConfig::default(),
        );
        assert_eq!((p.iterations, p.histogram), (50, Some(true)));
        assert_eq!(p.export_file.as_deref(), Some("r.csv"));
        let w = build_watch_config(&CliOptions::default(), &GlobalConfig::default());
        assert_eq!(
            (w.enabled, w.debounce, w.clear),
            (Some(true), Some(300), Some(true))
        );
        let w = build_watch_config(
            &cli(|c| {
                c.watch_debounce = Some(500);
                c.watch_clear = Some(false)
            }),
            &GlobalConfig::default(),
        );
        assert_eq!((w.debounce, w.clear), (Some(500), Some(false)));
    }

    // --- file loader ---
    #[test]
    fn config_file_discovery_order_and_global_extraction() {
        let dir = tempfile::tempdir().unwrap();
        let r = load_config_file(dir.path());
        assert_eq!(r.path, None);
        assert_eq!(r.config, GlobalConfig::default());
        assert!(r.messages.is_empty());

        std::fs::write(
            dir.path().join(".curl-runner.yml"),
            "global:\n  execution: parallel\n",
        )
        .unwrap();
        std::fs::write(
            dir.path().join("curl-runner.yml"),
            "global:\n  maxConcurrency: 3\n",
        )
        .unwrap();
        let r = load_config_file(dir.path());
        assert_eq!(r.path.as_deref(), Some("curl-runner.yml"));
        assert_eq!(r.config.max_concurrency, Some(3));
        assert_eq!(
            r.messages,
            vec![ConfigMessage::Info(
                "Loaded configuration from curl-runner.yml".into()
            )]
        );

        std::fs::write(
            dir.path().join("curl-runner.yaml"),
            "request:\n  url: http://x\n",
        )
        .unwrap();
        // First candidate has no `global`: loaded as empty, no fallthrough.
        let r = load_config_file(dir.path());
        assert_eq!(r.path.as_deref(), Some("curl-runner.yaml"));
        assert_eq!(r.config, GlobalConfig::default());
    }

    #[test]
    fn broken_config_file_warns_and_falls_through() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("curl-runner.yaml"), "global: [unclosed\n").unwrap();
        std::fs::write(
            dir.path().join("curl-runner.yml"),
            "global:\n  http2: true\n",
        )
        .unwrap();
        let r = load_config_file(dir.path());
        assert_eq!(r.path.as_deref(), Some("curl-runner.yml"));
        assert_eq!(r.config.http2, Some(true));
        assert!(matches!(&r.messages[0], ConfigMessage::Warning(m)
            if m.starts_with("Failed to load configuration from curl-runner.yaml: ")));
        assert!(matches!(&r.messages[1], ConfigMessage::Info(_)));
    }

    #[test]
    fn directory_named_like_config_is_skipped() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir(dir.path().join("curl-runner.yaml")).unwrap();
        let r = load_config_file(dir.path());
        assert_eq!(r.path, None);
        assert!(r.messages.is_empty());
    }

    #[test]
    fn defaults_match_defaults_ts() {
        assert_eq!(defaults::TIMEOUT, 30000);
        assert_eq!(defaults::RETRY_DELAY, 1000);
        assert_eq!(defaults::SNAPSHOT_DIR, "__snapshots__");
        assert_eq!(defaults::BASELINE_DIR, "__baselines__");
        assert_eq!((defaults::SHOW_BODY, defaults::WATCH_CLEAR), (true, true));
        assert_eq!(defaults::KEEPALIVE_TIME, 60);
    }

    fn parse_file(yaml: &str, layers: &DefaultsLayers) -> parser::ParsedFile {
        parser::from_raw_with_defaults(
            parser::parse_raw(yaml).unwrap(),
            Path::new("t.yaml"),
            layers,
        )
        .unwrap()
    }

    #[test]
    fn cli_defaults_beat_file_defaults_end_to_end() {
        let yaml = "global:\n  defaults:\n    timeout: 5000\n    retry: { count: 1, delay: 100 }\n  output:\n    prettyLevel: detailed\nrequests:\n  - url: http://a\n  - url: http://b\n    timeout: 1000\n";
        let file = parser::global_from_raw(&parser::parse_raw(yaml).unwrap()).unwrap();
        let c = cli(|c| {
            c.timeout = Some(30000);
            c.retries = Some(3);
            c.pretty_level = Some("minimal".into());
        });
        let layers = defaults_layers(None, &file, &no_env, &c);
        let f = parse_file(yaml, &layers);
        // file `global.defaults.timeout: 5000` + `--timeout 30000` -> 30000
        assert_eq!(f.requests[0].timeout, Some(30000));
        // request `timeout: 1000` + `--timeout 30000` -> 1000
        assert_eq!(f.requests[1].timeout, Some(1000));
        // file retry {1,100} + `--retries 3` -> count 3, delay 100
        let r = f.requests[0].retry.as_ref().unwrap();
        assert_eq!((r.count, r.delay), (Some(3), Some(100)));
        // file prettyLevel detailed + `--pretty-level minimal` -> minimal
        let o = f.requests[0].source_output_config.as_ref().unwrap();
        assert_eq!(o.pretty_level.as_deref(), Some("minimal"));
    }

    #[test]
    fn retry_delay_flag_does_not_wipe_count_and_env_layer_counts() {
        let yaml = "global:\n  defaults:\n    retry: { count: 2, delay: 100 }\nrequest:\n  url: http://a\n";
        let file = parser::global_from_raw(&parser::parse_raw(yaml).unwrap()).unwrap();
        let env = env_of(&[("CURL_RUNNER_TIMEOUT", "777")]);
        let layers = defaults_layers(None, &file, &env, &cli(|c| c.retry_delay = Some(5000)));
        let f = parse_file(yaml, &layers);
        let r = f.requests[0].retry.as_ref().unwrap();
        assert_eq!((r.count, r.delay), (Some(2), Some(5000)));
        assert_eq!(f.requests[0].timeout, Some(777));
    }

    fn http2_of(yaml: &str, c: &CliOptions) -> Vec<Option<bool>> {
        let file = parser::global_from_raw(&parser::parse_raw(yaml).unwrap()).unwrap();
        let layers = defaults_layers(None, &file, &no_env, c);
        parse_file(yaml, &layers)
            .requests
            .iter()
            .map(|r| r.http2)
            .collect()
    }

    #[test]
    fn global_http2_applies_to_every_request() {
        let y = "global:\n  http2: true\nrequests:\n  - url: http://a\n  - url: http://b\n";
        assert_eq!(http2_of(y, &cli(|_| {})), vec![Some(true), Some(true)]);
    }

    #[test]
    fn request_http2_false_beats_global_http2() {
        let y = "global:\n  http2: true\nrequests:\n  - url: http://a\n    http2: false\n  - url: http://b\n";
        assert_eq!(http2_of(y, &cli(|_| {})), vec![Some(false), Some(true)]);
    }

    #[test]
    fn defaults_http2_beats_global_http2_and_still_works_alone() {
        let y =
            "global:\n  http2: true\n  defaults:\n    http2: false\nrequests:\n  - url: http://a\n";
        assert_eq!(http2_of(y, &cli(|_| {})), vec![Some(false)]);
        let y = "global:\n  defaults:\n    http2: true\nrequests:\n  - url: http://a\n";
        assert_eq!(http2_of(y, &cli(|_| {})), vec![Some(true)]);
    }

    #[test]
    fn cli_http2_is_a_default_above_global_and_below_request() {
        let y = "global:\n  http2: false\n  defaults:\n    http2: false\nrequests:\n  - url: http://a\n  - url: http://b\n    http2: false\n";
        let on = cli(|c| c.http2 = Some(true));
        assert_eq!(http2_of(y, &on), vec![Some(true), Some(false)]);
    }

    #[test]
    fn config_file_only_needs_a_valid_global() {
        let dir = tempfile::tempdir().unwrap();
        // Valid YAML with a request whose types are wrong: TS ignores everything but `global`.
        std::fs::write(
            dir.path().join("curl-runner.yaml"),
            "global:\n  http2: true\nrequests:\n  - url: [1, 2]\n    timeout: abc\n",
        )
        .unwrap();
        let r = load_config_file(dir.path());
        assert_eq!(r.path.as_deref(), Some("curl-runner.yaml"));
        assert_eq!(r.config.http2, Some(true));
    }

    #[test]
    fn config_file_with_bad_global_type_warns_and_is_skipped() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("curl-runner.yaml"),
            "global:\n  maxConcurrency: abc\n",
        )
        .unwrap();
        let r = load_config_file(dir.path());
        assert_eq!(r.path, None);
        assert!(
            matches!(&r.messages[0], ConfigMessage::Warning(m) if m.contains("global.maxConcurrency"))
        );
    }
}
