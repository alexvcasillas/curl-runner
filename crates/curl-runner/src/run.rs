//! The `curl-runner [files...]` command (TS `CurlRunnerCLI.run` +
//! `executeMain` + `executeRequests`).
//!
//! Like TS, several files run as separate groups: each gets its own header,
//! executor run (store, stop-on-error) and summary, then a global summary is
//! printed. Unlike TS (which rewrote `--output` after every file, keeping
//! only the last one), the results file is written once at the end with the
//! aggregate summary of all files (also after Ctrl-C). The exit code is
//! computed on the same aggregate, as in TS.

use crate::args::{self, Command};
use crate::colors;
use crate::exit;
use crate::output::{self, Console, Logger, OutCfg, Reporter};
use crate::watch;
use curl_runner_core::config::{self, CliOptions, ConfigMessage, ExecutionMode};
use curl_runner_core::discover::find_yaml_files;
use curl_runner_core::executor::{self, Executor, PreparedRequest, create_summary};
use curl_runner_core::interpolate;
use curl_runner_core::model::{ExecutionSummary, GlobalConfig, Json, OutputConfig, RequestConfig};
use curl_runner_core::parser;
use curl_runner_core::security::redact::Redactor;
use curl_runner_core::snapshot::ManagerHook;
use curl_runner_core::{VERSION, env as dotenv};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

const HELP: &str = include_str!("help.txt");

/// Exit code when interrupted by Ctrl-C (128 + SIGINT).
pub const EXIT_INTERRUPTED: i32 = 130;

/// Run the CLI for `argv` (without the program name); returns the exit code.
pub fn main(argv: Vec<String>) -> i32 {
    let env = |k: &str| std::env::var(k).ok();
    // Update notice (TS: every mode but version/help; `upgrade` does its own check).
    let update = match args::parse(&argv).map(|p| p.command) {
        Ok(Command::Version | Command::Help | Command::Upgrade) | Err(_) => None,
        Ok(_) => {
            use std::io::IsTerminal;
            crate::version_check::start_from_env(&env, std::io::stdout().is_terminal())
        }
    };
    let code = run_command(argv);
    if let Some(u) = update {
        u.finish(colors::enabled_from_env(env), &mut Console::stdio());
    }
    code
}

fn run_command(argv: Vec<String>) -> i32 {
    let parsed = match args::parse(&argv) {
        Ok(p) => p,
        Err(e) => {
            eprintln!("{e}");
            return 1;
        }
    };
    match parsed.command {
        Command::Version => {
            println!("{VERSION}");
            return 0;
        }
        Command::Help => {
            println!("\n{HELP}");
            return 0;
        }
        Command::Run
        | Command::Diff
        | Command::Convert
        | Command::Validate
        | Command::Wizard
        | Command::Upgrade => {}
    };
    let env = |k: &str| std::env::var(k).ok();
    let color = colors::enabled_from_env(env);
    if parsed.command == Command::Upgrade {
        let mut console = Console::stdio();
        let rest = &parsed.rest[1..];
        if rest.iter().any(|a| a == "--help" || a == "-h") {
            crate::upgrade::help(color, &mut console);
            return 0;
        }
        return crate::upgrade::run(
            rest,
            color,
            &crate::upgrade::Ctx::from_env(&env),
            &mut console,
        );
    }
    if parsed.command == Command::Validate {
        return crate::validate_cmd::run(&parsed.rest, color, &mut Console::stdio());
    }
    if parsed.command == Command::Wizard {
        return crate::wizard::run(&parsed.rest, color, &mut Console::stdio());
    }
    if parsed.command == Command::Convert {
        return crate::convert_cmd::run(&parsed.rest, color, &mut Console::stdio());
    }
    let cwd = match std::env::current_dir() {
        Ok(d) => d,
        Err(e) => {
            eprintln!("✗ Cannot read the working directory: {e}");
            return 1;
        }
    };
    if parsed.command == Command::Diff {
        return crate::diff_mode::run_subcommand(
            &parsed.rest,
            &parsed.options,
            &cwd,
            color,
            &mut Console::stdio(),
        );
    }
    let interrupted = Arc::new(AtomicBool::new(false));
    let current: CancelSlot = Arc::default();
    let saving = Arc::new(AtomicBool::new(false));
    let watching = Arc::new(AtomicBool::new(false));
    {
        let (interrupted, current, saving, watching) = (
            interrupted.clone(),
            current.clone(),
            saving.clone(),
            watching.clone(),
        );
        // A second Ctrl-C exits at once, except while the results file is
        // being written (the run exits 130 right after the write).
        let _ = ctrlc::set_handler(move || {
            // Watch mode: TS exits 0 on SIGINT, silently, even mid-run (the
            // results file is written atomically, so nothing is truncated).
            if watching.load(Ordering::SeqCst) {
                std::process::exit(0);
            }
            if interrupted.swap(true, Ordering::SeqCst) {
                if !saving.load(Ordering::SeqCst) {
                    std::process::exit(EXIT_INTERRUPTED);
                }
                return;
            }
            if let Some(c) = current.lock().ok().and_then(|c| c.clone()) {
                c.store(true, Ordering::SeqCst);
            }
        });
    }
    let quiet_pre = argv.iter().any(|a| a == "--quiet" || a == "-q");
    let ctx = RunCtx {
        cwd,
        color,
        interrupted,
        current,
        saving,
        watching,
    };
    ctx.run(&parsed.options, quiet_pre, Console::stdio())
}

