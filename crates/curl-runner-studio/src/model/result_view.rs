//! What the inspector shows for one `ExecutionResult`: the per-check
//! breakdown, a body diff, the retry timeline, display limits, timing
//! segments and the status line. Pure, so it compiles on every target.
//!
//! Verdicts come from core's own validators, run on one leaf at a time, so
//! they agree with the CLI's pass/fail.

use curl_runner_core::executor::AttemptOutcome;
use curl_runner_core::model::{ExecutionResult, HeaderValue, Json, Metrics, StatusExpect};
use curl_runner_core::validate::{
    get_array_value, is_array_selector, is_regex_pattern, validate_body_properties,
    validate_headers, validate_range_pattern, validate_status,
};
use indexmap::IndexMap;
use std::collections::HashMap;

/// The body text cap, in bytes.
pub const BODY_LIMIT: usize = 256 * 1024;

#[derive(Debug, Clone, PartialEq)]
pub struct Check {
    /// `status`, `header.{name}`, `body.{path}`, `responseTime` or `failure`.
    pub label: String,
    /// `matches ^ch_` for a regex, else the JSON text.
    pub expected: String,
    pub actual: Option<String>,
    pub passed: bool,
}

/// One step from a body value to a child, used to find its line in the diff.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
enum Loc {
    Key(String),
    Idx(usize),
}

struct Leaf {
    /// `body.{path}`, `body` for a scalar body.
    label: String,
    /// Where the actual value sits; cut short where the path cannot be
    /// resolved to one concrete position (selectors, missing keys).
    locs: Vec<Loc>,
    /// Whether the whole path resolved (`actual` is the value at `locs`).
    exact: bool,
    expected: Json,
    actual: Option<Json>,
    passed: bool,
}

fn compact(v: &Json) -> String {
    serde_json::to_string(v).unwrap_or_default()
}

fn expected_text(v: &Json) -> String {
    match v {
        Json::String(s) if is_regex_pattern(s) => format!("matches {s}"),
        other => compact(other),
    }
}

/// `actual?.[key]` as `validate_body_properties` resolves it, plus whether the
/// key maps to one concrete child (object key or array index).
fn child(actual: Option<&Json>, key: &str) -> (Option<Json>, Option<Loc>) {
    match actual {
        Some(Json::Object(m)) => (m.get(key).cloned(), Some(Loc::Key(key.to_string()))),
        Some(Json::Array(a)) => {
            if is_array_selector(key) {
                let idx = key
                    .strip_prefix('[')
                    .and_then(|k| k.strip_suffix(']'))
                    .and_then(|k| k.parse::<i64>().ok())
                    .and_then(|n| {
                        let i = if n >= 0 { n } else { a.len() as i64 + n };
                        usize::try_from(i).ok().filter(|i| *i < a.len())
                    });
                return (get_array_value(a, key), idx.map(Loc::Idx));
            }
            if key == "length" {
                return (Some(Json::from(a.len())), None);
            }
            let is_index = key == "0"
                || (!key.starts_with('0')
                    && !key.is_empty()
                    && key.bytes().all(|b| b.is_ascii_digit()));
            match key.parse::<usize>().ok().filter(|_| is_index) {
                Some(i) => (a.get(i).cloned(), Some(Loc::Idx(i))),
                None => (None, None),
            }
        }
        _ => (None, None),
    }
}

