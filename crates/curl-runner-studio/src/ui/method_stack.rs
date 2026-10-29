//! `MethodStack`: the method slot of a multi-request file row (column A of
//! `TreeMethods.dc.html`). Methods sit in request order as a deck of pills,
//! each one layered on top of the previous and shifted [`STEP`] right, so
//! the lower pills show their first letters and the top one its full name.
//! Borders are the row background, so each edge reads cleanly. A `+N` chip
//! after the deck counts the rest.

use super::{app::mono_font, color, method_color};
use gpui_kit::prelude::*;
use gpui_kit::*;

/// Pill width and outer height (14px line plus the border), px.
pub const PILL_W: f32 = 40.;
pub const PILL_H: f32 = 17.;
pub const PILL_RADIUS: f32 = 4.;
pub const BORDER: f32 = 1.5;
/// Text inset from the pill's left border, px.
pub const TEXT_PAD: f32 = 5.;
/// How far each pill sits right of the one below it, px.
pub const STEP: f32 = 12.;
/// Most pills in the deck; further methods go to the `+N` chip.
pub const MAX: usize = 3;
/// Gap between the deck and the `+N` chip, px.
pub const CHIP_GAP: f32 = 6.;
pub const CHIP_H: f32 = 14.;
pub const CHIP_PAD: f32 = 4.;
pub const TEXT_SIZE: f32 = 9.5;
/// For `DELETE`, so it fits inside the border.
pub const TEXT_SIZE_LONG: f32 = 8.5;
/// For methods longer than `DELETE`.
pub const TEXT_SIZE_LONGEST: f32 = 7.;
/// Alpha of the method color over the row background.
pub const FILL_ALPHA: f32 = 0.16;
/// Geist Mono's advance, em.
const MONO_ADVANCE_EM: f32 = 0.6;

/// What the stack shows for a file's methods.
#[derive(Debug, PartialEq, Eq)]
pub struct Stack {
    /// Distinct methods in request order, bottom first, at most [`MAX`].
    pub deck: Vec<String>,
    /// Distinct methods beyond the deck.
    pub more: usize,
}

impl Stack {
    pub fn new<'a>(methods: impl IntoIterator<Item = &'a str>) -> Self {
        let mut distinct: Vec<String> = Vec::new();
        for m in methods {
            if !distinct.iter().any(|d| d == m) {
                distinct.push(m.to_string());
            }
        }
        let more = distinct.len().saturating_sub(MAX);
        distinct.truncate(MAX);
        Stack {
            deck: distinct,
            more,
        }
    }

    /// Left edge of deck pill `i` relative to the slot, px.
    pub fn pill_left(i: usize) -> f32 {
        STEP * i as f32
    }

    /// Width of the pills alone, px.
    pub fn deck_width(&self) -> f32 {
        match self.deck.len() {
            0 => 0.,
            n => Self::pill_left(n - 1) + PILL_W,
        }
    }

    /// Left edge of the `+N` chip, px.
    pub fn chip_left(&self) -> f32 {
        self.deck_width() + CHIP_GAP
    }

    /// Width of the whole stack, `+N` chip included, px.
    pub fn width(&self) -> f32 {
        match self.more {
            0 => self.deck_width(),
            n => self.chip_left() + chip_width(n),
        }
    }
}

/// The `+N` chip's width, px.
pub fn chip_width(n: usize) -> f32 {
    let chars = format!("+{n}").len() as f32;
    chars * MONO_ADVANCE_EM * TEXT_SIZE + 2. * CHIP_PAD
}

fn text_size(method: &str) -> f32 {
    match method.len() {
        0..=5 => TEXT_SIZE,
        6 => TEXT_SIZE_LONG,
        _ => TEXT_SIZE_LONGEST,
    }
}

/// The pill's opaque fill: the method color at [`FILL_ALPHA`] over `row_bg`.
pub fn pill_fill(method: &str, row_bg: Hsla) -> Hsla {
    row_bg.blend(method_color(method).opacity(FILL_ALPHA))
}

