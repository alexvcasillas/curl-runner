//! The title bar's Run button: blocked while the active file has errors.

use super::color;
use super::diagnostic_popover::kbd;
use super::icons::{self, glyph};
use super::workspace::Workspace;
use crate::model::diagnostics::{StudioDiagnostic, can_run, counts};
use gpui_kit::component::tooltip::Tooltip;
use gpui_kit::prelude::*;
use gpui_kit::*;
use std::path::PathBuf;

/// Asks to run a request file. Emitted by the workspace when Run is
/// pressed; plan 2 runs it, until then it is only logged.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RunRequested {
    pub rel_path: PathBuf,
}

impl EventEmitter<RunRequested> for Workspace {}

/// `Ok` when a file with `diagnostics` can run, else why not.
pub fn run_state(diagnostics: &[StudioDiagnostic]) -> Result<(), String> {
    if can_run(diagnostics) {
        return Ok(());
    }
    Err(match counts(diagnostics).errors {
        1 => "Fix 1 error to run".to_string(),
        n => format!("Fix {n} errors to run"),
    })
}

impl Workspace {
    /// Run per the mockup: 32px, primary fill, play icon, `⌘↵` hint;
    /// greyed with the reason as tooltip while it can't run.
    pub(super) fn render_run_button(&self, cx: &mut Context<Self>) -> impl IntoElement + use<> {
        let state = match self.state.active_document() {
            Some(doc) => run_state(&doc.diagnostics),
            None => Err("Open a request to run".to_string()),
        };
        let (bg, fg, hint) = match state {
            Ok(()) => (
                color("primary"),
                color("on-primary"),
                color("on-primary").opacity(0.55),
            ),
            Err(_) => (
                color("border-200"),
                color("text-tertiary"),
                color("text-disabled"),
            ),
        };
        let button = div()
            .id("run")
            .flex()
            .flex_none()
            .items_center()
            .gap(px(8.))
            .h(px(32.))
            .px(px(12.))
            .rounded(px(6.))
            .bg(bg)
            .text_color(fg)
            .text_size(px(13.))
            .font_weight(FontWeight::MEDIUM)
            .child(glyph(icons::PLAY).size(px(12.)).text_color(fg))
            .child("Run")
            .child(kbd("⌘↵", 11., hint));
        match state {
            Ok(()) => button
                .cursor_pointer()
                .hover(|s| s.bg(color("primary-hover")))
                .on_click(cx.listener(|this, _, _, cx| this.request_run(cx))),
            Err(reason) => button
                .cursor_not_allowed()
                .tooltip(move |window, cx| Tooltip::new(reason.clone()).build(window, cx)),
        }
    }

    /// Emits [`RunRequested`] for the active file when it can run.
    pub(super) fn request_run(&mut self, cx: &mut Context<Self>) {
        let Some(ix) = self.state.active else { return };
        self.flush_diagnostics(ix, cx);
        let doc = &self.state.open[ix];
        if run_state(&doc.diagnostics).is_err() {
            return;
        }
        let rel_path = doc.rel_path.clone();
        log::info!("run requested: {}", rel_path.display());
        cx.emit(RunRequested { rel_path });
    }
}

#[cfg(test)]
mod tests {
    use super::run_state;
    use crate::model::diagnostics::StudioDiagnostic;
    use curl_runner_core::lint::Severity;

    fn diag(severity: Severity) -> StudioDiagnostic {
        StudioDiagnostic {
            range: 0..1,
            severity,
            message: "m".into(),
            path: "request.method".into(),
            fix: None,
            issue: None,
        }
    }

    #[test]
    fn run_disabled_reason_counts_errors() {
        use Severity::*;
        assert_eq!(
            run_state(&[diag(Error), diag(Warning), diag(Error)]),
            Err("Fix 2 errors to run".to_string())
        );
        assert_eq!(
            run_state(&[diag(Error), diag(Info)]),
            Err("Fix 1 error to run".to_string())
        );
        assert_eq!(run_state(&[diag(Warning)]), Ok(()));
        assert_eq!(run_state(&[]), Ok(()));
    }
}
