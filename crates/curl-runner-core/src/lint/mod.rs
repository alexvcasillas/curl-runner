//! The `curl-runner validate` rules, as a library: lint a YAML document and
//! get structured [`Issue`]s (with optional [`Fix`]es) back.
//!
//! Rules are split by group (`structure`, `global`, `request`, `url`,
//! `security`, `conditions`); [`PASSES`] is the registry of document-level
//! passes, run in TS order. Like TS, rules judge the raw parsed YAML (not the
//! typed model), and the `TypeError`s TS throws on some shapes surface as a
//! [`Crash`] carrying the same message.

mod conditions;
mod fix;
mod global;
mod issues;
mod request;
mod security;
pub mod spans;
mod structure;
mod url;

use crate::model::Json;
use crate::parser::{self, ParseError};
use issues::{Ctx, Res};

pub use fix::fix_document;
pub use issues::{Crash, Fix, FixValue, Issue, Severity};

type Pass = fn(&Json, &mut Ctx) -> Res;

/// Document-level passes, in the order TS `validateFile` runs them.
const PASSES: &[(&str, Pass)] = &[
    ("variables", structure::collect_defined_variables),
    ("structure", structure::validate_structure),
    ("global", structure::validate_global_pass),
    ("collection", structure::validate_collection_pass),
    ("request", structure::validate_request_pass),
    ("requests", structure::validate_requests_pass),
    ("duplicate names", structure::check_duplicate_names),
];

/// Result of [`lint_text`].
#[derive(Debug, Clone, PartialEq)]
pub enum LintOutcome {
    /// The text is not valid YAML; carries the full issue message, e.g.
    /// `Failed to parse YAML: YAML Parse error: Unexpected token`.
    ParseError(String),
    Linted {
        content: Json,
        issues: Vec<Issue>,
    },
}

/// Bun's YAML errors carry no detail (`Unexpected token`/`Unexpected EOF`).
fn parse_error_message(e: &ParseError) -> String {
    match e {
        ParseError::Yaml(y) => {
            let msg = y.to_string();
            if msg.contains("end of stream") {
                "YAML Parse error: Unexpected EOF".into()
            } else {
                "YAML Parse error: Unexpected token".into()
            }
        }
        other => other.to_string(),
    }
}

/// Run every pass over an already-parsed document, in order.
pub fn lint_document(content: &Json) -> Result<Vec<Issue>, Crash> {
    let mut cx = Ctx::default();
    for (_, pass) in PASSES {
        pass(content, &mut cx)?;
    }
    Ok(cx.issues)
}

/// Parse `text` and lint it. A parse failure is an outcome, not a crash.
pub fn lint_text(text: &str) -> Result<LintOutcome, Crash> {
    match parser::parse_raw_stream(text) {
        Ok(content) => {
            let issues = lint_document(&content)?;
            Ok(LintOutcome::Linted { content, issues })
        }
        Err(e) => Ok(LintOutcome::ParseError(format!(
            "Failed to parse YAML: {}",
            parse_error_message(&e)
        ))),
    }
}

/// True when no issue is an error (warnings and infos are fine).
pub fn is_valid(issues: &[Issue]) -> bool {
    !issues.iter().any(|i| i.severity == Severity::Error)
}

#[cfg(test)]
mod tests;
