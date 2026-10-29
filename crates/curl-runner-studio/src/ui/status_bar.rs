//! The status bar: collection, language, cursor and curl-runner version.
//!
//! A view of its own: it follows the active editor's cursor, so moving it
//! repaints only this bar.

use super::color;
use gpui_kit::component::input::{EditorState, Position};
use gpui_kit::prelude::*;
use gpui_kit::*;

pub const STATUS_BAR_HEIGHT: f32 = 28.;

#[derive(Default)]
pub struct StatusBar {
    collection: Option<SharedString>,
    editor: Option<Entity<EditorState>>,
    cursor: Option<Position>,
    _cursor_watch: Option<Subscription>,
}

impl StatusBar {
    /// Shows `collection` and follows `editor`'s cursor (none: no file open).
    pub fn set(
        &mut self,
        collection: Option<SharedString>,
        editor: Option<Entity<EditorState>>,
        cx: &mut Context<Self>,
    ) {
        self.collection = collection;
        if self.editor != editor {
            self.cursor = editor.as_ref().map(|e| e.read(cx).cursor_position());
            self._cursor_watch = editor.as_ref().map(|e| {
                cx.observe(e, |this, editor, cx| {
                    let cursor = Some(editor.read(cx).cursor_position());
                    if this.cursor != cursor {
                        this.cursor = cursor;
                        cx.notify();
                    }
                })
            });
            self.editor = editor;
        }
        cx.notify();
    }
}

/// `Ln 15, Col 5`, one-based like every editor shows it.
fn cursor_label(p: Position) -> String {
    format!("Ln {}, Col {}", p.line + 1, p.character + 1)
}

impl Render for StatusBar {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div()
            .id("status-bar")
            .flex()
            .flex_none()
            .items_center()
            .gap(px(16.))
            .h(px(STATUS_BAR_HEIGHT))
            .px(px(16.))
            .border_t_1()
            .border_color(color("border-100"))
            .text_size(px(12.))
            .text_color(color("text-secondary"))
            .children(self.collection.clone())
            .child(div().flex_1())
            .when_some(self.cursor, |bar, cursor| {
                bar.child("YAML").child(cursor_label(cursor))
            })
            .child(format!("curl-runner {}", curl_runner_core::VERSION))
    }
}

#[cfg(test)]
mod tests {
    use super::{Position, cursor_label};

    #[test]
    fn cursor_label_is_one_based() {
        assert_eq!(cursor_label(Position::new(14, 4)), "Ln 15, Col 5");
    }
}
