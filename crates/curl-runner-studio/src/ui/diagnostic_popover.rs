//! The diagnostic popover shown while the pointer rests on a squiggle,
//! replacing the Kit's (see `diagnostic_marks`): severity icon, message
//! with code chips, source line, allowed values and the quick fix.

use super::app::mono_font;
use super::color;
use super::diagnostic_marks::severity_glyph;
use super::diagnostics_view::{byte_to_position, severity_color};
use super::icons::glyph;
use super::workspace::Workspace;
use crate::model::diagnostics::{
    MessageSpan, StudioDiagnostic, allowed_values, message_spans, problem_order,
};
use curl_runner_core::lint::Severity;
use gpui_kit::prelude::*;
use gpui_kit::*;
use std::time::Duration;

/// How long the popover outlives the pointer leaving its squiggle, so the
/// pointer can cross the gap onto the popover.
const HOVER_GRACE: Duration = Duration::from_millis(250);
const POPOVER_WIDTH: f32 = 340.;
/// Left inset of the lines under the message: past the icon.
const BODY_INSET: f32 = 33.;

/// The diagnostic under the pointer.
pub(super) struct DiagnosticHover {
    pub diagnostic: StudioDiagnostic,
    /// The squiggle's window bounds when hovered.
    pub anchor: Bounds<Pixels>,
    /// Its zero-based line.
    pub line: u32,
}

/// A severity icon from the mockup glyphs, in the severity color.
pub fn severity_icon(severity: Severity, size: f32) -> impl IntoElement {
    glyph(severity_glyph(severity))
        .size(px(size))
        .flex_none()
        .text_color(severity_color(severity))
}

/// A message with its `"quoted"` tokens as mono code chips, laid out word
/// by word so it can wrap (`wrap`) or clip on one line.
pub fn message_line(message: &str, code_size: f32, wrap: bool) -> Div {
    let mut line = div().flex().items_center().min_w_0();
    line = if wrap {
        line.flex_wrap()
    } else {
        line.overflow_hidden()
    };
    for span in message_spans(message) {
        match span {
            MessageSpan::Text(text) => {
                for word in text.split_inclusive(' ') {
                    line = line.child(
                        div()
                            .flex_none()
                            .whitespace_nowrap()
                            .child(word.to_string()),
                    );
                }
            }
            MessageSpan::Code(code) => {
                line = line.child(code_chip(code, code_size));
            }
        }
    }
    line
}

fn code_chip(code: &str, size: f32) -> Div {
    div()
        .flex_none()
        .px(px(3.))
        .rounded(px(3.))
        .bg(color("surface-300"))
        .font_family(mono_font())
        .text_size(px(size))
        .child(code.to_string())
}

/// A keyboard shortcut hint, as in the mockup's `kbd`.
pub fn kbd(keys: &'static str, size: f32, fg: Hsla) -> Div {
    div()
        .flex_none()
        .font_family(mono_font())
        .text_size(px(size))
        .text_color(fg)
        .child(keys)
}

impl Workspace {
    /// Pointer moved over the editor at `position` (window coordinates):
    /// shows the popover for the squiggle under it, errors first, or lets
    /// the open one close.
    pub(super) fn hover_diagnostics_at(&mut self, position: Point<Pixels>, cx: &mut Context<Self>) {
        let Some(ix) = self.state.active else { return };
        let found = {
            let marks = self.tabs[ix].diagnostics.marks.borrow();
            let state = self.tabs[ix].editor.read(cx);
            problem_order(&marks.shown).into_iter().find_map(|i| {
                let d = &marks.shown[i];
                let anchor = state.range_to_bounds(&d.range)?;
                let hit = Bounds::new(
                    anchor.origin,
                    size(anchor.size.width.max(px(6.)), anchor.size.height),
                );
                hit.contains(&position).then(|| DiagnosticHover {
                    diagnostic: d.clone(),
                    anchor,
                    line: byte_to_position(&self.state.open[ix].text, d.range.start).0,
                })
            })
        };
        match found {
            Some(hover) => {
                self.hover_close = None;
                let same = self
                    .hover
                    .as_ref()
                    .is_some_and(|h| h.diagnostic == hover.diagnostic);
                if !same {
                    self.hover = Some(hover);
                    cx.notify();
                }
            }
            None => self.release_hover(cx),
        }
    }

