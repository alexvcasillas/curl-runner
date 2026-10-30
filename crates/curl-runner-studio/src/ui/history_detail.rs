//! The History screen's detail pane: the picked run's title, Export and
//! Re-run, its stats, the duration chart and the comparison with an
//! earlier run of the same target and environment.

use super::{
    HistoryView, HistoryViewEvent, Menu, change_text, checks_text, chevron, compare_label,
    entry_title, env_pill, failure_detail, menu_item, menu_panel, slower_text, small_button,
    unchanged_note, when_text,
};
use crate::model::history::{EntryKind, HistoryEntry, StoredResult, compare, duration_series};
use crate::ui::app::mono_font;
use crate::ui::color;
use crate::ui::icons::{self, glyph};
use crate::ui::inspector::env_label;
use crate::ui::run_view::{ExportFormat, elapsed_text};
use crate::ui::sidebar::method_badge;
use gpui_kit::prelude::*;
use gpui_kit::*;

/// The detail pane's width (the board's 460px aside).
const DETAIL_WIDTH: f32 = 460.;
/// Bars in the duration chart.
const SERIES: usize = 12;
/// Earlier runs the comparison picker offers.
const COMPARE_CHOICES: usize = 10;

impl HistoryView {
    pub(super) fn render_detail(&self, cx: &mut Context<Self>) -> impl IntoElement + use<> {
        let pane = div()
            .id("history-detail")
            .role(Role::Complementary)
            .test_support()
            .aria_label("Run detail")
            .flex()
            .flex_col()
            .flex_none()
            .w(px(DETAIL_WIDTH))
            .h_full()
            .border_l_1()
            .border_color(color("border-100"));
        let Some(e) = self.selected_entry() else {
            return pane.child(
                div()
                    .p(px(16.))
                    .text_size(px(13.))
                    .text_color(color("text-tertiary"))
                    .child(if self.entries.is_empty() {
                        ""
                    } else {
                        "Pick a run to see its detail."
                    }),
            );
        };
        pane.child(self.render_summary(e, cx))
            .child(self.render_comparison(e, cx))
    }