/// Watch mode: SIGTERM exits 0 like TS (`process.exit(0)`). Registered only
/// here, so SIGTERM keeps its default behavior in normal runs.
#[cfg(unix)]
fn exit_on_sigterm() {
    if let Ok(mut signals) = signal_hook::iterator::Signals::new([signal_hook::consts::SIGTERM]) {
        std::thread::spawn(move || {
            if signals.forever().next().is_some() {
                std::process::exit(0);
            }
        });
    }
}

#[cfg(not(unix))]
fn exit_on_sigterm() {}

/// Watch-mode output (TS `Logger` watch calls and `console.clear`).
struct WatchUi<'a> {
    log: &'a Logger,
    console: Console,
}

impl watch::Ui for WatchUi<'_> {
    fn clear(&mut self) {
        use std::io::{IsTerminal, Write};
        // Like `console.clear()`: only a terminal is cleared.
        if std::io::stdout().is_terminal() {
            print!("\x1b[1;1H\x1b[0J");
            let _ = std::io::stdout().flush();
        }
    }
    fn file_changed(&mut self, name: &str) {
        self.log.log_file_changed(&mut self.console, name);
    }
    fn watch_ready(&mut self) {
        self.log.log_watch_ready(&mut self.console);
    }
    fn watching(&mut self, files: &[String]) {
        self.log.log_watch(&mut self.console, files);
    }
    fn error(&mut self, msg: &str) {
        self.log.log_error(&mut self.console, msg);
    }
    fn warning(&mut self, msg: &str) {
        self.log.log_warning(&mut self.console, msg);
    }
}

pub(crate) type CancelSlot = Arc<Mutex<Option<Arc<AtomicBool>>>>;

struct RunCtx {
    cwd: PathBuf,
    color: bool,
    interrupted: Arc<AtomicBool>,
    current: CancelSlot,
    saving: Arc<AtomicBool>,
    /// Watch mode: Ctrl-C exits 0 at once (TS `process.exit(0)`).
    watching: Arc<AtomicBool>,
}

impl RunCtx {
    fn run(&self, cli: &CliOptions, quiet_pre: bool, mut console: Console) -> i32 {
        let env = |k: &str| std::env::var(k).ok();
        let redactor = Arc::new(Mutex::new(Redactor::default()));
        let pre = Logger::new(
            OutCfg::from_config(None),
            self.color,
            Some(redactor.clone()),
        );

        let project = config::load_config_file(&self.cwd);
        if !quiet_pre {
            for m in &project.messages {
                match m {
                    ConfigMessage::Info(s) => pre.log_info(&mut console, s),
                    ConfigMessage::Warning(s) => pre.log_warning(&mut console, s),
                }
            }
        }
        let project_cfg = project.config;
        // TS: the CLI logger uses the resolved config before test files load.
        let resolved = config::resolve(
            GlobalConfig::default(),
            Some(project_cfg.clone()),
            &env,
            cli,
        );
        let log = Logger::new(
            OutCfg::from_config(resolved.output.as_ref()),
            self.color,
            Some(redactor.clone()),
        );
        let mode = config::determine_execution_mode(cli, &resolved);
        let files = find_yaml_files(&cli.files, cli.all == Some(true), &self.cwd);
        if files.is_empty() {
            log.log_error(&mut console, "No YAML files found");
            return 1;
        }
        log.log_info(&mut console, &format!("Found {} YAML file(s)", files.len()));
        for f in &files {
            log.log_info(&mut console, &format!("Processing: {f}"));
        }
        let paths: Vec<PathBuf> = files.iter().map(PathBuf::from).collect();
        let run = match executor::load_run(&paths, Some(&project_cfg), cli, &env, &self.cwd) {
            Ok(r) => r,
            Err(e) => {
                log.log_error(&mut console, &e.to_string());
                return 1;
            }
        };
        if run.requests.is_empty() {
            log.log_error(&mut console, "No requests found in YAML files");
            return 1;
        }
        if mode == ExecutionMode::Watch {
            return self.watch(cli, &project_cfg, &files, run, &redactor, &log);
        }
        self.execute_run(cli, mode, &files, run, &redactor, &log, console)
    }

