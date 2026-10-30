//! The inspector's failed-request parts (board: FailedRequest): the
//! expanded check list, the body diff with its Diff | Raw switch and the
//! retry timeline.

use super::app::mono_font;
use super::color;
use super::icons::{self, glyph};
use super::inspector::{
    BODY_MAX_HEIGHT, DiffView, JsonTok, ResultView, format_ms, format_wait, section,
};
use super::runs::FileRun;
use super::workspace::Workspace;
use crate::model::result_view::{AttemptRow, DiffKind, attempt_rows};
use gpui_kit::prelude::*;
use gpui_kit::*;
use std::ops::Range;

/// `run`'s attempts at request `index`, as timeline rows.
pub(super) fn timeline_rows(run: &FileRun, index: usize, v: &ResultView) -> Vec<AttemptRow> {
    run.attempts.get(&index).map_or_else(Vec::new, |a| {
        let waits = run.waits.get(&index).cloned().unwrap_or_default();
        attempt_rows(a, &waits, v.failed(), v.ok)
    })
}

/// The Checks tab: a compact row per passed check, a card with expected
/// and actual per failed one.
pub(super) fn render_check_list(v: &ResultView) -> impl IntoElement + use<> {
    let mono_row = || {
        div()
            .flex()
            .items_center()
            .gap(px(8.))
            .h(px(30.))
            .px(px(10.))
    };
    let label = |text: &str| {
        div()
            .flex_1()
            .min_w_0()
            .text_ellipsis()
            .font_family(mono_font())
            .text_size(px(12.))
            .child(text.to_string())
    };
    div()
        .flex()
        .flex_col()
        .flex_none()
        .gap(px(6.))
        .children(v.checks.iter().map(move |c| {
            if c.passed {
                return mono_row()
                    .child(
                        glyph(icons::CHECK)
                            .size(px(13.))
                            .flex_none()
                            .text_color(color("success")),
                    )
                    .child(label(&c.label))
                    .child(
                        div()
                            .flex_none()
                            .max_w(px(200.))
                            .text_ellipsis()
                            .font_family(mono_font())
                            .text_size(px(12.))
                            .text_color(color("text-secondary"))
                            .child(c.expected.clone()),
                    )
                    .into_any_element();
            }
            let line = |name: &'static str, value: String, tone: &str| {
                div()
                    .flex()
                    .px(px(10.))
                    .bg(color(tone).opacity(0.1))
                    .text_color(color(tone))
                    .child(
                        div()
                            .flex_none()
                            .w(px(70.))
                            .text_color(color("text-tertiary"))
                            .child(name),
                    )
                    .child(div().flex_1().min_w_0().child(value))
            };
            let verdict = if c.actual.is_some() {
                "mismatch"
            } else {
                "missing"
            };
            div()
                .id(SharedString::from(format!("check-{}", c.label)))
                .role(Role::Group)
                .test_support()
                .aria_label(format!("{} {verdict}", c.label))
                .flex_none()
                .border_1()
                .border_color(color("danger-border"))
                .rounded(px(6.))
                .overflow_hidden()
                .child(
                    mono_row()
                        .bg(color("danger-subtle"))
                        .child(
                            glyph(icons::CROSS)
                                .size(px(13.))
                                .flex_none()
                                .text_color(color("danger")),
                        )
                        .child(label(&c.label))
                        .child(
                            div()
                                .flex_none()
                                .text_size(px(12.))
                                .text_color(color("danger"))
                                .child(verdict),
                        ),
                )
                .child(
                    div()
                        .font_family(mono_font())
                        .text_size(px(12.))
                        .line_height(px(22.))
                        .child(line("expected", c.expected.clone(), "success"))
                        .child(line(
                            "actual",
                            c.actual.clone().unwrap_or_else(|| "nothing".into()),
                            "danger",
                        )),
                )
                .into_any_element()
        }))
}

