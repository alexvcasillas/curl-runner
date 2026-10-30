//! The 400px inspector: the last response of the request under the cursor,
//! its checks, headers, timing and the request as sent.

use super::app::mono_font;
use super::color;
use super::icons::{self, glyph};
use super::inspector_failed::{
    render_check_list, render_diff_card, render_timeline, timeline_rows,
};
use super::runs::FileRun;
use super::workspace::Workspace;
use crate::model::collection::{request_at_line, request_lines};
use crate::model::duration_text;
use crate::model::history::relative_time;
use crate::model::result_view::{
    Check, DiffKind, body_diff, checks, display_body, retry_line, status_line, timing_segments,
};
use crate::model::run::RunTarget;
use chrono::Local;
use curl_runner_core::model::{ExecutionResult, HeaderValue, Json};
use gpui_kit::component::spinner::Spinner;
use gpui_kit::component::{Sizable as _, Size};
use gpui_kit::prelude::*;
use gpui_kit::*;
use std::ops::Range;

pub const INSPECTOR_WIDTH: f32 = 400.;
/// Tab padding and label-to-count gap, tighter than the board's 12/6 so
/// `Response, Checks 1, Headers 22, Timing, Request` fit 400px unscrolled.
const TAB_PADDING: f32 = 10.;
const TAB_GAP: f32 = 5.;
const TAB_STRIP_PADDING: f32 = 6.;
/// How long Copy as curl reads `Copied`.
const COPIED_FOR: std::time::Duration = std::time::Duration::from_millis(1500);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InspectorTab {
    Response,
    Checks,
    Attempts,
    Headers,
    Timing,
    Request,
}

impl InspectorTab {
    fn label(self) -> &'static str {
        match self {
            InspectorTab::Response => "Response",
            InspectorTab::Checks => "Checks",
            InspectorTab::Attempts => "Attempts",
            InspectorTab::Headers => "Headers",
            InspectorTab::Timing => "Timing",
            InspectorTab::Request => "Request",
        }
    }
}

/// The tabs a result shows: Attempts only after a retry, and then no
/// Request tab (Copy as curl has it), so the strip fits the column.
pub fn visible_tabs(attempts: usize) -> Vec<InspectorTab> {
    use InspectorTab::*;
    if attempts > 1 {
        vec![Response, Checks, Attempts, Headers, Timing]
    } else {
        vec![Response, Checks, Headers, Timing, Request]
    }
}

/// A JSON token kind, colored like the YAML editor's tokens.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JsonTok {
    Key,
    String,
    Number,
    Punct,
}

/// Token ranges of JSON `text` (pretty-printed or not); whitespace is left
/// out. A string followed by `:` is a key; `true`, `false` and `null` color
/// like numbers, as in the YAML theme.
pub fn json_tokens(text: &str) -> Vec<(Range<usize>, JsonTok)> {
    let b = text.as_bytes();
    let mut out = Vec::new();
    let mut i = 0;
    while i < b.len() {
        match b[i] {
            b'"' => {
                let start = i;
                i += 1;
                while i < b.len() && b[i] != b'"' {
                    i += if b[i] == b'\\' { 2 } else { 1 };
                }
                i = (i + 1).min(b.len());
                let mut j = i;
                while j < b.len() && b[j].is_ascii_whitespace() {
                    j += 1;
                }
                let kind = if b.get(j) == Some(&b':') {
                    JsonTok::Key
                } else {
                    JsonTok::String
                };
                out.push((start..i, kind));
            }
            b'{' | b'}' | b'[' | b']' | b',' | b':' => {
                out.push((i..i + 1, JsonTok::Punct));
                i += 1;
            }
            c if c == b'-' || c.is_ascii_alphanumeric() => {
                let start = i;
                while i < b.len()
                    && (b[i].is_ascii_alphanumeric() || matches!(b[i], b'-' | b'+' | b'.'))
                {
                    i += 1;
                }
                out.push((start..i, JsonTok::Number));
            }
            _ => i += 1,
        }
    }
    out
}

/// `412 B`, `1.2 KB`, `3.4 MB`.
pub fn format_bytes(bytes: f64) -> String {
    const KB: f64 = 1024.;
    if bytes < KB {
        format!("{} B", bytes.round() as u64)
    } else if bytes < KB * KB {
        format!("{:.1} KB", bytes / KB)
    } else {
        format!("{:.1} MB", bytes / (KB * KB))
    }
}

/// An environment as the picker and status bar show it: `.env.staging` is
/// `Staging`.
pub fn env_label(name: &str) -> String {
    let mut c = name.chars();
    c.next().map_or_else(String::new, |f| {
        f.to_uppercase().collect::<String>() + c.as_str()
    })
}

/// `4 of 4 checks passed`, or how many failed.
pub fn checks_summary(passed: usize, total: usize) -> String {
    match total {
        0 => "No checks".to_string(),
        _ if passed == total => format!("{passed} of {total} checks passed"),
        _ => {
            let failed = total - passed;
            format!(
                "{failed} of {total} {} failed",
                if total == 1 { "check" } else { "checks" }
            )
        }
    }
}

/// A part of the inspector's summary line, for its color.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SummaryPart {
    Checks,
    Dot,
    Attempts,
}

/// `2 of 4 checks failed · 3 attempts` as one text and its parts' ranges.
pub fn summary_text(checks: &str, attempts: usize) -> (String, Vec<(Range<usize>, SummaryPart)>) {
    let attempts = plural(attempts, "attempt", "attempts");
    let dot = checks.len()..checks.len() + " · ".len();
    let text = format!("{checks} · {attempts}");
    let runs = vec![
        (0..dot.start, SummaryPart::Checks),
        (dot.clone(), SummaryPart::Dot),
        (dot.end..text.len(), SummaryPart::Attempts),
    ];
    (text, runs)
}

fn plural(n: usize, one: &str, many: &str) -> String {
    format!("{n} {}", if n == 1 { one } else { many })
}

/// `1 variable not set: API_TOKEN`.
pub fn missing_line(missing: &[String]) -> Option<String> {
    (!missing.is_empty()).then(|| {
        format!(
            "{} not set: {}",
            plural(missing.len(), "variable", "variables"),
            missing.join(", ")
        )
    })
}

