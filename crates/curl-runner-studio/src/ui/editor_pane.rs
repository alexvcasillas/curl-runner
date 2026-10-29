//! The editor column: file tabs, breadcrumb with problem counts,
//! disk-conflict callout, the YAML editor and the Problems panel.

use super::color;
use super::diagnostic_marks::{EDITOR_TEXT, paint_marks};
use super::diagnostic_popover::severity_icon;
use super::sidebar::method_badge;
use super::workspace::Workspace;
use crate::model::diagnostics::counts;
use crate::model::document::Document;
use curl_runner_core::lint::Severity;
use gpui_kit::component::button::Button;
use gpui_kit::component::input::Editor;
use gpui_kit::component::{Icon, IconName, Sizable as _};
use gpui_kit::prelude::*;
use gpui_kit::*;
use std::path::{Component, Path};

const TAB_STRIP_HEIGHT: f32 = 40.;
const BREADCRUMB_HEIGHT: f32 = 36.;

/// `2 errors`, `1 warning`.
fn count_label(n: usize, noun: &str) -> String {
    if n == 1 {
        format!("1 {noun}")
    } else {
        format!("{n} {noun}s")
    }
}

/// `N errors`, `M warnings`, each hidden at zero.
fn count_parts(errors: usize, warnings: usize) -> Vec<(Severity, String)> {
    let mut parts = Vec::new();
    if errors > 0 {
        parts.push((Severity::Error, count_label(errors, "error")));
    }
    if warnings > 0 {
        parts.push((Severity::Warning, count_label(warnings, "warning")));
    }
    parts
}

/// The breadcrumb's save state.
fn save_label(dirty: bool) -> &'static str {
    if dirty { "· Unsaved" } else { "· Saved" }
}

/// `collection / folders / file` for a file at `rel`.
pub fn breadcrumb(collection: &str, rel: &Path) -> Vec<String> {
    std::iter::once(collection.to_string())
        .chain(rel.components().filter_map(|c| match c {
            Component::Normal(part) => Some(part.to_string_lossy().into_owned()),
            _ => None,
        }))
        .collect()
}

impl Workspace {
    pub(super) fn render_editor_pane(&self, cx: &mut Context<Self>) -> impl IntoElement + use<> {
        let pane = div()
            .id("editor-pane")
            .flex()
            .flex_col()
            .flex_1()
            .min_w(px(560.))
            .h_full();
        let Some(active) = self.state.active else {
            return pane
                .child(self.render_tabs(cx))
                .child(render_nothing_open());
        };
        let doc = &self.state.open[active];
        let state = self.tabs[active].editor.clone();
        let editor = Editor::new(&state)
            .bordered(false)
            .h_full()
            .text_size(px(EDITOR_TEXT))
            .line_height(px(18.));
        // Painted after the editor, from the layout it just painted.
        let marks = self.tabs[active].diagnostics.marks.clone();
        let hide_ghost = self.hovered_line();
        let overlay = canvas(
            |_, _, _| {},
            move |_, _, window, cx| paint_marks(&state, &marks.borrow(), hide_ghost, window, cx),
        )
        .absolute()
        .size_full();
        let editor = div()
            .id("editor-area")
            .relative()
            .flex_1()
            .min_h_0()
            .pt(px(8.))
            .child(editor)
            .child(overlay)
            .children(self.render_diagnostic_popover(cx))
            .on_mouse_move(cx.listener(|this, e: &MouseMoveEvent, _, cx| {
                this.hover_diagnostics_at(e.position, cx)
            }))
            .on_hover(cx.listener(|this, hovered: &bool, _, cx| {
                if !*hovered {
                    this.release_hover(cx);
                }
            }));
        pane.child(self.render_tabs(cx))
            .child(self.render_breadcrumb(doc))
            .when(doc.disk_conflict, |pane| pane.child(render_conflict(cx)))
            .child(editor)
            .when(self.problems_open, |pane| {
                pane.child(self.render_problems(doc, cx))
            })
    }

