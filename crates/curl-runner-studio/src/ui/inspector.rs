//! The 400px inspector: the last response of the request under the cursor,
//! its checks, headers, timing and the request as sent.

use super::app::mono_font;
use super::color;
use super::icons::{self, glyph};
use super::runs::FileRun;
use super::workspace::Workspace;
use crate::model::collection::{request_at_line, request_lines};
use crate::model::result_view::{
    AttemptRow, Check, attempt_rows, checks, display_body, status_line, timing_segments,
};
use chrono::{DateTime, Local};
use curl_runner_core::model::{ExecutionResult, HeaderValue, Json};
use gpui_kit::component::spinner::Spinner;
use gpui_kit::component::{Sizable as _, Size};
use gpui_kit::prelude::*;
use gpui_kit::*;
use std::ops::Range;

pub const INSPECTOR_WIDTH: f32 = 400.;

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

/// The tabs a result shows: Attempts only after a retry.
pub fn visible_tabs(attempts: usize) -> Vec<InspectorTab> {
    use InspectorTab::*;
    let mut tabs = vec![Response, Checks];
    if attempts > 1 {
        tabs.push(Attempts);
    }
    tabs.extend([Headers, Timing, Request]);
    tabs
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

/// `184 ms`, or `1.24 s` from a second.
pub fn format_ms(ms: f64) -> String {
    if ms < 1000. {
        format!("{} ms", ms.round() as u64)
    } else {
        format!("{:.2} s", ms / 1000.)
    }
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

/// `just now`, `2 min ago`, `3 h ago`, else the date.
pub fn ago(at: DateTime<Local>, now: DateTime<Local>) -> String {
    let secs = (now - at).num_seconds().max(0);
    match secs {
        0..60 => "just now".to_string(),
        60..3600 => format!("{} min ago", secs / 60),
        3600..86400 => format!("{} h ago", secs / 3600),
        _ => at.format("%b %-d").to_string(),
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
        1 if passed == 1 => "1 of 1 checks passed".to_string(),
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

/// The response body as shown.
pub struct BodyView {
    pub text: SharedString,
    pub tokens: Vec<(Range<usize>, JsonTok)>,
    pub truncated: bool,
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
            BodyView {
                text: shown.text.into(),
                tokens,
                truncated: shown.truncated,
            }
        });
        let status = match (r.status, r.skipped) {
            (Some(_), _) => status_line(r),
            (None, Some(true)) => "Skipped".to_string(),
            (None, _) => "No response".to_string(),
        };
        let req = &r.request;
        ResultView {
            status,
            ok: r.success,
            duration: r.metrics.as_ref().map(|m| format_ms(m.duration)),
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
                .child(self.render_result_header(run, index, view))
                .child(self.render_result_tabs(run, index, view, cx))
                .child(self.render_result_body(run, index, view, cx)),
            None if self.runs.is_running_file(file) => aside.child(centered(
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
                .children(missing_line(&self.missing).map(|line| {
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
                }))
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
    /// attempts summary.
    fn render_result_header(&self, run: &FileRun, index: usize, v: &ResultView) -> Div {
        let (fg, bg, border) = if v.ok {
            ("success", "success-subtle", "success-border")
        } else {
            ("danger", "danger-subtle", "danger-border")
        };
        let dot = || div().text_color(color("text-disabled")).child("·");
        let mut facts = div().flex().items_center().gap(px(8.));
        let mut first = true;
        for fact in [v.duration.clone(), v.size.clone()].into_iter().flatten() {
            if !first {
                facts = facts.child(dot());
            }
            first = false;
            facts = facts.child(div().text_color(color("text-secondary")).child(fact));
        }
        let all_passed = v.passed == v.checks.len();
        let checks_color = match (v.checks.is_empty(), all_passed) {
            (true, _) => color("text-secondary"),
            (false, true) => color("success"),
            (false, false) => color("danger"),
        };
        let summary = checks_summary(v.passed, v.checks.len());
        let attempts = run.attempt_count(index).max(1);
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
                            .flex_none()
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
                            .text_size(px(12.))
                            .text_color(color("text-secondary"))
                            .child(format!(
                                "{} · {}",
                                env_label(&run.env),
                                ago(run.at, Local::now())
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
                        div()
                            .id("inspector-checks")
                            .role(Role::Label)
                            .test_support()
                            .aria_label(summary.clone())
                            .flex()
                            .items_center()
                            .gap(px(6.))
                            .text_color(checks_color)
                            .when(!v.checks.is_empty(), |d| {
                                d.child(
                                    glyph(if all_passed {
                                        icons::CHECK
                                    } else {
                                        icons::CROSS
                                    })
                                    .size(px(13.))
                                    .flex_none(),
                                )
                            })
                            .child(summary),
                    )
                    .child(dot())
                    .child(plural(attempts, "attempt", "attempts")),
            )
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
            .h(px(40.))
            .px(px(8.))
            .border_b_1()
            .border_color(color("border-100"))
            .children(tabs.into_iter().map(|tab| {
                let count = match tab {
                    InspectorTab::Headers => Some(v.headers.len()),
                    InspectorTab::Attempts => Some(attempts),
                    _ => None,
                };
                let selected = tab == current;
                div()
                    .id(("inspector-tab", tab as usize))
                    .flex()
                    .items_center()
                    .gap(px(6.))
                    .h(px(32.))
                    .px(px(12.))
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
                            .text_color(color("text-tertiary"))
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
                .when(!v.checks.is_empty(), |c| c.child(render_checks(v, false)))
                .when(!v.timing.is_empty(), |c| c.child(render_timing(v))),
            InspectorTab::Checks => content.child(if v.checks.is_empty() {
                note("This request has no checks.").into_any_element()
            } else {
                render_checks(v, true).into_any_element()
            }),
            InspectorTab::Attempts => {
                let rows = run.attempts.get(&index).map_or_else(Vec::new, |a| {
                    let waits = run.waits.get(&index).cloned().unwrap_or_default();
                    attempt_rows(a, &waits, v.checks.len() - v.passed, v.ok)
                });
                content.child(render_attempts(&rows))
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

fn section(title: &'static str) -> Div {
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
            let tok_color = |t: JsonTok| colors.iter().find(|(k, _)| *k == t).map(|(_, c)| *c);
            let styles = body.tokens.iter().map(move |(r, t)| {
                (
                    r.clone(),
                    HighlightStyle {
                        color: tok_color(*t),
                        ..Default::default()
                    },
                )
            });
            let default = if body.tokens.is_empty() {
                color("text-primary")
            } else {
                color("syntax-punctuation")
            };
            div()
                .text_color(default)
                .child(StyledText::new(body.text.clone()).with_highlights(styles))
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
                .whitespace_nowrap()
                .overflow_x_scroll()
                .child(text),
        )
}

/// One row per check; `detail` adds expected and actual to failed ones.
fn render_checks(v: &ResultView, detail: bool) -> impl IntoElement + use<> {
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
            let right = if c.passed || !detail {
                c.expected.clone()
            } else {
                match &c.actual {
                    Some(actual) => format!("expected {} · got {actual}", c.expected),
                    None => format!("expected {}", c.expected),
                }
            };
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
                .children(v.timing.iter().map(|(name, ms)| {
                    let mut seg = div().h_full().flex_basis(px(0.)).bg(segment_color(name));
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
                        .child(format!("{name} {}", format_ms(*ms)))
                })),
        )
}

fn render_attempts(rows: &[AttemptRow]) -> impl IntoElement + use<> {
    div()
        .flex()
        .flex_col()
        .gap(px(2.))
        .child(section("Attempts").pb(px(6.)))
        .children(rows.iter().map(|r| {
            let status = r.status.map_or_else(|| "—".to_string(), |s| s.to_string());
            let ok = r.status.is_some_and(|s| s < 400);
            div()
                .flex()
                .flex_col()
                .gap(px(2.))
                .py(px(6.))
                .child(
                    div()
                        .flex()
                        .items_center()
                        .gap(px(8.))
                        .child(
                            div()
                                .w(px(64.))
                                .text_color(color("text-secondary"))
                                .child(format!("Attempt {}", r.attempt)),
                        )
                        .child(
                            div()
                                .font_family(mono_font())
                                .text_size(px(12.))
                                .text_color(color(if ok { "success" } else { "danger" }))
                                .child(status),
                        )
                        .child(div().flex_1())
                        .child(
                            div()
                                .text_size(px(12.))
                                .text_color(color("text-secondary"))
                                .child(format_ms(r.duration_ms)),
                        ),
                )
                .child(
                    div()
                        .text_size(px(12.))
                        .text_color(color("text-secondary"))
                        .child(r.note.clone()),
                )
                .children(r.wait_ms.map(|w| {
                    div()
                        .text_size(px(12.))
                        .text_color(color("text-tertiary"))
                        .child(format!("Waited {}", format_ms(w as f64)))
                }))
        }))
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
        ExecutionResult, InspectorTab, JsonTok, Local, ResultView, ago, checks_summary, env_label,
        format_bytes, format_ms, json_tokens, missing_line, visible_tabs,
    };
    use chrono::TimeZone as _;

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
    fn formats_follow_the_board() {
        assert_eq!(format_ms(184.4), "184 ms");
        assert_eq!(format_ms(1240.), "1.24 s");
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
    fn ago_reads_like_the_board() {
        let now = Local.with_ymd_and_hms(2026, 10, 30, 12, 0, 0).unwrap();
        let before = |s| now - chrono::Duration::seconds(s);
        assert_eq!(ago(before(5), now), "just now");
        assert_eq!(ago(before(150), now), "2 min ago");
        assert_eq!(ago(before(7300), now), "2 h ago");
        assert_eq!(ago(before(3 * 86400), now), "Oct 27");
    }

    #[test]
    fn attempts_tab_only_after_a_retry() {
        assert!(!visible_tabs(1).contains(&InspectorTab::Attempts));
        assert_eq!(
            visible_tabs(3),
            [
                InspectorTab::Response,
                InspectorTab::Checks,
                InspectorTab::Attempts,
                InspectorTab::Headers,
                InspectorTab::Timing,
                InspectorTab::Request
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
}
