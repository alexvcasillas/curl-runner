//! The Problems panel under the editor: the active file's diagnostics.

use super::app::mono_font;
use super::color;
use super::diagnostics_view::{byte_to_position, severity_color};
use super::workspace::Workspace;
use crate::model::document::Document;
use curl_runner_core::lint::Severity;
use gpui_kit::component::{Icon, IconName};
use gpui_kit::prelude::*;
use gpui_kit::*;

pub const PROBLEMS_HEIGHT: f32 = 178.;

pub fn severity_icon(severity: Severity) -> Icon {
    let name = match severity {
        Severity::Error => IconName::CircleX,
        Severity::Warning => IconName::TriangleAlert,
        Severity::Info => IconName::Info,
    };
    Icon::new(name)
        .size(px(14.))
        .text_color(severity_color(severity))
}

/// `14:12`, one-based like the editor shows it.
fn line_col(text: &str, offset: usize) -> String {
    let (line, character) = byte_to_position(text, offset);
    format!("{}:{}", line + 1, character + 1)
}

impl Workspace {
    pub(super) fn render_problems(
        &self,
        doc: &Document,
        cx: &mut Context<Self>,
    ) -> impl IntoElement + use<> {
        let rows = doc.diagnostics.iter().enumerate().map(|(i, d)| {
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
                .cursor_pointer()
                .hover(|s| s.bg(color("surface-100")))
                .child(severity_icon(d.severity))
                .child(
                    div()
                        .flex_1()
                        .min_w_0()
                        .truncate()
                        .text_color(color("text-primary"))
                        .child(d.message.clone()),
                )
                .child(
                    div()
                        .font_family(mono_font())
                        .text_size(px(11.))
                        .text_color(color("text-tertiary"))
                        .child(d.path.clone()),
                )
                .child(
                    div()
                        .w(px(48.))
                        .flex_none()
                        .text_right()
                        .font_family(mono_font())
                        .text_size(px(11.))
                        .text_color(color("text-secondary"))
                        .child(line_col(&doc.text, d.range.start)),
                )
                .on_click(
                    cx.listener(move |this, _, window, cx| this.reveal_problem(i, window, cx)),
                )
        });
        let count = doc.diagnostics.len();
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
            .child(
                div()
                    .flex()
                    .flex_none()
                    .items_center()
                    .gap(px(6.))
                    .h(px(34.))
                    .px(px(16.))
                    .border_b_1()
                    .border_color(color("border-100"))
                    .child(div().text_color(color("text-primary")).child("Problems"))
                    .child(
                        div()
                            .px(px(6.))
                            .rounded_full()
                            .bg(color("surface-300"))
                            .text_size(px(11.))
                            .child(count.to_string()),
                    )
                    .child(div().flex_1())
                    .child(
                        div()
                            .text_color(color("text-tertiary"))
                            .child(format!("curl-runner {}", curl_runner_core::VERSION)),
                    ),
            )
            .child(
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
                    }),
            )
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
                cx.new(|cx| Workspace::new(Some(root), window, cx))
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