    /// Watch mode: run now, then again on every change to `files`. Files
    /// are reloaded from scratch for each run, as in TS (a run that fails to
    /// load is reported and watching goes on). Never returns normally: Ctrl-C
    /// exits 0 (see `watching`).
    fn watch(
        &self,
        cli: &CliOptions,
        project_cfg: &GlobalConfig,
        files: &[String],
        first: executor::LoadedRun,
        redactor: &Arc<Mutex<Redactor>>,
        log: &Logger,
    ) -> i32 {
        let wc = config::build_watch_config(cli, &first.global);
        let cfg = watch::Config {
            debounce: std::time::Duration::from_millis(wc.debounce.unwrap_or(300)),
            clear: wc.clear != Some(false),
        };
        self.watching.store(true, Ordering::SeqCst);
        exit_on_sigterm();
        let paths: Vec<PathBuf> = files.iter().map(PathBuf::from).collect();
        let env = |k: &str| std::env::var(k).ok();
        let mut first = Some(first);
        let mut on_run = || -> Result<(), String> {
            let run = match first.take() {
                Some(r) => r,
                None => {
                    let r = executor::load_run(&paths, Some(project_cfg), cli, &env, &self.cwd)
                        .map_err(|e| e.to_string())?;
                    if r.requests.is_empty() {
                        return Err("No requests found in YAML files".into());
                    }
                    r
                }
            };
            self.execute_run(
                cli,
                ExecutionMode::Watch,
                files,
                run,
                redactor,
                log,
                Console::stdio(),
            );
            Ok(())
        };
        let mut ui = WatchUi {
            log,
            console: Console::stdio(),
        };
        watch::start(
            files,
            &cfg,
            &mut ui,
            &mut on_run,
            std::sync::mpsc::channel(),
        );
        0
    }

    /// Everything after loading: run the requests (or profile them) and
    /// compute the exit code.
    #[allow(clippy::too_many_arguments)]
    fn execute_run(
        &self,
        cli: &CliOptions,
        mode: ExecutionMode,
        files: &[String],
        run: executor::LoadedRun,
        redactor: &Arc<Mutex<Redactor>>,
        log: &Logger,
        mut console: Console,
    ) -> i32 {
        let global = run.global;
        if let Ok(mut r) = redactor.lock() {
            *r = env_redactor(cli, &global, &run.dotenv);
            // TS registers every request's inline secrets before running
            // (cli.ts registerRequestSecrets), so a later request's token
            // echoed by an earlier response is masked too. Values that only
            // resolve from the store are registered by the executor.
            register_all_request_secrets(&mut r, &run.requests);
        }

        if mode == ExecutionMode::Profile {
            let cfg = config::build_profile_config(cli, &global);
            let plog = Logger::new(
                OutCfg::from_config(global.output.as_ref()),
                self.color,
                Some(redactor.clone()),
            );
            return crate::profile::run(
                &run.requests,
                &global,
                &cfg,
                &self.cwd,
                &plog,
                &mut console,
                &self.interrupted,
                &self.current,
            );
        }

        let groups = group_by_file(files, run.requests);
        let mut reporter =
            Reporter::new(console, global.output.clone(), self.color, redactor.clone());
        let summary = self.execute(&groups, &global, run.dotenv, redactor, &mut reporter, log);
        // One results file for the whole run (also after Ctrl-C).
        let save_log = reporter.global_logger();
        self.saving.store(true, Ordering::SeqCst);
        self.save(&summary, &global, redactor, &mut reporter, &save_log);
        self.saving.store(false, Ordering::SeqCst);

        // TS handleDiffMode runs after the results file is written (the
        // executor saves it): first file only, results of every file.
        if crate::diff_mode::wanted(global.diff.as_ref())
            && !self.interrupted.load(Ordering::SeqCst)
            && let (Some(dc), Some(first)) = (global.diff.as_ref(), files.first())
            && let Err(e) = crate::diff_mode::handle_diff_mode(
                first,
                &summary.results,
                dc,
                &self.cwd,
                self.color,
                log,
                &mut reporter.console,
                redactor,
            )
        {
            log.log_error(&mut reporter.console, &e);
            reporter.console.flush();
            return 1;
        }

        if self.interrupted.load(Ordering::SeqCst) {
            log.log_error(&mut reporter.console, "Interrupted");
            reporter.console.flush();
            return EXIT_INTERRUPTED;
        }
        reporter.console.flush();
        exit::determine(&summary, &global)
    }