    /// The pointer left the squiggle: closes the popover after a grace
    /// period unless the pointer is on it by then.
    pub(super) fn release_hover(&mut self, cx: &mut Context<Self>) {
        if self.hover.is_none() || self.hover_close.is_some() || self.hover_pinned {
            return;
        }
        self.hover_close = Some(cx.spawn(async move |this, cx| {
            cx.background_executor().timer(HOVER_GRACE).await;
            this.update(cx, |this, cx| {
                this.hover_close = None;
                if !this.on_popover {
                    this.close_hover(cx);
                }
            })
            .ok();
        }));
    }

    /// Development aid (`CURL_RUNNER_STUDIO_SHOW=problem`): selects the
    /// active file's first problem, puts the cursor on it and, once the
    /// editor is laid out, hovers its squiggle.
    pub(super) fn dev_show_first_problem(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(ix) = self.state.active else { return };
        self.flush_diagnostics(ix, cx);
        let Some(&first) = problem_order(&self.state.open[ix].diagnostics).first() else {
            return;
        };
        self.selected_problem = Some(first);
        self.reveal_problem(first, window, cx);
        cx.spawn(async move |this, cx| {
            cx.background_executor()
                .timer(Duration::from_millis(500))
                .await;
            this.update(cx, |this, cx| {
                let range = this.state.open[ix].diagnostics[first].range.clone();
                let at = this.tabs[ix].editor.read(cx).range_to_bounds(&range);
                if let Some(at) = at {
                    this.hover_diagnostics_at(at.center(), cx);
                    // The real pointer is elsewhere: keep it open.
                    this.hover_pinned = true;
                }
            })
            .ok();
        })
        .detach();
    }

    pub(super) fn close_hover(&mut self, cx: &mut Context<Self>) {
        self.hover_close = None;
        self.on_popover = false;
        if self.hover.take().is_some() {
            cx.notify();
        }
    }

    /// The open popover's line, if it still belongs to the active file's
    /// current diagnostics.
    pub(super) fn hovered_line(&self) -> Option<u32> {
        let hover = self.hover.as_ref()?;
        let doc = self.state.active_document()?;
        doc.diagnostics
            .contains(&hover.diagnostic)
            .then_some(hover.line)
    }

