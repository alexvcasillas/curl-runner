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

/// A piece of a diagnostic message: plain text, or a `"quoted"` token shown
/// as a code chip (without its quotes).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MessageSpan<'a> {
    Text(&'a str),
    Code(&'a str),
}

/// Splits `message` at its double quotes: `Unknown key "timout"` is the
/// text `Unknown key ` then the code `timout`. An unpaired quote stays text.
pub fn message_spans(message: &str) -> Vec<MessageSpan<'_>> {
    let mut spans = Vec::new();
    let mut rest = message;
    while let Some(open) = rest.find('"') {
        let Some(len) = rest[open + 1..].find('"') else {
            break;
        };
        if open > 0 {
            spans.push(MessageSpan::Text(&rest[..open]));
        }
        spans.push(MessageSpan::Code(&rest[open + 1..open + 1 + len]));
        rest = &rest[open + len + 2..];
    }
    if !rest.is_empty() {
        spans.push(MessageSpan::Text(rest));
    }
    spans
}

const ALLOWED: &str = " Must be: ";

/// `Invalid method "FETCH". Must be: GET, POST` gives the message before
/// the list (`Invalid method "FETCH".`) and the allowed values; `None` when
/// the message lists none.
pub fn allowed_values(message: &str) -> (&str, Option<Vec<&str>>) {
    match message.find(ALLOWED) {
        Some(at) if message[..at].ends_with('.') => {
            let values = message[at + ALLOWED.len()..]
                .split(',')
                .map(str::trim)
                .filter(|v| !v.is_empty())
                .collect::<Vec<_>>();
            if values.is_empty() {
                (message, None)
            } else {
                (&message[..at], Some(values))
            }
        }
        _ => (message, None),
    }
}

/// The inline hint after a line: the message without its allowed values
/// and final period.
pub fn ghost_text(message: &str) -> &str {
    let (head, _) = allowed_values(message);
    head.strip_suffix('.').unwrap_or(head)
}

/// Most severe first.
pub fn rank(severity: Severity) -> u8 {
    match severity {
        Severity::Error => 0,
        Severity::Warning => 1,
        Severity::Info => 2,
    }
}

/// The order the Problems panel lists `d` in, as indices: errors, then
/// warnings, then info, each in document order.
pub fn problem_order(d: &[StudioDiagnostic]) -> Vec<usize> {
    let mut order: Vec<usize> = (0..d.len()).collect();
    order.sort_by_key(|&i| (rank(d[i].severity), d[i].range.start));
    order
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
    fn message_spans_turn_quotes_into_code() {
        use MessageSpan::*;
        assert_eq!(
            message_spans(r#"Unknown key "timout" (did you mean "timeout"?)"#),
            [
                Text("Unknown key "),
                Code("timout"),
                Text(" (did you mean "),
                Code("timeout"),
                Text("?)"),
            ]
        );
        assert_eq!(message_spans(r#""a""#), [Code("a")]);
        assert_eq!(message_spans(r#"say "x"#), [Text(r#"say "x"#)]);
        assert_eq!(message_spans("\"\" then"), [Code(""), Text(" then")]);
        assert_eq!(message_spans(""), []);
    }

    #[test]
    fn allowed_values_split_off_the_list() {
        let (head, values) =
            allowed_values(r#"Invalid method "FETCH". Must be: GET, POST, PUT, DELETE"#);
        assert_eq!(head, r#"Invalid method "FETCH"."#);
        assert_eq!(values.unwrap(), ["GET", "POST", "PUT", "DELETE"]);
        assert_eq!(
            allowed_values("Must be a positive number"),
            ("Must be a positive number", None)
        );
        assert_eq!(allowed_values("Bad. Must be: "), ("Bad. Must be: ", None));
    }

    #[test]
    fn ghost_text_is_the_first_sentence() {
        assert_eq!(
            ghost_text(r#"Invalid method "FETCH". Must be: GET, POST"#),
            r#"Invalid method "FETCH""#
        );
        assert_eq!(
            ghost_text("Retry count must be a non-negative number"),
            "Retry count must be a non-negative number"
        );
    }

    #[test]
    fn problem_order_puts_errors_first() {
        let d = |start, severity| StudioDiagnostic {
            range: start..start + 1,
            severity,
            message: String::new(),
            path: String::new(),
            fix: None,
            issue: None,
        };
        let list = [
            d(30, Severity::Warning),
            d(20, Severity::Error),
            d(40, Severity::Info),
            d(10, Severity::Warning),
            d(50, Severity::Error),
        ];
        assert_eq!(problem_order(&list), [1, 4, 3, 0, 2]);
    }

    #[test]
    fn invalid_value_blocks_run() {
        let d = diagnose("request:\n  url: https://api.example.com\n  method: FETCH\n");
        assert!(!can_run(&d), "{d:?}");
    }
}