    /// The reference multi-file run semantics (TS `executeMain` loop): one
    /// `Executor` per file group, each with its own store, the process env +
    /// `.env` lookup, the shared redactor and the snapshot hook; a failure
    /// stops only its own group (unless `continueOnError`), the next group
    /// still runs; results and durations aggregate into one summary. Core has
    /// no equivalent on purpose; embedders should mirror this.
    fn execute(
        &self,
        groups: &[(String, Vec<PreparedRequest>)],
        global: &GlobalConfig,
        dotenv: indexmap::IndexMap<String, String>,
        redactor: &Arc<Mutex<Redactor>>,
        rep: &mut Reporter,
        cli_log: &Logger,
    ) -> ExecutionSummary {
        let multi = groups.len() > 1;
        let quiet = global.output.as_ref().and_then(|o| o.quiet) == Some(true);
        let mode = global
            .execution
            .clone()
            .unwrap_or_else(|| "sequential".into());
        let mut all = Vec::new();
        let mut duration = 0.0;
        let mut single = None;
        for (i, (file, reqs)) in groups.iter().enumerate() {
            if self.interrupted.load(Ordering::SeqCst) {
                break;
            }
            let dotenv = dotenv.clone();
            let ex = Executor::new(global.clone())
                .with_cwd(&self.cwd)
                .with_env(move |k| interpolate::process_env(k).or_else(|| dotenv.get(k).cloned()))
                .with_redactor(redactor.clone())
                .with_snapshot_hook(Box::new(ManagerHook::from_global(
                    global.snapshot.as_ref(),
                    &self.cwd,
                )));
            let cancel = ex.cancel_handle();
            if let Ok(mut slot) = self.current.lock() {
                *slot = Some(cancel.clone());
            }
            if self.interrupted.load(Ordering::SeqCst) {
                cancel.store(true, Ordering::SeqCst);
            }
            let log = rep.global_logger();
            if multi {
                log.log_file_header(&mut rep.console, file, reqs.len());
            }
            log.log_execution_start(&mut rep.console, reqs.len(), &mode);
            let s = ex.run(reqs.clone(), rep);
            log.log_summary(&mut rep.console, &s, false);
            if multi && i + 1 < groups.len() && !quiet {
                rep.console.out("");
            }
            duration += s.duration;
            if multi {
                all.extend(s.results);
            } else {
                single = Some(s);
            }
        }
        if let Ok(mut slot) = self.current.lock() {
            *slot = None;
        }
        if let Some(s) = single {
            return s;
        }
        let summary = create_summary(all, duration);
        if multi {
            cli_log.log_summary(&mut rep.console, &summary, true);
        }
        summary
    }

    fn save(
        &self,
        s: &ExecutionSummary,
        global: &GlobalConfig,
        redactor: &Arc<Mutex<Redactor>>,
        rep: &mut Reporter,
        log: &Logger,
    ) {
        let wanted = global
            .output
            .as_ref()
            .and_then(|o| o.save_to_file.as_deref());
        if wanted.is_none_or(str::is_empty) {
            return;
        }
        let r = redactor.lock().map(|r| r.clone()).unwrap_or_default();
        let s = with_file_source_output(s);
        match output::save_summary(&s, global, &self.cwd, &r) {
            Ok(Some(_)) => {
                let file = global
                    .output
                    .as_ref()
                    .and_then(|o| o.save_to_file.as_deref())
                    .unwrap_or_default();
                log.log_info(&mut rep.console, &format!("Results saved to {file}"));
            }
            Ok(None) => {}
            Err(e) => log.log_error(&mut rep.console, &e),
        }
    }
}