/// The body card's height cap: about the top half of the column at the
/// 900px default window, so Checks and Timing stay in view below a long
/// body (which scrolls inside the card).
pub const BODY_MAX_HEIGHT: f32 = 420.;
/// Bodies up to this many lines wrap with a hanging indent, one element per
/// line; longer ones wrap as one text, so huge bodies stay cheap.
pub const HANGING_INDENT_MAX_LINES: usize = 300;

/// The response body as shown.
pub struct BodyView {
    pub text: SharedString,
    pub tokens: Vec<(Range<usize>, JsonTok)>,
    /// One entry per line when the body is short enough to wrap each line
    /// under its own indentation.
    pub lines: Option<Vec<BodyLine>>,
    pub truncated: bool,
}

/// One body line: its leading spaces apart, so wrapped text hangs under the
/// line's own indentation.
#[derive(Debug, Clone, PartialEq)]
pub struct BodyLine {
    pub indent: SharedString,
    pub text: SharedString,
    /// Token ranges relative to `text`.
    pub tokens: Vec<(Range<usize>, JsonTok)>,
}

/// `text` split into [`BodyLine`]s, `tokens` clipped and re-based to each.
pub fn body_lines(text: &str, tokens: &[(Range<usize>, JsonTok)]) -> Vec<BodyLine> {
    let mut out = Vec::new();
    let mut toks = tokens.iter().peekable();
    let mut start = 0;
    for line in text.split('\n') {
        let end = start + line.len();
        let indent = line.len() - line.trim_start_matches(' ').len();
        let from = start + indent;
        let mut line_tokens = Vec::new();
        while let Some((r, t)) = toks.peek() {
            if r.start >= end {
                break;
            }
            let (a, b) = (r.start.max(from), r.end.min(end));
            if a < b {
                line_tokens.push((a - from..b - from, *t));
            }
            if r.end <= end {
                toks.next();
            } else {
                break;
            }
        }
        out.push(BodyLine {
            indent: line[..indent].to_string().into(),
            text: line[indent..].to_string().into(),
            tokens: line_tokens,
        });
        start = end + 1;
    }
    out
}

/// The request as it was sent (already redacted by the run).
pub struct SentRequest {
    pub method: String,
    pub url: String,
    pub headers: Vec<(String, String)>,
    pub body: Option<String>,
}

/// What the inspector draws for one result, built once when it arrives.
pub struct ResultView {
    /// `201 Created`; `No response` or `Skipped` without a status.
    pub status: String,
    pub ok: bool,
    /// The response itself succeeded (below 400), whatever the checks say:
    /// the status pill's color.
    pub http_ok: bool,
    pub duration: Option<String>,
    pub size: Option<String>,
    pub checks: Vec<Check>,
    pub passed: usize,
    pub content_type: Option<String>,
    pub body: Option<BodyView>,
    pub headers: Vec<(String, String)>,
    pub timing: Vec<(&'static str, f64)>,
    pub error: Option<String>,
    pub request: SentRequest,
    /// The body against `expect.body`, when a body check failed.
    pub diff: Option<DiffView>,
    /// `count 3 · delay 1000 · backoff ×2`.
    pub retry: Option<String>,
}

/// One line of the body diff, ready to draw.
pub struct DiffRow {
    pub kind: DiffKind,
    /// The line as pretty-printed.
    pub text: SharedString,
    /// Token ranges of `text`.
    pub tokens: Vec<(Range<usize>, JsonTok)>,
    /// An added or removed line as the Diff view shows it: `+` or `-` in
    /// the indent's first column.
    pub marked: SharedString,
}

pub struct DiffView {
    pub rows: Vec<DiffRow>,
    pub truncated: bool,
}

/// `text` with `marker` and a space over its first two columns of indent.
pub fn marked_line(marker: char, text: &str) -> String {
    let rest = text.strip_prefix("  ").unwrap_or(text);
    format!("{marker} {rest}")
}

impl DiffView {
    fn new(expected: &Json, actual: &Json) -> Self {
        let diff = body_diff(expected, actual);
        let rows = diff
            .lines
            .into_iter()
            .map(|l| {
                let marked = match l.kind {
                    DiffKind::Same => String::new(),
                    DiffKind::Added => marked_line('+', &l.text),
                    DiffKind::Removed => marked_line('-', &l.text),
                };
                DiffRow {
                    kind: l.kind,
                    tokens: json_tokens(&l.text),
                    text: l.text.into(),
                    marked: marked.into(),
                }
            })
            .collect();
        DiffView {
            rows,
            truncated: diff.truncated,
        }
    }
}

/// `1.0 s`: a retry wait as the timeline shows it.
pub fn format_wait(ms: u64) -> String {
    format!("{:.1} s", ms as f64 / 1000.)
}

/// What Re-run runs: the one request of a multi-request file, else the
/// file.
pub fn rerun_target(file: &std::path::Path, index: usize, requests: usize) -> RunTarget {
    if requests > 1 {
        RunTarget::Request {
            file: file.to_path_buf(),
            index,
        }
    } else {
        RunTarget::File(file.to_path_buf())
    }
}

impl ResultView {
    /// Checks that failed.
    pub fn failed(&self) -> usize {
        self.checks.len() - self.passed
    }
}

fn header_text(v: &HeaderValue) -> String {
    match v {
        HeaderValue::One(s) => s.clone(),
        HeaderValue::Many(v) => v.join(", "),
    }
}

impl ResultView {
    pub fn new(r: &ExecutionResult) -> Self {
        let checks = if r.skipped == Some(true) {
            Vec::new()
        } else {
            checks(r)
        };
        let passed = checks.iter().filter(|c| c.passed).count();
        let headers: Vec<(String, String)> = r
            .headers
            .iter()
            .flatten()
            .map(|(k, v)| (k.clone(), header_text(v)))
            .collect();
        let content_type = headers
            .iter()
            .find(|(k, _)| k.eq_ignore_ascii_case("content-type"))
            .map(|(_, v)| v.clone());
        let body = r.body.as_ref().filter(|b| !b.is_null()).map(|b| {
            let shown = display_body(b);
            let tokens = if matches!(b, Json::String(_)) {
                Vec::new()
            } else {
                json_tokens(&shown.text)
            };
            let lines = (shown.text.lines().count() <= HANGING_INDENT_MAX_LINES)
                .then(|| body_lines(&shown.text, &tokens));
            BodyView {
                text: shown.text.into(),
                tokens,
                lines,
                truncated: shown.truncated,
            }
        });
        let status = match (r.status, r.skipped) {
            (Some(_), _) => status_line(r),
            (None, Some(true)) => "Skipped".to_string(),
            (None, _) => "No response".to_string(),
        };
        let req = &r.request;
        let body_failed = checks
            .iter()
            .any(|c| !c.passed && c.label.starts_with("body"));
        let diff = match (req.expect.as_ref().and_then(|e| e.body.as_ref()), &r.body) {
            (Some(expected), Some(actual))
                if body_failed && !matches!(actual, Json::String(_) | Json::Null) =>
            {
                Some(DiffView::new(expected, actual))
            }
            _ => None,
        };
        ResultView {
            status,
            ok: r.success,
            http_ok: r.status.is_some_and(|s| s < 400),
            diff,
            retry: retry_line(req.retry.as_ref()),
            duration: r.metrics.as_ref().map(|m| duration_text(m.duration)),
            size: r.metrics.as_ref().and_then(|m| m.size).map(format_bytes),
            passed,
            checks,
            content_type,
            body,
            headers,
            timing: r.metrics.as_ref().map(timing_segments).unwrap_or_default(),
            error: r.error.clone().or_else(|| r.skip_reason.clone()),
            request: SentRequest {
                method: req.method.clone().unwrap_or_else(|| "GET".into()),
                url: req.url.clone(),
                headers: req
                    .headers
                    .iter()
                    .flatten()
                    .map(|(k, v)| (k.clone(), v.clone()))
                    .collect(),
                body: req.body.as_ref().map(|b| display_body(b).text),
            },
        }
    }
}

impl Workspace {
    /// The request of tab `ix` holding the editor's cursor (zero-based).
    pub(super) fn request_under_cursor(&self, ix: usize, cx: &App) -> usize {
        let line = self.tabs[ix].editor.read(cx).cursor_position().line;
        request_at_line(&request_lines(&self.state.open[ix].text), line)
    }

