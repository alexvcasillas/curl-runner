//! Snapshot comparison with exclusions, wildcards and regex rules
//! (`snapshot/snapshot-differ.ts`).

use super::json::js_entries;
use crate::jsfile::headers_map;
use crate::jsval::{JsVal, regex_test};
use crate::model::{Json, Snapshot, SnapshotConfig, SnapshotDiff};
use indexmap::IndexMap;
use serde_json::Map;

#[derive(Debug, Clone, PartialEq)]
pub struct DiffResult {
    pub matches: bool,
    pub differences: Vec<SnapshotDiff>,
}

pub struct SnapshotDiffer {
    exclude: Vec<String>,
    rules: IndexMap<String, String>,
}

fn diff(path: &str, expected: Option<&Json>, received: Option<&Json>, ty: &str) -> SnapshotDiff {
    SnapshotDiff {
        path: path.to_string(),
        expected: expected.cloned(),
        received: received.cloned(),
        diff_type: ty.to_string(),
    }
}

/// JS `typeof`-like tag used by `getType`.
fn type_of(v: Option<&Json>) -> &'static str {
    match v {
        None => "undefined",
        Some(Json::Null) => "null",
        Some(Json::Array(_)) => "array",
        Some(Json::Object(_)) => "object",
        Some(Json::String(_)) => "string",
        Some(Json::Number(_)) => "number",
        Some(Json::Bool(_)) => "boolean",
    }
}

fn primitives_equal(a: &Json, b: &Json) -> bool {
    match (a, b) {
        (Json::Number(x), Json::Number(y)) => x.as_f64() == y.as_f64(),
        _ => a == b,
    }
}

/// JS `===` on two possibly-undefined values (objects are never identical).
pub(crate) fn strict_eq(a: Option<&Json>, b: Option<&Json>) -> bool {
    match (a, b) {
        (None, None) => true,
        (Some(Json::Object(_) | Json::Array(_)), _)
        | (_, Some(Json::Object(_) | Json::Array(_))) => false,
        (Some(x), Some(y)) => type_of(a) == type_of(b) && primitives_equal(x, y),
        _ => false,
    }
}

impl SnapshotDiffer {
    pub fn new(config: &SnapshotConfig) -> Self {
        let mut exclude: Vec<String> = Vec::new();
        for p in config.exclude.iter().flatten() {
            if !exclude.contains(p) {
                exclude.push(p.clone());
            }
        }
        Self {
            exclude,
            rules: config.match_rules.clone().unwrap_or_default(),
        }
    }

    /// Differ for explicit exclusions and match rules (response diffing).
    pub(crate) fn from_parts(exclude: &[String], rules: IndexMap<String, String>) -> Self {
        let mut ex: Vec<String> = Vec::new();
        for p in exclude {
            if !ex.contains(p) {
                ex.push(p.clone());
            }
        }
        Self { exclude: ex, rules }
    }

    /// Compares two snapshots (status, then headers, then body).
    pub fn compare(&self, expected: &Snapshot, received: &Snapshot) -> DiffResult {
        let mut differences = Vec::new();

        if (expected.status.is_some() || received.status.is_some())
            && !strict_eq(expected.status.as_ref(), received.status.as_ref())
            && !self.is_excluded("status")
        {
            differences.push(diff(
                "status",
                expected.status.as_ref(),
                received.status.as_ref(),
                "changed",
            ));
        }

        if expected.headers.is_some() || received.headers.is_some() {
            differences.extend(self.compare_objects(
                &headers_map(expected.headers.as_ref()),
                &headers_map(received.headers.as_ref()),
                "headers",
            ));
        }

        if expected.body.is_some() || received.body.is_some() {
            differences.extend(self.deep_compare(
                expected.body.as_ref(),
                received.body.as_ref(),
                "body",
            ));
        }

        DiffResult {
            matches: differences.is_empty(),
            differences,
        }
    }

