//! Response comparison and the baseline workflow (`diff/response-differ.ts`).
//!
//! The comparison rules (exclusions, `*.x` and `[*]` wildcards, match rules,
//! type mismatches) are the snapshot differ's: the TS classes are identical
//! apart from names, so [`ResponseDiffer`] wraps [`SnapshotDiffer`].

use super::baseline::{BaselineManager, request_key};
use crate::jsfile::headers_map;
use crate::model::{
    Baseline, DiffCompareResult, DiffConfig, DiffSummary, ExecutionResult, GlobalDiffConfig, Json,
    ResponseDiff, SnapshotDiff, TimingDiff,
};
use crate::snapshot::SnapshotDiffer;
use crate::snapshot::differ::strict_eq;

pub struct ResponseDiffer {
    inner: SnapshotDiffer,
    include_timings: bool,
}

fn to_response(d: SnapshotDiff) -> ResponseDiff {
    ResponseDiff {
        path: d.path,
        baseline: d.expected,
        current: d.received,
        diff_type: d.diff_type,
    }
}

impl ResponseDiffer {
    pub fn new(config: &DiffConfig) -> Self {
        Self {
            inner: SnapshotDiffer::from_parts(
                config.exclude.as_deref().unwrap_or_default(),
                config.match_rules.clone().unwrap_or_default(),
            ),
            include_timings: config.include_timings == Some(true),
        }
    }

    /// Compares the current response against the baseline: status, headers,
    /// body, then (with `includeTimings`) the timing change.
    pub fn compare(
        &self,
        baseline: &Baseline,
        current: &Baseline,
        baseline_label: &str,
        current_label: &str,
        request_name: &str,
    ) -> DiffCompareResult {
        let mut differences: Vec<ResponseDiff> = Vec::new();

        if (baseline.status.is_some() || current.status.is_some())
            && !strict_eq(baseline.status.as_ref(), current.status.as_ref())
            && !self.is_excluded("status")
        {
            differences.push(ResponseDiff {
                path: "status".into(),
                baseline: baseline.status.clone(),
                current: current.status.clone(),
                diff_type: "changed".into(),
            });
        }

        if baseline.headers.is_some() || current.headers.is_some() {
            differences.extend(
                self.inner
                    .compare_objects(
                        &headers_map(baseline.headers.as_ref()),
                        &headers_map(current.headers.as_ref()),
                        "headers",
                    )
                    .into_iter()
                    .map(to_response),
            );
        }

        if baseline.body.is_some() || current.body.is_some() {
            differences.extend(self.deep_compare(
                baseline.body.as_ref(),
                current.body.as_ref(),
                "body",
            ));
        }

        let timing_diff = match (self.include_timings, baseline.timing, current.timing) {
            (true, Some(b), Some(c)) => Some(TimingDiff {
                baseline: b,
                current: c,
                change_percent: ((c - b) / b) * 100.0,
            }),
            _ => None,
        };
        DiffCompareResult {
            request_name: request_name.to_string(),
            has_differences: !differences.is_empty(),
            is_new_baseline: false,
            baseline_label: baseline_label.to_string(),
            current_label: current_label.to_string(),
            differences,
            timing_diff,
        }
    }

    /// Deep comparison with path tracking (`None` = JS `undefined`).
    pub fn deep_compare(
        &self,
        baseline: Option<&Json>,
        current: Option<&Json>,
        path: &str,
    ) -> Vec<ResponseDiff> {
        self.inner
            .deep_compare(baseline, current, path)
            .into_iter()
            .map(to_response)
            .collect()
    }

    pub fn is_excluded(&self, path: &str) -> bool {
        self.inner.is_excluded(path)
    }

    pub fn matches_rule(&self, path: &str, value: Option<&Json>) -> bool {
        self.inner.matches_rule(path, value)
    }
}

/// Orchestrates response diffing between runs.
#[derive(Debug, Clone)]
pub struct DiffOrchestrator {
    manager: BaselineManager,
}

