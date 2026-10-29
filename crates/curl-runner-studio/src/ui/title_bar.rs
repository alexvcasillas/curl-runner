//! The title bar: the app mark and the open collection.

use super::app::TITLE_BAR_HEIGHT;
use super::color;
use super::workspace::Workspace;
use gpui_kit::component::TitleBar;
use gpui_kit::prelude::*;
use gpui_kit::*;

impl Workspace {
    pub(super) fn render_title_bar(&self) -> impl IntoElement + use<> {
        let collection = match &self.state.collection {
            Some(c) => div().font_weight(FontWeight::MEDIUM).child(c.name.clone()),
            None => div()
                .text_color(color("text-secondary"))
                .child("No collection"),
        };
        TitleBar::new()
            .h(px(TITLE_BAR_HEIGHT))
            .bg(color("bg-100"))
            .border_color(color("border-100"))
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(12.))
                    .text_size(px(13.))
                    .child(
                        div()
                            .px(px(8.))
                            .font_weight(FontWeight::SEMIBOLD)
                            .child("curl-runner studio"),
                    )
                    .child(
                        div()
                            .text_size(px(18.))
                            .text_color(color("border-200"))
                            .child("/"),
                    )
                    .child(collection.px(px(8.))),
            )
    }
}
