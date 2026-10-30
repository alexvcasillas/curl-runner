//! The panel under the editor: the active file's Problems, and the
//! Output tab (empty until runs arrive).

use super::app::mono_font;
use super::color;
use super::diagnostic_popover::{message_line, severity_icon};
use super::diagnostics_view::byte_to_position;
use super::workspace::Workspace;
use crate::model::diagnostics::problem_order;
use crate::model::document::Document;
use gpui_kit::prelude::*;
use gpui_kit::*;

pub const PROBLEMS_HEIGHT: f32 = 178.;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PanelTab {
    Problems,
    Output,
}

/// `14:12`, one-based like the editor shows it.
fn line_col(text: &str, offset: usize) -> String {
    let (line, character) = byte_to_position(text, offset);
    format!("{}:{}", line + 1, character + 1)
}

impl Workspace {
    fn render_panel_tab(
        &self,
        tab: PanelTab,
        label: &'static str,
        count: Option<usize>,
        cx: &mut Context<Self>,
    ) -> impl IntoElement + use<> {
        let selected = self.panel_tab == tab;
        div()
            .id(label)
            .flex()
            .items_center()
            .gap(px(6.))
            .h(px(33.))
            .px(px(12.))
            .border_b_1()
            .cursor_pointer()
            .map(|t| {
                if selected {
                    t.text_color(color("text-primary"))
                        .border_color(color("text-primary"))
                } else {
                    t.text_color(color("text-secondary"))
                        .border_color(gpui_kit::transparent_black())
                        .hover(|s| s.text_color(color("text-primary")))
                }
            })
            .child(label)
            .children(count.map(|n| {
                div()
                    .px(px(6.))
                    .rounded_full()
                    .bg(color("surface-300"))
                    .text_color(color("text-primary"))
                    .text_size(px(11.))
                    .child(n.to_string())
            }))
            .on_click(cx.listener(move |this, _, _, cx| {
                this.panel_tab = tab;
                cx.notify();
            }))
    }

    pub(super) fn render_problems(
        &self,
        doc: &Document,
        cx: &mut Context<Self>,
    ) -> impl IntoElement + use<> {
        let count = doc.diagnostics.len();
        let header = div()
            .flex()
            .flex_none()
            .items_center()
            .gap(px(2.))
            .h(px(34.))
            .px(px(8.))
            .border_b_1()
            .border_color(color("border-100"))
            .child(self.render_panel_tab(PanelTab::Problems, "Problems", Some(count), cx))
            .child(self.render_panel_tab(PanelTab::Output, "Output", None, cx))
            .child(div().flex_1())
            .child(
                div()
                    .pr(px(8.))
                    .text_color(color("text-tertiary"))
                    .child(format!(
                        "Checked against curl-runner {} schema",
                        curl_runner_core::VERSION
                    )),
            );
        let body = match self.panel_tab {
            PanelTab::Problems => self.render_problem_rows(doc, cx).into_any_element(),
            PanelTab::Output => div()
                .px(px(16.))
                .py(px(10.))
                .text_color(color("text-secondary"))
                .child("No output yet. Run a request to see its log here.")
                .into_any_element(),
        };
        div()
            .id("problems-panel")
            .role(Role::List)
            .test_support()
            .flex()
            .flex_col()
            .flex_none()
            .h(px(PROBLEMS_HEIGHT))
            .border_t_1()
            .border_color(color("border-100"))
            .text_size(px(12.))
            .child(header)
            .child(body)
    }

