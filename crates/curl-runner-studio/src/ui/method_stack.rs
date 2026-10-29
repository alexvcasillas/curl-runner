//! `MethodStack`: the method slot of a multi-request file row. The first
//! method is a pill in front, exactly in the single-request badge slot so
//! names line up across rows; the next ones sit behind it, shifted left, so
//! only their colored edges peek out. A `+N` chip after the name counts the
//! rest.

use super::{app::mono_font, color, method_color};
use gpui_kit::prelude::*;
use gpui_kit::*;

/// Pill width and height, px. The width is the badge slot's
/// (`METHOD_BADGE_WIDTH` in `sidebar.rs`).
pub const PILL_W: f32 = 34.;
pub const PILL_H: f32 = 14.;
pub const PILL_RADIUS: f32 = 4.;
/// How far each pill behind the front one is shifted left, px.
pub const PEEK: f32 = 4.;
/// Most pills in the deck; further methods go to the `+N` chip.
pub const MAX: usize = 3;
/// The chip's minimum width, px (fits `+9`).
pub const CHIP_W: f32 = 20.;
pub const TEXT_SIZE: f32 = 9.5;
/// For methods longer than 4 characters, so they fit inside the border.
pub const TEXT_SIZE_LONG: f32 = 8.;
/// For methods longer than `DELETE`.
pub const TEXT_SIZE_LONGEST: f32 = 7.;
/// Alpha of each pill's border in its method color.
pub const BORDER_ALPHA: f32 = 0.4;
/// Room the deck takes left of the slot, px.
pub const PEEK_ROOM: f32 = PEEK * (MAX - 1) as f32;

/// What the stack shows for a file's methods.
#[derive(Debug, PartialEq, Eq)]
pub struct Stack {
    /// Distinct methods in request order, front first, at most [`MAX`].
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
        -PEEK * i as f32
    }
}

fn text_size(method: &str) -> f32 {
    match method.len() {
        0..=4 => TEXT_SIZE,
        5..=6 => TEXT_SIZE_LONG,
        _ => TEXT_SIZE_LONGEST,
    }
}

/// The deck for `stack` in a [`PILL_W`] slot; the pills behind it overflow
/// [`PEEK_ROOM`] to the left. `fill` is the row background behind it.
pub fn method_stack(stack: &Stack, fill: Hsla) -> Div {
    let pill = |i: usize, method: &str| {
        let tint = method_color(method);
        div()
            .absolute()
            .top_0()
            .left(px(Stack::pill_left(i)))
            .w(px(PILL_W))
            .h(px(PILL_H))
            .rounded(px(PILL_RADIUS))
            .border_1()
            .border_color(tint.opacity(BORDER_ALPHA))
            .bg(fill)
            .flex()
            .items_center()
            .justify_center()
            .when(i == 0, |p| {
                p.text_color(tint)
                    .text_size(px(text_size(method)))
                    .line_height(px(PILL_H - 2.))
                    .child(method.to_string())
            })
    };
    let mut slot = div()
        .relative()
        .flex_none()
        .w(px(PILL_W))
        .h(px(PILL_H))
        .font_family(mono_font())
        .font_weight(FontWeight::MEDIUM);
    // Back to front: later children paint over earlier ones.
    for (i, m) in stack.deck.iter().enumerate().rev() {
        slot = slot.child(pill(i, m));
    }
    slot
}

/// The `+N` chip for methods beyond the deck, if any.
pub fn more_chip(stack: &Stack) -> Option<Div> {
    (stack.more > 0).then(|| {
        div()
            .flex_none()
            .min_w(px(CHIP_W))
            .h(px(PILL_H))
            .px(px(3.))
            .rounded(px(PILL_RADIUS))
            .bg(color("surface-200"))
            .text_color(color("text-secondary"))
            .font_family(mono_font())
            .text_size(px(TEXT_SIZE))
            .font_weight(FontWeight::MEDIUM)
            .flex()
            .items_center()
            .justify_center()
            .line_height(px(PILL_H))
            .child(format!("+{}", stack.more))
    })
}

#[cfg(test)]
mod tests {
    use super::{PEEK, PEEK_ROOM, PILL_W, Stack, text_size};

    /// Geist Mono's advance, em.
    const MONO_ADVANCE_EM: f32 = 0.6;

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
    }

    #[test]
    fn pills_peek_left_within_the_room() {
        assert_eq!(Stack::pill_left(0), 0.);
        assert_eq!(Stack::pill_left(1), -PEEK);
        assert_eq!(-Stack::pill_left(super::MAX - 1), PEEK_ROOM);
    }

    #[test]
    fn every_method_fits_inside_the_pill_border() {
        for method in ["GET", "POST", "PUT", "PATCH", "DELETE", "HEAD", "OPTIONS"] {
            let width = method.len() as f32 * MONO_ADVANCE_EM * text_size(method);
            assert!(width <= PILL_W - 4., "{method}: {width}");
        }
    }
}
