//! Snapshot result display (`snapshot/snapshot-formatter.ts`).

use crate::colors::Color;
use curl_runner_core::jsval::num_to_string;
use curl_runner_core::model::{Json, SnapshotCompareResult, SnapshotDiff};
use curl_runner_core::snapshot::js_stringify;

/// Formats comparison results. `color` turns ANSI on; `redact` is applied to
/// every dynamic string (request name, diff values) before coloring.
pub struct SnapshotFormatter<'a> {
    color: bool,
    redact: &'a dyn Fn(&str) -> String,
}

impl<'a> SnapshotFormatter<'a> {
    pub fn new(color: bool, redact: &'a dyn Fn(&str) -> String) -> Self {
        Self { color, redact }
    }

    fn c(&self, text: &str, color: Color) -> String {
        if self.color {
            super::colorize(text, color)
        } else {
            text.to_string()
        }
    }

    /// Multi-line text for one request's snapshot result (no trailing newline).
    pub fn format_result(&self, request_name: &str, result: &SnapshotCompareResult) -> String {
        let name = self.c(&(self.redact)(request_name), Color::Bright);
        let mut lines: Vec<String> = Vec::new();
        if result.is_new && result.updated {
            lines.push(format!(
                "  {} Snapshot created for \"{name}\"",
                self.c("NEW", Color::Cyan)
            ));
            return lines.join("\n");
        }
        if result.updated {
            lines.push(format!(
                "  {} Snapshot updated for \"{name}\"",
                self.c("UPDATED", Color::Yellow)
            ));
            return lines.join("\n");
        }
        if result.matches {
            lines.push(format!(
                "  {} Snapshot matches for \"{name}\"",
                self.c("PASS", Color::Green)
            ));
            return lines.join("\n");
        }
        lines.push(format!(
            "  {} Snapshot mismatch for \"{name}\"",
            self.c("FAIL", Color::Red)
        ));
        lines.push(String::new());
        lines.push(self.format_diff(&result.differences));
        lines.push(String::new());
        lines.push(self.c("  Run with --update-snapshots (-u) to update", Color::Dim));
        lines.join("\n")
    }

    pub fn format_diff(&self, differences: &[SnapshotDiff]) -> String {
        let mut lines = vec![
            format!("  {}", self.c("- Expected", Color::Red)),
            format!("  {}", self.c("+ Received", Color::Green)),
            String::new(),
        ];
        for d in differences {
            lines.push(self.format_difference(d));
        }
        lines.join("\n")
    }

    fn format_difference(&self, d: &SnapshotDiff) -> String {
        let path = if d.path.is_empty() {
            "(root)"
        } else {
            d.path.as_str()
        };
        let path_c = self.c(&(self.redact)(path), Color::Cyan);
        let minus = |v: Option<&Json>, suffix: &str| {
            self.c(&format!("- {}{suffix}", self.stringify(v)), Color::Red)
        };
        let plus = |v: Option<&Json>, suffix: &str| {
            self.c(&format!("+ {}{suffix}", self.stringify(v)), Color::Green)
        };
        let (e, r) = (d.expected.as_ref(), d.received.as_ref());
        let mut lines: Vec<String> = Vec::new();
        match d.diff_type.as_str() {
            "added" => {
                lines.push(format!("  {path_c}:"));
                lines.push(format!("    {}", plus(r, "")));
            }
            "removed" => {
                lines.push(format!("  {path_c}:"));
                lines.push(format!("    {}", minus(e, "")));
            }
            "changed" => {
                lines.push(format!("  {path_c}:"));
                lines.push(format!("    {}", minus(e, "")));
                lines.push(format!("    {}", plus(r, "")));
            }
            "type_mismatch" => {
                lines.push(format!("  {path_c} (type mismatch):"));
                lines.push(format!("    {}", minus(e, &format!(" ({})", type_of(e)))));
                lines.push(format!("    {}", plus(r, &format!(" ({})", type_of(r)))));
            }
            _ => {}
        }
        lines.join("\n")
    }

    /// Display string: strings quoted raw, objects as JSON cut to 80 UTF-16
    /// units (77 + `...`).
    pub(super) fn stringify(&self, v: Option<&Json>) -> String {
        let s = match v {
            None => "undefined".to_string(),
            Some(Json::Null) => "null".to_string(),
            Some(Json::String(s)) => format!("\"{s}\""),
            Some(Json::Bool(b)) => b.to_string(),
            Some(Json::Number(n)) => num_to_string(n.as_f64().unwrap_or(f64::NAN)),
            Some(o) => {
                let s = js_stringify(o);
                let units: Vec<u16> = s.encode_utf16().collect();
                if units.len() > 80 {
                    format!("{}...", String::from_utf16_lossy(&units[..77]))
                } else {
                    s
                }
            }
        };
        (self.redact)(&s)
    }
}

