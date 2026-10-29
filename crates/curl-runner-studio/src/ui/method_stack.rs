//! `MethodStack`: the method slot of a multi-request file row. The first
//! method is a pill in front; the next ones sit behind it, offset on x, so
//! only their colored edges peek out; a `+N` chip counts the rest.

use super::{app::mono_font, color, method_color};
use gpui_kit::prelude::*;
use gpui_kit::*;

/// Pill width and height, px.
pub const PILL_W: f32 = 40.;
pub const PILL_H: f32 = 14.;
pub const PILL_RADIUS: f32 = 4.;
/// How far each pill behind the front one is shifted right, px.
pub const PEEK: f32 = 4.;
/// Most pills in the deck; further methods go to the `+N` chip.
pub const MAX: usize = 3;
/// Gap between the deck and the `+N` chip, px.
pub const CHIP_GAP: f32 = 6.;
/// The chip's width, px (fits `+9`; wider counts still center).
pub const CHIP_W: f32 = 20.;
pub const TEXT_SIZE: f32 = 9.5;
/// Alpha of each pill's border in its method color.
pub const BORDER_ALPHA: f32 = 0.4;
/// The slot's fixed width, so names after it line up from row to row.
pub const SLOT_W: f32 = PILL_W + PEEK * (MAX - 1) as f32 + CHIP_GAP + CHIP_W;

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

    /// Left edge of deck pill `i`, px.
    pub fn pill_left(i: usize) -> f32 {
        PEEK * i as f32
    }

    /// Left edge of the `+N` chip, px, when there is one.
    pub fn chip_left(&self) -> Option<f32> {
        (self.more > 0)
            .then(|| Self::pill_left(self.deck.len().saturating_sub(1)) + PILL_W + CHIP_GAP)
    }
}

/// The stack for `methods`; `fill` is the row background behind it.
pub fn method_stack<'a>(methods: impl IntoIterator<Item = &'a str>, fill: Hsla) -> Div {
    let stack = Stack::new(methods);
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
                    .line_height(px(PILL_H - 2.))
                    .child(method.to_string())
            })
    };
    let mut slot = div()
        .relative()
        .flex_none()
        .w(px(SLOT_W))
        .h(px(PILL_H))
        .font_family(mono_font())
        .text_size(px(TEXT_SIZE))
        .font_weight(FontWeight::MEDIUM);
    // Back to front: later children paint over earlier ones.
    for (i, m) in stack.deck.iter().enumerate().rev() {
        slot = slot.child(pill(i, m));
    }
    if let Some(left) = stack.chip_left() {
        slot = slot.child(
            div()
                .absolute()
                .top_0()
                .left(px(left))
                .min_w(px(CHIP_W))
                .h(px(PILL_H))
                .px(px(3.))
                .rounded(px(PILL_RADIUS))
                .bg(color("surface-200"))
                .text_color(color("text-secondary"))
                .flex()
                .items_center()
                .justify_center()
                .line_height(px(PILL_H))
                .child(format!("+{}", stack.more)),
        );
    }
    slot
}

#[cfg(test)]
mod tests {
    use super::{CHIP_GAP, CHIP_W, PEEK, PILL_W, SLOT_W, Stack};

    #[test]
    fn deck_keeps_distinct_methods_in_order() {
        let s = Stack::new(["GET", "GET", "POST", "GET"]);
        assert_eq!(s.deck, ["GET", "POST"]);
        assert_eq!(s.more, 0);
        assert_eq!(s.chip_left(), None);
    }

    #[test]
    fn deck_caps_at_max_and_counts_the_rest() {
        let s = Stack::new(["POST", "GET", "PUT", "DELETE", "GET", "PATCH"]);
        assert_eq!(s.deck, ["POST", "GET", "PUT"]);
        assert_eq!(s.more, 2);
        assert_eq!(s.chip_left(), Some(PEEK * 2. + PILL_W + CHIP_GAP));
    }

    #[test]
    fn empty_stack_is_empty() {
        let s = Stack::new([]);
        assert!(s.deck.is_empty());
        assert_eq!(s.more, 0);
    }

    #[test]
    fn slot_fits_full_deck_and_chip() {
        let s = Stack::new(["A", "B", "C", "D"]);
        assert!(s.chip_left().unwrap() + CHIP_W <= SLOT_W);
        assert_eq!(Stack::pill_left(1), PEEK);
    }
}