    /// Dev hook: the cursor on zero-based `line` of the active file.
    pub(super) fn dev_move_cursor(
        &mut self,
        line: u32,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(ix) = self.state.active else { return };
        self.tabs[ix].editor.update(cx, |e, cx| {
            e.set_cursor_position(
                gpui_kit::component::input::Position::new(line, 0),
                window,
                cx,
            );
        });
        self.tabs[ix].cursor_request = request_at_line(&self.tabs[ix].request_lines, line);
    }

    /// Dev hook: the inspector tab named `name` (`checks`, `timing`, …).
    pub(super) fn dev_show_tab(&mut self, name: &str) {
        use InspectorTab::*;
        if let Some(tab) = [Response, Checks, Attempts, Headers, Timing, Request]
            .into_iter()
            .find(|t| t.label().eq_ignore_ascii_case(name))
        {
            self.inspector_tab = tab;
        }
    }

    /// The inspector's file and request, when a file is open.
    fn inspected(&self) -> Option<(usize, usize)> {
        let ix = self.state.active?;
        Some((ix, self.tabs[ix].cursor_request))
    }

    /// Copies the inspector's request as the curl command it ran as.
    pub(super) fn copy_as_curl(&mut self, cx: &mut Context<Self>) {
        let Some((ix, index)) = self.inspected() else {
            return;
        };
        let command = self
            .runs
            .last
            .get(&self.state.open[ix].rel_path)
            .and_then(|r| r.commands.get(&index));
        if let Some(command) = command {
            cx.write_to_clipboard(ClipboardItem::new_string(command.clone()));
            // The button reads `Copied` for a moment.
            self.copied = Some(cx.spawn(async move |this, cx| {
                cx.background_executor().timer(COPIED_FOR).await;
                this.update(cx, |this, cx| {
                    this.copied = None;
                    cx.notify();
                })
                .ok();
            }));
            cx.notify();
        }
    }

    /// Runs the inspector's request again, as it ran.
    pub(super) fn rerun(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some((ix, index)) = self.inspected() else {
            return;
        };
        let file = self.state.open[ix].rel_path.clone();
        let target = rerun_target(&file, index, self.tabs[ix].request_lines.len());
        self.run(target, window, cx);
    }

    /// Failed checks across `file`'s last run, and whether any request
    /// failed: the breadcrumb's `2 checks failed on last run`.
    pub(super) fn last_run_failures(&self, file: &std::path::Path) -> Option<String> {
        let run = self.runs.last.get(file)?;
        let failed: usize = run.views.values().map(ResultView::failed).sum();
        match failed {
            0 if run.views.values().all(|v| v.ok) && run.load_error.is_none() => None,
            0 => Some("Failed on last run".to_string()),
            n => Some(format!(
                "{} failed on last run",
                plural(n, "check", "checks")
            )),
        }
    }

    pub(super) fn render_inspector(&self, cx: &mut Context<Self>) -> impl IntoElement + use<> {
        let aside = div()
            .id("inspector")
            .role(Role::Complementary)
            .test_support()
            .flex()
            .flex_col()
            .flex_none()
            .w(px(INSPECTOR_WIDTH))
            .h_full()
            .min_h_0()
            .border_l_1()
            .border_color(color("border-100"))
            .bg(color("bg-100"));
        let Some(ix) = self.state.active else {
            return aside.child(centered(
                div()
                    .text_color(color("text-secondary"))
                    .child("Open a request to see its response"),
            ));
        };
        let file = &self.state.open[ix].rel_path;
        let index = self.tabs[ix].cursor_request;
        let run = self.runs.last.get(file);
        let running = self
            .runs
            .progress
            .as_ref()
            .filter(|p| &p.file == file && p.index == index);
        if let Some(p) = running {
            return aside.child(centered(
                div()
                    .id("inspector-running")
                    .role(Role::Label)
                    .test_support()
                    .aria_label(p.label())
                    .flex()
                    .items_center()
                    .gap(px(8.))
                    .text_color(color("text-secondary"))
                    .child(
                        Spinner::new()
                            .with_size(Size::Small)
                            .color(color("text-secondary")),
                    )
                    .child(p.label()),
            ));
        }
        if let Some(message) = run.and_then(|r| r.load_error.clone()) {
            return aside.child(render_load_error(message));
        }
        match run.and_then(|r| Some((r, r.views.get(&index)?))) {
            Some((run, view)) => aside
                .child(self.render_result_header(run, index, view, cx))
                .child(self.render_result_tabs(run, index, view, cx))
                .child(self.render_result_body(run, index, view, cx)),
            None if self.runs.is_pending(file, index) => aside.child(centered(
                div()
                    .text_color(color("text-secondary"))
                    .child("Waiting to run…"),
            )),
            None => aside.child(self.render_empty()),
        }
    }

