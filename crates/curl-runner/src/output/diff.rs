//! Response diff output (`diff/diff-formatter.ts`): terminal, json, markdown.

use super::snapshot::{SnapshotFormatter, type_of};
use crate::colors::Color;
use curl_runner_core::jsval::num_to_string;
use curl_runner_core::model::{DiffCompareResult, DiffSummary, Json, ResponseDiff, TimingDiff};
use curl_runner_core::snapshot::js_stringify_pretty;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Format {
    Terminal,
    Json,
    Markdown,
}

/// `color` is the terminal's color capability; like TS, colors are only used
/// for the `terminal` format (any unknown format string prints the terminal
/// layout without colors).
pub struct DiffFormatter<'a> {
    format: Format,
    color: bool,
    redact: &'a dyn Fn(&str) -> String,
    values: SnapshotFormatter<'a>,
}

/// JS `Number.prototype.toFixed(0)` (ties away from zero, `-0.3` is `-0`).
pub fn to_fixed0(x: f64) -> String {
    if x.is_nan() {
        "NaN".into()
    } else if x.is_infinite() {
        if x > 0.0 { "Infinity" } else { "-Infinity" }.into()
    } else if x < 0.0 {
        format!("-{}", num_to_string((-x).round()))
    } else {
        num_to_string(x.round())
    }
}

/// `100ms → 150ms` and `(+50%)` (sign for `>= 0`, JS `toFixed(0)`).
fn timing_parts(t: &TimingDiff) -> (String, String) {
    let p = t.change_percent;
    let sign = if p >= 0.0 { "+" } else { "" };
    (
        format!(
            "{}ms → {}ms",
            num_to_string(t.baseline),
            num_to_string(t.current)
        ),
        format!("({sign}{}%)", to_fixed0(p)),
    )
}

impl<'a> DiffFormatter<'a> {
    pub fn new(output_format: &str, color: bool, redact: &'a dyn Fn(&str) -> String) -> Self {
        let format = match output_format {
            "json" => Format::Json,
            "markdown" => Format::Markdown,
            _ => Format::Terminal,
        };
        Self {
            format,
            color: color && output_format == "terminal",
            redact,
            values: SnapshotFormatter::new(false, redact),
        }
    }

    fn c(&self, text: &str, color: Color) -> String {
        if self.color {
            super::colorize(text, color)
        } else {
            text.to_string()
        }
    }

    fn r(&self, s: &str) -> String {
        (self.redact)(s)
    }

    pub fn format_summary(&self, s: &DiffSummary, baseline: &str, current: &str) -> String {
        match self.format {
            Format::Json => self.summary_json(s),
            Format::Markdown => self.summary_markdown(s, baseline, current),
            Format::Terminal => self.summary_terminal(s, baseline, current),
        }
    }

    fn summary_terminal(&self, s: &DiffSummary, baseline: &str, current: &str) -> String {
        let mut lines = vec![
            String::new(),
            format!(
                "{} {} → {}",
                self.c("Response Diff:", Color::Bright),
                self.c(&self.r(baseline), Color::Cyan),
                self.c(&self.r(current), Color::Cyan)
            ),
            String::new(),
        ];
        for r in &s.results {
            lines.push(self.format_result_terminal(r));
            lines.push(String::new());
        }
        lines.push(self.stats(s));
        lines.join("\n")
    }

    fn summary_json(&self, s: &DiffSummary) -> String {
        let results: Vec<Json> = s
            .results
            .iter()
            .map(|r| {
                let mut o = serde_json::Map::new();
                o.insert("request".into(), Json::String(r.request_name.clone()));
                let status = if r.has_differences {
                    "changed"
                } else if r.is_new_baseline {
                    "new"
                } else {
                    "unchanged"
                };
                o.insert("status".into(), status.into());
                o.insert(
                    "differences".into(),
                    serde_json::to_value(&r.differences).unwrap_or(Json::Null),
                );
                if let Some(t) = &r.timing_diff {
                    o.insert(
                        "timingDiff".into(),
                        serde_json::to_value(t).unwrap_or(Json::Null),
                    );
                }
                Json::Object(o)
            })
            .collect();
        let v = serde_json::json!({
            "summary": {
                "total": s.total_requests,
                "unchanged": s.unchanged,
                "changed": s.changed,
                "newBaselines": s.new_baselines,
            },
            "results": results,
        });
        js_stringify_pretty(&self.redact_strings(v), 2)
    }

    /// Redacts every string leaf (keys are fixed names or request names).
    fn redact_strings(&self, v: Json) -> Json {
        match v {
            Json::String(s) => Json::String(self.r(&s)),
            Json::Array(a) => Json::Array(a.into_iter().map(|x| self.redact_strings(x)).collect()),
            Json::Object(m) => Json::Object(
                m.into_iter()
                    .map(|(k, x)| (k, self.redact_strings(x)))
                    .collect(),
            ),
            other => other,
        }
    }

