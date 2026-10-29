//! The empty state shown when no collection is open (`Welcome.dc.html`).

use super::app::mono_font;
use super::color;
use super::workspace::Workspace;
use gpui_kit::component::tooltip::Tooltip;
use gpui_kit::component::{Icon, IconName};
use gpui_kit::prelude::*;
use gpui_kit::*;

/// One of the four start actions.
struct Card {
    id: &'static str,
    icon: IconName,
    keys: &'static str,
    title: &'static str,
    detail: &'static str,
    /// `None`: shown disabled with "Coming soon".
    action: Option<fn(&mut Workspace, &mut Window, &mut Context<Workspace>)>,
}

const CARDS: [Card; 4] = [
    Card {
        id: "welcome-new-collection",
        icon: IconName::Plus,
        keys: "⇧⌘N",
        title: "New collection",
        detail: "Pick an empty folder and start with a first request.",
        action: Some(|ws, _, cx| ws.new_collection(cx)),
    },
    Card {
        id: "welcome-open-folder",
        icon: IconName::FolderOpen,
        keys: "⌘O",
        title: "Open folder",
        detail: "Work on YAML files already on disk, for example a repo checkout.",
        action: Some(|ws, window, cx| ws.open_folder(window, cx)),
    },
    Card {
        id: "welcome-open-link",
        icon: IconName::Globe,
        keys: "⌘L",
        title: "Open a shared link",
        detail: "Paste a curl-runner link someone sent you. Opens read-only.",
        action: None,
    },
    Card {
        id: "welcome-import",
        icon: IconName::ArrowDown,
        keys: "⌘I",
        title: "Import",
        detail: "Turn a curl command into a request file.",
        action: Some(|ws, _, cx| ws.import(cx)),
    },
];

impl Workspace {
    pub(super) fn render_welcome(&self, cx: &mut Context<Self>) -> impl IntoElement + use<> {
        let intro = div()
            .flex()
            .flex_col()
            .gap(px(12.))
            .child(
                div()
                    .flex()
                    .items_center()
                    .justify_center()
                    .size(px(40.))
                    .rounded(px(10.))
                    .bg(color("primary"))
                    .text_color(color("on-primary"))
                    .font_family(mono_font())
                    .font_weight(FontWeight::MEDIUM)
                    .text_size(px(15.))
                    .child(">_"),
            )
            .child(
                div()
                    .mt(px(8.))
                    .text_size(px(28.))
                    .font_weight(FontWeight::SEMIBOLD)
                    .child("Welcome to curl-runner studio"),
            )
            .child(
                div()
                    .text_size(px(14.))
                    .line_height(relative(1.6))
                    .text_color(color("text-secondary"))
                    .child(
                        "Write requests as YAML, run them as tests, and share collections \
                         read-only with the people who call your API.",
                    ),
            );
        div()
            .id("welcome")
            .flex()
            .flex_1()
            .min_w_0()
            .justify_center()
            .overflow_y_scroll()
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap(px(32.))
                    .w(px(720.))
                    .pt(px(72.))
                    .pb(px(40.))
                    .child(intro)
                    .child(
                        div()
                            .grid()
                            .grid_cols(2)
                            .gap(px(12.))
                            .children(CARDS.iter().map(|card| render_card(card, cx))),
                    ),
            )
    }
}

fn render_card(card: &Card, cx: &mut Context<Workspace>) -> impl IntoElement + use<> {
    let enabled = card.action.is_some();
    let fg = |token| color(if enabled { token } else { "text-disabled" });
    let top = div()
        .flex()
        .w_full()
        .items_center()
        .justify_between()
        .child(
            div()
                .flex()
                .items_center()
                .justify_center()
                .size(px(32.))
                .rounded(px(6.))
                .border_1()
                .border_color(color("border-200"))
                .child(
                    Icon::new(card.icon.clone())
                        .size(px(16.))
                        .text_color(fg("text-primary")),
                ),
        )
        .child(
            div()
                .px(px(5.))
                .rounded(px(4.))
                .border_1()
                .border_color(color("border-200"))
                .font_family(mono_font())
                .text_size(px(11.))
                .line_height(px(18.))
                .text_color(fg("text-secondary"))
                .child(card.keys),
        );
    div()
        .id(card.id)
        .flex()
        .flex_col()
        .items_start()
        .gap(px(10.))
        .p(px(16.))
        .rounded(px(8.))
        .border_1()
        .border_color(color("border-100"))
        .child(top)
        .child(
            div()
                .text_size(px(14.))
                .font_weight(FontWeight::MEDIUM)
                .text_color(fg("text-primary"))
                .child(card.title),
        )
        .child(
            div()
                .text_size(px(12.))
                .line_height(relative(1.5))
                .text_color(fg("text-secondary"))
                .child(card.detail),
        )
        .map(|el| match card.action {
            Some(action) => el
                .cursor_pointer()
                .hover(|s| s.bg(color("surface-100")))
                .on_click(cx.listener(move |this, _, window, cx| action(this, window, cx))),
            None => el
                .cursor_not_allowed()
                .tooltip(|window, cx| Tooltip::new("Coming soon").build(window, cx)),
        })
}