    /// Errors first, then warnings; each row carries its diagnostic's index.
    fn render_problem_rows(
        &self,
        doc: &Document,
        cx: &mut Context<Self>,
    ) -> impl IntoElement + use<> {
        let rows = problem_order(&doc.diagnostics).into_iter().map(|i| {
            let d = &doc.diagnostics[i];
            let selected = self.selected_problem == Some(i);
            div()
                .id(("problem", i))
                .role(Role::ListItem)
                .test_support()
                .flex()
                .flex_none()
                .items_center()
                .gap(px(10.))
                .h(px(30.))
                .px(px(8.))
                .rounded(px(6.))
                .text_color(color("text-primary"))
                .cursor_pointer()
                .map(|row| {
                    if selected {
                        row.bg(color("surface-200"))
                    } else {
                        row.hover(|s| s.bg(color("surface-100")))
                    }
                })
                .child(severity_icon(d.severity, 13.))
                .child(message_line(&d.message, 11.5, false).flex_1())
                .child(
                    div()
                        .flex_none()
                        .font_family(mono_font())
                        .text_size(px(11.))
                        .text_color(color("text-tertiary"))
                        .child(d.path.clone()),
                )
                .child(
                    div()
                        .w(px(40.))
                        .flex_none()
                        .text_right()
                        .font_family(mono_font())
                        .text_size(px(11.))
                        .text_color(color("text-secondary"))
                        .child(line_col(&doc.text, d.range.start)),
                )
                .on_click(cx.listener(move |this, _, window, cx| {
                    this.selected_problem = Some(i);
                    this.reveal_problem(i, window, cx);
                    cx.notify();
                }))
        });
        let count = doc.diagnostics.len();
        div()
            .id("problems-list")
            .flex()
            .flex_col()
            .flex_1()
            .min_h_0()
            .overflow_y_scroll()
            .py(px(4.))
            .px(px(8.))
            .children(rows)
            .when(count == 0, |list| {
                list.child(
                    div()
                        .px(px(8.))
                        .py(px(6.))
                        .text_color(color("text-secondary"))
                        .child("No problems in this file."),
                )
            })
    }
}

#[cfg(test)]
mod tests {
    use super::super::diagnostics_view::byte_to_position;
    use super::super::workspace::Workspace;
    use super::line_col;
    use gpui_kit::test::TestWindowExt as _;
    use gpui_kit::{
        AnyWindowHandle, AppContext as _, Bounds, Entity, Point, Role, TestAppContext,
        WindowBounds, WindowOptions, px, size,
    };
    use std::path::Path;

    const BAD: &str = "request:\n  name: Create charge\n  url: https://api.example.com/v1/charges\n  method: FETCH\n  timout: 5000\n  retry:\n    count: three\n    backoff: 2\n";
    const OK: &str = "request:\n  url: https://api.example.com\n";

    fn open(cx: &mut TestAppContext) -> (tempfile::TempDir, AnyWindowHandle, Entity<Workspace>) {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("bad.yaml"), BAD).unwrap();
        std::fs::write(dir.path().join("ok.yaml"), OK).unwrap();
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
                        window,
                        cx,
                    )
                })
            })
            .unwrap()
        });
        (dir, window, ws)
    }

    #[test]
    fn line_col_is_one_based() {
        assert_eq!(line_col("a\n  bc: 1\n", 4), "2:3");
    }

    fn problem_rows(window: &mut gpui_kit::Window) -> usize {
        window
            .within("problems-panel")
            .find_all_by_role(Role::ListItem)
            .len()
    }

    #[gpui_kit::test]
    fn problems_panel_lists_active_file_issues(cx: &mut TestAppContext) {
        let (_dir, window, ws) = open(cx);
        cx.update_window(window, |_, window, cx| {
            ws.update(cx, |ws, cx| {
                ws.open_file(Path::new("ok.yaml"), window, cx);
                ws.open_file(Path::new("bad.yaml"), window, cx);
            });
            window.render_frame(cx);
            let expected = ws.read(cx).state.open[1].diagnostics.len();
            assert!(expected >= 3, "{expected}");
            assert_eq!(problem_rows(window), expected);

            // A click moves the cursor to the problem.
            window
                .within("problems-panel")
                .click(("problem", 1usize), cx);
            window.render_frame(cx);
            let ws = ws.read(cx);
            let doc = &ws.state.open[1];
            let (line, character) = byte_to_position(&doc.text, doc.diagnostics[1].range.start);
            let cursor = ws.tabs[1].editor.read(cx).cursor_position();
            assert_eq!((cursor.line, cursor.character), (line, character));
        })
        .unwrap();
        cx.update_window(window, |_, window, cx| {
            ws.update(cx, |ws, cx| ws.activate_tab(0, window, cx));
            window.render_frame(cx);
            assert_eq!(problem_rows(window), 0, "only the active file is listed");
        })
        .unwrap();
    }
}