    fn summary_markdown(&self, s: &DiffSummary, baseline: &str, current: &str) -> String {
        let mut l: Vec<String> = vec![
            format!(
                "# Response Diff: {} → {}",
                self.r(baseline),
                self.r(current)
            ),
            String::new(),
            "| Metric | Count |".into(),
            "|--------|-------|".into(),
            format!("| Total Requests | {} |", s.total_requests),
            format!("| Unchanged | {} |", s.unchanged),
            format!("| Changed | {} |", s.changed),
            format!("| New Baselines | {} |", s.new_baselines),
            String::new(),
        ];
        if s.changed > 0 {
            l.push("## Changes".into());
            l.push(String::new());
            for r in s.results.iter().filter(|r| r.has_differences) {
                l.push(self.result_markdown(r));
            }
        }
        if s.new_baselines > 0 {
            l.push("## New Requests".into());
            l.push(String::new());
            for r in s.results.iter().filter(|r| r.is_new_baseline) {
                l.push(format!("- `{}`", self.r(&r.request_name)));
            }
            l.push(String::new());
        }
        l.join("\n")
    }

    pub fn format_result_terminal(&self, r: &DiffCompareResult) -> String {
        let name = self.c(&self.r(&r.request_name), Color::Bright);
        if r.is_new_baseline {
            return format!("  {} {name}", self.c("NEW", Color::Cyan));
        }
        if !r.has_differences {
            return format!(
                "  {} {name} {}",
                self.c("✓", Color::Green),
                self.c("(no changes)", Color::Dim)
            );
        }
        let mut lines = vec![format!("  {} {name}", self.c("✗", Color::Red))];
        for d in &r.differences {
            lines.push(self.difference_terminal(d));
        }
        if let Some(t) = &r.timing_diff {
            let p = t.change_percent;
            let color = if p > 20.0 {
                Color::Red
            } else if p < -20.0 {
                Color::Green
            } else {
                Color::Yellow
            };
            let (range, change) = timing_parts(t);
            lines.push(format!(
                "    {} {range} {}",
                self.c("timing:", Color::Cyan),
                self.c(&change, color)
            ));
        }
        lines.join("\n")
    }

    fn difference_terminal(&self, d: &ResponseDiff) -> String {
        let path = self.r(if d.path.is_empty() { "(root)" } else { &d.path });
        let path_c = self.c(&path, Color::Cyan);
        let (b, c) = (d.baseline.as_ref(), d.current.as_ref());
        let minus = |suffix: &str| {
            self.c(
                &format!("- {}{suffix}", self.values.stringify(b)),
                Color::Red,
            )
        };
        let plus = |suffix: &str| {
            self.c(
                &format!("+ {}{suffix}", self.values.stringify(c)),
                Color::Green,
            )
        };
        let mut lines: Vec<String> = Vec::new();
        match d.diff_type.as_str() {
            "added" => {
                lines.push(format!("    {path_c}:"));
                lines.push(format!("      {}", plus("")));
            }
            "removed" => {
                lines.push(format!("    {path_c}:"));
                lines.push(format!("      {}", minus("")));
            }
            "changed" => {
                lines.push(format!("    {path_c}:"));
                lines.push(format!("      {}", minus("")));
                lines.push(format!("      {}", plus("")));
            }
            "type_mismatch" => {
                lines.push(format!("    {path_c} (type mismatch):"));
                lines.push(format!("      {}", minus(&format!(" ({})", type_of(b)))));
                lines.push(format!("      {}", plus(&format!(" ({})", type_of(c)))));
            }
            _ => {}
        }
        lines.join("\n")
    }

    fn result_markdown(&self, r: &DiffCompareResult) -> String {
        let mut l = vec![
            format!("### `{}`", self.r(&r.request_name)),
            String::new(),
            "```diff".to_string(),
        ];
        for d in &r.differences {
            l.push(self.difference_markdown(d));
        }
        l.push("```".into());
        if let Some(t) = &r.timing_diff {
            let (range, change) = timing_parts(t);
            l.push(String::new());
            l.push(format!("**Timing:** {range} {change}"));
        }
        l.push(String::new());
        l.join("\n")
    }

