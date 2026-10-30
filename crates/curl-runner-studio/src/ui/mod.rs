//! The studio window: title bar, collection tree, tabs, YAML editor and
//! status bar, built with GPUI-kit (macOS only).

mod app;
mod collection_settings;
mod diagnostic_marks;
mod diagnostic_popover;
mod diagnostics_view;
mod editor_pane;
mod history_view;
mod icons;
mod import_dialog;
mod inspector;
mod inspector_failed;
mod method_stack;
mod new_dialog;
mod open_shared;
mod palette;
mod problems_panel;
mod read_only;
mod result_marks;
mod run_button;
mod run_view;
mod runs;
mod settings_kit;
mod settings_view;
mod share_dialog;
mod share_sync;
mod sidebar;
mod sign_in;
mod state;
mod status_bar;
mod sync;
mod title_bar;
mod watch;
mod welcome;
mod workspace;

pub use app::run;

use crate::theme;
use gpui_kit::{App, Hsla, Rgba, Task};

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

/// Runs one [`Backend`](crate::model::backend::Backend) call on the
/// background executor, so the UI thread never waits on the network.
fn backend_call<T: Send + 'static>(
    cx: &App,
    backend: &std::sync::Arc<dyn crate::model::backend::Backend>,
    call: impl for<'b> FnOnce(
        &'b dyn crate::model::backend::Backend,
    ) -> crate::model::backend::BoxFut<'b, T>
    + Send
    + 'static,
) -> Task<Result<T, crate::model::backend::BackendError>> {
    let backend = backend.clone();
    cx.background_executor()
        .spawn(async move { call(&*backend).await })
}