    /// Deep comparison with path tracking (`None` = JS `undefined`).
    pub fn deep_compare(
        &self,
        expected: Option<&Json>,
        received: Option<&Json>,
        path: &str,
    ) -> Vec<SnapshotDiff> {
        if self.is_excluded(path) || self.matches_rule(path, received) {
            return Vec::new();
        }
        if matches!(
            (expected, received),
            (Some(Json::Null), Some(Json::Null)) | (None, None)
        ) {
            return Vec::new();
        }
        let et = type_of(expected);
        if et != type_of(received) {
            return vec![diff(path, expected, received, "type_mismatch")];
        }
        match (expected, received) {
            (Some(Json::Array(a)), Some(Json::Array(b))) => self.compare_arrays(a, b, path),
            (Some(Json::Object(a)), Some(Json::Object(b))) => self.compare_objects(a, b, path),
            (Some(a), Some(b)) => {
                if primitives_equal(a, b) {
                    Vec::new()
                } else {
                    vec![diff(path, expected, received, "changed")]
                }
            }
            _ => Vec::new(),
        }
    }

    fn compare_arrays(
        &self,
        expected: &[Json],
        received: &[Json],
        path: &str,
    ) -> Vec<SnapshotDiff> {
        let mut out = Vec::new();
        for i in 0..expected.len().max(received.len()) {
            let item_path = format!("{path}[{i}]");
            match (expected.get(i), received.get(i)) {
                (None, r) => {
                    if !self.is_excluded(&item_path) {
                        out.push(diff(&item_path, None, r, "added"));
                    }
                }
                (e, None) => {
                    if !self.is_excluded(&item_path) {
                        out.push(diff(&item_path, e, None, "removed"));
                    }
                }
                (e, r) => out.extend(self.deep_compare(e, r, &item_path)),
            }
        }
        out
    }

    pub(crate) fn compare_objects(
        &self,
        expected: &Map<String, Json>,
        received: &Map<String, Json>,
        path: &str,
    ) -> Vec<SnapshotDiff> {
        let mut out = Vec::new();
        let mut keys: Vec<&String> = Vec::new();
        let mut seen = std::collections::HashSet::new();
        // `new Set([...Object.keys(expected), ...Object.keys(received)])`
        for (k, _) in js_entries(expected).into_iter().chain(js_entries(received)) {
            if seen.insert(k) {
                keys.push(k);
            }
        }
        for key in keys {
            let key_path = if path.is_empty() {
                key.clone()
            } else {
                format!("{path}.{key}")
            };
            match (expected.get(key), received.get(key)) {
                (None, Some(r)) => {
                    if !self.is_excluded(&key_path) {
                        out.push(diff(&key_path, None, Some(r), "added"));
                    }
                }
                (Some(e), None) => {
                    if !self.is_excluded(&key_path) {
                        out.push(diff(&key_path, Some(e), None, "removed"));
                    }
                }
                (e, r) => out.extend(self.deep_compare(e, r, &key_path)),
            }
        }
        out
    }

    /// Exact, `*.suffix` and `[*]` exclusion patterns.
    pub fn is_excluded(&self, path: &str) -> bool {
        if self.exclude.iter().any(|p| p == path) {
            return true;
        }
        for pattern in &self.exclude {
            if let Some(suffix) = pattern.strip_prefix("*.") {
                if path.ends_with(&format!(".{suffix}")) {
                    return true;
                }
                if path.rsplit('.').next() == Some(suffix) {
                    return true;
                }
            }
            if pattern.contains("[*]") {
                let re = format!(
                    "^{}$",
                    pattern.replace("[*]", "\\[\\d+\\]").replace('.', "\\.")
                );
                if regex_test(&re, path, false) == Some(true) {
                    return true;
                }
            }
        }
        false
    }

