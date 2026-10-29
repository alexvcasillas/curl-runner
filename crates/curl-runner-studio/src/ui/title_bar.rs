//! The title bar: the app mark, the open collection, the search button
//! (opens the palette) and Run.

use super::app::{TITLE_BAR_HEIGHT, mono_font};
use super::color;
use super::icons::{self, glyph};
use super::workspace::Workspace;
use gpui_kit::component::TitleBar;
use gpui_kit::prelude::*;
use gpui_kit::*;

impl Workspace {
    pub(super) fn render_title_bar(&self, cx: &mut Context<Self>) -> impl IntoElement + use<> {
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
                // Equal flexible sides keep the search button centred.
                div()
                    .flex_1()
                    .flex_basis(px(0.))
                    .min_w_0()
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
            .child(self.render_search_button(cx))
            .child(
                div()
                    .flex_1()
                    .flex_basis(px(0.))
                    .min_w_0()
                    .flex()
                    .justify_end()
                    // Mirrors the Kit's traffic-light inset on the left.
                    .pl(px(TRAFFIC_LIGHTS_INSET))
                    .pr(px(12.))
                    .child(self.render_run_button(cx)),
            )
    }

    /// "Search requests… ⌘K", opening the command palette.
    fn render_search_button(&self, cx: &mut Context<Self>) -> impl IntoElement + use<> {
        div()
            .id("title-search")
            .flex()
            .flex_none()
            .items_center()
            .gap(px(8.))
            .w(px(SEARCH_WIDTH))
            .h(px(32.))
            .pl(px(10.))
            .pr(px(8.))
            .border_1()
            .border_color(color("border-200"))
            .rounded(px(6.))
            .bg(color("bg-100"))
            .text_size(px(13.))
            .text_color(color("text-secondary"))
            .cursor_pointer()
            .hover(|s| s.bg(color("surface-100")))
            .child(glyph(icons::SEARCH).size(px(14.)).flex_none())
            .child(div().flex_1().child("Search requests…"))
            .child(
                div()
                    .flex_none()
                    .px(px(5.))
                    .border_1()
                    .border_color(color("border-200"))
                    .rounded(px(4.))
                    .font_family(mono_font())
                    .text_size(px(11.))
                    .child("⌘K"),
            )
            .on_mouse_down(MouseButton::Left, |_, window, _| window.prevent_default())
            .on_click(cx.listener(|this, _, window, cx| {
                this.open_palette(window, cx);
            }))
    }
}

const SEARCH_WIDTH: f32 = 280.;
/// The Kit title bar's left padding for the traffic lights on macOS.
const TRAFFIC_LIGHTS_INSET: f32 = 80.;
