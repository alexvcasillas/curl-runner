//! The editor column: file tabs, breadcrumb, disk-conflict callout and the
//! YAML editor.

use super::color;
use super::sidebar::method_badge;
use super::workspace::Workspace;
use gpui_kit::component::button::Button;
use gpui_kit::component::input::Editor;
use gpui_kit::component::{Icon, IconName, Sizable as _};
use gpui_kit::prelude::*;
use gpui_kit::*;
use std::path::{Component, Path};

const TAB_STRIP_HEIGHT: f32 = 40.;
const BREADCRUMB_HEIGHT: f32 = 36.;

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
        let editor = Editor::new(&self.tabs[active].editor)
            .bordered(false)
            .h_full()
            .text_size(px(13.))
            .line_height(px(18.));
        pane.child(self.render_tabs(cx))
            .child(self.render_breadcrumb(&doc.rel_path))
            .when(doc.disk_conflict, |pane| pane.child(render_conflict(cx)))
            .child(div().flex_1().min_h_0().pt(px(8.)).child(editor))
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

    fn render_breadcrumb(&self, rel: &Path) -> impl IntoElement + use<> {
        let collection = self
            .state
            .collection
            .as_ref()
            .map_or("", |c| c.name.as_str());
        let parts = breadcrumb(collection, rel);
        let last = parts.len() - 1;
        div()
            .flex()
            .flex_none()
            .items_center()
            .gap(px(6.))
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
    use super::breadcrumb;
    use std::path::Path;

    #[test]
    fn breadcrumb_lists_collection_folders_and_file() {
        assert_eq!(
            breadcrumb("Payments API", Path::new("charges/create-charge.yaml")),
            ["Payments API", "charges", "create-charge.yaml"]
        );
        assert_eq!(breadcrumb("acme", Path::new("a.yaml")), ["acme", "a.yaml"]);
    }
}
