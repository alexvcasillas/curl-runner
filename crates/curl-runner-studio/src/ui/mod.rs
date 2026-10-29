//! The studio window: title bar, collection tree, tabs, YAML editor and
//! status bar, built with GPUI-kit (macOS only).

mod app;
mod diagnostic_marks;
mod diagnostic_popover;
mod diagnostics_view;
mod editor_pane;
mod icons;
mod method_stack;
mod new_dialog;
mod palette;
mod problems_panel;
mod run_button;
mod sidebar;
mod state;
mod status_bar;
mod title_bar;
mod watch;
mod welcome;
mod workspace;

pub use app::run;

use crate::theme;
use gpui_kit::{Hsla, Rgba};

/// A dark design token as a GPUI color.
fn color(name: &str) -> Hsla {
    to_hsla(theme::color(name))
}

fn method_color(method: &str) -> Hsla {
    to_hsla(theme::method_color(method))
}

fn to_hsla(c: theme::Rgba) -> Hsla {
    Rgba {
        r: c.r,
        g: c.g,
        b: c.b,
        a: c.a,
    }
    .into()
}
