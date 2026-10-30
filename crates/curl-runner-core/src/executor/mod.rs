//! Request execution (`executor/request-executor.ts`,
//! `pooled-curl-executor.ts`, `core/execution/*`).
//!
//! Input is the loader's merged raw-JSON requests ([`PreparedRequest`]);
//! each is typed at execution time after the response-store interpolation,
//! so `${store.x}` may sit in a numeric field. A request that cannot be typed
//! fails with the error naming the field.
//!
//! Per request: store interpolation (sequential) -> `when` (sequential) ->
//! secret registration -> `on_start` -> security guards -> `on_command` ->
//! dry run | send with retries -> `expect` -> snapshot hook -> store extract
//! (sequential). Sequential stops on the first failure unless
//! `continueOnError`. Parallel mode (`execution: parallel`) runs on one
//! `curl::multi::Multi` with at most `maxConcurrency` requests live, and, as
//! in TS, evaluates neither `when` nor the response store.
//!
//! Cancellation ([`Executor::cancel_handle`]): no new request starts, in-flight
//! transfers are dropped, and the summary holds the completed results only.

mod events;
mod loader;
mod parallel;
pub mod retry;
mod sequential;

pub use events::{AttemptOutcome, EventSink, NoopSink, SnapshotHook};
pub use loader::{LoadError, LoadedRun, load_run};

use crate::interpolate::{self, EnvLookup, InterpCtx, Store};
use crate::model::{ExecutionResult, ExecutionSummary, GlobalConfig, Json, Metrics, RequestConfig};
use crate::parser::ParseError;
use crate::security::path_guard;
use crate::security::redact::Redactor;
use crate::transport::{self, Response};
use crate::{postprocess, security};
use indexmap::IndexMap;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Instant;

/// A merged, file-interpolated request (raw JSON, typed at execution).
/// Relative local paths in it (formData files, `ssl`, `output`) resolve
/// against the executor's `cwd`, like TS (`process.cwd()`), not the YAML
/// file's directory.
#[derive(Debug, Clone, PartialEq)]
pub struct PreparedRequest {
    pub raw: Json,
}

impl PreparedRequest {
    /// Wrap an already typed request (tests, profile mode).
    pub fn from_typed(req: &RequestConfig) -> Self {
        PreparedRequest {
            raw: serde_json::to_value(req).unwrap_or_default(),
        }
    }
}

type EnvFn = dyn Fn(&str) -> Option<String> + Send + Sync;

/// Runs requests sequentially or in parallel per `global.execution`.
pub struct Executor {
    global: GlobalConfig,
    cancel: Arc<AtomicBool>,
    cwd: PathBuf,
    env: Arc<EnvFn>,
    redactor: Option<Arc<Mutex<Redactor>>>,
    snapshot: Option<Mutex<Box<dyn SnapshotHook>>>,
    initial_store: Option<Store>,
}

// Compile-time guarantees: an `Executor` is `Send + Sync` and boxed
// sinks/hooks are `Send`, so embedders can run it on another thread.
const _: fn() = || {
    fn send_sync<T: Send + Sync + ?Sized>() {}
    fn send<T: Send + ?Sized>() {}
    send_sync::<Executor>();
    send::<Box<dyn EventSink>>();
    send::<Box<dyn SnapshotHook>>();
};

impl Executor {
    /// Executor for the resolved `global` config. `cwd` defaults to the
    /// process working directory, the env lookup to the process env.
    pub fn new(global: GlobalConfig) -> Self {
        Executor {
            global,
            cancel: Arc::new(AtomicBool::new(false)),
            cwd: std::env::current_dir().unwrap_or_else(|_| PathBuf::from("/")),
            env: Arc::new(interpolate::process_env),
            redactor: None,
            snapshot: None,
            initial_store: None,
        }
    }

    /// Absolute working directory for path guards and `output` files.
    pub fn with_cwd(mut self, cwd: impl Into<PathBuf>) -> Self {
        self.cwd = cwd.into();
        self
    }

    /// Env lookup for `${VAR}` re-resolution during store interpolation.
    pub fn with_env(
        mut self,
        env: impl Fn(&str) -> Option<String> + Send + Sync + 'static,
    ) -> Self {
        self.env = Arc::new(env);
        self
    }

    /// Shared redactor; each request's inline secrets are registered before
    /// any event about it is emitted.
    pub fn with_redactor(mut self, redactor: Arc<Mutex<Redactor>>) -> Self {
        self.redactor = Some(redactor);
        self
    }

