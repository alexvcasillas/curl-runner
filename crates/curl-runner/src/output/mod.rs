//! Console output (`utils/logger.ts`): the [`Logger`] port, the run
//! [`Reporter`] (an executor `EventSink`) and the summary file.
//!
//! Every dynamic string (URLs, commands, bodies, headers, errors, stored
//! values) goes through the shared `Redactor` before it is colored.

pub mod diff;
pub mod format;
mod json;
mod pretty;
mod profile;
mod raw;
pub mod snapshot;
pub mod tree;

use crate::colors::{Color, colorize};
use curl_runner_core::executor::EventSink;
use curl_runner_core::interpolate::Store;
use curl_runner_core::model::{
    ExecutionResult, ExecutionSummary, GlobalConfig, OutputConfig, RequestConfig,
};
use curl_runner_core::security::redact::Redactor;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

/// Where output goes: stdout (`console.log`) and stderr (`console.error`/`warn`).
pub struct Console {
    out: Box<dyn Write + Send>,
    err: Box<dyn Write + Send>,
}

/// Captured output of a [`Console::capture`] console.
#[cfg(test)]
#[derive(Clone, Default)]
pub struct Captured {
    out: Arc<Mutex<Vec<u8>>>,
    err: Arc<Mutex<Vec<u8>>>,
}

#[cfg(test)]
impl Captured {
    pub fn stdout(&self) -> String {
        String::from_utf8_lossy(&self.out.lock().unwrap()).into_owned()
    }
    pub fn stderr(&self) -> String {
        String::from_utf8_lossy(&self.err.lock().unwrap()).into_owned()
    }
}

#[cfg(test)]
struct SharedBuf(Arc<Mutex<Vec<u8>>>);
#[cfg(test)]
impl Write for SharedBuf {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.0.lock().unwrap().extend_from_slice(buf);
        Ok(buf.len())
    }
    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

impl Console {
    pub fn stdio() -> Self {
        Console {
            out: Box::new(std::io::stdout()),
            err: Box::new(std::io::stderr()),
        }
    }

    /// In-memory console for tests.
    #[cfg(test)]
    pub fn capture() -> (Self, Captured) {
        let c = Captured::default();
        (
            Console {
                out: Box::new(SharedBuf(c.out.clone())),
                err: Box::new(SharedBuf(c.err.clone())),
            },
            c,
        )
    }

    /// One `console.log` line (write errors such as a closed pipe are ignored).
    pub fn out(&mut self, line: &str) {
        let _ = writeln!(self.out, "{line}");
    }

    /// One `console.error` / `console.warn` line.
    pub fn err(&mut self, line: &str) {
        let _ = writeln!(self.err, "{line}");
    }

    pub fn flush(&mut self) {
        let _ = self.out.flush();
        let _ = self.err.flush();
    }
}

/// Output settings with the TS `Logger` constructor defaults applied.
#[derive(Debug, Clone, PartialEq)]
pub struct OutCfg {
    pub verbose: bool,
    pub quiet: bool,
    pub show_headers: bool,
    /// `showBody !== false`
    pub show_body: bool,
    pub show_metrics: bool,
    pub format: String,
    pub pretty_level: String,
    pub dry_run: bool,
}

impl OutCfg {
    pub fn from_config(c: Option<&OutputConfig>) -> Self {
        let d = OutputConfig::default();
        let c = c.unwrap_or(&d);
        OutCfg {
            verbose: c.verbose.unwrap_or(false),
            quiet: c.quiet.unwrap_or(false),
            show_headers: c.show_headers.unwrap_or(false),
            show_body: c.show_body.unwrap_or(true),
            show_metrics: c.show_metrics.unwrap_or(false),
            format: c.format.clone().unwrap_or_else(|| "pretty".into()),
            pretty_level: c.pretty_level.clone().unwrap_or_else(|| "minimal".into()),
            dry_run: c.dry_run.unwrap_or(false),
        }
    }
}