    /// Before any run: a missing-variable warning and the ⌘↵ hint.
    fn render_empty(&self) -> impl IntoElement + use<> {
        centered(
            div()
                .flex()
                .flex_col()
                .items_center()
                .gap(px(12.))
                .children(missing_line(&self.missing).map(missing_note))
                .child(
                    div()
                        .flex()
                        .items_center()
                        .gap(px(8.))
                        .text_color(color("text-secondary"))
                        .child("Run this request to see the response")
                        .child(key_chip("⌘↵")),
                ),
        )
    }

    /// Status pill, time and size, environment and age; then the checks and
    /// attempts summary. A failed result trades the environment and age for
    /// Copy as curl and Re-run, and moves them to the summary line.
    fn render_result_header(
        &self,
        run: &FileRun,
        index: usize,
        v: &ResultView,
        cx: &mut Context<Self>,
    ) -> Div {
        let (fg, bg, border) = if v.ok || v.http_ok {
            ("success", "success-subtle", "success-border")
        } else {
            ("danger", "danger-subtle", "danger-border")
        };
        let attempts = run.attempt_count(index).max(1);
        let dot = || {
            div()
                .flex_none()
                .text_color(color("text-disabled"))
                .child("·")
        };
        let mut facts = div().flex().flex_none().items_center().gap(px(8.));
        let duration = v.duration.clone().map(|d| {
            if attempts > 1 {
                format!("{d} total")
            } else {
                d
            }
        });
        let mut first = true;
        for fact in [duration, v.size.clone()].into_iter().flatten() {
            if !first {
                facts = facts.child(dot());
            }
            first = false;
            facts = facts.child(
                div()
                    .flex_none()
                    .whitespace_nowrap()
                    .text_color(color("text-secondary"))
                    .child(fact),
            );
        }
        let all_passed = v.failed() == 0;
        let checks_color = match (v.checks.is_empty(), all_passed) {
            (true, _) => color("text-secondary"),
            (false, true) => color("success"),
            (false, false) => color("danger"),
        };
        let summary = checks_summary(v.passed, v.checks.len());
        let actions = (!v.ok).then(|| {
            div()
                .flex()
                .flex_none()
                .items_center()
                .gap(px(8.))
                .when(run.commands.contains_key(&index), |d| {
                    d.child(small_button(
                        "inspector-copy-curl",
                        icons::COPY,
                        if self.copied.is_some() {
                            "Copied"
                        } else {
                            "Copy as curl"
                        },
                        false,
                        cx.listener(|this, _, _, cx| this.copy_as_curl(cx)),
                    ))
                })
                .child(small_button(
                    "inspector-rerun",
                    icons::RERUN,
                    "Re-run",
                    true,
                    cx.listener(|this, _, window, cx| this.rerun(window, cx)),
                ))
        });
        div()
            .flex()
            .flex_col()
            .flex_none()
            .gap(px(10.))
            .pt(px(14.))
            .px(px(16.))
            .pb(px(12.))
            .border_b_1()
            .border_color(color("border-100"))
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(8.))
                    .child(
                        div()
                            .id("inspector-status")
                            .role(Role::Label)
                            .test_support()
                            .aria_label(v.status.clone())
                            // Only the status gives way when the row can't fit.
                            .flex_shrink(1.)
                            .min_w_0()
                            .whitespace_nowrap()
                            .text_ellipsis()
                            .px(px(8.))
                            .py(px(2.))
                            .rounded_full()
                            .border_1()
                            .border_color(color(border))
                            .bg(color(bg))
                            .text_size(px(12.))
                            .line_height(px(16.))
                            .font_weight(FontWeight::MEDIUM)
                            .text_color(color(fg))
                            .child(v.status.clone()),
                    )
                    .child(facts)
                    .child(div().flex_1())
                    .child(
                        div()
                            .flex_none()
                            .whitespace_nowrap()
                            .text_size(px(12.))
                            .text_color(color("text-secondary"))
                            .child(format!(
                                "{} · {}",
                                env_label(&run.env),
                                relative_time(run.at, Local::now())
                            )),
                    ),
            )
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(8.))
                    .text_size(px(12.))
                    .text_color(color("text-secondary"))
                    .child(
                        // One text, so the summary wraps at word boundaries
                        // and the dot hugs it; the buttons keep their width.
                        div()
                            .id("inspector-checks")
                            .role(Role::Label)
                            .test_support()
                            .aria_label(summary.clone())
                            .flex()
                            .flex_1()
                            .min_w_0()
                            .items_center()
                            .gap(px(6.))
                            .when(!v.checks.is_empty(), |d| {
                                d.child(
                                    glyph(if all_passed {
                                        icons::CHECK
                                    } else {
                                        icons::CROSS
                                    })
                                    .size(px(13.))
                                    .flex_none()
                                    .text_color(checks_color),
                                )
                            })
                            .child({
                                let (text, runs) = summary_text(&summary, attempts);
                                let tone = |part: SummaryPart| match part {
                                    SummaryPart::Checks => checks_color,
                                    SummaryPart::Dot => color("text-disabled"),
                                    SummaryPart::Attempts if attempts > 1 => color("warning"),
                                    SummaryPart::Attempts => color("text-secondary"),
                                };
                                let styles = runs
                                    .into_iter()
                                    .map(|(r, part)| {
                                        let style = HighlightStyle {
                                            color: Some(tone(part)),
                                            ..Default::default()
                                        };
                                        (r, style)
                                    })
                                    .collect::<Vec<_>>();
                                div()
                                    .flex_1()
                                    .min_w_0()
                                    .child(StyledText::new(text).with_highlights(styles))
                            }),
                    )
                    .children(actions),
            )
            .children(missing_line(&self.missing).map(missing_note))
    }

    fn render_result_tabs(
        &self,
        run: &FileRun,
        index: usize,
        v: &ResultView,
        cx: &mut Context<Self>,
    ) -> impl IntoElement + use<> {
        let attempts = run.attempt_count(index);
        let tabs = visible_tabs(attempts);
        let current = self.shown_tab(&tabs);
        div()
            .id("inspector-tabs")
            .flex()
            .flex_none()
            .items_end()
            // A safety net: the tabs fit the 400px column.
            .overflow_x_scroll()
            .h(px(40.))
            .px(px(TAB_STRIP_PADDING))
            .border_b_1()
            .border_color(color("border-100"))
            .children(tabs.into_iter().map(|tab| {
                let failed = v.failed();
                let (count, count_color) = match tab {
                    // Dropped beside the Attempts tab, as on the board.
                    InspectorTab::Headers if attempts <= 1 => {
                        (Some(v.headers.len()), "text-tertiary")
                    }
                    InspectorTab::Attempts => (Some(attempts), "text-tertiary"),
                    InspectorTab::Checks => (Some(failed), "danger"),
                    _ => (None, "text-tertiary"),
                };
                let selected = tab == current;
                div()
                    .id(("inspector-tab", tab as usize))
                    .flex()
                    .flex_none()
                    .items_center()
                    .gap(px(TAB_GAP))
                    .h(px(32.))
                    .px(px(TAB_PADDING))
                    .text_size(px(13.))
                    .border_b_1()
                    .cursor_pointer()
                    .map(|t| {
                        if selected {
                            t.text_color(color("text-primary"))
                                .border_color(color("primary"))
                        } else {
                            t.text_color(color("text-secondary"))
                                .border_color(gpui_kit::transparent_black())
                                .hover(|s| s.text_color(color("text-primary")))
                        }
                    })
                    .child(tab.label())
                    .children(count.filter(|n| *n > 0).map(|n| {
                        div()
                            .text_size(px(11.))
                            .text_color(color(count_color))
                            .child(n.to_string())
                    }))
                    .on_click(cx.listener(move |this, _, _, cx| {
                        this.inspector_tab = tab;
                        cx.notify();
                    }))
            }))
    }

    /// The picked tab, or Response when this result does not show it.
    fn shown_tab(&self, tabs: &[InspectorTab]) -> InspectorTab {
        if tabs.contains(&self.inspector_tab) {
            self.inspector_tab
        } else {
            InspectorTab::Response
        }
    }

    fn render_result_body(
        &self,
        run: &FileRun,
        index: usize,
        v: &ResultView,
        cx: &mut Context<Self>,
    ) -> impl IntoElement + use<> {
        let attempts = run.attempt_count(index);
        let content = div()
            .id("inspector-content")
            .flex()
            .flex_col()
            .flex_1()
            .min_h_0()
            .gap(px(16.))
            .py(px(12.))
            .px(px(16.))
            .overflow_y_scroll();
        match self.shown_tab(&visible_tabs(attempts)) {
            InspectorTab::Response => content
                .child(render_body_card(v, cx))
                .when(!v.checks.is_empty(), |c| c.child(render_checks(v)))
                .when(!v.timing.is_empty(), |c| c.child(render_timing(v))),
            InspectorTab::Checks => {
                let rows = timeline_rows(run, index, v);
                content
                    .child(if v.checks.is_empty() {
                        note("This request has no checks.").into_any_element()
                    } else {
                        render_check_list(v).into_any_element()
                    })
                    .children(
                        v.diff
                            .as_ref()
                            .map(|d| render_diff_card(d, self.body_raw, cx)),
                    )
                    .when(rows.len() > 1, |c| {
                        c.child(render_timeline(&rows, v.retry.as_deref(), v.ok))
                    })
            }
            InspectorTab::Attempts => {
                let rows = timeline_rows(run, index, v);
                content.child(render_timeline(&rows, v.retry.as_deref(), v.ok))
            }
            InspectorTab::Headers => content.child(if v.headers.is_empty() {
                note("No response headers.").into_any_element()
            } else {
                key_values(&v.headers).into_any_element()
            }),
            InspectorTab::Timing => content.child(if v.timing.is_empty() {
                note("No timing for this request.").into_any_element()
            } else {
                render_timing(v).into_any_element()
            }),
            InspectorTab::Request => content.child(render_request(&v.request)),
        }
    }
}