/// The stack for `stack` in a `width`-wide slot. `row_bg` is the row
/// background behind it (bg-100, or the selected row's gray).
pub fn method_stack(stack: &Stack, row_bg: Hsla, width: f32) -> Div {
    let pill = |i: usize, method: &str| {
        div()
            .absolute()
            .top_0()
            .left(px(Stack::pill_left(i)))
            .w(px(PILL_W))
            .h(px(PILL_H))
            .rounded(px(PILL_RADIUS))
            .border(px(BORDER))
            .border_color(row_bg)
            .bg(pill_fill(method, row_bg))
            .overflow_hidden()
            .flex()
            .items_center()
            .pl(px(TEXT_PAD))
            .text_color(method_color(method))
            .text_size(px(text_size(method)))
            .line_height(px(PILL_H - 2. * BORDER))
            .whitespace_nowrap()
            .child(method.to_string())
    };
    let mut slot = div()
        .relative()
        .flex_none()
        .w(px(width))
        .h(px(PILL_H))
        .font_family(mono_font())
        .font_weight(FontWeight::MEDIUM);
    // Request order: later children paint over earlier ones.
    for (i, m) in stack.deck.iter().enumerate() {
        slot = slot.child(pill(i, m));
    }
    if stack.more > 0 {
        slot = slot.child(
            div()
                .absolute()
                .top(px((PILL_H - CHIP_H) / 2.))
                .left(px(stack.chip_left()))
                .h(px(CHIP_H))
                .px(px(CHIP_PAD))
                .rounded(px(PILL_RADIUS))
                .bg(color("surface-200"))
                .text_color(color("text-secondary"))
                .text_size(px(TEXT_SIZE))
                .line_height(px(CHIP_H))
                .whitespace_nowrap()
                .child(format!("+{}", stack.more)),
        );
    }
    slot
}

#[cfg(test)]
mod tests {
    use super::{
        BORDER, CHIP_GAP, MAX, MONO_ADVANCE_EM, PILL_W, STEP, Stack, TEXT_PAD, chip_width,
        text_size,
    };

    #[test]
    fn deck_keeps_distinct_methods_in_order() {
        let s = Stack::new(["GET", "GET", "POST", "GET"]);
        assert_eq!(s.deck, ["GET", "POST"]);
        assert_eq!(s.more, 0);
    }

    #[test]
    fn deck_caps_at_max_and_counts_the_rest() {
        let s = Stack::new(["POST", "GET", "PUT", "DELETE", "GET", "PATCH"]);
        assert_eq!(s.deck, ["POST", "GET", "PUT"]);
        assert_eq!(s.more, 2);
    }

    #[test]
    fn empty_stack_is_empty() {
        let s = Stack::new([]);
        assert!(s.deck.is_empty());
        assert_eq!(s.more, 0);
        assert_eq!(s.width(), 0.);
    }

    #[test]
    fn later_pills_step_right() {
        assert_eq!(Stack::pill_left(0), 0.);
        assert_eq!(Stack::pill_left(1), STEP);
        assert_eq!(Stack::pill_left(MAX - 1), STEP * 2.);
    }

    #[test]
    fn width_covers_the_deck_and_the_chip() {
        // TreeMethods.dc.html: 40px pills 12px apart, `+N` at 70px.
        assert_eq!(Stack::new(["GET"]).width(), PILL_W);
        assert_eq!(Stack::new(["GET", "POST"]).width(), 52.);
        let s = Stack::new(["GET", "POST", "PUT", "PATCH", "DELETE"]);
        assert_eq!(s.deck_width(), 64.);
        assert_eq!(s.chip_left(), 64. + CHIP_GAP);
        assert_eq!(s.width(), 70. + chip_width(2));
    }

    #[test]
    fn every_method_fits_inside_the_top_pill() {
        for method in ["GET", "POST", "PUT", "PATCH", "DELETE", "HEAD", "OPTIONS"] {
            let width = method.len() as f32 * MONO_ADVANCE_EM * text_size(method);
            assert!(
                width <= PILL_W - 2. * BORDER - TEXT_PAD,
                "{method}: {width}"
            );
        }
    }

    #[test]
    fn lower_pills_show_their_first_letter() {
        // A glyph's ink is narrower than its advance.
        let ink = 0.9 * MONO_ADVANCE_EM * text_size("GET");
        assert!(BORDER + TEXT_PAD + ink <= STEP);
    }
}