    fn difference_markdown(&self, d: &ResponseDiff) -> String {
        let path = self.r(if d.path.is_empty() { "(root)" } else { &d.path });
        let (b, c) = (d.baseline.as_ref(), d.current.as_ref());
        let (sb, sc) = (self.values.stringify(b), self.values.stringify(c));
        let lines: Vec<String> = match d.diff_type.as_str() {
            "added" => vec![format!("# {path}:"), format!("+ {sc}")],
            "removed" => vec![format!("# {path}:"), format!("- {sb}")],
            "changed" => vec![format!("# {path}:"), format!("- {sb}"), format!("+ {sc}")],
            "type_mismatch" => vec![
                format!("# {path} (type mismatch):"),
                format!("- {sb} ({})", type_of(b)),
                format!("+ {sc} ({})", type_of(c)),
            ],
            _ => vec![],
        };
        lines.join("\n")
    }

    fn stats(&self, s: &DiffSummary) -> String {
        let mut parts = Vec::new();
        if s.unchanged > 0 {
            parts.push(self.c(&format!("{} unchanged", s.unchanged), Color::Green));
        }
        if s.changed > 0 {
            parts.push(self.c(&format!("{} changed", s.changed), Color::Red));
        }
        if s.new_baselines > 0 {
            parts.push(self.c(&format!("{} new", s.new_baselines), Color::Cyan));
        }
        format!("Summary: {} ({} total)", parts.join(", "), s.total_requests)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn id(s: &str) -> String {
        s.to_string()
    }

    fn d(path: &str, b: Option<Json>, c: Option<Json>, t: &str) -> ResponseDiff {
        ResponseDiff {
            path: path.into(),
            baseline: b,
            current: c,
            diff_type: t.into(),
        }
    }

    fn res(name: &str, diffs: Vec<ResponseDiff>, new: bool) -> DiffCompareResult {
        DiffCompareResult {
            request_name: name.into(),
            has_differences: !diffs.is_empty(),
            is_new_baseline: new,
            baseline_label: "a".into(),
            current_label: "b".into(),
            differences: diffs,
            timing_diff: None,
        }
    }

    fn summary() -> DiffSummary {
        let mut chg = res(
            "Chg",
            vec![
                d("status", Some(json!(200)), Some(json!(201)), "changed"),
                d("body.n", None, Some(json!("x")), "added"),
                d("body.o", Some(json!({"k": 1})), None, "removed"),
                d("body.t", Some(json!("1")), Some(json!(1)), "type_mismatch"),
                d("", None, Some(json!("exists")), "added"),
            ],
            false,
        );
        chg.timing_diff = Some(curl_runner_core::model::TimingDiff {
            baseline: 100.0,
            current: 150.0,
            change_percent: 50.0,
        });
        DiffSummary {
            total_requests: 3,
            unchanged: 1,
            changed: 1,
            new_baselines: 1,
            results: vec![res("Same", vec![], false), chg, res("Fresh", vec![], true)],
        }
    }

    #[test]
    fn to_fixed0_matches_js() {
        assert_eq!(to_fixed0(12.5), "13");
        assert_eq!(to_fixed0(-12.5), "-13");
        assert_eq!(to_fixed0(0.5), "1");
        assert_eq!(to_fixed0(2.5), "3");
        assert_eq!(to_fixed0(-0.3), "-0");
        assert_eq!(to_fixed0(-0.0), "0");
        assert_eq!(to_fixed0(49.4999), "49");
        assert_eq!(to_fixed0(f64::INFINITY), "Infinity");
        assert_eq!(to_fixed0(f64::NAN), "NaN");
    }

    #[test]
    fn terminal_plain_layout() {
        let f = DiffFormatter::new("terminal", false, &id);
        let out = f.format_summary(&summary(), "a", "b");
        let expected = "\nResponse Diff: a → b\n\n  ✓ Same (no changes)\n\n  ✗ Chg\n    status:\n      - 200\n      + 201\n    body.n:\n      + \"x\"\n    body.o:\n      - {\"k\":1}\n    body.t (type mismatch):\n      - \"1\" (string)\n      + 1 (number)\n    (root):\n      + \"exists\"\n    timing: 100ms → 150ms (+50%)\n\n  NEW Fresh\n\nSummary: 1 unchanged, 1 changed, 1 new (3 total)";
        assert_eq!(out, expected);
    }

    #[test]
    fn terminal_colors_match_ts_codes() {
        let f = DiffFormatter::new("terminal", true, &id);
        let r = res("X", vec![], true);
        assert_eq!(
            f.format_result_terminal(&r),
            "  \x1b[36mNEW\x1b[0m \x1b[1mX\x1b[0m"
        );
        let r = res("X", vec![], false);
        assert_eq!(
            f.format_result_terminal(&r),
            "  \x1b[32m✓\x1b[0m \x1b[1mX\x1b[0m \x1b[2m(no changes)\x1b[0m"
        );
        // non-terminal formats never color; unknown format is terminal layout, no color
        assert!(!DiffFormatter::new("json", true, &id).color);
        assert!(!DiffFormatter::new("weird", true, &id).color);
    }

    #[test]
    fn timing_colors_and_rounding() {
        let f = DiffFormatter::new("terminal", true, &id);
        let mut r = res(
            "X",
            vec![d("status", Some(json!(1)), Some(json!(2)), "changed")],
            false,
        );
        for (p, color, text) in [
            (50.0, "\x1b[31m", "(+50%)"),
            (-50.0, "\x1b[32m", "(-50%)"),
            (12.5, "\x1b[33m", "(+13%)"),
            (-12.5, "\x1b[33m", "(-13%)"),
            (f64::INFINITY, "\x1b[31m", "(+Infinity%)"),
            (f64::NAN, "\x1b[33m", "(NaN%)"),
        ] {
            r.timing_diff = Some(curl_runner_core::model::TimingDiff {
                baseline: 8.0,
                current: 9.0,
                change_percent: p,
            });
            let out = f.format_result_terminal(&r);
            assert!(
                out.ends_with(&format!("{color}{text}\x1b[0m")),
                "{p}: {out:?}"
            );
        }
    }

    #[test]
    fn stats_with_nothing() {
        let f = DiffFormatter::new("terminal", false, &id);
        let s = DiffSummary::default();
        assert_eq!(
            f.format_summary(&s, "a", "b"),
            "\nResponse Diff: a → b\n\nSummary:  (0 total)"
        );
    }

    #[test]
    fn markdown_layout() {
        let f = DiffFormatter::new("markdown", false, &id);
        let out = f.format_summary(&summary(), "a", "b");
        let expected = "# Response Diff: a → b\n\n| Metric | Count |\n|--------|-------|\n| Total Requests | 3 |\n| Unchanged | 1 |\n| Changed | 1 |\n| New Baselines | 1 |\n\n## Changes\n\n### `Chg`\n\n```diff\n# status:\n- 200\n+ 201\n# body.n:\n+ \"x\"\n# body.o:\n- {\"k\":1}\n# body.t (type mismatch):\n- \"1\" (string)\n+ 1 (number)\n# (root):\n+ \"exists\"\n```\n\n**Timing:** 100ms → 150ms (+50%)\n\n## New Requests\n\n- `Fresh`\n";
        assert_eq!(out, expected);
    }

    #[test]
    fn json_layout_omits_undefined_and_nan() {
        let f = DiffFormatter::new("json", false, &id);
        let mut s = summary();
        s.results[1].timing_diff.as_mut().unwrap().change_percent = f64::INFINITY;
        let out = f.format_summary(&s, "a", "b");
        let v: Json = serde_json::from_str(&out).unwrap();
        assert_eq!(
            v["summary"],
            json!({"total": 3, "unchanged": 1, "changed": 1, "newBaselines": 1})
        );
        assert_eq!(
            v["results"][0],
            json!({"request": "Same", "status": "unchanged", "differences": []})
        );
        assert_eq!(v["results"][2]["status"], "new");
        let chg = &v["results"][1];
        assert_eq!(chg["status"], "changed");
        assert_eq!(
            chg["differences"][1],
            json!({"path": "body.n", "current": "x", "type": "added"})
        );
        assert_eq!(
            chg["timingDiff"],
            json!({"baseline": 100, "current": 150, "changePercent": null})
        );
        assert!(out.starts_with("{\n  \"summary\": {\n    \"total\": 3,"));
    }

    #[test]
    fn redaction_applies_to_names_and_values() {
        let red = |s: &str| s.replace("sekret", "[REDACTED]");
        let f = DiffFormatter::new("terminal", false, &red);
        let s = DiffSummary {
            total_requests: 1,
            changed: 1,
            results: vec![res(
                "sekret req",
                vec![d(
                    "body.t",
                    Some(json!("sekret")),
                    Some(json!("y")),
                    "changed",
                )],
                false,
            )],
            ..Default::default()
        };
        let out = f.format_summary(&s, "a", "b");
        assert!(!out.contains("sekret"), "{out}");
        let out = DiffFormatter::new("json", false, &red).format_summary(&s, "a", "b");
        assert!(!out.contains("sekret"), "{out}");
    }

    #[test]
    fn long_objects_are_cut_at_77_units() {
        let f = DiffFormatter::new("markdown", false, &id);
        let long = json!({"k": "x".repeat(100)});
        let out = f.difference_markdown(&d("p", Some(long), None, "removed"));
        let line = out.lines().nth(1).unwrap();
        assert!(line.ends_with("..."));
        assert_eq!(line.encode_utf16().count(), 2 + 77 + 3);
    }
}