    /// Snapshot comparison hook (see [`SnapshotHook`]).
    pub fn with_snapshot_hook(mut self, hook: Box<dyn SnapshotHook>) -> Self {
        self.snapshot = Some(Mutex::new(hook));
        self
    }

    /// Seed the sequential runner's response store (captured values from
    /// earlier runs). The parallel runner has no store and ignores it.
    pub fn with_store(mut self, store: Store) -> Self {
        self.initial_store = Some(store);
        self
    }

    /// Set to `true` to cancel a running [`Executor::run`].
    pub fn cancel_handle(&self) -> Arc<AtomicBool> {
        self.cancel.clone()
    }

    pub fn global(&self) -> &GlobalConfig {
        &self.global
    }

    /// Execute `requests` and summarize (TS `RequestExecutor.execute`, minus
    /// logging and the summary file; see [`save_summary`]).
    pub fn run(
        &self,
        requests: Vec<PreparedRequest>,
        sink: &mut dyn EventSink,
    ) -> ExecutionSummary {
        let start = Instant::now();
        let results = if self.global.execution.as_deref() == Some("parallel") {
            parallel::run(self, requests, sink)
        } else {
            sequential::run(self, requests, sink)
        };
        create_summary(results, ms_since(start))
    }

    fn cancelled(&self) -> bool {
        self.cancel.load(Ordering::Relaxed)
    }

    fn continue_on_error(&self) -> bool {
        self.global.continue_on_error == Some(true)
    }

    fn dry_run(&self) -> bool {
        self.global.dry_run == Some(true)
    }

    /// Type a raw request, interpolating the response store first when it
    /// holds values (TS skips re-interpolation while the store is empty).
    fn type_request(&self, raw: &Json, store: Option<&Store>) -> Result<RequestConfig, String> {
        let res = match store.filter(|s| !s.is_empty()) {
            Some(store) => {
                let vars = IndexMap::new();
                let env: EnvLookup = &*self.env;
                interpolate::interpolate_request_json(raw, &InterpCtx::new(&vars, Some(store), env))
            }
            None => {
                serde_path_to_error::deserialize(raw.clone()).map_err(|e| ParseError::Invalid {
                    field: e.path().to_string(),
                    message: e.inner().to_string(),
                })
            }
        };
        res.map_err(|e| e.to_string())
    }

    /// Steps before the transfer: secrets, `on_start`, guards, command, dry
    /// run. `Some(result)` ends the request here.
    fn preflight(
        &self,
        index: usize,
        req: &RequestConfig,
        start: Instant,
        sink: &mut dyn EventSink,
    ) -> Option<ExecutionResult> {
        if let Some(r) = &self.redactor
            && let Ok(mut r) = r.lock()
        {
            r.register_request_secrets(std::slice::from_ref(req));
        }
        sink.on_start(index, req);
        // Guards run before the dry-run branch (request-executor.ts:142-186).
        if let Err(e) = security::check_request(req, &self.global, &self.cwd) {
            return Some(failed(req, e.message, start));
        }
        sink.on_command(index, req, &transport::to_curl_command(req, &self.global));
        if self.dry_run() {
            // Missing formData files fail in dry-run too (TS checks them
            // first); building the handle checks them without I/O side effects.
            if let Err(e) = transport::build_easy(req, &self.global, &self.cwd) {
                return Some(failed(req, e.message, start));
            }
            let mut r = ExecutionResult::new(req.clone(), true);
            r.dry_run = Some(true);
            r.metrics = Some(Metrics::default());
            return Some(r);
        }
        None
    }

