//! `curl-runner validate [files...] [--fix] [--quiet] [--strict]`
//! (`commands/validate.ts`).
//!
//! Rules are split by group (`structure`, `global`, `request`, `url`,
//! `security`, `conditions`); [`PASSES`] is the registry of document-level
//! passes, run in TS order. Like TS, rules judge the raw parsed YAML (not the
//! typed model), and the `TypeError`s TS throws on some shapes end the run
//! with the same message and exit code 1.

mod conditions;
mod fix;
mod global;
mod issues;
mod report;
mod request;
mod security;
mod structure;
mod url;

use crate::colors::Color;
use crate::output::Console;
use curl_runner_core::discover::find_yaml_files;
use curl_runner_core::model::Json;
use curl_runner_core::parser::{self, ParseError};
use issues::{Crash, Ctx, Issue, Res, Severity};
use report::{Counts, Paint};
use std::path::Path;

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

fn validate_file(path: &Path, fix: bool) -> Result<FileResult, Crash> {
    let mut cx = Ctx::default();
    let parsed = std::fs::read_to_string(path)
        .map_err(|e| e.to_string())
        .and_then(|s| parser::parse_raw_stream(&s).map_err(|e| parse_error_message(&e)));
    let content = match parsed {
        Ok(c) => c,
        Err(msg) => {
            cx.add("", Severity::Error, format!("Failed to parse YAML: {msg}"));
            return Ok(FileResult {
                valid: false,
                issues: cx.issues,
                fixed: None,
                fix_skipped: false,
            });
        }
    };
    for (_, pass) in PASSES {
        pass(&content, &mut cx)?;
    }
    // A multi-document file parses to an array of documents; TS rewrote it
    // from that array and erased it. Only a single mapping is rewritten.
    let has_fixes = fix && cx.issues.iter().any(|i| i.fix.is_some());
    let fix_skipped = has_fixes && !content.is_object();
    let fixed = if has_fixes && !fix_skipped {
        fix::fix_document(&content, &cx.issues).map(|(text, _)| text)
    } else {
        None
    };
    Ok(FileResult {
        valid: !cx.issues.iter().any(|i| i.severity == Severity::Error),
        issues: cx.issues,
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
