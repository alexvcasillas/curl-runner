//! Executor callbacks. The core never prints: the CLI (or a GUI) observes a
//! run through an [`EventSink`] and renders it, applying
//! `Redactor::redact` to anything it shows (the executor registers each
//! request's inline secrets before emitting events for it).

use crate::interpolate::Store;
use crate::model::{ExecutionResult, RequestConfig, SnapshotCompareResult};

/// One finished transfer (an attempt), reported through
/// [`EventSink::on_attempt`].
#[derive(Debug, Clone, PartialEq)]
pub struct AttemptOutcome {
    /// HTTP status; `None` when the transfer failed before a response.
    pub status: Option<u16>,
    /// Wall time of this single transfer.
    pub duration_ms: f64,
    /// Transport error message, when the transfer failed.
    pub error: Option<String>,
}

impl AttemptOutcome {
    pub(super) fn from_attempt(a: &super::retry::Attempt, duration_ms: f64) -> Self {
        match a {
            Ok(r) => AttemptOutcome {
                status: Some(r.status),
                duration_ms,
                error: None,
            },
            Err(e) => AttemptOutcome {
                status: None,
                duration_ms,
                error: Some(e.message.clone()),
            },
        }
    }
}

/// Run observer. `index` is the 1-based position of the request in the run
/// (TS `logRequestStart(config, index)`). All methods default to no-ops.
pub trait EventSink: Send {
    /// A request is about to be checked and sent (not called for skipped
    /// requests or requests that failed to type).
    fn on_start(&mut self, _index: usize, _req: &RequestConfig) {}
    /// The equivalent curl command (TS `logCommand`; the dry-run output).
    fn on_command(&mut self, _index: usize, _req: &RequestConfig, _command: &str) {}
    /// Transfer number `attempt` (1-based) of `req` finished, including the
    /// last one and transport errors. Fires before the matching `on_retry`
    /// (if another attempt follows) or `on_result`.
    fn on_attempt(
        &mut self,
        _index: usize,
        _req: &RequestConfig,
        _attempt: u32,
        _outcome: &AttemptOutcome,
    ) {
    }
    /// Retry `attempt` (1-based) of `req` starts after `delay_ms`.
    fn on_retry(&mut self, _index: usize, _req: &RequestConfig, _attempt: u32, _delay_ms: u64) {}
    /// A request finished (also skipped, blocked and dry-run requests). In
    /// parallel mode results arrive in completion order; the summary is in
    /// request order.
    fn on_result(&mut self, _index: usize, _result: &ExecutionResult) {}
    /// Values a successful request saved to the response store (sequential).
    fn on_stored(&mut self, _values: &Store) {}
    /// A failure stops the run (`continueOnError` off).
    fn on_stop(&mut self) {}
}

/// Sink that ignores every event.
#[derive(Debug, Default, Clone, Copy)]
pub struct NoopSink;
impl EventSink for NoopSink {}

/// Snapshot comparison seam (implemented by `snapshot::ManagerHook`). Called only for successful results of requests with a
/// `sourceFile`; return `None` when snapshots are not enabled for `req`
/// (TS `SnapshotManager.mergeConfig` returned null). The executor applies
/// the outcome with `postprocess::apply_snapshot`.
pub trait SnapshotHook: Send {
    fn compare(
        &mut self,
        req: &RequestConfig,
        result: &ExecutionResult,
    ) -> Option<SnapshotCompareResult>;
}