/// The amber `1 variable not set: API_TOKEN` line.
fn missing_note(line: String) -> impl IntoElement {
    div()
        .id("inspector-missing")
        .role(Role::Label)
        .test_support()
        .aria_label(line.clone())
        .flex()
        .items_center()
        .gap(px(6.))
        .text_size(px(12.))
        .text_color(color("warning"))
        .child(glyph(icons::WARNING).size(px(13.)).flex_none())
        .child(line)
}

/// `child` in the middle of the inspector.
fn centered(child: impl IntoElement) -> Div {
    div()
        .flex()
        .flex_1()
        .min_h_0()
        .items_center()
        .justify_center()
        .px(px(24.))
        .text_size(px(13.))
        .child(child)
}

fn note(text: &'static str) -> Div {
    div().text_color(color("text-secondary")).child(text)
}

/// The mockups' keyboard chip.
fn key_chip(keys: &'static str) -> Div {
    div()
        .flex_none()
        .px(px(5.))
        .rounded(px(4.))
        .border_1()
        .border_color(color("border-200"))
        .font_family(mono_font())
        .text_size(px(11.))
        .line_height(px(18.))
        .text_color(color("text-secondary"))
        .child(keys)
}

pub(super) fn section(title: &'static str) -> Div {
    div()
        .text_size(px(12.))
        .font_weight(FontWeight::MEDIUM)
        .text_color(color("text-secondary"))
        .child(title)
}