fn new_result(name: &str, baseline_label: &str, current_label: &str) -> DiffCompareResult {
    DiffCompareResult {
        request_name: name.to_string(),
        has_differences: false,
        is_new_baseline: true,
        baseline_label: baseline_label.to_string(),
        current_label: current_label.to_string(),
        differences: Vec::new(),
        timing_diff: None,
    }
}

fn summarize(results: Vec<DiffCompareResult>, count_new: bool) -> DiffSummary {
    let n = |f: &dyn Fn(&DiffCompareResult) -> bool| results.iter().filter(|r| f(r)).count() as u64;
    DiffSummary {
        total_requests: results.len() as u64,
        unchanged: n(&|r| !r.has_differences && !(count_new && r.is_new_baseline)),
        changed: n(&|r| r.has_differences),
        new_baselines: if count_new {
            n(&|r| r.is_new_baseline)
        } else {
            0
        },
        results,
    }
}

impl DiffOrchestrator {
    pub fn new(global: Option<&GlobalDiffConfig>, cwd: impl Into<std::path::PathBuf>) -> Self {
        Self {
            manager: BaselineManager::new(global).with_cwd(cwd),
        }
    }

    pub fn manager(&self) -> &BaselineManager {
        &self.manager
    }

    /// Compares execution results against the stored baseline `baseline_label`.
    /// A missing baseline file or request counts as a new baseline.
    pub fn compare_with_baseline(
        &self,
        yaml_path: &str,
        results: &[ExecutionResult],
        current_label: &str,
        baseline_label: &str,
        config: &DiffConfig,
    ) -> DiffSummary {
        let file = self.manager.load(yaml_path, baseline_label);
        let differ = ResponseDiffer::new(config);
        let mut out = Vec::new();
        for r in results {
            if r.skipped == Some(true) || !r.success {
                continue;
            }
            let name = request_key(r);
            let current = self.manager.create_baseline(r, config);
            let stored = file.as_ref().and_then(|f| f.baselines.get(&name));
            out.push(match stored {
                None => new_result(&name, baseline_label, current_label),
                Some(stored) => {
                    differ.compare(stored, &current, baseline_label, current_label, &name)
                }
            });
        }
        summarize(out, true)
    }

    /// Compares two stored baselines (offline). `Err` carries TS's
    /// `Baseline '<label>' not found`.
    pub fn compare_two_baselines(
        &self,
        yaml_path: &str,
        label1: &str,
        label2: &str,
        config: &DiffConfig,
    ) -> Result<DiffSummary, String> {
        let file1 = self
            .manager
            .load(yaml_path, label1)
            .ok_or_else(|| format!("Baseline '{label1}' not found"))?;
        let file2 = self
            .manager
            .load(yaml_path, label2)
            .ok_or_else(|| format!("Baseline '{label2}' not found"))?;
        let differ = ResponseDiffer::new(config);

        let names: indexmap::IndexSet<&String> = file1
            .baselines
            .keys()
            .chain(file2.baselines.keys())
            .collect();
        let one_sided = |name: &str, baseline: Option<Json>, current: Option<Json>, ty: &str| {
            DiffCompareResult {
                request_name: name.to_string(),
                has_differences: true,
                is_new_baseline: false,
                baseline_label: label1.to_string(),
                current_label: label2.to_string(),
                differences: vec![ResponseDiff {
                    path: String::new(),
                    baseline,
                    current,
                    diff_type: ty.into(),
                }],
                timing_diff: None,
            }
        };
        let mut out = Vec::new();
        for name in names {
            out.push(
                match (file1.baselines.get(name), file2.baselines.get(name)) {
                    (None, _) => {
                        one_sided(name, None, Some(Json::String("exists".into())), "added")
                    }
                    (Some(_), None) => {
                        one_sided(name, Some(Json::String("exists".into())), None, "removed")
                    }
                    (Some(a), Some(b)) => differ.compare(a, b, label1, label2, name),
                },
            );
        }
        Ok(summarize(out, false))
    }