/// `{...global.output, ...request.sourceOutputConfig}` (TS `mergeOutputConfig`).
pub fn merge_output(global: Option<&OutputConfig>, src: Option<&OutputConfig>) -> OutputConfig {
    let g = global.cloned().unwrap_or_default();
    let Some(s) = src else { return g };
    OutputConfig {
        verbose: s.verbose.or(g.verbose),
        quiet: s.quiet.or(g.quiet),
        show_headers: s.show_headers.or(g.show_headers),
        show_body: s.show_body.or(g.show_body),
        show_metrics: s.show_metrics.or(g.show_metrics),
        format: s.format.clone().or(g.format),
        pretty_level: s.pretty_level.clone().or(g.pretty_level),
        save_to_file: s.save_to_file.clone().or(g.save_to_file),
        dry_run: s.dry_run.or(g.dry_run),
    }
}

/// Port of the TS `Logger`.
#[derive(Clone)]
pub struct Logger {
    pub cfg: OutCfg,
    color: bool,
    redactor: Option<Arc<Mutex<Redactor>>>,
}

impl Logger {
    pub fn new(cfg: OutCfg, color: bool, redactor: Option<Arc<Mutex<Redactor>>>) -> Self {
        Logger {
            cfg,
            color,
            redactor,
        }
    }

    pub fn color(&self, text: &str, c: Color) -> String {
        if self.color {
            colorize(text, c)
        } else {
            text.to_string()
        }
    }

    /// Full redaction (explicit secrets + built-in token patterns): only
    /// where TS redacts, i.e. request URLs and curl commands.
    fn redact(&self, text: &str) -> String {
        match self.redactor.as_ref().and_then(|r| r.lock().ok()) {
            Some(r) => r.redact(text),
            None => text.to_string(),
        }
    }

    /// Explicit secrets only (`SECRET_*` values, registered request
    /// secrets): everything else that is printed. Token-like strings stay
    /// visible so CI output remains debuggable.
    fn secrets(&self, text: &str) -> String {
        match self.redactor.as_ref().and_then(|r| r.lock().ok()) {
            Some(r) => r.redact_secrets(text),
            None => text.to_string(),
        }
    }

    fn level(&self) -> &str {
        &self.cfg.pretty_level
    }

    fn is_pretty(&self) -> bool {
        self.cfg.format == "pretty"
    }

    pub fn should_show_output(&self) -> bool {
        if self.cfg.quiet || self.cfg.format == "raw" {
            return false;
        }
        self.is_pretty() || self.cfg.verbose
    }

    /// Shared shape of `shouldShowHeaders/Metrics/RequestDetails`.
    fn by_level(&self, flag: bool) -> bool {
        if !self.is_pretty() {
            return flag;
        }
        match self.level() {
            "minimal" => false,
            "detailed" => true,
            _ => flag,
        }
    }

    pub fn should_show_headers(&self) -> bool {
        self.by_level(self.cfg.show_headers)
    }
    pub fn should_show_body(&self) -> bool {
        self.by_level(self.cfg.show_body)
    }
    pub fn should_show_metrics(&self) -> bool {
        self.by_level(self.cfg.show_metrics)
    }
    pub fn should_show_request_details(&self) -> bool {
        self.by_level(self.cfg.verbose)
    }
    fn should_show_separators(&self) -> bool {
        !self.is_pretty() || self.level() != "minimal"
    }

    pub fn log_execution_start(&self, c: &mut Console, count: usize, mode: &str) {
        if !self.should_show_output() {
            return;
        }
        if self.should_show_separators() {
            c.out("");
            c.out(&self.color(
                &format!("Executing {count} request(s) in {mode} mode"),
                Color::Dim,
            ));
            c.out("");
        } else {
            c.out("");
        }
    }