/// JSON text with the YAML theme's token colors.
pub(super) fn json_text(text: &SharedString, tokens: &[(Range<usize>, JsonTok)]) -> StyledText {
    let styles = tokens
        .iter()
        .map(|(r, t)| {
            let c = color(match t {
                JsonTok::Key => "syntax-key",
                JsonTok::String => "syntax-string",
                JsonTok::Number => "syntax-number",
                JsonTok::Punct => "syntax-punctuation",
            });
            let style = HighlightStyle {
                color: Some(c),
                ..Default::default()
            };
            (r.clone(), style)
        })
        .collect::<Vec<_>>();
    StyledText::new(text.clone()).with_highlights(styles)
}

/// `Response body · diff against expected`, with the Diff | Raw switch.
pub(super) fn render_diff_card(
    d: &DiffView,
    raw: bool,
    cx: &mut Context<Workspace>,
) -> impl IntoElement + use<> {
    let segment = |id: &'static str, label: &'static str, on: bool| {
        div()
            .id(id)
            .px(px(6.))
            .rounded(px(3.))
            .cursor_pointer()
            .when(on, |s| {
                s.bg(color("surface-300")).text_color(color("text-primary"))
            })
            .when(!on, |s| s.hover(|s| s.text_color(color("text-primary"))))
            .child(label)
    };
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
        .child(div().flex_1().min_w_0().text_ellipsis().child(if raw {
            "Response body"
        } else {
            "Response body · diff against expected"
        }))
        .child(
            div()
                .flex()
                .flex_none()
                .gap(px(2.))
                .p(px(1.))
                .rounded(px(5.))
                .border_1()
                .border_color(color("border-200"))
                .child(
                    segment("diff-mode-diff", "Diff", !raw).on_click(cx.listener(
                        |this, _, _, cx| {
                            this.body_raw = false;
                            cx.notify();
                        },
                    )),
                )
                .child(segment("diff-mode-raw", "Raw", raw).on_click(cx.listener(
                    |this, _, _, cx| {
                        this.body_raw = true;
                        cx.notify();
                    },
                ))),
        );
    let rows = d
        .rows
        .iter()
        .filter(|r| !raw || r.kind != DiffKind::Added)
        .map(|r| {
            let row = div().flex().min_h(px(20.)).px(px(12.));
            let (indent, rest) = if raw || r.kind == DiffKind::Same {
                let at = r.text.len() - r.text.trim_start_matches(' ').len();
                (r.text[..at].to_string(), None)
            } else {
                let at = r.marked.len() - r.marked[1..].trim_start_matches(' ').len();
                (r.marked[..at].to_string(), Some(r.marked[at..].to_string()))
            };
            let row = row.when(!indent.is_empty(), |row| {
                row.child(div().flex_none().whitespace_nowrap().child(indent.clone()))
            });
            match rest {
                Some(rest) => {
                    let tone = if r.kind == DiffKind::Added {
                        "success"
                    } else {
                        "danger"
                    };
                    row.bg(color(tone).opacity(0.1))
                        .text_color(color(tone))
                        .child(div().flex_1().min_w_0().child(rest))
                }
                None => {
                    let at = indent.len();
                    let text: SharedString = r.text[at..].to_string().into();
                    let tokens: Vec<(Range<usize>, JsonTok)> = r
                        .tokens
                        .iter()
                        .filter(|(t, _)| t.start >= at)
                        .map(|(t, k)| (t.start - at..t.end - at, *k))
                        .collect();
                    row.child(div().flex_1().min_w_0().child(json_text(&text, &tokens)))
                }
            }
        });
    div()
        .id("inspector-diff")
        .flex_none()
        .border_1()
        .border_color(color("border-100"))
        .rounded(px(8.))
        .overflow_hidden()
        .child(header)
        .child(
            div()
                .id("inspector-diff-body")
                .py(px(6.))
                .font_family(mono_font())
                .text_size(px(12.))
                .line_height(px(20.))
                .text_color(color("syntax-punctuation"))
                .max_h(px(BODY_MAX_HEIGHT))
                .overflow_y_scroll()
                .children(rows)
                .when(d.truncated, |b| {
                    b.child(
                        div()
                            .px(px(12.))
                            .pt(px(8.))
                            .font_family(super::app::ui_font())
                            .text_color(color("text-tertiary"))
                            .child("Showing first 256 KB"),
                    )
                }),
        )
}