/// Mirror of `validate_body_properties`'s walk, one leaf per scalar (or per
/// array item). Each verdict is core's own, run on the parent with only that
/// leaf expected.
fn leaves(
    actual: Option<&Json>,
    expected: &Json,
    path: &str,
    locs: &[Loc],
    exact: bool,
    out: &mut Vec<Leaf>,
) {
    let shown = if path.is_empty() { "body" } else { path };
    match expected {
        Json::Array(items) if matches!(actual, Some(Json::Array(_))) => {
            for (i, item) in items.iter().enumerate() {
                out.push(Leaf {
                    label: format!("{shown}[{i}]"),
                    // "any item matches": point at the array itself
                    locs: locs.to_vec(),
                    exact: false,
                    expected: item.clone(),
                    actual: actual.cloned(),
                    passed: validate_body_properties(
                        actual,
                        &Json::Array(vec![item.clone()]),
                        path,
                    )
                    .is_empty(),
                });
            }
        }
        Json::Object(props) => {
            for (key, value) in props {
                let current = if path.is_empty() {
                    key.clone()
                } else {
                    format!("{path}.{key}")
                };
                let (child_actual, loc) = child(actual, key);
                let mut child_locs = locs.to_vec();
                let child_exact = exact && loc.is_some() && child_actual.is_some();
                if child_exact {
                    child_locs.extend(loc);
                }
                if matches!(value, Json::Array(_) | Json::Object(_)) {
                    leaves(
                        child_actual.as_ref(),
                        value,
                        &current,
                        &child_locs,
                        child_exact,
                        out,
                    );
                } else {
                    let mut one = serde_json::Map::new();
                    one.insert(key.clone(), value.clone());
                    out.push(Leaf {
                        label: current,
                        locs: child_locs,
                        exact: child_exact,
                        expected: value.clone(),
                        actual: child_actual,
                        passed: validate_body_properties(actual, &Json::Object(one), path)
                            .is_empty(),
                    });
                }
            }
        }
        _ => out.push(Leaf {
            label: shown.to_string(),
            locs: locs.to_vec(),
            exact,
            expected: expected.clone(),
            actual: actual.cloned(),
            passed: validate_body_properties(actual, expected, path).is_empty(),
        }),
    }
}

fn header_text(v: &HeaderValue) -> String {
    match v {
        HeaderValue::One(s) => s.clone(),
        HeaderValue::Many(vs) => vs.join(", "),
    }
}

