//! The pieces both settings screens draw: the collection settings
//! (board: Environments) and the app's Settings (board: Settings). Buttons,
//! pills and the switch in the boards' `.btn` / `.tgl` shapes, the left
//! nav's rows, and the Settings board's cards.

use super::app::mono_font;
use super::color;
use super::icons::{self, glyph};
use super::run_view::pill;
use gpui_kit::prelude::*;
use gpui_kit::*;

/// An element the smoke tests can find.
pub type Observed = gpui_kit::base::ObservedElement<Stateful<Div>>;

/// The board's `.btn` shape at 28px, without colors.
pub fn button_base(id: impl Into<ElementId>, label: impl Into<SharedString>) -> Observed {
    let label = label.into();
    div()
        .id(id)
        .role(Role::Button)
        .test_support()
        .aria_label(label)
        .flex()
        .flex_none()
        .items_center()
        .gap(px(8.))
        .h(px(28.))
        .px(px(10.))
        .border_1()
        .rounded(px(6.))
        .text_size(px(12.))
        .whitespace_nowrap()
        .cursor_pointer()
}

/// The board's `.btn.sm`: a 28px outlined button.
pub fn small_button(id: impl Into<ElementId>, label: impl Into<SharedString>) -> Observed {
    button_base(id, label)
        .border_color(color("border-200"))
        .bg(color("bg-100"))
        .text_color(color("text-primary"))
        .hover(|s| s.bg(color("surface-100")))
}

/// The board's `.btn.sm.pri`.
pub fn primary_button(id: impl Into<ElementId>, label: impl Into<SharedString>) -> Observed {
    button_base(id, label)
        .border_color(color("primary"))
        .bg(color("primary"))
        .text_color(color("on-primary"))
        .font_weight(FontWeight::MEDIUM)
        .hover(|s| s.bg(color("primary-hover")))
}

/// A tinted pill: `tone` text, a 30% border and an 8% fill.
pub fn tinted_pill(tone: &str) -> Div {
    pill(("text-secondary", "", "border-200"))
        .font_weight(FontWeight::NORMAL)
        .text_color(color(tone))
        .border_color(color(tone).opacity(0.3))
        .bg(color(tone).opacity(0.08))
}

pub fn neutral_pill() -> Div {
    pill(("text-secondary", "", "border-200")).font_weight(FontWeight::NORMAL)
}

/// The design system's 28×16 toggle.
pub fn switch(on: bool, enabled: bool) -> Div {
    let knob = div()
        .absolute()
        .top(px(2.))
        .size(px(12.))
        .rounded_full()
        .map(|k| if on { k.right(px(2.)) } else { k.left(px(2.)) })
        .bg(color(if on { "bg-100" } else { "text-secondary" }));
    div()
        .relative()
        .flex_none()
        .w(px(28.))
        .h(px(16.))
        .mt(px(2.))
        .rounded_full()
        .bg(color(if on { "primary" } else { "surface-300" }))
        .when(!enabled, |d| d.opacity(0.5))
        .child(knob)
}

/// A 12px medium label in `text-secondary`.
pub fn label_text(text: impl Into<SharedString>) -> Div {
    div()
        .text_size(px(12.))
        .font_weight(FontWeight::MEDIUM)
        .text_color(color("text-secondary"))
        .child(text.into())
}

/// The `Soon` chip on a nav row that does nothing yet.
pub fn soon_chip() -> Div {
    div()
        .flex_none()
        .px(px(6.))
        .rounded_full()
        .border_1()
        .border_color(color("border-200"))
        .text_size(px(11.))
        .text_color(color("text-disabled"))
        .child("Soon")
}

/// A nav row of `height`: `surface-300` and `text-primary` when
/// `Some(true)`, a hover step when `Some(false)`, `text-disabled` and inert
/// when `None`. Add the icon and label as children.
pub fn nav_item(
    id: impl Into<ElementId>,
    label: &'static str,
    height: f32,
    on: Option<bool>,
) -> Observed {
    div()
        .id(id)
        .role(Role::Button)
        .test_support()
        .aria_label(label)
        .flex()
        .flex_none()
        .items_center()
        .rounded(px(6.))
        .text_size(px(13.))
        .whitespace_nowrap()
        .h(px(height))
        .map(|d| match on {
            Some(true) => d
                .cursor_pointer()
                .bg(color("surface-300"))
                .text_color(color("text-primary")),
            Some(false) => d
                .cursor_pointer()
                .text_color(color("text-secondary"))
                .hover(|s| s.bg(color("surface-100"))),
            None => d.text_color(color("text-disabled")),
        })
}