/// The retry timeline: a ring per attempt on a rail, the wait between.
pub(super) fn render_timeline(
    rows: &[AttemptRow],
    retry: Option<&str>,
    ok: bool,
) -> impl IntoElement + use<> {
    const RING: f32 = 11.;
    let last = rows.len().saturating_sub(1);
    let rail = || div().w(px(1.)).bg(color("border-200"));
    let mut list = div().flex().flex_col().gap(px(8.));
    for (i, r) in rows.iter().enumerate() {
        let status_ok = r.status.is_some_and(|s| s < 400);
        // The last ring tells the result; earlier ones, their attempt.
        let ring_ok = if i == last { ok } else { status_ok };
        let title = format!("Attempt {}", r.attempt);
        list = list.child(
            div()
                .relative()
                .flex()
                .gap(px(12.))
                .pb(px(6.))
                .when(i < last, |d| {
                    d.child(rail().absolute().left(px(5.)).top(px(18.)).bottom(px(-6.)))
                })
                .child(
                    div()
                        .flex_none()
                        .size(px(RING))
                        .mt(px(4.))
                        .rounded_full()
                        .border_2()
                        .border_color(color(if ring_ok { "success" } else { "danger" }))
                        .bg(color("bg-100")),
                )
                .child(
                    div()
                        .flex()
                        .flex_col()
                        .flex_1()
                        .min_w_0()
                        .gap(px(2.))
                        .child(
                            div()
                                .flex()
                                .items_center()
                                .gap(px(8.))
                                .child(
                                    div()
                                        .id(SharedString::from(format!("attempt-{}", r.attempt)))
                                        .role(Role::Label)
                                        .test_support()
                                        .aria_label(title.clone())
                                        .font_weight(FontWeight::MEDIUM)
                                        .child(title),
                                )
                                .child(
                                    div()
                                        .font_family(mono_font())
                                        .text_size(px(12.))
                                        .text_color(color(if status_ok {
                                            "success"
                                        } else {
                                            "danger"
                                        }))
                                        .child(
                                            r.status
                                                .map_or_else(|| "—".to_string(), |s| s.to_string()),
                                        ),
                                )
                                .child(
                                    div()
                                        .text_size(px(12.))
                                        .text_color(color("text-secondary"))
                                        .child(format_ms(r.duration_ms)),
                                ),
                        )
                        .child(
                            div()
                                .id(SharedString::from(format!("attempt-note-{}", r.attempt)))
                                .role(Role::Label)
                                .test_support()
                                .aria_label(r.note.clone())
                                .text_size(px(12.))
                                .text_color(color("text-tertiary"))
                                .child(r.note.clone()),
                        ),
                ),
        );
        if let Some(wait) = r.wait_ms {
            list = list.child(
                div()
                    .flex()
                    .gap(px(12.))
                    .pb(px(6.))
                    .text_size(px(11.))
                    .text_color(color("text-tertiary"))
                    .child(
                        div()
                            .flex()
                            .flex_none()
                            .justify_center()
                            .w(px(RING))
                            .child(rail().h(px(14.))),
                    )
                    .child(
                        div()
                            .font_family(mono_font())
                            .child(format!("wait {}", format_wait(wait))),
                    ),
            );
        }
    }
    div()
        .flex()
        .flex_col()
        .flex_none()
        .gap(px(8.))
        .child(
            div()
                .flex()
                .items_center()
                .child(section("Retry timeline").flex_1())
                .children(retry.map(|line| {
                    div()
                        .font_family(mono_font())
                        .text_size(px(11.))
                        .text_color(color("text-tertiary"))
                        .child(line.to_string())
                })),
        )
        .child(list)
}