/// The per-check breakdown of `result.request.expect`. Empty without one.
///
/// With `expect.failure`, the CLI passes only a 4xx/5xx; that shows as an
/// extra `failure` check so the checks as a whole still agree with
/// `result.success`.
pub fn checks(result: &ExecutionResult) -> Vec<Check> {
    let Some(expect) = &result.request.expect else {
        return Vec::new();
    };
    let mut out = Vec::new();
    let actual_status = result.status.unwrap_or(0);

    if let Some(status) = &expect.status {
        let text = match status {
            StatusExpect::One(s) => s.to_string(),
            StatusExpect::Many(v) => v
                .iter()
                .map(u16::to_string)
                .collect::<Vec<_>>()
                .join(" or "),
        };
        out.push(Check {
            label: "status".into(),
            expected: text,
            actual: result.status.map(|s| s.to_string()),
            passed: validate_status(actual_status, status).is_empty(),
        });
    }

    if let Some(headers) = &expect.headers {
        for (name, value) in headers {
            let one: IndexMap<String, String> = IndexMap::from([(name.clone(), value.clone())]);
            let actual = result
                .headers
                .as_ref()
                .and_then(|h| h.get(&name.to_lowercase()))
                .map(header_text);
            out.push(Check {
                label: format!("header.{name}"),
                expected: value.clone(),
                actual,
                passed: validate_headers(&result.headers.clone().unwrap_or_default(), &one)
                    .is_empty(),
            });
        }
    }

    if let Some(body) = &expect.body {
        let mut found = Vec::new();
        leaves(result.body.as_ref(), body, "", &[], true, &mut found);
        for leaf in found {
            out.push(Check {
                label: if leaf.label == "body" {
                    leaf.label
                } else {
                    format!("body.{}", leaf.label)
                },
                expected: expected_text(&leaf.expected),
                actual: leaf.actual.as_ref().map(compact),
                passed: leaf.passed,
            });
        }
    }

    if let Some(rt) = &expect.response_time {
        let ms = result.metrics.as_ref().map(|m| m.duration);
        let passed = ms
            .and_then(serde_json::Number::from_f64)
            .map(Json::Number)
            .is_some_and(|n| validate_range_pattern(Some(&n), rt));
        out.push(Check {
            label: "responseTime".into(),
            expected: rt.clone(),
            actual: ms.map(|m| format!("{m:.0}ms")),
            passed,
        });
    }

    if expect.failure == Some(true) {
        out.push(Check {
            label: "failure".into(),
            expected: "4xx/5xx".into(),
            actual: result.status.map(|s| s.to_string()),
            passed: actual_status >= 400,
        });
    }
    out
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiffKind {
    Same,
    Added,
    Removed,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiffLine {
    pub kind: DiffKind,
    pub text: String,
}

struct PrettyLine {
    indent: usize,
    /// `"key": ` or empty.
    key: String,
    /// The value, or `{` / `[` / `}` / `]`.
    value: String,
    comma: bool,
}

impl PrettyLine {
    fn render(&self) -> String {
        format!(
            "{}{}{}{}",
            "  ".repeat(self.indent),
            self.key,
            self.value,
            if self.comma { "," } else { "" }
        )
    }
}

/// `serde_json::to_string_pretty` layout, line by line, remembering the line
/// where each value starts.
fn pretty_lines(
    v: &Json,
    key: String,
    indent: usize,
    comma: bool,
    at: &mut Vec<Loc>,
    out: &mut Vec<PrettyLine>,
    index: &mut HashMap<Vec<Loc>, usize>,
) {
    index.insert(at.clone(), out.len());
    let (open, close, empty) = match v {
        Json::Object(m) => ("{", "}", m.is_empty()),
        Json::Array(a) => ("[", "]", a.is_empty()),
        scalar => {
            out.push(PrettyLine {
                indent,
                key,
                value: compact(scalar),
                comma,
            });
            return;
        }
    };
    if empty {
        out.push(PrettyLine {
            indent,
            key,
            value: format!("{open}{close}"),
            comma,
        });
        return;
    }
    out.push(PrettyLine {
        indent,
        key,
        value: open.into(),
        comma: false,
    });
    match v {
        Json::Object(m) => {
            let last = m.len() - 1;
            for (i, (k, c)) in m.iter().enumerate() {
                at.push(Loc::Key(k.clone()));
                pretty_lines(
                    c,
                    format!("{}: ", compact(&Json::from(k.as_str()))),
                    indent + 1,
                    i < last,
                    at,
                    out,
                    index,
                );
                at.pop();
            }
        }
        Json::Array(a) => {
            let last = a.len() - 1;
            for (i, c) in a.iter().enumerate() {
                at.push(Loc::Idx(i));
                pretty_lines(c, String::new(), indent + 1, i < last, at, out, index);
                at.pop();
            }
        }
        _ => {}
    }
    out.push(PrettyLine {
        indent,
        key: String::new(),
        value: close.into(),
        comma,
    });
}

/// The actual body, pretty-printed. For each failed body check the expected
/// line is `Added` and the actual line `Removed`, at the actual line's
/// position. A key missing from the actual body is shown as an `Added` line
/// under its parent.
pub fn body_diff(expected: &Json, actual: &Json) -> Vec<DiffLine> {
    let mut lines = Vec::new();
    let mut index = HashMap::new();
    pretty_lines(
        actual,
        String::new(),
        0,
        false,
        &mut Vec::new(),
        &mut lines,
        &mut index,
    );

    let mut found = Vec::new();
    leaves(Some(actual), expected, "", &[], true, &mut found);

    // line -> (expected replacement for it, or an insertion under it)
    let mut replace: HashMap<usize, String> = HashMap::new();
    let mut insert: HashMap<usize, Vec<String>> = HashMap::new();
    for leaf in found.iter().filter(|l| !l.passed) {
        let Some(&line) = index.get(&leaf.locs) else {
            continue;
        };
        let want = compact(&leaf.expected);
        if leaf.exact {
            replace.entry(line).or_insert_with(|| {
                let l = &lines[line];
                PrettyLine {
                    indent: l.indent,
                    key: l.key.clone(),
                    value: want,
                    comma: l.comma,
                }
                .render()
            });
        } else if let Some(name) = leaf.label.rsplit('.').next() {
            // not on the page: say what was wanted under the nearest parent
            let l = &lines[line];
            insert.entry(line).or_default().push(
                PrettyLine {
                    indent: l.indent + 1,
                    key: format!("{}: ", compact(&Json::from(name))),
                    value: want,
                    comma: false,
                }
                .render(),
            );
        }
    }

    let mut out = Vec::new();
    for (i, l) in lines.iter().enumerate() {
        if let Some(want) = replace.get(&i) {
            out.push(DiffLine {
                kind: DiffKind::Added,
                text: want.clone(),
            });
            out.push(DiffLine {
                kind: DiffKind::Removed,
                text: l.render(),
            });
        } else {
            out.push(DiffLine {
                kind: DiffKind::Same,
                text: l.render(),
            });
        }
        for text in insert.remove(&i).unwrap_or_default() {
            out.push(DiffLine {
                kind: DiffKind::Added,
                text,
            });
        }
    }
    out
}

#[derive(Debug, Clone, PartialEq)]
pub struct AttemptRow {
    pub attempt: u32,
    pub status: Option<u16>,
    pub duration_ms: f64,
    pub note: String,
    /// The wait before the next attempt.
    pub wait_ms: Option<u64>,
}

/// The reason phrase for common codes.
pub fn reason_phrase(code: u16) -> Option<&'static str> {
    Some(match code {
        200 => "OK",
        201 => "Created",
        204 => "No Content",
        301 => "Moved Permanently",
        302 => "Found",
        304 => "Not Modified",
        400 => "Bad Request",
        401 => "Unauthorized",
        403 => "Forbidden",
        404 => "Not Found",
        409 => "Conflict",
        422 => "Unprocessable Entity",
        429 => "Too Many Requests",
        500 => "Internal Server Error",
        502 => "Bad Gateway",
        503 => "Service Unavailable",
        504 => "Gateway Timeout",
        _ => return None,
    })
}

fn sentence_case(s: &str) -> String {
    let mut c = s.chars();
    c.next().map_or_else(String::new, |f| {
        f.to_uppercase().collect::<String>() + &c.as_str().to_lowercase()
    })
}

/// The retry timeline. `waits[i]` is the pause after attempt `i`;
/// `final_checks_failed` counts the failed checks of the last response.
pub fn attempt_rows(
    attempts: &[(u32, AttemptOutcome)],
    waits: &[u64],
    final_checks_failed: usize,
    retries_left: bool,
) -> Vec<AttemptRow> {
    let n = attempts.len();
    attempts
        .iter()
        .enumerate()
        .map(|(i, (attempt, o))| {
            let last = i + 1 == n;
            let note = if !last {
                match (o.status, &o.error) {
                    (Some(s), _) if s >= 400 => format!(
                        "{} — retrying",
                        reason_phrase(s).map_or_else(|| format!("HTTP {s}"), sentence_case)
                    ),
                    (Some(_), _) => "Response received — retrying".to_string(),
                    (None, Some(e)) => format!("{e} — retrying"),
                    (None, None) => "No response — retrying".to_string(),
                }
            } else {
                let tail = if retries_left {
                    ""
                } else {
                    " — no retries left"
                };
                match (o.status, &o.error) {
                    (None, e) => format!("{}{tail}", e.as_deref().unwrap_or("No response")),
                    (Some(_), _) if final_checks_failed > 0 => format!(
                        "Response received · {final_checks_failed} {} failed{tail}",
                        if final_checks_failed == 1 {
                            "check"
                        } else {
                            "checks"
                        }
                    ),
                    (Some(_), _) => "Response received".to_string(),
                }
            };
            AttemptRow {
                attempt: *attempt,
                status: o.status,
                duration_ms: o.duration_ms,
                note,
                wait_ms: if last { None } else { waits.get(i).copied() },
            }
        })
        .collect()
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DisplayBody {
    pub text: String,
    pub truncated: bool,
}

/// The body as shown: strings as-is, anything else pretty-printed JSON, cut
/// to [`BODY_LIMIT`] bytes on a char boundary.
pub fn display_body(body: &Json) -> DisplayBody {
    let mut text = match body {
        Json::String(s) => s.clone(),
        other => serde_json::to_string_pretty(other).unwrap_or_default(),
    };
    let truncated = text.len() > BODY_LIMIT;
    if truncated {
        let mut end = BODY_LIMIT;
        while !text.is_char_boundary(end) {
            end -= 1;
        }
        text.truncate(end);
    }
    DisplayBody { text, truncated }
}

/// Where the time went, in ms: `DNS`, `TLS`, `Wait` (to first byte) and
/// `Body`. Core's metrics are curl's cumulative marks, so each segment is the
/// difference to the previous mark. Segments whose marks are missing, or that
/// did not happen (no TLS on plain http), are omitted.
pub fn timing_segments(m: &Metrics) -> Vec<(&'static str, f64)> {
    let mut out = Vec::new();
    let dns = m.dns_lookup.filter(|v| *v > 0.0);
    if let Some(d) = dns {
        out.push(("DNS", d));
    }
    let connected = m.tcp_connection.filter(|v| *v > 0.0).or(dns).unwrap_or(0.0);
    let mut mark = connected;
    if let Some(tls) = m.tls_handshake.filter(|v| *v > 0.0) {
        out.push(("TLS", (tls - connected).max(0.0)));
        mark = tls;
    }
    if let Some(fb) = m.first_byte {
        out.push(("Wait", (fb - mark).max(0.0)));
        if let Some(total) = m.download {
            out.push(("Body", (total - fb).max(0.0)));
        }
    }
    out
}

/// `201 Created`, or just the number for an unlisted code; empty without a
/// response.
pub fn status_line(result: &ExecutionResult) -> String {
    match result.status {
        Some(s) => match reason_phrase(s) {
            Some(r) => format!("{s} {r}"),
            None => s.to_string(),
        },
        None => String::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use curl_runner_core::model::{ExpectConfig, RequestConfig};
    use serde_json::json;

    fn result(expect: Json, status: u16, body: Json, success: bool) -> ExecutionResult {
        let expect: ExpectConfig = serde_json::from_value(expect).unwrap();
        let request = RequestConfig {
            url: "https://api.example.com/charges".into(),
            expect: Some(expect),
            ..Default::default()
        };
        let mut r = ExecutionResult::new(request, success);
        r.status = Some(status);
        r.body = Some(body);
        r.headers = Some(IndexMap::from([(
            "content-type".to_string(),
            HeaderValue::One("application/json".into()),
        )]));
        r.metrics = Some(Metrics {
            duration: 120.0,
            ..Default::default()
        });
        r
    }

    fn charge() -> ExecutionResult {
        result(
            json!({"status": 201, "body": {"id": "^ch_", "amount": 2500}}),
            201,
            json!({"id": "ch_1", "amount": 2000}),
            false,
        )
    }

    #[test]
    fn checks_split_body_leaves() {
        let c = checks(&charge());
        assert_eq!(c.len(), 3);
        assert_eq!(c[0].label, "status");
        assert!(c[0].passed);
        assert_eq!(c[1].label, "body.id");
        assert_eq!(c[1].expected, "matches ^ch_");
        assert!(c[1].passed);
        assert_eq!(c[2].label, "body.amount");
        assert_eq!(c[2].expected, "2500");
        assert_eq!(c[2].actual.as_deref(), Some("2000"));
        assert!(!c[2].passed);
    }

    #[test]
    fn check_verdicts_agree_with_cli() {
        use curl_runner_core::transport::Response;
        use curl_runner_core::validate::check;
        let fixtures = [
            (json!({"status": 200}), 200, json!({})),
            (json!({"status": [200, 201]}), 404, json!({})),
            (
                json!({"body": {"a": {"b": [1, 2]}, "c": "^x", "d": ">= 3", "e": "$exists"}}),
                200,
                json!({"a": {"b": [2, 1]}, "c": "xy", "d": 5, "e": null}),
            ),
            (
                json!({"body": {"a": {"b": [1, 3]}, "c": "$absent", "[0]": {"k": 1}}}),
                200,
                json!([{"k": 2}]),
            ),
            (json!({"failure": true}), 200, json!({})),
            (json!({"failure": true, "status": 404}), 404, json!({})),
            (
                json!({"headers": {"Content-Type": "application/json", "x-a": "1"}, "responseTime": "< 100"}),
                200,
                json!({}),
            ),
            (
                json!({"body": ["a", {"k": 1}]}),
                200,
                json!(["a", {"k": 2}]),
            ),
        ];
        for (expect, status, body) in fixtures {
            let mut r = result(expect.clone(), status, body.clone(), false);
            let resp = Response {
                status,
                headers: r.headers.clone().unwrap(),
                header_history: vec![],
                body,
                metrics: r.metrics.clone().unwrap(),
            };
            let cfg = r.request.expect.clone().unwrap();
            r.success = check(&cfg, &resp).passed;
            let all = checks(&r).iter().all(|c| c.passed);
            assert_eq!(all, r.success, "{expect}");
        }
    }

    #[test]
    fn diff_marks_mismatched_lines() {
        let r = charge();
        let d = body_diff(
            r.request.expect.as_ref().unwrap().body.as_ref().unwrap(),
            r.body.as_ref().unwrap(),
        );
        let at = d
            .iter()
            .position(|l| l.kind == DiffKind::Added && l.text.trim() == "\"amount\": 2500")
            .expect("added line");
        assert_eq!(d[at + 1].kind, DiffKind::Removed);
        assert_eq!(d[at + 1].text.trim(), "\"amount\": 2000");
        assert_eq!(d.iter().filter(|l| l.kind != DiffKind::Same).count(), 2);
    }

    #[test]
    fn diff_shows_missing_key_under_parent() {
        let d = body_diff(&json!({"a": {"b": 1}}), &json!({"a": {}}));
        assert!(
            d.iter()
                .any(|l| l.kind == DiffKind::Added && l.text.contains("\"b\": 1"))
        );
    }

    fn outcome(status: u16, ms: f64) -> AttemptOutcome {
        AttemptOutcome {
            status: Some(status),
            duration_ms: ms,
            error: None,
        }
    }

    #[test]
    fn attempt_notes_match_board() {
        let rows = attempt_rows(
            &[
                (1, outcome(503, 90.0)),
                (2, outcome(503, 80.0)),
                (3, outcome(200, 70.0)),
            ],
            &[1000, 2000],
            2,
            false,
        );
        assert_eq!(rows[0].note, "Service unavailable — retrying");
        assert_eq!(rows[1].note, "Service unavailable — retrying");
        assert_eq!(
            rows[2].note,
            "Response received · 2 checks failed — no retries left"
        );
        assert_eq!(rows[0].wait_ms, Some(1000));
        assert_eq!(rows[2].wait_ms, None);
        let ok = attempt_rows(&[(1, outcome(200, 1.0))], &[], 0, false);
        assert_eq!(ok[0].note, "Response received");
    }

    #[test]
    fn large_body_truncates() {
        let d = display_body(&Json::String("é".repeat(5 * 1024 * 1024 / 2)));
        assert!(d.truncated);
        assert!(d.text.len() <= BODY_LIMIT);
        let small = display_body(&json!({"a": 1}));
        assert!(!small.truncated);
        assert!(small.text.contains("\n"));
    }

    #[test]
    fn non_utf8_body_does_not_panic() {
        let d = display_body(&Json::String("a\u{fffd}\u{fffd}b".into()));
        assert_eq!(d.text, "a\u{fffd}\u{fffd}b");
    }

    #[test]
    fn status_line_and_timing() {
        assert_eq!(status_line(&charge()), "201 Created");
        let mut r = charge();
        r.status = Some(418);
        assert_eq!(status_line(&r), "418");
        let m = Metrics {
            duration: 150.0,
            dns_lookup: Some(8.0),
            tcp_connection: Some(20.0),
            tls_handshake: Some(42.0),
            first_byte: Some(162.0),
            download: Some(170.0),
            ..Default::default()
        };
        assert_eq!(
            timing_segments(&m),
            vec![("DNS", 8.0), ("TLS", 22.0), ("Wait", 120.0), ("Body", 8.0)]
        );
        assert!(timing_segments(&Metrics::default()).is_empty());
    }
}
