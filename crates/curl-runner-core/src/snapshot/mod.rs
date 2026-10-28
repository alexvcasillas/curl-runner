//! Snapshot testing: comparison, files and the executor hook.
//!
//! No printing here; the pretty output lives in the CLI crate.

pub mod differ;
pub mod json;
pub mod manager;

pub use differ::{DiffResult, SnapshotDiffer};
pub use json::{js_stringify, js_stringify_pretty};
pub use manager::{SnapshotManager, UpdateMode, filter_snapshot_body, snapshot_hash};

use crate::executor::SnapshotHook;
use crate::model::{
    ExecutionResult, GlobalSnapshotConfig, Json, RequestConfig, SnapshotCompareResult, SnapshotDiff,
};

/// [`SnapshotHook`] backed by a [`SnapshotManager`]: merges the request's
/// `snapshot` with the global config, then compares and updates.
pub struct ManagerHook {
    manager: SnapshotManager,
    global: Option<GlobalSnapshotConfig>,
}

impl ManagerHook {
    pub fn new(manager: SnapshotManager, global: Option<GlobalSnapshotConfig>) -> Self {
        Self { manager, global }
    }

    /// Hook for `global.snapshot` (`dir`, `updateMode`, `ci`), with relative
    /// YAML paths resolved against `cwd`.
    pub fn from_global(global: Option<&GlobalSnapshotConfig>, cwd: &std::path::Path) -> Self {
        Self::new(
            SnapshotManager::from_global(global).with_cwd(cwd),
            global.cloned(),
        )
    }
}

impl SnapshotHook for ManagerHook {
    fn compare(
        &mut self,
        req: &RequestConfig,
        result: &ExecutionResult,
    ) -> Option<SnapshotCompareResult> {
        let cfg = SnapshotManager::merge_config(self.global.as_ref(), req.snapshot.as_ref())?;
        Some(self.manager.compare(req, result, &cfg).unwrap_or_else(|e| {
            // TS rejected the request on an I/O error; here it is a failed
            // comparison that names the error (unusable or unwritable file).
            SnapshotCompareResult {
                matches: false,
                is_new: false,
                updated: false,
                differences: vec![SnapshotDiff {
                    path: String::new(),
                    expected: Some(Json::String("usable snapshot file".into())),
                    received: Some(Json::String(format!("error: {e}"))),
                    diff_type: "changed".into(),
                }],
            }
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn req(snapshot: Json) -> RequestConfig {
        serde_json::from_value(
            json!({"url": "http://x", "name": "R", "sourceFile": "t.yaml", "snapshot": snapshot}),
        )
        .unwrap()
    }

    fn result(r: &RequestConfig) -> ExecutionResult {
        let mut e = ExecutionResult::new(r.clone(), true);
        e.body = Some(json!({"a": 1}));
        e
    }

    #[test]
    fn hook_is_off_unless_request_or_global_enables_snapshots() {
        let tmp = tempfile::tempdir().unwrap();
        let mut hook = ManagerHook::from_global(None, tmp.path());
        let r = req(json!(false));
        assert_eq!(hook.compare(&r, &result(&r)), None);
        assert!(!tmp.path().join("__snapshots__").exists());

        let g = GlobalSnapshotConfig {
            enabled: Some(true),
            ..Default::default()
        };
        let mut hook = ManagerHook::from_global(Some(&g), tmp.path());
        let r: RequestConfig =
            serde_json::from_value(json!({"url": "http://x", "sourceFile": "t.yaml"})).unwrap();
        let out = hook.compare(&r, &result(&r)).unwrap();
        assert!(out.matches && out.is_new && out.updated);
    }

    #[test]
    fn hook_reports_write_failures_as_a_failed_comparison() {
        let tmp = tempfile::tempdir().unwrap();
        // a file where the snapshot directory should go
        std::fs::write(tmp.path().join("__snapshots__"), "x").unwrap();
        let mut hook = ManagerHook::from_global(None, tmp.path());
        let r = req(json!(true));
        let out = hook.compare(&r, &result(&r)).unwrap();
        assert!(!out.matches && !out.updated);
        assert_eq!(out.differences.len(), 1);
    }

    #[test]
    fn hook_honors_global_update_mode_and_ci() {
        let tmp = tempfile::tempdir().unwrap();
        let ci = GlobalSnapshotConfig {
            ci: Some(true),
            ..Default::default()
        };
        let mut hook = ManagerHook::from_global(Some(&ci), tmp.path());
        let r = req(json!(true));
        let out = hook.compare(&r, &result(&r)).unwrap();
        assert!(!out.matches && out.is_new);
    }
}
