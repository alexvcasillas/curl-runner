//! Editor diagnostics: lint results mapped onto byte ranges of the buffer.

use curl_runner_core::lint::spans::{issue_range, parse_error_range};
use curl_runner_core::lint::{Issue, LintOutcome, Severity, lint_text};
use std::ops::Range;

#[derive(Debug, Clone, PartialEq)]
pub struct StudioDiagnostic {
    pub range: Range<usize>,
    pub severity: Severity,
    pub message: String,
    pub path: String,
    /// Description of the quick fix, if the issue has one. The edit itself is
    /// computed on demand (see `Document::apply_fix`).
    pub fix: Option<String>,
    /// `None` for parse errors and crashes.
    pub issue: Option<Issue>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Counts {
    pub errors: usize,
    pub warnings: usize,
}

fn whole_text_error(text: &str, message: String) -> Vec<StudioDiagnostic> {
    vec![StudioDiagnostic {
        range: parse_error_range(text),
        severity: Severity::Error,
        message,
        path: String::new(),
        fix: None,
        issue: None,
    }]
}

pub fn diagnose(text: &str) -> Vec<StudioDiagnostic> {
    match lint_text(text) {
        Err(crash) => whole_text_error(text, crash.0),
        Ok(LintOutcome::ParseError(message)) => whole_text_error(text, message),
        Ok(LintOutcome::Linted { issues, .. }) => issues
            .into_iter()
            .map(|issue| StudioDiagnostic {
                range: issue_range(text, &issue),
                severity: issue.severity,
                message: issue.message.clone(),
                path: issue.path.clone(),
                fix: issue.fix.as_ref().map(|f| f.description.clone()),
                issue: Some(issue),
            })
            .collect(),
    }
}

pub fn counts(d: &[StudioDiagnostic]) -> Counts {
    let n = |s| d.iter().filter(|x| x.severity == s).count();
    Counts {
        errors: n(Severity::Error),
        warnings: n(Severity::Warning),
    }
}

/// A document can run when no diagnostic is an error.
pub fn can_run(d: &[StudioDiagnostic]) -> bool {
    counts(d).errors == 0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn crash_becomes_single_error_on_first_line() {
        let d = diagnose("hello\n");
        assert_eq!(d.len(), 1);
        assert_eq!(d[0].severity, Severity::Error);
        assert_eq!(d[0].range, 0..5);
        assert_eq!(
            d[0].message,
            "content is not an Object. (evaluating '\"request\" in content')"
        );
        assert!(d[0].issue.is_none());
    }

    #[test]
    fn unknown_key_warning_does_not_block_run() {
        let d = diagnose("request:\n  url: https://api.example.com\n  mehtod: GET\n");
        assert!(can_run(&d), "{d:?}");
        assert_eq!(
            counts(&d),
            Counts {
                errors: 0,
                warnings: 1
            }
        );
    }

    #[test]
    fn invalid_value_blocks_run() {
        let d = diagnose("request:\n  url: https://api.example.com\n  method: FETCH\n");
        assert!(!can_run(&d), "{d:?}");
    }
}