    /// Final result of a sent request: failure after the last attempt, or
    /// success followed by `expect` and the snapshot hook. Returns the
    /// response for store extraction when the result succeeded and the
    /// request has a `store` block (otherwise the response is moved into the
    /// result without copies).
    fn complete(
        &self,
        req: &RequestConfig,
        policy: &retry::RetryPolicy,
        last: retry::Attempt,
        start: Instant,
    ) -> (ExecutionResult, Option<Response>) {
        let mut resp = match last {
            Err(e) => return (failed(req, e.message, start), None),
            Ok(r) if policy.is_retryable_status(r.status) => {
                let mut f = failed(req, retry::RETRIABLE_EXHAUSTED.to_string(), start);
                f.status = Some(r.status);
                return (f, None);
            }
            Ok(r) => r,
        };
        // TS `metrics.duration` (and the `expect.responseTime` it validates):
        // wall time since the request started, retries and delays included.
        resp.metrics.duration = ms_since(start);
        let mut result = ExecutionResult::new(req.clone(), true);
        postprocess::apply_validation(&mut result, req.expect.as_ref(), &resp);
        let kept = (result.success && req.store.is_some()).then(|| resp.clone());
        let Response {
            status,
            headers,
            header_history,
            body,
            metrics,
        } = resp;
        result.status = Some(status);
        result.headers = Some(headers);
        result.header_history = Some(header_history);
        result.body = Some(body);
        result.metrics = Some(metrics);
        if result.success
            && req.source_file.is_some()
            && let Some(hook) = &self.snapshot
            && let Ok(mut hook) = hook.lock()
            && let Some(s) = hook.compare(req, &result)
        {
            postprocess::apply_snapshot(&mut result, s);
        }
        let ok = result.success;
        (result, kept.filter(|_| ok))
    }
}

fn ms_since(start: Instant) -> f64 {
    start.elapsed().as_secs_f64() * 1000.0
}

fn failed(req: &RequestConfig, error: String, start: Instant) -> ExecutionResult {
    let mut r = ExecutionResult::new(req.clone(), false);
    r.error = Some(error);
    r.metrics = Some(Metrics {
        duration: ms_since(start),
        ..Default::default()
    });
    r
}

/// Result for a request that could not be typed: the request is rebuilt
/// from whatever string fields are usable so the CLI can still name it.
fn untyped_failure(raw: &Json, error: String, start: Instant) -> ExecutionResult {
    let s = |k: &str| raw.get(k).and_then(Json::as_str).map(String::from);
    let req = RequestConfig {
        name: s("name"),
        url: s("url").unwrap_or_default(),
        method: s("method"),
        source_file: s("sourceFile"),
        ..Default::default()
    };
    failed(&req, error, start)
}

/// TS `createSummary`: skipped results count as neither successful nor failed.
pub fn create_summary(results: Vec<ExecutionResult>, duration: f64) -> ExecutionSummary {
    let skipped = results.iter().filter(|r| r.skipped == Some(true)).count() as u64;
    let successful = results
        .iter()
        .filter(|r| r.success && r.skipped != Some(true))
        .count() as u64;
    let failed = results.iter().filter(|r| !r.success).count() as u64;
    ExecutionSummary {
        total: results.len() as u64,
        successful,
        failed,
        skipped,
        duration,
        results,
    }
}

/// Write the summary to `global.output.saveToFile` (TS `saveSummaryToFile`):
/// pretty JSON (2 spaces), redacted. The path goes through the path guard
/// relative to `cwd`. `Ok(None)` when no file is configured, `Ok(Some(path))`
/// when written (`Results saved to {file}`), `Err` with the line to print
/// (`Cannot save results: <error>`).
pub fn save_summary(
    summary: &ExecutionSummary,
    global: &GlobalConfig,
    cwd: &Path,
    redactor: Option<&Redactor>,
) -> Result<Option<PathBuf>, String> {
    let Some(file) = global
        .output
        .as_ref()
        .and_then(|o| o.save_to_file.as_deref())
        .filter(|f| !f.is_empty())
    else {
        return Ok(None);
    };
    let path = path_guard::resolve_safe(file, cwd, cwd, security::allow_paths(global))
        .map_err(|e| format!("Cannot save results: {e}"))?;
    // `JSON.stringify(summary, null, 2)`: JS number text and key order.
    let value = serde_json::to_value(summary).map_err(|e| format!("Cannot save results: {e}"))?;
    let raw = crate::snapshot::js_stringify_pretty(&value, 2);
    let content = match redactor {
        Some(r) => r.redact(&raw),
        None => Redactor::default().redact(&raw),
    };
    write_atomic(&path, content.as_bytes()).map_err(|e| format!("Cannot save results: {e}"))?;
    Ok(Some(path))
}

/// Write `bytes` to a temp file next to `path`, then rename it over `path`,
/// so an interrupted process never leaves a truncated file.
fn write_atomic(path: &Path, bytes: &[u8]) -> std::io::Result<()> {
    let name = path
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default();
    let tmp = path.with_file_name(format!(".{name}.tmp-{}", std::process::id()));
    let res = std::fs::write(&tmp, bytes).and_then(|()| std::fs::rename(&tmp, path));
    if res.is_err() {
        let _ = std::fs::remove_file(&tmp);
    }
    res
}

#[cfg(test)]
mod tests;