    fn render_tabs(&self, cx: &mut Context<Self>) -> impl IntoElement + use<> {
        let tabs = self.state.open.iter().enumerate().map(|(ix, doc)| {
            let selected = self.state.active == Some(ix);
            let name = doc
                .rel_path
                .file_name()
                .map(|n| n.to_string_lossy().into_owned())
                .unwrap_or_default();
            let group: SharedString = format!("tab-{ix}").into();
            let close = div()
                .id(("tab-close", ix))
                .relative()
                .flex()
                .items_center()
                .justify_center()
                .size(px(16.))
                .rounded(px(4.))
                .hover(|s| s.bg(color("surface-300")))
                .when(doc.is_dirty(), |slot| {
                    slot.child(
                        div()
                            .absolute()
                            .size(px(6.))
                            .rounded_full()
                            .bg(color("text-secondary"))
                            .group_hover(group.clone(), |s| s.invisible()),
                    )
                })
                .child(
                    div()
                        .invisible()
                        .group_hover(group.clone(), |s| s.visible())
                        .child(
                            Icon::new(IconName::Close)
                                .size(px(12.))
                                .text_color(color("text-secondary")),
                        ),
                )
                .on_click(cx.listener(move |this, _, window, cx| {
                    cx.stop_propagation();
                    this.request_close_tab(ix, window, cx);
                }));
            div()
                .id(("tab", ix))
                .group(group)
                .flex()
                .flex_none()
                .items_center()
                .gap(px(6.))
                .h(px(32.))
                .pl(px(12.))
                .pr(px(8.))
                .text_size(px(13.))
                .border_b_1()
                .cursor_pointer()
                .map(|tab| {
                    if selected {
                        tab.text_color(color("text-primary"))
                            .border_color(color("primary"))
                    } else {
                        tab.text_color(color("text-secondary"))
                            .border_color(gpui_kit::transparent_black())
                    }
                })
                .child(method_badge(self.method_of(&doc.rel_path).as_deref()).w_auto())
                .child(name)
                .child(close)
                .on_click(cx.listener(move |this, _, window, cx| this.activate_tab(ix, window, cx)))
        });
        div()
            .id("tabs")
            .flex()
            .flex_none()
            .items_end()
            .gap(px(2.))
            .h(px(TAB_STRIP_HEIGHT))
            .px(px(8.))
            .border_b_1()
            .border_color(color("border-100"))
            .overflow_x_scroll()
            .children(tabs)
    }

    /// The file's path, then its error and warning counts (hidden at zero)
    /// and `Unsaved` while dirty.
    fn render_breadcrumb(&self, doc: &Document) -> impl IntoElement + use<> {
        let collection = self
            .state
            .collection
            .as_ref()
            .map_or("", |c| c.name.as_str());
        let parts = breadcrumb(collection, &doc.rel_path);
        let last = parts.len() - 1;
        let counts = counts(&doc.diagnostics);
        let counts =
            count_parts(counts.errors, counts.warnings)
                .into_iter()
                .map(|(severity, label)| {
                    div()
                        .flex()
                        .items_center()
                        .gap(px(6.))
                        .text_color(super::diagnostics_view::severity_color(severity))
                        .child(severity_icon(severity, 13.))
                        .child(label)
                });
        div()
            .flex()
            .flex_none()
            .items_center()
            .gap(px(10.))
            .h(px(BREADCRUMB_HEIGHT))
            .px(px(16.))
            .text_size(px(12.))
            .text_color(color("text-secondary"))
            .border_b_1()
            .border_color(color("border-100"))
            .children(parts.into_iter().enumerate().flat_map(move |(i, part)| {
                let sep = (i > 0).then(|| div().child("/"));
                let part = div()
                    .when(i == last, |d| d.text_color(color("text-primary")))
                    .child(part);
                sep.into_iter().chain(std::iter::once(part))
            }))
            .child(div().flex_1())
            .children(counts)
            .child(save_label(doc.is_dirty()))
    }
}

