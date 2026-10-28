//! Post-processing (`core/execution/post-processor.ts`): validation, then
//! snapshot outcome, mutating the `ExecutionResult`.
//!
//! The executor calls, in order:
//! 1. `apply_validation(&mut result, config.expect.as_ref(), &response)`
//! 2. if `result.success` and snapshots are configured (manager, config,
//!    `source_file`): run the snapshot comparison (name defaults to
//!    `"Request"`), then `apply_snapshot(&mut result, compare_result)`.

use crate::model::{ExecutionResult, ExpectConfig, SnapshotCompareResult};
use crate::transport::Response;
use crate::validate;

/// Validate `resp` against `expect`. Skipped when the result already failed
/// (e.g. transport error) or there is no `expect`; a failed validation sets
/// `success = false` and `error` to the `"; "`-joined messages.
pub fn apply_validation(
    result: &mut ExecutionResult,
    expect: Option<&ExpectConfig>,
    resp: &Response,
) {
    let Some(expect) = expect else { return };
    if !result.success {
        return;
    }
    let outcome = validate::check(expect, resp);
    if !outcome.passed {
        result.success = false;
        result.error = outcome.error_message();
    }
}

/// Record a snapshot comparison: always stored on the result; a mismatch that
/// was not updated fails the result with `"Snapshot mismatch"` unless an error
/// is already set.
pub fn apply_snapshot(result: &mut ExecutionResult, snapshot: SnapshotCompareResult) {
    let failed = !snapshot.matches && !snapshot.updated;
    result.snapshot_result = Some(snapshot);
    if failed {
        result.success = false;
        if result.error.is_none() {
            result.error = Some("Snapshot mismatch".to_string());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{Headers, Metrics, RequestConfig};
    use serde_json::json;

    fn result(success: bool, error: Option<&str>) -> ExecutionResult {
        let mut r = ExecutionResult::new(
            RequestConfig {
                url: "https://example.com".into(),
                ..Default::default()
            },
            success,
        );
        r.error = error.map(String::from);
        r
    }
    fn resp(status: u16) -> Response {
        Response {
            status,
            headers: Headers::new(),
            header_history: vec![],
            body: json!({"data": "test"}),
            metrics: Metrics {
                duration: 100.0,
                ..Default::default()
            },
        }
    }
    fn expect_status(s: u16) -> ExpectConfig {
        serde_json::from_value(json!({"status": s})).unwrap()
    }

    #[test]
    fn passes_without_expect() {
        let mut r = result(true, None);
        apply_validation(&mut r, None, &resp(200));
        assert!(r.success);
        assert_eq!(r.error, None);
    }
    #[test]
    fn passes_when_status_matches() {
        let mut r = result(true, None);
        apply_validation(&mut r, Some(&expect_status(200)), &resp(200));
        assert!(r.success);
    }
    #[test]
    fn fails_when_status_differs() {
        let mut r = result(true, None);
        apply_validation(&mut r, Some(&expect_status(200)), &resp(404));
        assert!(!r.success);
        assert_eq!(r.error.as_deref(), Some("Expected status 200, got 404"));
    }
    #[test]
    fn skips_validation_when_already_failed() {
        let mut r = result(false, Some("Connection failed"));
        apply_validation(&mut r, Some(&expect_status(200)), &resp(404));
        assert_eq!(r.error.as_deref(), Some("Connection failed"));
    }

    fn snap(matches: bool, updated: bool) -> SnapshotCompareResult {
        SnapshotCompareResult {
            matches,
            updated,
            ..Default::default()
        }
    }
    #[test]
    fn snapshot_match_keeps_success_and_records() {
        let mut r = result(true, None);
        apply_snapshot(&mut r, snap(true, false));
        assert!(r.success);
        assert!(r.snapshot_result.is_some());
    }
    #[test]
    fn snapshot_mismatch_fails() {
        let mut r = result(true, None);
        apply_snapshot(&mut r, snap(false, false));
        assert!(!r.success);
        assert_eq!(r.error.as_deref(), Some("Snapshot mismatch"));
        assert!(!r.snapshot_result.unwrap().matches);
    }
    #[test]
    fn snapshot_updated_does_not_fail() {
        let mut r = result(true, None);
        apply_snapshot(&mut r, snap(false, true));
        assert!(r.success);
        assert_eq!(r.error, None);
    }
    #[test]
    fn snapshot_keeps_existing_error() {
        let mut r = result(true, None);
        apply_validation(&mut r, Some(&expect_status(200)), &resp(404));
        // executor skips the comparison once failed; if applied anyway the error is kept
        apply_snapshot(&mut r, snap(false, false));
        assert!(r.error.unwrap().contains("status"));
    }
}