fn render_load_error(message: String) -> Div {
    div()
        .flex()
        .flex_col()
        .gap(px(10.))
        .pt(px(14.))
        .px(px(16.))
        .child(
            div().flex().child(
                div()
                    .id("inspector-status")
                    .role(Role::Label)
                    .test_support()
                    .aria_label("Couldn't load")
                    .px(px(8.))
                    .py(px(2.))
                    .rounded_full()
                    .border_1()
                    .border_color(color("danger-border"))
                    .bg(color("danger-subtle"))
                    .text_size(px(12.))
                    .line_height(px(16.))
                    .font_weight(FontWeight::MEDIUM)
                    .text_color(color("danger"))
                    .child("Couldn't load"),
            ),
        )
        .child(
            div()
                .font_family(mono_font())
                .text_size(px(12.))
                .line_height(px(20.))
                .text_color(color("text-primary"))
                .child(message),
        )
}

/// The body in a card: content type and copy button, then the text, JSON
/// in the YAML theme's colors.
fn render_body_card(v: &ResultView, cx: &mut Context<Workspace>) -> impl IntoElement + use<> {
    let copy_text = v.body.as_ref().map(|b| b.text.clone());
    let header = div()
        .flex()
        .items_center()
        .h(px(32.))
        .px(px(12.))
        .border_b_1()
        .border_color(color("border-100"))
        .bg(color("surface-raised"))
        .text_size(px(12.))
        .text_color(color("text-secondary"))
        .child(
            div()
                .flex_1()
                .min_w_0()
                .text_ellipsis()
                .child(v.content_type.clone().unwrap_or_else(|| "No body".into())),
        )
        .children(copy_text.map(|text| {
            div()
                .id("inspector-copy")
                .flex()
                .items_center()
                .justify_center()
                .size(px(24.))
                .mr(px(-6.))
                .rounded(px(4.))
                .cursor_pointer()
                .hover(|s| s.bg(color("surface-300")))
                .child(
                    glyph(icons::COPY)
                        .size(px(13.))
                        .text_color(color("text-secondary")),
                )
                .on_click(cx.listener(move |_, _, _, cx| {
                    cx.write_to_clipboard(ClipboardItem::new_string(text.to_string()));
                }))
        }));
    let text = match (&v.body, &v.error) {
        (Some(body), _) => {
            let colors = [
                (JsonTok::Key, color("syntax-key")),
                (JsonTok::String, color("syntax-string")),
                (JsonTok::Number, color("syntax-number")),
                (JsonTok::Punct, color("syntax-punctuation")),
            ];
            let styled = |text: &SharedString, tokens: &[(Range<usize>, JsonTok)]| {
                let styles = tokens
                    .iter()
                    .map(|(r, t)| {
                        let color = colors.iter().find(|(k, _)| k == t).map(|(_, c)| *c);
                        let style = HighlightStyle {
                            color,
                            ..Default::default()
                        };
                        (r.clone(), style)
                    })
                    .collect::<Vec<_>>();
                StyledText::new(text.clone()).with_highlights(styles)
            };
            let default = if body.tokens.is_empty() {
                color("text-primary")
            } else {
                color("syntax-punctuation")
            };
            let content = match &body.lines {
                Some(lines) => div()
                    .flex()
                    .flex_col()
                    .children(lines.iter().map(|line| {
                        div()
                            .flex()
                            .min_h(px(20.))
                            .when(!line.indent.is_empty(), |row| {
                                row.child(
                                    div()
                                        .flex_none()
                                        .whitespace_nowrap()
                                        .child(line.indent.clone()),
                                )
                            })
                            .child(
                                div()
                                    .flex_1()
                                    .min_w_0()
                                    .child(styled(&line.text, &line.tokens)),
                            )
                    }))
                    .into_any_element(),
                None => styled(&body.text, &body.tokens).into_any_element(),
            };
            div()
                .text_color(default)
                .child(content)
                .when(body.truncated, |d| {
                    d.child(
                        div()
                            .pt(px(8.))
                            .font_family(super::app::ui_font())
                            .text_color(color("text-tertiary"))
                            .child("Showing first 256 KB"),
                    )
                })
        }
        (None, Some(error)) => div().text_color(color("danger")).child(error.clone()),
        (None, None) => div().text_color(color("text-tertiary")).child("Empty body"),
    };
    div()
        .flex_none()
        .border_1()
        .border_color(color("border-100"))
        .rounded(px(8.))
        .overflow_hidden()
        .child(header)
        .child(
            div()
                .id("inspector-body")
                .py(px(10.))
                .px(px(12.))
                .font_family(mono_font())
                .text_size(px(12.))
                .line_height(px(20.))
                .max_h(px(BODY_MAX_HEIGHT))
                .overflow_y_scroll()
                .child(text),
        )
}

/// One row per check, under the response body.
fn render_checks(v: &ResultView) -> impl IntoElement + use<> {
    div()
        .flex()
        .flex_col()
        .gap(px(2.))
        .child(section("Checks").pb(px(6.)))
        .children(v.checks.iter().map(|c| {
            let (icon, fg) = if c.passed {
                (icons::CHECK, color("success"))
            } else {
                (icons::CROSS, color("danger"))
            };
            let right = c.expected.clone();
            div()
                .flex()
                .items_center()
                .gap(px(8.))
                .h(px(28.))
                .child(glyph(icon).size(px(13.)).flex_none().text_color(fg))
                .child(
                    div()
                        .flex_none()
                        .max_w(px(180.))
                        .text_ellipsis()
                        .child(c.label.clone()),
                )
                .child(
                    div()
                        .flex_1()
                        .min_w_0()
                        .flex()
                        .justify_end()
                        .font_family(mono_font())
                        .text_size(px(12.))
                        .text_color(if c.passed {
                            color("text-secondary")
                        } else {
                            fg
                        })
                        .child(div().min_w_0().text_ellipsis().child(right)),
                )
        }))
}

