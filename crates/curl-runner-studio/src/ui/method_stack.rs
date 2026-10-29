//! `MethodStack`: the method slot of a multi-request file row (option C,
//! "Method dots", of `TreeMethods.dc.html`). One filled dot per distinct
//! method in request order, each shifted [`STEP`] right of the previous and
//! painted on top of it. A ring in the row's background color cuts each
//! overlap cleanly. A `+N` chip after the dots counts the rest.

use super::{app::mono_font, color, method_color};
use gpui_kit::prelude::*;
use gpui_kit::*;

/// Dot diameter, px.
pub const DOT: f32 = 8.;
/// Ring width around a dot, px (drawn outside the dot).
pub const RING: f32 = 1.5;
/// How far each dot sits right of the one below it, px.
pub const STEP: f32 = 7.;
/// Most dots; further methods go to the `+N` chip.
pub const MAX: usize = 3;
/// Gap between the last dot and the `+N` chip, px.
pub const CHIP_GAP: f32 = 6.;
pub const CHIP_H: f32 = 14.;
pub const CHIP_PAD: f32 = 4.;
pub const CHIP_RADIUS: f32 = 4.;
pub const TEXT_SIZE: f32 = 9.5;
/// Geist Mono's advance, em.
const MONO_ADVANCE_EM: f32 = 0.6;

/// What the stack shows for a file's methods.
#[derive(Debug, PartialEq, Eq)]
pub struct Stack {
    /// Distinct methods in request order, bottom first, at most [`MAX`].
    pub dots: Vec<String>,
    /// Distinct methods beyond the dots.
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
            dots: distinct,
            more,
        }
    }

    /// Left edge of dot `i` relative to the slot, px.
    pub fn dot_left(i: usize) -> f32 {
        STEP * i as f32
    }

    /// Width of the dots alone, px.
    pub fn dots_width(&self) -> f32 {
        match self.dots.len() {
            0 => 0.,
            n => Self::dot_left(n - 1) + DOT,
        }
    }

    /// Left edge of the `+N` chip, px.
    pub fn chip_left(&self) -> f32 {
        self.dots_width() + CHIP_GAP
    }

    /// Width of the whole stack, `+N` chip included, px.
    pub fn width(&self) -> f32 {
        match self.more {
            0 => self.dots_width(),
            n => self.chip_left() + chip_width(n),
        }
    }
}

/// The `+N` chip's width, px.
pub fn chip_width(n: usize) -> f32 {
    let chars = format!("+{n}").len() as f32;
    chars * MONO_ADVANCE_EM * TEXT_SIZE + 2. * CHIP_PAD
}

/// The stack for `stack` in a `width`-wide slot. `row_bg` is the ring color
/// at rest; while the pointer is over the `group` row the rings take
/// `hover_bg` (when given) so they keep matching the row.
pub fn method_stack(
    stack: &Stack,
    row_bg: Hsla,
    hover_bg: Option<(SharedString, Hsla)>,
    width: f32,
) -> Div {
    let mut slot = div()
        .relative()
        .flex_none()
        .w(px(width))
        .h(px(DOT))
        .font_family(mono_font())
        .font_weight(FontWeight::MEDIUM);
    // Request order: later children paint over earlier ones.
    for (i, m) in stack.dots.iter().enumerate() {
        let ring = div()
            .absolute()
            .top(px(-RING))
            .left(px(Stack::dot_left(i) - RING))
            .size(px(DOT + 2. * RING))
            .rounded_full()
            .bg(row_bg)
            .flex()
            .items_center()
            .justify_center()
            .child(div().size(px(DOT)).rounded_full().bg(method_color(m)));
        slot = slot.child(match &hover_bg {
            Some((group, hover)) => {
                let hover = *hover;
                ring.group_hover(group.clone(), move |s| s.bg(hover))
            }
            None => ring,
        });
    }
    if stack.more > 0 {
        slot = slot.child(
            div()
                .absolute()
                .top(px((DOT - CHIP_H) / 2.))
                .left(px(stack.chip_left()))
                .h(px(CHIP_H))
                .px(px(CHIP_PAD))
                .rounded(px(CHIP_RADIUS))
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
    use super::{CHIP_GAP, DOT, MAX, STEP, Stack, chip_width};

    #[test]
    fn dots_keep_distinct_methods_in_order() {
        let s = Stack::new(["GET", "GET", "POST", "GET"]);
        assert_eq!(s.dots, ["GET", "POST"]);
        assert_eq!(s.more, 0);
    }

    #[test]
    fn dots_cap_at_max_and_count_the_rest() {
        let s = Stack::new(["POST", "GET", "PUT", "DELETE", "GET", "PATCH"]);
        assert_eq!(s.dots, ["POST", "GET", "PUT"]);
        assert_eq!(s.more, 2);
    }

    #[test]
    fn empty_stack_is_empty() {
        let s = Stack::new([]);
        assert!(s.dots.is_empty());
        assert_eq!(s.more, 0);
        assert_eq!(s.width(), 0.);
    }

    #[test]
    fn later_dots_step_right() {
        assert_eq!(Stack::dot_left(0), 0.);
        assert_eq!(Stack::dot_left(1), STEP);
        assert_eq!(Stack::dot_left(MAX - 1), STEP * 2.);
    }

    #[test]
    fn dots_overlap() {
        const { assert!(STEP < DOT) };
    }

    #[test]
    fn width_covers_the_dots_and_the_chip() {
        assert_eq!(Stack::new(["GET"]).width(), DOT);
        assert_eq!(Stack::new(["GET", "POST"]).width(), 15.);
        let s = Stack::new(["GET", "POST", "PUT", "PATCH", "DELETE"]);
        assert_eq!(s.dots_width(), 22.);
        assert_eq!(s.chip_left(), 22. + CHIP_GAP);
        assert_eq!(s.width(), 28. + chip_width(2));
    }
}