    pub fn log_command(&self, c: &mut Console, command: &str) {
        if self.cfg.dry_run || self.should_show_request_details() {
            c.out(&self.color("  Command:", Color::Dim));
            c.out(&self.color(&format!("    {}", self.redact(command)), Color::Dim));
        }
    }

    pub fn log_retry(&self, c: &mut Console, attempt: u32, max: u64) {
        if !self.should_show_output() {
            return;
        }
        c.out(&self.color(&format!("  ↻ Retry {attempt}/{max}..."), Color::Yellow));
    }

    pub fn log_request_complete(&self, c: &mut Console, r: &ExecutionResult) {
        if self.cfg.quiet {
            return;
        }
        match self.cfg.format.as_str() {
            "raw" => self.raw_result(c, r),
            "json" => {
                let v = self.result_json(r);
                c.out(&self.secrets(&format::stringify(&v, Some(2))));
            }
            _ => {
                if self.should_show_output() {
                    self.pretty_result(c, r);
                }
            }
        }
    }

    pub fn log_summary(&self, c: &mut Console, s: &ExecutionSummary, global: bool) {
        if self.cfg.quiet {
            return;
        }
        match self.cfg.format.as_str() {
            "raw" => {}
            "json" => {
                let v = self.summary_json(s);
                c.out(&self.secrets(&format::stringify(&v, Some(2))));
            }
            _ => {
                if self.should_show_output() {
                    self.pretty_summary(c, s, global);
                }
            }
        }
    }

    pub fn log_error(&self, c: &mut Console, msg: &str) {
        c.err(&self.color(&format!("✗ {}", self.secrets(msg)), Color::Red));
    }

    pub fn log_warning(&self, c: &mut Console, msg: &str) {
        if self.should_show_output() {
            c.err(&self.color(&format!("⚠ {}", self.secrets(msg)), Color::Yellow));
        }
    }

    pub fn log_info(&self, c: &mut Console, msg: &str) {
        if self.should_show_output() {
            c.out(&self.color(&format!("ℹ {}", self.secrets(msg)), Color::Blue));
        }
    }

    /// `logWatch`: printed once the watchers are set up.
    pub fn log_watch(&self, c: &mut Console, files: &[String]) {
        if !self.should_show_output() {
            return;
        }
        c.out("");
        c.out(&format!(
            "{} {}",
            self.color("Watching for changes...", Color::Cyan),
            self.color("(press Ctrl+C to stop)", Color::Dim)
        ));
        let list = if files.len() <= 3 {
            files.join(", ")
        } else {
            format!("{} files", files.len())
        };
        c.out(&self.color(&format!("   Files: {list}"), Color::Dim));
        c.out("");
    }

    /// `logWatchReady`: printed after every run.
    pub fn log_watch_ready(&self, c: &mut Console) {
        if !self.should_show_output() {
            return;
        }
        c.out("");
        c.out(&self.color("Watching for changes...", Color::Cyan));
    }

    /// `logFileChanged`: separator, local `[HH:MM:SS]` time and the file.
    pub fn log_file_changed(&self, c: &mut Console, filename: &str) {
        if !self.should_show_output() {
            return;
        }
        let ts = chrono::Local::now().format("%H:%M:%S").to_string();
        c.out(&self.color(&"-".repeat(50), Color::Dim));
        c.out(&format!(
            "{} File changed: {}",
            self.color(&format!("[{ts}]"), Color::Dim),
            self.color(filename, Color::Yellow)
        ));
        c.out("");
    }

    #[allow(dead_code)] // watch/profile ports
    pub fn log_success(&self, c: &mut Console, msg: &str) {
        if self.should_show_output() {
            c.out(&self.color(&format!("✓ {msg}"), Color::Green));
        }
    }