/// Redactor with the `.env` `SECRET_*` values (TS resolver): their values
/// as found in the final `variables`, unless `--no-redact` or
/// `env.redactSecrets: false`.
fn env_redactor(
    cli: &CliOptions,
    global: &GlobalConfig,
    dotenv_vars: &indexmap::IndexMap<String, String>,
) -> Redactor {
    let enabled = cli.no_redact != Some(true)
        && global.env.as_ref().and_then(|e| e.redact_secrets) != Some(false);
    let mut r = Redactor::default();
    if enabled {
        let vars = global.variables.clone().unwrap_or_default();
        r.add_secrets(
            dotenv::secret_keys(dotenv_vars)
                .into_iter()
                .filter_map(|k| vars.get(&k).cloned()),
        );
    }
    r
}

/// The saved summary carries each request's `sourceOutputConfig` as TS
/// does: the file's own `global.output` (`{}` when absent). In Rust that
/// field also holds the CLI/env output overrides (precedence fix), which
/// must not leak into the results file.
fn with_file_source_output(s: &ExecutionSummary) -> ExecutionSummary {
    let mut by_file: std::collections::HashMap<String, OutputConfig> = Default::default();
    let mut s = s.clone();
    for r in &mut s.results {
        let (Some(_), Some(file)) = (&r.request.source_output_config, &r.request.source_file)
        else {
            continue;
        };
        let output = by_file
            .entry(file.clone())
            .or_insert_with(|| {
                parser::load_raw(Path::new(file))
                    .and_then(|raw| parser::global_from_raw(&raw))
                    .ok()
                    .and_then(|g| g.output)
                    .unwrap_or_default()
            })
            .clone();
        r.request.source_output_config = Some(output);
    }
    s
}

/// Register the inline secrets of every request that types after file-level
/// interpolation (raw requests whose typed fields still hold `${store.x}`
/// are skipped here; the executor registers them once resolved).
fn register_all_request_secrets(r: &mut Redactor, requests: &[PreparedRequest]) {
    let typed: Vec<RequestConfig> = requests
        .iter()
        .filter_map(|p| serde_json::from_value(p.raw.clone()).ok())
        .collect();
    r.register_request_secrets(&typed);
}

/// Requests per discovered file, in file order (files with no requests
/// keep an empty group, as in TS).
fn group_by_file(
    files: &[String],
    requests: Vec<PreparedRequest>,
) -> Vec<(String, Vec<PreparedRequest>)> {
    let mut groups: Vec<(String, Vec<PreparedRequest>)> =
        files.iter().map(|f| (f.clone(), Vec::new())).collect();
    for r in requests {
        let src = r.raw.get("sourceFile").and_then(Json::as_str).unwrap_or("");
        let at = groups
            .iter()
            .position(|(f, _)| Path::new(f) == Path::new(src))
            .unwrap_or(0);
        groups[at].1.push(r);
    }
    groups
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn pr(src: &str) -> PreparedRequest {
        PreparedRequest {
            raw: json!({"url": "x", "sourceFile": src}),
        }
    }

    #[test]
    fn groups_keep_file_order_and_empty_files() {
        let files = vec!["a.yaml".to_string(), "b.yaml".into(), "c.yaml".into()];
        let g = group_by_file(&files, vec![pr("a.yaml"), pr("c.yaml"), pr("a.yaml")]);
        let counts: Vec<(&str, usize)> = g.iter().map(|(f, r)| (f.as_str(), r.len())).collect();
        assert_eq!(counts, [("a.yaml", 2), ("b.yaml", 0), ("c.yaml", 1)]);
    }

    #[test]
    fn env_redactor_uses_secret_keys_unless_disabled() {
        let mut dot = indexmap::IndexMap::new();
        dot.insert("SECRET_TOKEN".to_string(), "abc123xyz".to_string());
        dot.insert("PLAIN".to_string(), "visible".to_string());
        let global = GlobalConfig {
            variables: Some(dot.clone()),
            ..Default::default()
        };
        let r = env_redactor(&CliOptions::default(), &global, &dot);
        assert_eq!(r.redact("abc123xyz visible"), "[REDACTED] visible");
        let off = CliOptions {
            no_redact: Some(true),
            ..Default::default()
        };
        assert_eq!(
            env_redactor(&off, &global, &dot).redact("abc123xyz"),
            "abc123xyz"
        );
    }
}