/// The `Callout` (warning) shown while the buffer conflicts with the disk.
fn render_conflict(cx: &mut Context<Workspace>) -> impl IntoElement + use<> {
    div()
        .id("disk-conflict")
        .flex()
        .flex_none()
        .items_center()
        .gap(px(8.))
        .mx(px(16.))
        .mt(px(8.))
        .py(px(10.))
        .px(px(12.))
        .rounded(px(6.))
        .border_1()
        .border_color(color("warning-border"))
        .bg(color("warning-subtle"))
        .text_size(px(12.))
        .text_color(color("text-primary"))
        .child(
            Icon::new(IconName::TriangleAlert)
                .size(px(13.))
                .text_color(color("warning")),
        )
        .child(
            div()
                .flex_1()
                .child("Changed on disk. Reload to take the disk version, or keep your edits."),
        )
        .child(
            Button::new("conflict-reload")
                .outline()
                .xsmall()
                .label("Reload")
                .on_click(
                    cx.listener(|this, _, window, cx| this.resolve_conflict(false, window, cx)),
                ),
        )
        .child(
            Button::new("conflict-keep")
                .outline()
                .xsmall()
                .label("Keep mine")
                .on_click(
                    cx.listener(|this, _, window, cx| this.resolve_conflict(true, window, cx)),
                ),
        )
}

/// The editor area's `EmptyState` when the collection has no open tab.
fn render_nothing_open() -> impl IntoElement {
    div()
        .flex()
        .flex_col()
        .flex_1()
        .items_center()
        .justify_center()
        .gap(px(10.))
        .py(px(48.))
        .child(
            div()
                .flex()
                .items_center()
                .justify_center()
                .size(px(40.))
                .rounded(px(8.))
                .border_1()
                .border_dashed()
                .border_color(color("border-200"))
                .child(
                    Icon::new(IconName::FileText)
                        .size(px(16.))
                        .text_color(color("text-tertiary")),
                ),
        )
        .child(
            div()
                .text_size(px(16.))
                .font_weight(FontWeight::SEMIBOLD)
                .child("No request open"),
        )
        .child(
            div()
                .text_size(px(13.))
                .text_color(color("text-secondary"))
                .child("Pick a request in the sidebar to edit its YAML."),
        )
}

#[cfg(test)]
mod tests {
    use super::{breadcrumb, count_label, count_parts, save_label};
    use curl_runner_core::lint::Severity;
    use std::path::Path;

    #[test]
    fn count_parts_list_errors_then_warnings() {
        assert_eq!(
            count_parts(2, 1),
            [
                (Severity::Error, "2 errors".into()),
                (Severity::Warning, "1 warning".into()),
            ]
        );
        assert_eq!(
            count_parts(0, 3),
            [(Severity::Warning, "3 warnings".into())]
        );
        assert_eq!(count_parts(1, 0), [(Severity::Error, "1 error".into())]);
        assert_eq!(count_parts(0, 0), []);
    }

    #[test]
    fn save_label_follows_dirty() {
        assert_eq!(save_label(true), "· Unsaved");
        assert_eq!(save_label(false), "· Saved");
    }

    #[test]
    fn count_label_pluralizes() {
        assert_eq!(count_label(1, "warning"), "1 warning");
        assert_eq!(count_label(2, "error"), "2 errors");
    }

    #[test]
    fn breadcrumb_lists_collection_folders_and_file() {
        assert_eq!(
            breadcrumb("Payments API", Path::new("charges/create-charge.yaml")),
            ["Payments API", "charges", "create-charge.yaml"]
        );
        assert_eq!(breadcrumb("acme", Path::new("a.yaml")), ["acme", "a.yaml"]);
    }
}