    pub fn log_skipped(
        &self,
        c: &mut Console,
        req: &RequestConfig,
        index: usize,
        reason: Option<&str>,
    ) {
        if !self.should_show_output() {
            return;
        }
        if self.cfg.format == "json" {
            let v = self.skipped_json(req, reason);
            c.out(&self.secrets(&format::stringify(&v, Some(2))));
            return;
        }
        self.pretty_skipped(c, req, index, reason);
    }

    pub fn log_file_header(&self, c: &mut Console, file: &str, count: usize) {
        if !self.should_show_output() || !self.is_pretty() {
            return;
        }
        let short = format::short_filename(file);
        c.out("");
        c.out(&format!(
            "{}{}",
            self.color(&format!("▶ {short}.yaml"), Color::Bright),
            self.color(
                &format!(" ({count} request{})", if count == 1 { "" } else { "s" }),
                Color::Dim
            )
        ));
    }
}

/// The run's event sink: renders executor events like the TS executor's
/// loggers (per-request output config = global output + the request's
/// `sourceOutputConfig`).
pub struct Reporter {
    pub console: Console,
    global_output: Option<OutputConfig>,
    color: bool,
    redactor: Arc<Mutex<Redactor>>,
}

impl Reporter {
    pub fn new(
        console: Console,
        global_output: Option<OutputConfig>,
        color: bool,
        redactor: Arc<Mutex<Redactor>>,
    ) -> Self {
        Reporter {
            console,
            global_output,
            color,
            redactor,
        }
    }

    /// Logger for `output` (with the Logger defaults).
    pub fn logger_for(&self, output: Option<&OutputConfig>) -> Logger {
        Logger::new(
            OutCfg::from_config(output),
            self.color,
            Some(self.redactor.clone()),
        )
    }

    /// The executor-level logger (global output config).
    pub fn global_logger(&self) -> Logger {
        self.logger_for(self.global_output.as_ref())
    }

    fn request_logger(&self, req: &RequestConfig) -> Logger {
        let merged = merge_output(
            self.global_output.as_ref(),
            req.source_output_config.as_ref(),
        );
        self.logger_for(Some(&merged))
    }
}

impl EventSink for Reporter {
    fn on_command(&mut self, _index: usize, req: &RequestConfig, command: &str) {
        let l = self.request_logger(req);
        l.log_command(&mut self.console, command);
    }

    fn on_retry(&mut self, _index: usize, req: &RequestConfig, attempt: u32, _delay_ms: u64) {
        let max = req.retry.as_ref().and_then(|r| r.count).unwrap_or(0);
        let l = self.request_logger(req);
        l.log_retry(&mut self.console, attempt, max);
    }

    fn on_result(&mut self, index: usize, result: &ExecutionResult) {
        if result.skipped == Some(true) {
            let l = self.global_logger();
            l.log_skipped(
                &mut self.console,
                &result.request,
                index,
                result.skip_reason.as_deref(),
            );
            return;
        }
        let l = self.request_logger(&result.request);
        l.log_request_complete(&mut self.console, result);
    }

    fn on_stored(&mut self, values: &Store) {
        let l = self.global_logger();
        for (k, v) in values {
            let (prefix, cut) = format::js_prefix(v, 50);
            let shown = if cut { format!("{prefix}...") } else { prefix };
            l.log_info(&mut self.console, &format!("Stored: {k} = \"{shown}\""));
        }
    }

    fn on_stop(&mut self) {
        let l = self.global_logger();
        l.log_error(&mut self.console, "Stopping execution due to error");
    }
}

/// Write the summary to `global.output.saveToFile` (TS `saveSummaryToFile`):
/// 2-space JSON, redacted, path confined to `cwd` unless `allowPaths`.
/// Returns the configured file name when written.
pub fn save_summary(
    summary: &ExecutionSummary,
    global: &GlobalConfig,
    cwd: &Path,
    redactor: &Redactor,
) -> Result<Option<PathBuf>, String> {
    curl_runner_core::executor::save_summary(summary, global, cwd, Some(redactor))
}

#[cfg(test)]
mod tests;