    fn render_summary(&self, e: &HistoryEntry, cx: &mut Context<Self>) -> impl IntoElement + use<> {
        let (title, _) = entry_title(e);
        let kind = match e.kind {
            EntryKind::CollectionRun => "Collection run",
            EntryKind::Request => "Request",
        };
        let version = if e.version.is_empty() {
            curl_runner_core::VERSION
        } else {
            e.version.as_str()
        };
        let failed = e.total.saturating_sub(e.passed);
        let stat = |label: &'static str, value: String, token: &str| {
            div()
                .id(label)
                .role(Role::Group)
                .test_support()
                .aria_label(format!("{label} {value}"))
                .flex()
                .flex_col()
                .flex_1()
                .flex_basis(px(0.))
                .min_w_0()
                .gap(px(2.))
                .child(
                    div()
                        .text_size(px(12.))
                        .font_weight(FontWeight::MEDIUM)
                        .text_color(color("text-secondary"))
                        .child(label),
                )
                .child(
                    div()
                        .text_size(px(18.))
                        .line_height(px(24.))
                        .font_weight(FontWeight::MEDIUM)
                        .text_color(color(token))
                        .child(value),
                )
        };
        let rerun = div()
            .id("history-rerun")
            .role(Role::Button)
            .test_support()
            .aria_label("Re-run")
            .flex()
            .flex_none()
            .items_center()
            .gap(px(8.))
            .h(px(28.))
            .px(px(10.))
            .rounded(px(6.))
            .bg(color("primary"))
            .text_color(color("on-primary"))
            .text_size(px(12.))
            .font_weight(FontWeight::MEDIUM)
            .cursor_pointer()
            .hover(|s| s.bg(color("primary-hover")))
            .child(glyph(icons::RERUN).size(px(13.)))
            .child("Re-run")
            .on_click(cx.listener(|this, _, _, cx| {
                if let Some(e) = this.selected_entry() {
                    cx.emit(HistoryViewEvent::Rerun(Box::new(e.clone())));
                }
            }));
        div()
            .flex()
            .flex_col()
            .flex_none()
            .gap(px(12.))
            .p(px(16.))
            .border_b_1()
            .border_color(color("border-100"))
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(8.))
                    .child(
                        div()
                            .min_w_0()
                            .text_size(px(15.))
                            .font_weight(FontWeight::MEDIUM)
                            .whitespace_nowrap()
                            .text_ellipsis()
                            .child(title),
                    )
                    .child(env_pill(&e.environment))
                    .child(div().flex_1())
                    .child(self.render_export(cx))
                    .child(rerun),
            )
            .child(
                div()
                    .text_size(px(12.))
                    .text_color(color("text-secondary"))
                    .whitespace_nowrap()
                    .text_ellipsis()
                    .child(format!(
                        "{kind} · {} · local · curl-runner {version}",
                        when_text(e.at, self.now)
                    )),
            )
            .child(
                div()
                    .flex()
                    .gap(px(12.))
                    .child(stat("Passed", e.passed.to_string(), "success"))
                    .child(stat(
                        "Failed",
                        failed.to_string(),
                        if failed > 0 { "danger" } else { "text-primary" },
                    ))
                    .child(stat(
                        "Duration",
                        elapsed_text(e.duration_ms),
                        "text-primary",
                    ))
                    .child(stat(
                        "Retries",
                        e.retries.to_string(),
                        if e.retries > 0 {
                            "warning"
                        } else {
                            "text-primary"
                        },
                    )),
            )
            .child(self.render_chart(e))
    }

    /// The last runs of the target on the environment, the picked one
    /// (last) in full white.
    fn render_chart(&self, e: &HistoryEntry) -> impl IntoElement + use<> {
        let series = duration_series(&self.entries, e, SERIES);
        let max = series.iter().copied().fold(0f64, f64::max).max(1.);
        let others: Vec<f64> = self
            .entries
            .iter()
            .filter(|o| o.target == e.target && o.environment == e.environment && o.id != e.id)
            .map(|o| o.duration_ms)
            .collect();
        let slowest = !others.is_empty() && others.iter().all(|d| e.duration_ms >= *d);
        let n = series.len();
        let last = n.saturating_sub(1);
        let caption = match n {
            1 => format!("Duration, last run on {}", env_label(&e.environment)),
            _ => format!("Duration, last {n} runs on {}", env_label(&e.environment)),
        };
        div()
            .flex()
            .flex_col()
            .gap(px(6.))
            .child(
                div()
                    .flex()
                    .text_size(px(12.))
                    .text_color(color("text-tertiary"))
                    .whitespace_nowrap()
                    .child(div().flex_1().child(caption))
                    .when(slowest, |d| d.child("slowest in 30 days")),
            )
            .child(
                div()
                    .id("history-chart")
                    .role(Role::Image)
                    .test_support()
                    .aria_label(format!("{n} runs"))
                    .flex()
                    .items_end()
                    .gap(px(4.))
                    .h(px(44.))
                    .border_b_1()
                    .border_color(color("border-100"))
                    // Twelve slots, so a short series keeps the board's
                    // bar width and starts at the left.
                    .children(series.iter().enumerate().map(|(i, ms)| {
                        div()
                            .flex_1()
                            .h(relative((ms / max).clamp(0.04, 1.) as f32))
                            .rounded_t(px(2.))
                            .bg(color(if i == last {
                                "text-primary"
                            } else {
                                "indent-guide-active"
                            }))
                    }))
                    .children((n..SERIES).map(|_| div().flex_1())),
            )
    }

    fn render_export(&self, cx: &mut Context<Self>) -> impl IntoElement + use<> {
        let open = self.menu == Some(Menu::Export);
        let button = small_button("history-export")
            .aria_label("Export")
            .when(open, |b| b.bg(color("surface-300")))
            .child(glyph(icons::DOWNLOAD).size(px(13.)))
            .child("Export")
            .on_click(cx.listener(|this, _, _, cx| this.toggle_menu(Menu::Export, cx)));
        let ext = |f: ExportFormat| {
            div()
                .font_family(mono_font())
                .text_size(px(11.))
                .text_color(color("text-tertiary"))
                .child(format!(".{}", f.extension()))
        };
        let copied = self.copied.is_some();
        let menu = menu_panel("history-export-menu", 232.)
            .on_mouse_down_out(cx.listener(|this, _, _, cx| this.close_menu_outside(cx)))
            .child(menu_item("history-export-junit", "JUnit XML", false).child(ext(ExportFormat::Junit)))
            .child(menu_item("history-export-json", "JSON", false).child(ext(ExportFormat::Json)))
            .child(
                menu_item("history-export-md", "Markdown summary", true)
                    .child(ext(ExportFormat::Markdown))
                    .on_click(cx.listener(|this, _, window, cx| this.export_markdown(window, cx))),
            )
            .child(div().h(px(1.)).my(px(4.)).bg(color("border-100")))
            .child(
                menu_item(
                    "history-export-cli",
                    if copied { "Copied" } else { "Copy CLI command" },
                    true,
                )
                .text_color(color("text-secondary"))
                .on_click(cx.listener(|this, _, _, cx| this.copy_cli(cx))),
            )
            .child(
                div()
                    .px(px(10.))
                    .pt(px(6.))
                    .pb(px(4.))
                    .text_size(px(11.))
                    .line_height(relative(1.4))
                    .text_color(color("text-tertiary"))
                    .child("JUnit and JSON are only for runs in this session: history keeps results, not full responses."),
            );
        div()
            .relative()
            .flex()
            .flex_none()
            .child(button)
            .when(open, |d| d.child(deferred(menu).with_priority(1)))
    }

    fn render_comparison(
        &self,
        e: &HistoryEntry,
        cx: &mut Context<Self>,
    ) -> impl IntoElement + use<> {
        let body = div()
            .id("history-comparison")
            .flex()
            .flex_col()
            .flex_1()
            .min_h_0()
            .overflow_y_scroll()
            .gap(px(12.))
            .p(px(16.));
        let heading = div().flex().items_center().gap(px(8.)).h(px(28.)).child(
            div()
                .flex_1()
                .text_size(px(12.))
                .font_weight(FontWeight::MEDIUM)
                .text_color(color("text-secondary"))
                .child("Compared with previous run"),
        );
        let Some(prev) = self.compared(e) else {
            return body.child(heading).child(
                div()
                    .text_size(px(12.))
                    .text_color(color("text-tertiary"))
                    .child(format!(
                        "No earlier run of this on {} to compare with.",
                        env_label(&e.environment)
                    )),
            );
        };
        let c = compare(e, prev);
        let heading = heading.child(self.render_compare_picker(e, prev, cx));
        let card = |id: &'static str, title: &'static str, token: &str, n: usize| {
            div()
                .id(id)
                .flex()
                .flex_col()
                .flex_none()
                .border_1()
                .border_color(color("border-100"))
                .rounded(px(8.))
                .overflow_hidden()
                .child(
                    div()
                        .id(SharedString::from(format!("{id}-head")))
                        .role(Role::Label)
                        .test_support()
                        .aria_label(format!("{title} {n}"))
                        .flex()
                        .items_center()
                        .gap(px(8.))
                        .h(px(34.))
                        .px(px(12.))
                        .bg(color("surface-raised"))
                        .child(div().size(px(6.)).rounded_full().bg(color(token)))
                        .child(div().font_weight(FontWeight::MEDIUM).child(title))
                        .child(
                            div()
                                .text_color(color("text-tertiary"))
                                .child(n.to_string()),
                        ),
                )
        };
        let row = |icon: AnyElement, r: &StoredResult, detail: String, right: String| {
            div()
                .flex()
                .items_center()
                .gap(px(10.))
                .h(px(32.))
                .px(px(12.))
                .border_t_1()
                .border_color(color("surface-200"))
                .whitespace_nowrap()
                .child(icon)
                .child(method_badge(Some(&r.method)).w(px(44.)))
                .child(
                    div()
                        .flex()
                        .items_baseline()
                        .gap(px(8.))
                        .flex_1()
                        .min_w_0()
                        .overflow_hidden()
                        .child(div().flex_none().child(r.name.clone()))
                        .child(
                            div()
                                .min_w_0()
                                .text_ellipsis()
                                .text_size(px(12.))
                                .text_color(color("text-tertiary"))
                                .child(detail),
                        ),
                )
                .child(
                    div()
                        .flex_none()
                        .font_family(mono_font())
                        .text_size(px(12.))
                        .text_color(color("text-secondary"))
                        .child(right),
                )
        };
        let icon = |svg: &'static str, token: &str| {
            glyph(svg)
                .size(px(13.))
                .flex_none()
                .text_color(color(token))
                .into_any_element()
        };
        let mut body = body.child(heading);
        if !c.newly_failing.is_empty() {
            let mut k = card(
                "history-newly-failing",
                "Newly failing",
                "danger",
                c.newly_failing.len(),
            );
            for r in &c.newly_failing {
                k = k.child(row(
                    icon(icons::CROSS, "danger"),
                    r,
                    failure_detail(r),
                    checks_text(r),
                ));
            }
            body = body.child(k);
        }
        if !c.fixed.is_empty() {
            let mut k = card("history-fixed", "Fixed", "success", c.fixed.len());
            for r in &c.fixed {
                let detail = match r.attempts.saturating_sub(1) {
                    0 => "passed".to_string(),
                    1 => "passed after 1 retry".to_string(),
                    n => format!("passed after {n} retries"),
                };
                k = k.child(row(
                    icon(icons::CHECK, "success"),
                    r,
                    detail,
                    checks_text(r),
                ));
            }
            body = body.child(k);
        }
        if !c.slower.is_empty() {
            let mut k = card("history-slower", "Slower", "warning", c.slower.len());
            for (r, prev_ms) in &c.slower {
                k = k.child(row(
                    icon(icons::TREND, "warning"),
                    r,
                    slower_text(*prev_ms, r.duration_ms),
                    change_text(*prev_ms, r.duration_ms),
                ));
            }
            body = body.child(k);
        }
        let nothing = c.newly_failing.is_empty() && c.fixed.is_empty() && c.slower.is_empty();
        body.when(nothing, |b| {
            b.child(
                div()
                    .text_size(px(13.))
                    .text_color(color("text-secondary"))
                    .child("Nothing failed, got fixed or slowed down since that run."),
            )
        })
        .child(
            div()
                .text_size(px(12.))
                .line_height(relative(1.4))
                .text_color(color("text-tertiary"))
                .child(unchanged_note(c.unchanged)),
        )
    }

    fn render_compare_picker(
        &self,
        e: &HistoryEntry,
        prev: &HistoryEntry,
        cx: &mut Context<Self>,
    ) -> impl IntoElement + use<> {
        let open = self.menu == Some(Menu::Compare);
        let button = small_button("history-compare")
            .aria_label(compare_label(prev.at, self.now))
            .when(open, |b| b.bg(color("surface-300")))
            .child(compare_label(prev.at, self.now))
            .child(chevron())
            .on_click(cx.listener(|this, _, _, cx| this.toggle_menu(Menu::Compare, cx)));
        let mut menu = menu_panel("history-compare-menu", 240.)
            .on_mouse_down_out(cx.listener(|this, _, _, cx| this.close_menu_outside(cx)));
        for (i, o) in self.earlier(e).take(COMPARE_CHOICES).enumerate() {
            let id = o.id.clone();
            let failed = o.passed < o.total;
            menu = menu.child(
                menu_item(
                    ("history-compare-option", i),
                    compare_label(o.at, self.now),
                    true,
                )
                .when(o.id == prev.id, |d| d.bg(color("surface-200")))
                .child(
                    div()
                        .font_family(mono_font())
                        .text_size(px(11.))
                        .text_color(color(if failed { "danger" } else { "text-tertiary" }))
                        .child(format!("{}/{}", o.passed, o.total)),
                )
                .on_click(cx.listener(move |this, _, _, cx| {
                    this.compare_with = Some(id.clone());
                    this.menu = None;
                    cx.notify();
                })),
            );
        }
        div()
            .relative()
            .flex()
            .flex_none()
            .child(button)
            .when(open, |d| d.child(deferred(menu).with_priority(1)))
    }
}