/// The stacked bar and its legend: DNS, TLS, Wait, Body.
fn render_timing(v: &ResultView) -> impl IntoElement + use<> {
    let segment_color = |name: &str| {
        color(match name {
            "DNS" => "text-disabled",
            "TLS" => "text-tertiary",
            "Wait" => "accent-blue",
            _ => "success",
        })
    };
    let total: f64 = v.timing.iter().map(|(_, ms)| ms).sum();
    let segments = v.timing.len();
    div()
        .flex()
        .flex_col()
        .gap(px(8.))
        .child(section("Timing"))
        .child(
            div()
                .flex()
                .h(px(6.))
                .gap(px(2.))
                .rounded(px(3.))
                .overflow_hidden()
                .children(v.timing.iter().enumerate().map(|(i, (name, ms))| {
                    let mut seg = div()
                        .h_full()
                        .flex_basis(px(0.))
                        .bg(segment_color(name))
                        // GPUI clips to a square box, so the ends round here.
                        .when(i == 0, |s| s.rounded_l(px(3.)))
                        .when(i + 1 == segments, |s| s.rounded_r(px(3.)));
                    // Share of the total, at least a sliver so each shows.
                    seg.style().flex_grow = Some((*ms / total.max(1.)).max(0.01) as f32);
                    seg
                })),
        )
        .child(
            div()
                .flex()
                .gap(px(8.))
                .text_size(px(12.))
                .text_color(color("text-secondary"))
                .children(v.timing.iter().map(|(name, ms)| {
                    div()
                        .flex_1()
                        .flex_basis(px(0.))
                        .min_w_0()
                        .child(format!("{name} {}", duration_text(*ms)))
                })),
        )
}

/// The board's `.btn.sm`, outlined or (`primary`) filled.
fn small_button(
    id: &'static str,
    icon: &'static str,
    label: &'static str,
    primary: bool,
    on_click: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
) -> impl IntoElement {
    let (bg, fg, border) = if primary {
        ("primary", "on-primary", "primary")
    } else {
        ("bg-100", "text-primary", "border-200")
    };
    div()
        .id(id)
        .on_click(on_click)
        .role(Role::Button)
        .test_support()
        .aria_label(label)
        .flex()
        .flex_none()
        .items_center()
        .gap(px(8.))
        .h(px(28.))
        .px(px(10.))
        .rounded(px(6.))
        .border_1()
        .border_color(color(border))
        .bg(color(bg))
        .text_size(px(12.))
        .text_color(color(fg))
        .when(primary, |b| {
            b.font_weight(FontWeight::MEDIUM)
                .hover(|s| s.bg(color("primary-hover")))
        })
        .when(!primary, |b| b.hover(|s| s.bg(color("surface-100"))))
        .cursor_pointer()
        .child(glyph(icon).size(px(13.)).flex_none())
        .child(label)
}

fn key_values(pairs: &[(String, String)]) -> impl IntoElement + use<> {
    div()
        .flex()
        .flex_col()
        .gap(px(6.))
        .font_family(mono_font())
        .text_size(px(12.))
        .line_height(px(18.))
        .children(pairs.iter().map(|(k, v)| {
            div()
                .flex()
                .gap(px(12.))
                .child(
                    div()
                        .flex_none()
                        .w(px(132.))
                        .text_ellipsis()
                        .text_color(color("text-secondary"))
                        .child(k.clone()),
                )
                .child(
                    div()
                        .flex_1()
                        .min_w_0()
                        .text_color(color("text-primary"))
                        .child(v.clone()),
                )
        }))
}

fn render_request(r: &SentRequest) -> impl IntoElement + use<> {
    div()
        .flex()
        .flex_col()
        .gap(px(16.))
        .child(
            div()
                .flex()
                .gap(px(8.))
                .font_family(mono_font())
                .text_size(px(12.))
                .line_height(px(18.))
                .child(
                    div()
                        .flex_none()
                        .font_weight(FontWeight::MEDIUM)
                        .text_color(super::method_color(&r.method))
                        .child(r.method.clone()),
                )
                .child(
                    div()
                        .flex_1()
                        .min_w_0()
                        .text_color(color("text-primary"))
                        .child(r.url.clone()),
                ),
        )
        .when(!r.headers.is_empty(), |d| {
            d.child(
                div()
                    .flex()
                    .flex_col()
                    .gap(px(8.))
                    .child(section("Headers"))
                    .child(key_values(&r.headers)),
            )
        })
        .children(r.body.clone().map(|body| {
            div()
                .flex()
                .flex_col()
                .gap(px(8.))
                .child(section("Body"))
                .child(
                    div()
                        .font_family(mono_font())
                        .text_size(px(12.))
                        .line_height(px(20.))
                        .text_color(color("text-primary"))
                        .child(body),
                )
        }))
}

#[cfg(test)]
mod tests {
    use super::{
        DiffKind, ExecutionResult, InspectorTab, JsonTok, ResultView, RunTarget, body_lines,
        checks_summary, env_label, format_bytes, format_wait, json_tokens, marked_line,
        missing_line, rerun_target, visible_tabs,
    };

    #[test]
    fn json_tokens_tell_keys_from_values() {
        let text = "{\n  \"id\": \"ch_1\",\n  \"n\": -2.5,\n  \"ok\": true\n}";
        let toks: Vec<(&str, JsonTok)> = json_tokens(text)
            .into_iter()
            .map(|(r, t)| (&text[r], t))
            .collect();
        use JsonTok::*;
        assert_eq!(
            toks,
            [
                ("{", Punct),
                ("\"id\"", Key),
                (":", Punct),
                ("\"ch_1\"", String),
                (",", Punct),
                ("\"n\"", Key),
                (":", Punct),
                ("-2.5", Number),
                (",", Punct),
                ("\"ok\"", Key),
                (":", Punct),
                ("true", Number),
                ("}", Punct),
            ]
        );
    }

