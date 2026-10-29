//! The title bar's Run button: blocked while the active file has errors.

use super::workspace::Workspace;
use crate::model::diagnostics::{StudioDiagnostic, can_run, counts};
use gpui_kit::component::button::{Button, ButtonVariants as _};
use gpui_kit::component::{Disableable as _, IconName, Sizable as _};
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
    pub(super) fn render_run_button(&self, cx: &mut Context<Self>) -> impl IntoElement + use<> {
        let state = match self.state.active_document() {
            Some(doc) => run_state(&doc.diagnostics),
            None => Err("Open a request to run".to_string()),
        };
        let button = Button::new("run")
            .primary()
            .small()
            .icon(IconName::Play)
            .label("Run")
            .on_click(cx.listener(|this, _, _, cx| this.request_run(cx)));
        match state {
            Ok(()) => button,
            Err(reason) => button.disabled(true).tooltip(reason),
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