    pub(super) fn render_diagnostic_popover(
        &self,
        cx: &mut Context<Self>,
    ) -> Option<impl IntoElement + use<>> {
        self.hovered_line()?;
        let hover = self.hover.as_ref()?;
        let d = &hover.diagnostic;
        // Hidden once the text scrolled under it (the editor takes the
        // wheel, so the pane never sees it); the next pointer move reopens.
        let editor = &self.tabs[self.state.active?].editor;
        if editor.read(cx).range_to_bounds(&d.range) != Some(hover.anchor) {
            return None;
        }
        let (head, values) = allowed_values(&d.message);
        let source = if d.path.is_empty() {
            "curl-runner".to_string()
        } else {
            format!("curl-runner · {}", d.path)
        };
        let fix = d.fix.clone().map(|fix| {
            let path = d.path.clone();
            let label = fix.clone();
            div()
                .flex()
                .gap(px(4.))
                .p(px(6.))
                .border_t_1()
                .border_color(color("border-200"))
                .child(
                    div()
                        .id("popover-fix")
                        .flex()
                        .items_center()
                        .gap(px(8.))
                        .h(px(26.))
                        .px(px(8.))
                        .rounded(px(5.))
                        .bg(color("surface-300"))
                        .text_color(color("text-primary"))
                        .cursor_pointer()
                        .hover(|s| s.bg(color("border-200")))
                        .child(label)
                        .child(kbd("⌘.", 10.5, color("text-tertiary")))
                        .on_click(cx.listener(move |this, _, window, cx| {
                            if let Some(ix) = this.state.active {
                                this.apply_named_fix(ix, &path, &fix, window, cx);
                            }
                            this.close_hover(cx);
                        })),
                )
        });
        let card = div()
            .id("diagnostic-popover")
            .occlude()
            .w(px(POPOVER_WIDTH))
            .flex()
            .flex_col()
            .bg(color("surface-raised"))
            .border_1()
            .border_color(color("border-200"))
            .rounded(px(8.))
            .shadow(vec![BoxShadow {
                color: hsla(0., 0., 0., 0.6),
                offset: point(px(0.), px(8.)),
                blur_radius: px(24.),
                spread_radius: px(0.),
                inset: false,
            }])
            .text_size(px(12.))
            .line_height(relative(1.45))
            .text_color(color("text-primary"))
            .child(
                div()
                    .flex()
                    .items_start()
                    .gap(px(8.))
                    .pt(px(10.))
                    .px(px(12.))
                    .pb(px(8.))
                    .child(div().mt(px(2.)).child(severity_icon(d.severity, 13.)))
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .flex_1()
                            .min_w_0()
                            .child(message_line(head, 11.5, true))
                            .child(
                                div()
                                    .mt(px(4.))
                                    .text_size(px(11.))
                                    .text_color(color("text-tertiary"))
                                    .child(source),
                            ),
                    ),
            )
            .when_some(values, |card, values| {
                card.child(
                    div()
                        .pl(px(BODY_INSET))
                        .pr(px(12.))
                        .pb(px(10.))
                        .text_color(color("text-secondary"))
                        .child("Allowed values"),
                )
                .child(
                    div()
                        .flex()
                        .flex_wrap()
                        .gap(px(4.))
                        .pl(px(BODY_INSET))
                        .pr(px(12.))
                        .pb(px(10.))
                        .children(values.into_iter().map(|v| {
                            div()
                                .px(px(6.))
                                .py(px(1.))
                                .border_1()
                                .border_color(color("border-200"))
                                .rounded(px(4.))
                                .font_family(mono_font())
                                .text_size(px(11.))
                                .child(v.to_string())
                        })),
                )
            })
            .children(fix)
            .on_hover(cx.listener(|this, hovered: &bool, _, cx| {
                this.on_popover = *hovered;
                if !*hovered {
                    this.release_hover(cx);
                }
            }));
        // Under the squiggle, its icon left of the squiggle's start as in
        // the mockup (`top: 20px; left: -4px` from the token).
        let at = point(
            hover.anchor.origin.x - px(4.),
            hover.anchor.bottom() + px(2.),
        );
        Some(deferred(
            anchored()
                .position(at)
                .snap_to_window_with_margin(px(8.))
                .child(card),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::super::workspace::Workspace;
    use gpui_kit::test::TestWindowExt as _;
    use gpui_kit::{
        AnyWindowHandle, AppContext as _, Bounds, Entity, Point, TestAppContext, WindowBounds,
        WindowOptions, px, size,
    };
    use std::path::Path;

    const BAD: &str = "request:\n  url: https://api.example.com\n  method: FETCH\n";

    fn open(cx: &mut TestAppContext) -> (tempfile::TempDir, AnyWindowHandle, Entity<Workspace>) {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("bad.yaml"), BAD).unwrap();
        let root = dir.path().to_path_buf();
        let (window, ws) = cx.update(|cx| {
            gpui_kit::init(cx);
            super::super::app::init(cx);
            let options = WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(Bounds {
                    origin: Point::default(),
                    size: size(px(1440.), px(900.)),
                })),
                ..Default::default()
            };
            gpui_kit::open_window(options, cx, |window, cx| {
                cx.new(|cx| {
                    Workspace::new(
                        Some(root),
                        super::super::state::StateStore::none(),
                        super::super::workspace::test_secrets(),
                        super::super::workspace::test_backend(),
                        window,
                        cx,
                    )
                })
            })
            .unwrap()
        });
        (dir, window, ws)
    }

    #[gpui_kit::test]
    fn hovering_a_squiggle_opens_the_popover_until_the_pointer_leaves(cx: &mut TestAppContext) {
        let (_dir, window, ws) = open(cx);
        cx.update_window(window, |_, window, cx| {
            ws.update(cx, |ws, cx| ws.open_file(Path::new("bad.yaml"), window, cx));
            window.render_frame(cx);
            // The squiggle's centre, from the editor's own layout.
            let at = ws.update(cx, |ws, cx| {
                let doc = &ws.state.open[0];
                let range = doc.diagnostics[0].range.clone();
                assert_eq!(&doc.text[range.clone()], "FETCH");
                let bounds = ws.tabs[0].editor.read(cx).range_to_bounds(&range);
                bounds.expect("laid out").center()
            });
            ws.update(cx, |ws, cx| ws.hover_diagnostics_at(at, cx));
            assert_eq!(ws.read(cx).hovered_line(), Some(2));
            window.render_frame(cx);

            // Away from it: the popover waits out its grace period.
            ws.update(cx, |ws, cx| ws.hover_diagnostics_at(Point::default(), cx));
            assert_eq!(ws.read(cx).hovered_line(), Some(2));
        })
        .unwrap();
        cx.executor().advance_clock(super::HOVER_GRACE * 2);
        cx.run_until_parked();
        cx.update(|cx| assert_eq!(ws.read(cx).hovered_line(), None));
    }
}
