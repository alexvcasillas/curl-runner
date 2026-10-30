//! `curl-runner validate [files...] [--fix] [--quiet] [--strict]`
//! (`commands/validate.ts`).
//!
//! The rules live in `curl_runner_core::lint`; this module handles options,
//! file discovery, `--fix` writes and the printed report. A [`Crash`] (the
//! `TypeError`s TS throws on some shapes) ends the run with the same message
//! and exit code 1.

mod report;

use crate::colors::Color;
use crate::output::Console;
use curl_runner_core::discover::find_yaml_files;
use curl_runner_core::lint::{
    Crash, Issue, LintOutcome, ROOT_PATH, Severity, fix_document, is_valid, lint_text,
};
use report::{Counts, Paint};
use std::path::Path;

#[derive(Default)]
struct Options {
    fix: bool,
    quiet: bool,
    strict: bool,
}

fn parse_options(args: &[String]) -> Options {
    let mut o = Options::default();
    for a in args {
        match a.as_str() {
            "--fix" | "-f" => o.fix = true,
            "--quiet" | "-q" => o.quiet = true,
            "--strict" | "-s" => o.strict = true,
            _ => {}
        }
    }
    o
}

struct FileResult {
    valid: bool,
    issues: Vec<Issue>,
    fixed: Option<String>,
    /// `--fix` had fixes to apply but the root is not one mapping.
    fix_skipped: bool,
}

fn validate_file(path: &Path, fix: bool) -> Result<FileResult, Crash> {
    let outcome = match std::fs::read_to_string(path) {
        Ok(text) => lint_text(&text)?,
        Err(e) => LintOutcome::ParseError(format!("Failed to parse YAML: {e}")),
    };
    let (content, issues) = match outcome {
        LintOutcome::ParseError(message) => {
            return Ok(FileResult {
                valid: false,
                issues: vec![Issue {
                    path: ROOT_PATH.into(),
                    severity: Severity::Error,
                    message,
                    fix: None,
                }],
                fixed: None,
                fix_skipped: false,
            });
        }
        LintOutcome::Linted { content, issues } => (content, issues),
    };
    // A multi-document file parses to an array of documents; TS rewrote it
    // from that array and erased it. Only a single mapping is rewritten.
    let has_fixes = fix && issues.iter().any(|i| i.fix.is_some());
    let fix_skipped = has_fixes && !content.is_object();
    let fixed = if has_fixes && !fix_skipped {
        fix_document(&content, &issues).map(|(text, _)| text)
    } else {
        None
    };
    Ok(FileResult {
        valid: is_valid(&issues),
        issues,
        fixed,
        fix_skipped,
    })
}

/// Run `validate`; `argv[0]` is `validate`. Returns the exit code.
pub fn run(argv: &[String], color: bool, console: &mut Console) -> i32 {
    let cwd = std::env::current_dir().unwrap_or_else(|_| ".".into());
    run_in(argv, &cwd, color, console)
}

pub fn run_in(argv: &[String], cwd: &Path, color: bool, c: &mut Console) -> i32 {
    let p = Paint(color);
    if argv.iter().any(|a| a == "--help" || a == "-h") {
        c.out(&report::help(p));
        return 0;
    }
    match validate_all(argv.get(1..).unwrap_or_default(), cwd, p, c) {
        Ok(code) => code,
        Err(Crash(msg)) => {
            c.err(&p.c(&format!("✗ {msg}"), Color::Red));
            1
        }
    }
}

fn validate_all(args: &[String], cwd: &Path, p: Paint, c: &mut Console) -> Result<i32, Crash> {
    let opts = parse_options(args);
    let mut patterns: Vec<String> = args
        .iter()
        .filter(|a| !a.starts_with('-'))
        .cloned()
        .collect();
    if patterns.is_empty() {
        patterns = vec!["*.yaml".into(), "*.yml".into()];
    }

    c.out("");
    c.out(&p.c("curl-runner validate", Color::Bright));
    c.out("");

    let files = find_yaml_files(&patterns, true, cwd);
    if files.is_empty() {
        c.out(&p.c("No YAML files found", Color::Yellow));
        return Ok(0);
    }
    c.out(&format!(
        "{} {} found",
        p.c("Files:", Color::Cyan),
        files.len()
    ));
    c.out("");

    let (mut total_issues, mut total_fixed, mut invalid_files) = (0, 0, 0);
    for file in &files {
        let r = validate_file(&cwd.join(file), opts.fix)?;
        let n = Counts::of(&r.issues);
        let invalid = !r.valid || (opts.strict && n.warnings > 0);
        if invalid || n.warnings > 0 || n.infos > 0 {
            if invalid {
                invalid_files += 1;
                c.out(&format!("{} {file}", p.c("✗", Color::Red)));
            } else {
                c.out(&format!("{} {file}", p.c("✓", Color::Green)));
            }
            total_issues += r.issues.len();
            report::print_issues(c, p, &r.issues, opts.fix);
            if let Some(content) = r.fixed.as_ref().filter(|_| opts.fix) {
                std::fs::write(cwd.join(file), content).map_err(|e| Crash(e.to_string()))?;
                let count = r.issues.iter().filter(|i| i.fix.is_some()).count();
                total_fixed += count;
                report::print_fixed(c, p, count);
            }
            if opts.fix && r.fix_skipped {
                report::print_not_fixed(c, p);
            }
            report::print_counts(c, p, &n, invalid);
            c.out("");
        } else if !opts.quiet {
            c.out(&format!("{} {file}", p.c("✓", Color::Green)));
        }
    }

    c.out("");
    if invalid_files == 0 {
        c.out(&p.c("All files valid!", Color::Green));
        return Ok(0);
    }
    c.out(&format!(
        "{} {invalid_files} file(s) with {total_issues} issue(s)",
        p.c("Summary:", Color::Cyan)
    ));
    if opts.fix {
        c.out(&format!(
            "{} {total_fixed} issue(s)",
            p.c("Fixed:", Color::Green)
        ));
    }
    Ok(1)
}

#[cfg(test)]
mod tests;