/// JS `typeof` (`null` and arrays are `object`).
pub(super) fn type_of(v: Option<&Json>) -> &'static str {
    match v {
        None => "undefined",
        Some(Json::String(_)) => "string",
        Some(Json::Number(_)) => "number",
        Some(Json::Bool(_)) => "boolean",
        Some(_) => "object",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn id(s: &str) -> String {
        s.to_string()
    }

    fn fmt(color: bool) -> SnapshotFormatter<'static> {
        SnapshotFormatter::new(color, &id)
    }

    fn diff(path: &str, e: Option<Json>, r: Option<Json>, t: &str) -> SnapshotDiff {
        SnapshotDiff {
            path: path.into(),
            expected: e,
            received: r,
            diff_type: t.into(),
        }
    }

    fn res(
        matches: bool,
        is_new: bool,
        updated: bool,
        d: Vec<SnapshotDiff>,
    ) -> SnapshotCompareResult {
        SnapshotCompareResult {
            matches,
            is_new,
            updated,
            differences: d,
        }
    }

    #[test]
    fn new_updated_and_pass() {
        let f = fmt(false);
        assert_eq!(
            f.format_result("Get User", &res(true, true, true, vec![])),
            "  NEW Snapshot created for \"Get User\""
        );
        assert_eq!(
            f.format_result("Get User", &res(true, false, true, vec![])),
            "  UPDATED Snapshot updated for \"Get User\""
        );
        assert_eq!(
            f.format_result("Get User", &res(true, false, false, vec![])),
            "  PASS Snapshot matches for \"Get User\""
        );
    }

    #[test]
    fn colors_match_ts() {
        assert_eq!(
            fmt(true).format_result("A", &res(true, true, true, vec![])),
            "  \x1b[36mNEW\x1b[0m Snapshot created for \"\x1b[1mA\x1b[0m\""
        );
    }

    #[test]
    fn mismatch_lists_every_kind_of_difference() {
        let long = json!({"k": "x".repeat(100)});
        let d = vec![
            diff("body.a", Some(json!(1)), Some(json!(2)), "changed"),
            diff("body.b", None, Some(json!("new")), "added"),
            diff("body.c", Some(json!(null)), None, "removed"),
            diff(
                "body.d",
                Some(json!(42)),
                Some(json!("42")),
                "type_mismatch",
            ),
            diff("", Some(json!("snapshot")), Some(json!("none")), "removed"),
            diff("body.e", Some(long), Some(json!([1, 2])), "type_mismatch"),
        ];
        let out = fmt(false).format_result("R", &res(false, false, false, d));
        let long_json = format!("{{\"k\":\"{}", "x".repeat(71)); // 77 units
        assert_eq!(
            out,
            format!(
                "  FAIL Snapshot mismatch for \"R\"\n\n  - Expected\n  + Received\n\n  body.a:\n    - 1\n    + 2\n  body.b:\n    + \"new\"\n  body.c:\n    - null\n  body.d (type mismatch):\n    - 42 (number)\n    + \"42\" (string)\n  (root):\n    - \"snapshot\"\n  body.e (type mismatch):\n    - {long_json}... (object)\n    + [1,2] (object)\n\n  Run with --update-snapshots (-u) to update"
            )
        );
    }

    #[test]
    fn added_value_may_be_undefined_and_bool() {
        let f = fmt(false);
        let d = diff("body.x", Some(json!(true)), None, "changed");
        assert_eq!(
            f.format_difference(&d),
            "  body.x:\n    - true\n    + undefined"
        );
    }

    #[test]
    fn redactor_applies_to_name_and_values() {
        let redact = |s: &str| s.replace("sekret", "[REDACTED]");
        let f = SnapshotFormatter::new(false, &redact);
        let out = f.format_result(
            "sekret name",
            &res(
                false,
                false,
                false,
                vec![diff(
                    "body.t",
                    Some(json!("sekret")),
                    Some(json!("b")),
                    "changed",
                )],
            ),
        );
        assert!(!out.contains("sekret"));
        assert!(out.contains("[REDACTED] name") && out.contains("- \"[REDACTED]\""));
    }
}