    /// `*` accepts anything; `regex:<pattern>` tests `String(value)`.
    pub fn matches_rule(&self, path: &str, value: Option<&Json>) -> bool {
        let Some(rule) = self.rules.get(path).filter(|r| !r.is_empty()) else {
            return false;
        };
        if rule == "*" {
            return true;
        }
        if let Some(pattern) = rule.strip_prefix("regex:") {
            let text = JsVal::from_opt(value).to_js_string();
            return regex_test(pattern, &text, false) == Some(true);
        }
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn snap(v: Json) -> Snapshot {
        let mut v = v;
        let o = v.as_object_mut().unwrap();
        o.entry("hash").or_insert(json!("a"));
        o.entry("updatedAt").or_insert(json!(""));
        serde_json::from_value(v).unwrap()
    }

    fn cfg(v: Json) -> SnapshotConfig {
        serde_json::from_value(v).unwrap()
    }

    fn cmp(c: Json, e: Json, r: Json) -> DiffResult {
        SnapshotDiffer::new(&cfg(c)).compare(&snap(e), &snap(r))
    }

    #[test]
    fn matches_identical_snapshots() {
        let s = json!({"status": 200, "body": {"id": 1, "name": "test"}, "hash": "abc123", "updatedAt": "2024-01-01"});
        let r = SnapshotDiffer::new(&cfg(json!({}))).compare(&snap(s.clone()), &snap(s));
        assert!(r.matches);
        assert!(r.differences.is_empty());
    }

    #[test]
    fn detects_status_changes() {
        let r = cmp(json!({}), json!({"status": 200}), json!({"status": 201}));
        assert!(!r.matches);
        assert_eq!(r.differences.len(), 1);
        assert_eq!(
            r.differences[0],
            SnapshotDiff {
                path: "status".into(),
                expected: Some(json!(200)),
                received: Some(json!(201)),
                diff_type: "changed".into()
            }
        );
    }

    #[test]
    fn detects_body_value_changes() {
        let r = cmp(
            json!({}),
            json!({"body": {"name": "old"}}),
            json!({"body": {"name": "new"}}),
        );
        assert!(!r.matches);
        assert_eq!(r.differences[0].path, "body.name");
        assert_eq!(r.differences[0].diff_type, "changed");
    }

    #[test]
    fn detects_added_fields() {
        let r = cmp(
            json!({}),
            json!({"body": {"id": 1}}),
            json!({"body": {"id": 1, "newField": "value"}}),
        );
        assert!(!r.matches);
        assert_eq!(r.differences[0].path, "body.newField");
        assert_eq!(r.differences[0].diff_type, "added");
        assert_eq!(r.differences[0].expected, None);
        assert_eq!(r.differences[0].received, Some(json!("value")));
    }

    #[test]
    fn detects_removed_fields() {
        let r = cmp(
            json!({}),
            json!({"body": {"id": 1, "oldField": "value"}}),
            json!({"body": {"id": 1}}),
        );
        assert!(!r.matches);
        assert_eq!(r.differences[0].path, "body.oldField");
        assert_eq!(r.differences[0].diff_type, "removed");
        assert_eq!(r.differences[0].received, None);
    }

    #[test]
    fn compares_array_elements() {
        let r = cmp(
            json!({}),
            json!({"body": {"items": [1, 2, 3]}}),
            json!({"body": {"items": [1, 2, 4]}}),
        );
        assert!(!r.matches);
        assert_eq!(r.differences[0].path, "body.items[2]");
    }

    #[test]
    fn detects_added_array_elements() {
        let r = cmp(
            json!({}),
            json!({"body": {"items": [1, 2]}}),
            json!({"body": {"items": [1, 2, 3]}}),
        );
        assert!(!r.matches);
        assert_eq!(r.differences[0].path, "body.items[2]");
        assert_eq!(r.differences[0].diff_type, "added");
    }

    #[test]
    fn excludes_exact_paths() {
        let r = cmp(
            json!({"exclude": ["body.timestamp"]}),
            json!({"body": {"id": 1, "timestamp": "2024-01-01"}}),
            json!({"body": {"id": 1, "timestamp": "2024-12-31"}}),
        );
        assert!(r.matches);
    }

    #[test]
    fn excludes_wildcard_paths() {
        let r = cmp(
            json!({"exclude": ["*.createdAt"]}),
            json!({"body": {"user": {"createdAt": "2024-01-01"}, "post": {"createdAt": "2024-01-01"}}}),
            json!({"body": {"user": {"createdAt": "2024-12-31"}, "post": {"createdAt": "2024-12-31"}}}),
        );
        assert!(r.matches);
    }

    #[test]
    fn excludes_array_wildcard_paths() {
        let r = cmp(
            json!({"exclude": ["body.items[*].id"]}),
            json!({"body": {"items": [{"id": 1, "name": "a"}, {"id": 2, "name": "b"}]}}),
            json!({"body": {"items": [{"id": 99, "name": "a"}, {"id": 100, "name": "b"}]}}),
        );
        assert!(r.matches);
    }

    #[test]
    fn match_rule_wildcard_accepts_any_value() {
        let r = cmp(
            json!({"match": {"body.id": "*"}}),
            json!({"body": {"id": 1}}),
            json!({"body": {"id": 999}}),
        );
        assert!(r.matches);
    }

    #[test]
    fn match_rule_regex() {
        let c = json!({"match": {"body.version": "regex:^v\\d+\\.\\d+"}});
        let ok = cmp(
            c.clone(),
            json!({"body": {"version": "v1.0.0"}}),
            json!({"body": {"version": "v2.5.3"}}),
        );
        assert!(ok.matches);
        let bad = cmp(
            c,
            json!({"body": {"version": "v1.0"}}),
            json!({"body": {"version": "invalid"}}),
        );
        assert!(!bad.matches);
    }

    #[test]
    fn invalid_regex_rule_never_matches() {
        let r = cmp(
            json!({"match": {"body.v": "regex:("}}),
            json!({"body": {"v": "a"}}),
            json!({"body": {"v": "b"}}),
        );
        assert!(!r.matches);
    }

    #[test]
    fn detects_type_changes() {
        let r = cmp(
            json!({}),
            json!({"body": {"count": 42}}),
            json!({"body": {"count": "42"}}),
        );
        assert!(!r.matches);
        assert_eq!(r.differences[0].diff_type, "type_mismatch");
    }

    #[test]
    fn detects_object_to_array_change() {
        let r = cmp(
            json!({}),
            json!({"body": {"data": {"key": "value"}}}),
            json!({"body": {"data": ["value"]}}),
        );
        assert!(!r.matches);
        assert_eq!(r.differences[0].diff_type, "type_mismatch");
    }

    #[test]
    fn null_vs_missing_body_is_a_type_mismatch() {
        let r = cmp(json!({}), json!({"body": null}), json!({"status": 200}));
        assert_eq!(r.differences.len(), 2);
        assert_eq!(r.differences[1].path, "body");
        assert_eq!(r.differences[1].diff_type, "type_mismatch");
        assert_eq!(r.differences[1].expected, Some(Json::Null));
        assert_eq!(r.differences[1].received, None);
    }

    #[test]
    fn compares_deeply_nested_values() {
        let r = cmp(
            json!({}),
            json!({"body": {"level1": {"level2": {"level3": {"value": "old"}}}}}),
            json!({"body": {"level1": {"level2": {"level3": {"value": "new"}}}}}),
        );
        assert!(!r.matches);
        assert_eq!(r.differences[0].path, "body.level1.level2.level3.value");
    }

    #[test]
    fn whole_floats_equal_integers() {
        let r = cmp(json!({}), json!({"body": 1}), json!({"body": 1.0}));
        assert!(r.matches);
    }

    #[test]
    fn compares_headers() {
        let r = cmp(
            json!({}),
            json!({"headers": {"content-type": "application/json"}}),
            json!({"headers": {"content-type": "text/html"}}),
        );
        assert!(!r.matches);
        assert_eq!(r.differences[0].path, "headers.content-type");
    }

    #[test]
    fn excludes_headers() {
        let r = cmp(
            json!({"exclude": ["headers.date", "headers.x-request-id"]}),
            json!({"headers": {"content-type": "application/json", "date": "2024-01-01"}}),
            json!({"headers": {"content-type": "application/json", "date": "2024-12-31"}}),
        );
        assert!(r.matches);
    }

    #[test]
    fn excluded_status_is_ignored() {
        let r = cmp(
            json!({"exclude": ["status"]}),
            json!({"status": 200}),
            json!({"status": 500}),
        );
        assert!(r.matches);
    }
}