    pub fn save_baseline(
        &self,
        yaml_path: &str,
        label: &str,
        results: &[ExecutionResult],
        config: &DiffConfig,
    ) -> std::io::Result<()> {
        self.manager
            .save_baseline(yaml_path, label, results, config)
    }

    pub fn list_labels(&self, yaml_path: &str) -> Vec<String> {
        self.manager.list_labels(yaml_path)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn bl(v: serde_json::Value) -> Baseline {
        let mut b: Baseline = serde_json::from_value(v).unwrap();
        b.hash = "a".into();
        b
    }
    fn cfg(v: serde_json::Value) -> DiffConfig {
        serde_json::from_value(v).unwrap()
    }
    fn differ(v: serde_json::Value) -> ResponseDiffer {
        ResponseDiffer::new(&cfg(v))
    }
    fn cmp(d: &ResponseDiffer, a: serde_json::Value, b: serde_json::Value) -> DiffCompareResult {
        d.compare(&bl(a), &bl(b), "v1", "v2", "Get")
    }
    fn rd(
        path: &str,
        b: Option<serde_json::Value>,
        c: Option<serde_json::Value>,
        t: &str,
    ) -> ResponseDiff {
        ResponseDiff {
            path: path.into(),
            baseline: b,
            current: c,
            diff_type: t.into(),
        }
    }

    #[test]
    fn identical_baselines_have_no_differences() {
        let b = bl(json!({"status": 200, "body": {"id": 1, "name": "Test"}}));
        let r = differ(json!({})).compare(&b, &b, "staging", "production", "Get User");
        assert!(!r.has_differences);
        assert!(r.differences.is_empty());
        assert_eq!(r.request_name, "Get User");
        assert_eq!(r.baseline_label, "staging");
        assert_eq!(r.current_label, "production");
        assert!(!r.is_new_baseline);
    }

    #[test]
    fn status_change() {
        let r = cmp(
            &differ(json!({})),
            json!({"status": 200}),
            json!({"status": 201}),
        );
        assert_eq!(
            r.differences,
            [rd("status", Some(json!(200)), Some(json!(201)), "changed")]
        );
    }

    #[test]
    fn body_field_changes_added_removed() {
        let d = differ(json!({}));
        let r = cmp(
            &d,
            json!({"body": {"id": 1, "name": "Old"}}),
            json!({"body": {"id": 1, "name": "New"}}),
        );
        assert!(r.differences.contains(&rd(
            "body.name",
            Some(json!("Old")),
            Some(json!("New")),
            "changed"
        )));
        let r = cmp(
            &d,
            json!({"body": {"id": 1}}),
            json!({"body": {"id": 1, "newField": "value"}}),
        );
        assert!(
            r.differences
                .contains(&rd("body.newField", None, Some(json!("value")), "added"))
        );
        let r = cmp(
            &d,
            json!({"body": {"id": 1, "oldField": "value"}}),
            json!({"body": {"id": 1}}),
        );
        assert!(r.differences.contains(&rd(
            "body.oldField",
            Some(json!("value")),
            None,
            "removed"
        )));
    }

    #[test]
    fn type_mismatch() {
        let r = cmp(
            &differ(json!({})),
            json!({"body": {"count": "10"}}),
            json!({"body": {"count": 10}}),
        );
        assert_eq!(r.differences[0].diff_type, "type_mismatch");
    }

    #[test]
    fn string_status_differs_from_number() {
        let r = cmp(
            &differ(json!({})),
            json!({"status": "200"}),
            json!({"status": 200}),
        );
        assert_eq!(r.differences.len(), 1);
    }

    #[test]
    fn timing_diff_when_configured() {
        let d = differ(json!({"includeTimings": true}));
        let r = cmp(
            &d,
            json!({"body": {}, "timing": 100}),
            json!({"body": {}, "timing": 150}),
        );
        let t = r.timing_diff.unwrap();
        assert_eq!(
            (t.baseline, t.current, t.change_percent),
            (100.0, 150.0, 50.0)
        );
        // off by default, and needs both sides
        let r = cmp(
            &differ(json!({})),
            json!({"timing": 100}),
            json!({"timing": 150}),
        );
        assert!(r.timing_diff.is_none());
        let r = cmp(&d, json!({"timing": 100}), json!({}));
        assert!(r.timing_diff.is_none());
        // zero baseline: Infinity like JS
        let r = cmp(&d, json!({"timing": 0}), json!({"timing": 5}));
        assert!(r.timing_diff.unwrap().change_percent.is_infinite());
    }

    #[test]
    fn deep_compare_cases() {
        let d = differ(json!({}));
        let diffs = d.deep_compare(
            Some(&json!({"user": {"profile": {"age": 25}}})),
            Some(&json!({"user": {"profile": {"age": 26}}})),
            "body",
        );
        assert_eq!(diffs.len(), 1);
        assert_eq!(diffs[0].path, "body.user.profile.age");
        let diffs = d.deep_compare(
            Some(&json!({"items": [1, 2, 3]})),
            Some(&json!({"items": [1, 2, 4]})),
            "body",
        );
        assert_eq!(diffs[0].path, "body.items[2]");
        let diffs = d.deep_compare(
            Some(&json!({"items": [1, 2]})),
            Some(&json!({"items": [1, 2, 3]})),
            "body",
        );
        assert_eq!(
            (
                diffs.len(),
                diffs[0].diff_type.as_str(),
                diffs[0].path.as_str()
            ),
            (1, "added", "body.items[2]")
        );
        assert!(
            d.deep_compare(
                Some(&json!({"value": null})),
                Some(&json!({"value": null})),
                "body"
            )
            .is_empty()
        );
    }

    #[test]
    fn is_excluded_cases() {
        let d = differ(json!({"exclude": ["body.timestamp"]}));
        assert!(d.is_excluded("body.timestamp"));
        assert!(!d.is_excluded("body.id"));
        let d = differ(json!({"exclude": ["*.createdAt"]}));
        assert!(d.is_excluded("body.createdAt"));
        assert!(d.is_excluded("body.user.createdAt"));
        assert!(!d.is_excluded("body.id"));
        let d = differ(json!({"exclude": ["body.items[*].id"]}));
        assert!(d.is_excluded("body.items[0].id"));
        assert!(d.is_excluded("body.items[99].id"));
        assert!(!d.is_excluded("body.items[0].name"));
    }

    #[test]
    fn matches_rule_cases() {
        let d = differ(json!({"match": {"body.requestId": "*"}}));
        assert!(d.matches_rule("body.requestId", Some(&json!("any-value"))));
        assert!(d.matches_rule("body.requestId", Some(&json!(12345))));
        assert!(!d.matches_rule("body.otherId", Some(&json!("value"))));
        let d = differ(json!({"match": {"body.uuid": "regex:^[a-f0-9-]{36}$"}}));
        assert!(d.matches_rule(
            "body.uuid",
            Some(&json!("550e8400-e29b-41d4-a716-446655440000"))
        ));
        assert!(!d.matches_rule("body.uuid", Some(&json!("invalid"))));
        let d = differ(json!({"match": {"body.id": "*"}}));
        assert!(!d.matches_rule("body.name", Some(&json!("value"))));
    }

    #[test]
    fn compare_with_exclusions_and_rules() {
        let d = differ(json!({"exclude": ["body.timestamp", "body.requestId"]}));
        let r = cmp(
            &d,
            json!({"body": {"id": 1, "timestamp": "a", "requestId": "abc"}}),
            json!({"body": {"id": 1, "timestamp": "b", "requestId": "xyz"}}),
        );
        assert!(!r.has_differences);
        let d = differ(json!({"match": {"body.token": "*"}}));
        let r = cmp(
            &d,
            json!({"body": {"id": 1, "token": "old"}}),
            json!({"body": {"id": 1, "token": "new"}}),
        );
        assert!(!r.has_differences);
    }

    #[test]
    fn header_comparison() {
        let d = differ(json!({}));
        let r = cmp(
            &d,
            json!({"headers": {"content-type": "application/json"}}),
            json!({"headers": {"content-type": "text/plain"}}),
        );
        assert!(r.has_differences);
        assert_eq!(r.differences[0].path, "headers.content-type");
        let r = cmp(
            &d,
            json!({"headers": {}}),
            json!({"headers": {"x-new-header": "value"}}),
        );
        assert_eq!(r.differences[0].diff_type, "added");
    }

    // ---- orchestrator ----

    fn result(name: &str, body: serde_json::Value) -> ExecutionResult {
        let req: crate::model::RequestConfig =
            serde_json::from_value(json!({"url": "http://x", "name": name})).unwrap();
        let mut r = ExecutionResult::new(req, true);
        r.status = Some(200);
        r.body = Some(body);
        r
    }

    #[test]
    fn compare_with_baseline_flow() {
        let tmp = tempfile::tempdir().unwrap();
        let o = DiffOrchestrator::new(None, tmp.path());
        let c = DiffConfig::default();
        let a = result("A", json!({"v": 1}));
        let b = result("B", json!({"v": 1}));
        // no file: everything new
        let s = o.compare_with_baseline("t.yaml", &[a.clone(), b.clone()], "cur", "base", &c);
        assert_eq!(
            (s.total_requests, s.new_baselines, s.unchanged, s.changed),
            (2, 2, 0, 0)
        );
        o.save_baseline("t.yaml", "base", std::slice::from_ref(&a), &c)
            .unwrap();
        let a2 = result("A", json!({"v": 2}));
        let mut failed = result("F", json!({}));
        failed.success = false;
        let s = o.compare_with_baseline("t.yaml", &[a2, b.clone(), failed], "cur", "base", &c);
        assert_eq!(
            (s.total_requests, s.new_baselines, s.unchanged, s.changed),
            (2, 1, 0, 1)
        );
        assert!(s.results[1].is_new_baseline);
        let s = o.compare_with_baseline("t.yaml", &[a], "cur", "base", &c);
        assert_eq!((s.unchanged, s.changed), (1, 0));
    }

    #[test]
    fn compare_two_baselines_flow() {
        let tmp = tempfile::tempdir().unwrap();
        let o = DiffOrchestrator::new(None, tmp.path());
        let c = DiffConfig::default();
        assert_eq!(
            o.compare_two_baselines("t.yaml", "x", "y", &c).unwrap_err(),
            "Baseline 'x' not found"
        );
        o.save_baseline(
            "t.yaml",
            "x",
            &[
                result("Same", json!(1)),
                result("Gone", json!(1)),
                result("Chg", json!(1)),
            ],
            &c,
        )
        .unwrap();
        assert_eq!(
            o.compare_two_baselines("t.yaml", "x", "y", &c).unwrap_err(),
            "Baseline 'y' not found"
        );
        o.save_baseline(
            "t.yaml",
            "y",
            &[
                result("Same", json!(1)),
                result("Chg", json!(2)),
                result("Fresh", json!(1)),
            ],
            &c,
        )
        .unwrap();
        let s = o.compare_two_baselines("t.yaml", "x", "y", &c).unwrap();
        assert_eq!(
            (s.total_requests, s.unchanged, s.changed, s.new_baselines),
            (4, 1, 3, 0)
        );
        let names: Vec<_> = s.results.iter().map(|r| r.request_name.as_str()).collect();
        assert_eq!(names, ["Same", "Gone", "Chg", "Fresh"]);
        assert_eq!(
            s.results[1].differences,
            [rd("", Some(json!("exists")), None, "removed")]
        );
        assert_eq!(
            s.results[3].differences,
            [rd("", None, Some(json!("exists")), "added")]
        );
        assert_eq!(
            (
                s.results[3].baseline_label.as_str(),
                s.results[3].current_label.as_str()
            ),
            ("x", "y")
        );
    }
}