/// `‹ Back to workspace` (or wherever `label` says), `size` px text. Wire
/// its click.
pub fn back_link(id: impl Into<ElementId>, label: &'static str, size: f32) -> Observed {
    div()
        .id(id)
        .role(Role::Button)
        .test_support()
        .aria_label(label)
        .flex()
        .flex_none()
        .items_center()
        .gap(px(8.))
        .text_size(px(size))
        .text_color(color("text-secondary"))
        .cursor_pointer()
        .hover(|s| s.text_color(color("text-primary")))
        .child(glyph(icons::CHEVRON_LEFT).size(px(13.)))
        .child(label)
}

/// The Settings board's `.card`: a hairline box on `bg-100`.
pub fn card(id: impl Into<ElementId>) -> Observed {
    div()
        .id(id)
        .role(Role::Group)
        .test_support()
        .relative()
        .flex()
        .flex_col()
        .flex_none()
        .border_1()
        .border_color(color("border-100"))
        .rounded(px(8.))
        .bg(color("bg-100"))
        .overflow_hidden()
}

/// `.card > .bd`: 20×24 padding, 12px between its parts.
pub fn card_body() -> Div {
    div().flex().flex_col().gap(px(12.)).px(px(24.)).py(px(20.))
}

/// The card's `h2`.
pub fn card_title(text: impl Into<SharedString>) -> Div {
    div()
        .text_size(px(16.))
        .font_weight(FontWeight::SEMIBOLD)
        .text_color(color("text-primary"))
        .child(text.into())
}

/// The card's `p`.
pub fn card_text(text: impl Into<SharedString>) -> Div {
    div()
        .text_size(px(13.))
        .line_height(relative(1.5))
        .text_color(color("text-secondary"))
        .child(text.into())
}

/// `.card > .ft`: the 52px footer on `surface-raised`.
pub fn card_footer() -> Div {
    div()
        .flex()
        .flex_none()
        .items_center()
        .gap(px(12.))
        .min_h(px(52.))
        .px(px(24.))
        .border_t_1()
        .border_color(color("border-100"))
        .bg(color("surface-raised"))
        .text_size(px(12.))
        .text_color(color("text-secondary"))
}

/// A setting inside a card: its label and description on the left, its
/// control on the right. Rows after the first get a hairline above.
pub fn setting_row(
    id: impl Into<ElementId>,
    label: &'static str,
    description: impl IntoElement,
    first: bool,
    control: impl IntoElement,
) -> Observed {
    div()
        .id(id)
        .role(Role::Group)
        .test_support()
        .aria_label(label)
        .flex()
        .items_center()
        .gap(px(24.))
        .px(px(24.))
        .py(px(16.))
        .when(!first, |d| d.border_t_1().border_color(color("border-100")))
        .child(
            div()
                .flex()
                .flex_col()
                .flex_1()
                .min_w_0()
                .gap(px(2.))
                .child(div().text_size(px(13.)).child(label))
                .child(
                    div()
                        .text_size(px(12.))
                        .line_height(relative(1.45))
                        .text_color(color("text-secondary"))
                        .child(description),
                ),
        )
        .child(div().flex().flex_none().items_center().child(control))
}

/// A table head cell: 11px `text-tertiary`.
pub fn th(text: &'static str) -> Div {
    div()
        .text_size(px(11.))
        .text_color(color("text-tertiary"))
        .whitespace_nowrap()
        .child(text)
}

/// One keyboard chord in a `kbd` key: Geist Mono on `surface-300`.
pub fn kbd_key(keys: impl Into<SharedString>) -> Div {
    div()
        .flex_none()
        .px(px(6.))
        .h(px(20.))
        .flex()
        .items_center()
        .rounded(px(4.))
        .border_1()
        .border_color(color("border-200"))
        .bg(color("surface-300"))
        .font_family(mono_font())
        .text_size(px(12.))
        .text_color(color("text-secondary"))
        .child(keys.into())
}

/// What a masked value reads as: never its length.
pub const MASK: &str = "••••••••••••";

/// The board's segmented control (`role=radiogroup`): add [`segment`]s.
pub fn segments(id: impl Into<ElementId>) -> Observed {
    div()
        .id(id)
        .role(Role::RadioGroup)
        .test_support()
        .flex()
        .flex_none()
        .items_center()
        .gap(px(2.))
        .p(px(3.))
        .border_1()
        .border_color(color("border-200"))
        .rounded(px(8.))
}

/// One option of [`segments`]: `surface-300` when `on`. Add its icon and
/// label as children.
pub fn segment(id: impl Into<ElementId>, label: impl Into<SharedString>, on: bool) -> Observed {
    div()
        .id(id)
        .role(Role::RadioButton)
        .test_support()
        .aria_label(label.into())
        .flex()
        .items_center()
        .gap(px(6.))
        .h(px(26.))
        .px(px(10.))
        .rounded(px(5.))
        .text_size(px(13.))
        .whitespace_nowrap()
        .cursor_pointer()
        .map(|d| {
            if on {
                d.bg(color("surface-300")).text_color(color("text-primary"))
            } else {
                d.text_color(color("text-secondary"))
                    .hover(|s| s.text_color(color("text-primary")))
            }
        })
}