    #[test]
    fn json_tokens_handle_escapes_and_unterminated_strings() {
        let text = r#"["a\"b", "c"#;
        let toks: Vec<&str> = json_tokens(text)
            .into_iter()
            .map(|(r, _)| &text[r])
            .collect();
        assert_eq!(toks, ["[", r#""a\"b""#, ",", "\"c"]);
    }

    #[test]
    fn body_lines_hang_tokens_under_their_indent() {
        let text = "{\n  \"id\": 1\n}";
        let lines = body_lines(text, &json_tokens(text));
        assert_eq!(lines.len(), 3);
        assert_eq!(lines[1].indent.as_ref(), "  ");
        assert_eq!(lines[1].text.as_ref(), "\"id\": 1");
        let toks: Vec<(&str, JsonTok)> = lines[1]
            .tokens
            .iter()
            .map(|(r, t)| (&lines[1].text[r.clone()], *t))
            .collect();
        assert_eq!(
            toks,
            [
                ("\"id\"", JsonTok::Key),
                (":", JsonTok::Punct),
                ("1", JsonTok::Number)
            ]
        );
        assert_eq!(lines[2].tokens, [(0..1, JsonTok::Punct)]);
        let plain = body_lines("a\n\n  b", &[]);
        assert_eq!(plain[1].text.as_ref(), "");
        assert_eq!(plain[2].indent.as_ref(), "  ");
    }

    #[test]
    fn formats_follow_the_board() {
        assert_eq!(format_bytes(412.), "412 B");
        assert_eq!(format_bytes(1536.), "1.5 KB");
        assert_eq!(format_bytes(3.5 * 1024. * 1024.), "3.5 MB");
        assert_eq!(env_label("staging"), "Staging");
        assert_eq!(env_label("Default"), "Default");
        assert_eq!(checks_summary(4, 4), "4 of 4 checks passed");
        assert_eq!(checks_summary(1, 1), "1 of 1 checks passed");
        assert_eq!(checks_summary(3, 4), "1 of 4 checks failed");
        assert_eq!(checks_summary(0, 0), "No checks");
        assert_eq!(
            missing_line(&["API_TOKEN".into()]).as_deref(),
            Some("1 variable not set: API_TOKEN")
        );
        assert_eq!(
            missing_line(&["A".into(), "B".into()]).as_deref(),
            Some("2 variables not set: A, B")
        );
        assert_eq!(missing_line(&[]), None);
    }

    #[test]
    fn attempts_tab_replaces_request_after_a_retry() {
        assert!(!visible_tabs(1).contains(&InspectorTab::Attempts));
        assert!(visible_tabs(1).contains(&InspectorTab::Request));
        assert_eq!(
            visible_tabs(3),
            [
                InspectorTab::Response,
                InspectorTab::Checks,
                InspectorTab::Attempts,
                InspectorTab::Headers,
                InspectorTab::Timing,
            ]
        );
    }

    #[test]
    fn result_view_reads_status_checks_and_sent_request() {
        let mut request = curl_runner_core::model::RequestConfig {
            url: "https://api.example.com/charges".into(),
            method: Some("POST".into()),
            headers: Some(
                [("Authorization".to_string(), "Bearer ***".to_string())]
                    .into_iter()
                    .collect(),
            ),
            ..Default::default()
        };
        request.expect = serde_json::from_value(serde_json::json!({ "status": 201 })).ok();
        let mut r = ExecutionResult::new(request, true);
        r.status = Some(201);
        r.body = Some(serde_json::json!({ "id": "ch_1" }));
        r.metrics = Some(curl_runner_core::model::Metrics {
            duration: 184.,
            size: Some(412.),
            ..Default::default()
        });
        let v = ResultView::new(&r);
        assert_eq!(v.status, "201 Created");
        assert_eq!(v.duration.as_deref(), Some("184 ms"));
        assert_eq!(v.size.as_deref(), Some("412 B"));
        assert_eq!((v.passed, v.checks.len()), (1, 1));
        assert_eq!(v.request.method, "POST");
        assert_eq!(v.request.headers[0].1, "Bearer ***");
        assert!(v.body.unwrap().text.contains("\"ch_1\""));
    }

    #[test]
    fn diff_rows_mark_the_indent_like_the_board() {
        assert_eq!(
            marked_line('+', "  \"amount\": 2500,"),
            "+ \"amount\": 2500,"
        );
        assert_eq!(marked_line('-', "    \"x\": 1"), "-   \"x\": 1");
        assert_eq!(marked_line('+', "{"), "+ {");
        assert_eq!(format_wait(1000), "1.0 s");
        assert_eq!(format_wait(500), "0.5 s");
    }

    #[test]
    fn summary_is_one_text_with_colored_parts() {
        use super::{SummaryPart, summary_text};
        let (text, runs) = summary_text("2 of 4 checks failed", 3);
        assert_eq!(text, "2 of 4 checks failed · 3 attempts");
        let parts: Vec<(&str, SummaryPart)> =
            runs.iter().map(|(r, p)| (&text[r.clone()], *p)).collect();
        assert_eq!(
            parts,
            [
                ("2 of 4 checks failed", SummaryPart::Checks),
                (" · ", SummaryPart::Dot),
                ("3 attempts", SummaryPart::Attempts),
            ]
        );
        assert_eq!(summary_text("No checks", 1).0, "No checks · 1 attempt");
    }

    #[test]
    fn rerun_targets_the_inspected_request_or_its_one_request_file() {
        let file = std::path::Path::new("charges/get.yaml");
        assert_eq!(
            rerun_target(file, 1, 3),
            RunTarget::Request {
                file: file.into(),
                index: 1
            }
        );
        assert_eq!(rerun_target(file, 0, 1), RunTarget::File(file.into()));
    }

    #[test]
    fn failed_body_check_gets_a_diff_and_retry_line() {
        let mut request = curl_runner_core::model::RequestConfig {
            url: "https://api.example.com/charges/ch_1".into(),
            ..Default::default()
        };
        request.expect = serde_json::from_value(
            serde_json::json!({ "status": 200, "body": { "amount": 2500 } }),
        )
        .ok();
        request.retry =
            serde_json::from_value(serde_json::json!({ "count": 3, "delay": 1000, "backoff": 2 }))
                .ok();
        let mut r = ExecutionResult::new(request, false);
        r.status = Some(200);
        r.body = Some(serde_json::json!({ "id": "ch_1", "amount": 2000 }));
        let v = ResultView::new(&r);
        assert!(v.http_ok && !v.ok);
        assert_eq!(
            v.retry.as_deref(),
            Some("count 3 · delay 1000 · backoff ×2")
        );
        let diff = v.diff.expect("a diff");
        let changed: Vec<(DiffKind, &str)> = diff
            .rows
            .iter()
            .filter(|r| r.kind != DiffKind::Same)
            .map(|r| (r.kind, r.marked.as_ref()))
            .collect();
        assert_eq!(
            changed,
            [
                (DiffKind::Added, "+ \"amount\": 2500"),
                (DiffKind::Removed, "- \"amount\": 2000")
            ]
        );
        r.body = Some(serde_json::json!({ "id": "ch_1", "amount": 2500 }));
        assert!(
            ResultView::new(&r).diff.is_none(),
            "no diff when the body passes"
        );
    }
}
