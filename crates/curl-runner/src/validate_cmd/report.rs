//! Console output of `validate` (the reporting half of TS `run`, and
//! `showValidateHelp`).

use crate::colors::{Color, colorize};
use crate::output::Console;
use curl_runner_core::lint::{Issue, Severity};

/// TS `color()` (always on in TS; off here when `NO_COLOR` is set).
#[derive(Clone, Copy)]
pub struct Paint(pub bool);

impl Paint {
    pub fn c(self, text: &str, color: Color) -> String {
        if self.0 {
            colorize(text, color)
        } else {
            text.to_string()
        }
    }
}

pub struct Counts {
    pub errors: usize,
    pub warnings: usize,
    pub infos: usize,
}

impl Counts {
    pub fn of(issues: &[Issue]) -> Self {
        let n = |s| issues.iter().filter(|i| i.severity == s).count();
        Counts {
            errors: n(Severity::Error),
            warnings: n(Severity::Warning),
            infos: n(Severity::Info),
        }
    }
}

/// The issue lines of one file (icon, path, message, and the fix hint
/// unless fixing).
pub fn print_issues(c: &mut Console, p: Paint, issues: &[Issue], fixing: bool) {
    for issue in issues {
        let color = match issue.severity {
            Severity::Error => Color::Red,
            Severity::Warning => Color::Yellow,
            Severity::Info => Color::Blue,
        };
        c.out(&format!(
            "  {} {}: {}",
            p.c("●", color),
            issue.path,
            issue.message
        ));
        if let Some(fix) = issue.fix.as_ref().filter(|_| !fixing) {
            c.out(&format!(
                "    {} {}",
                p.c("fix:", Color::Cyan),
                fix.description
            ));
        }
    }
}

pub fn print_fixed(c: &mut Console, p: Paint, count: usize) {
    c.out(&format!(
        "  {} Fixed {count} issue(s)",
        p.c("✓", Color::Green)
    ));
}

/// Rust only: `--fix` leaves files that are not one mapping document alone.
pub fn print_not_fixed(c: &mut Console, p: Paint) {
    c.out(&format!(
        "  {} Not fixed: --fix only rewrites a file with a single mapping document",
        p.c("⚠", Color::Yellow)
    ));
}

/// `  → 1 error(s), 2 warning(s), 3 info` (errors only for invalid files).
pub fn print_counts(c: &mut Console, p: Paint, n: &Counts, with_errors: bool) {
    let mut parts = Vec::new();
    if with_errors && n.errors > 0 {
        parts.push(format!("{} error(s)", n.errors));
    }
    if n.warnings > 0 {
        parts.push(format!("{} warning(s)", n.warnings));
    }
    if n.infos > 0 {
        parts.push(format!("{} info", n.infos));
    }
    if !parts.is_empty() {
        c.out(&format!("  {} {}", p.c("→", Color::Dim), parts.join(", ")));
    }
}

pub fn help(p: Paint) -> String {
    let y = |s: &str| p.c(s, Color::Yellow);
    let cy = |s: &str| p.c(s, Color::Cyan);
    format!(
        "
{title}

Validate YAML files against curl-runner schema and curl options.
Discovers issues and proposes fixes, or auto-fixes with --fix.

{usage}
  curl-runner validate [files...] [options]

{options}
  -f, --fix       Auto-fix issues where possible
  -q, --quiet     Only show files with issues
  -s, --strict    Treat warnings as errors
  -h, --help      Show this help message

{arguments}
  files           File paths or glob patterns (default: *.yaml, *.yml)

{examples}
  curl-runner validate                    # Validate all YAML in current dir
  curl-runner validate api.yaml           # Validate single file
  curl-runner validate \"tests/**/*.yaml\"  # Validate with glob pattern
  curl-runner validate --fix              # Validate and auto-fix issues
  curl-runner validate -fq                # Fix quietly (only show errors)
  curl-runner validate --strict           # Fail on warnings too

{categories}

  {structure}
    • YAML syntax and parsing
    • Required fields (url, auth.type, etc.)
    • Unknown/misspelled keys with suggestions
    • Duplicate request names

  {urls}
    • Valid URL syntax
    • Common typos (htpp, localhsot, etc.)
    • Double slashes in paths
    • Missing protocol (http/https)
    • Valid port numbers (1-65535)
    • Spaces in URLs

  {http}
    • Valid methods (GET, POST, PUT, DELETE, PATCH, HEAD, OPTIONS)
    • Method casing (lowercase → UPPERCASE)
    • Headers casing (content-type → Content-Type)
    • Content-Type consistency with body
    • Duplicate headers

  {auth}
    • Valid auth types (basic, bearer)
    • Required auth fields (username, password, token)
    • Hardcoded credentials warning
    • Insecure mode warnings
    • HTTP vs HTTPS warnings

  {configuration}
    • Boolean string values (\"true\" → true)
    • Numeric string values
    • Valid execution modes
    • Valid output formats
    • SSL certificate consistency

  {timeouts}
    • Positive timeout values
    • Reasonable timeout ranges
    • Retry count limits
    • Non-idempotent retry warnings

  {conditions}
    • When condition operators
    • Store path references
    • Status code ranges (100-599)
    • Response time format

{autofix}
  • Lowercase HTTP methods → UPPERCASE
  • Missing https:// prefix on URLs
  • URL typos (htpp → http, localhsot → localhost)
  • Double slashes in URL paths
  • Spaces in URLs (→ %20)
  • Header casing (content-type → Content-Type)
  • Boolean strings (\"true\" → true)
  • Numeric strings (timeout: \"5000\" → 5000)
  • Status code strings (\"200\" → 200)
  • Execution mode typos
  • Unknown keys (with suggestions)
",
        title = p.c("curl-runner validate", Color::Bright),
        usage = y("USAGE:"),
        options = y("OPTIONS:"),
        arguments = y("ARGUMENTS:"),
        examples = y("EXAMPLES:"),
        categories = y("VALIDATION CATEGORIES:"),
        structure = cy("Structure"),
        urls = cy("URLs"),
        http = cy("HTTP"),
        auth = cy("Auth & Security"),
        configuration = cy("Configuration"),
        timeouts = cy("Timeouts & Retries"),
        conditions = cy("Conditions & Expectations"),
        autofix = y("AUTO-FIXABLE ISSUES:"),
    )
}
