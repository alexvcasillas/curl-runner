//! Response diffing in the CLI: the post-run `handleDiffMode` and the
//! `curl-runner diff <label1> <label2> [file]` subcommand (`cli.ts`).

use crate::discover::find_yaml_files;
use crate::output::diff::DiffFormatter;
use crate::output::{Console, Logger, OutCfg};
use curl_runner_core::config::CliOptions;
use curl_runner_core::diff::{BaselineManager, DiffOrchestrator};
use curl_runner_core::model::{DiffConfig, DiffSetting, ExecutionResult, GlobalDiffConfig};
use curl_runner_core::security::redact::Redactor;
use std::path::Path;
use std::sync::{Arc, Mutex};

/// JS truthiness of an optional string flag.
fn truthy(s: &Option<String>) -> Option<&str> {
    s.as_deref().filter(|s| !s.is_empty())
}

/// Whether the run should go through [`handle_diff_mode`]
/// (`diff.enabled || diff.save || diff.compareWith`).
pub fn wanted(diff: Option<&GlobalDiffConfig>) -> bool {
    diff.is_some_and(|d| {
        d.enabled == Some(true) || d.save == Some(true) || truthy(&d.compare_with).is_some()
    })
}

/// `handleDiffMode`: save and/or compare the run against stored baselines of
/// the first YAML file. `Err` carries a message TS would have thrown (a write
/// failure); the caller logs it and exits 1 like TS's top-level catch.
#[allow(clippy::too_many_arguments)]
pub fn handle_diff_mode(
    yaml_path: &str,
    results: &[ExecutionResult],
    dc: &GlobalDiffConfig,
    cwd: &Path,
    color: bool,
    log: &Logger,
    console: &mut Console,
    redactor: &Arc<Mutex<Redactor>>,
) -> Result<(), String> {
    let redact = |s: &str| match redactor.lock() {
        Ok(r) => r.redact_secrets(s),
        Err(_) => s.to_string(),
    };
    let orch = DiffOrchestrator::new(Some(dc), cwd);
    let fmt = DiffFormatter::new(
        truthy(&dc.output_format).unwrap_or("terminal"),
        color,
        &redact,
    );
    let config =
        BaselineManager::merge_config(Some(dc), Some(&DiffSetting::Bool(true))).unwrap_or_default();
    let current = truthy(&dc.label).unwrap_or("current");
    let save = dc.save == Some(true);
    let write = |label: &str| -> Result<(), String> {
        orch.save_baseline(yaml_path, label, results, &config)
            .map_err(|e| e.to_string())
    };

    if save {
        write(current)?;
        log.log_info(console, &format!("Baseline saved as '{current}'"));
    }

    if let Some(compare) = truthy(&dc.compare_with) {
        let summary = orch.compare_with_baseline(yaml_path, results, current, compare, &config);
        // Only a baseline file that does not exist is created. An existing one
        // is never replaced: not when no request name matches (renamed
        // requests) nor when no request succeeded (TS did both), and not when
        // it cannot be read.
        match orch.manager().read_file(yaml_path, compare) {
            Ok(None) => {
                log.log_warning(
                    console,
                    &format!("No baseline '{compare}' found. Saving current run as baseline."),
                );
                return write(compare);
            }
            Err(e) => {
                log.log_error(console, &e.to_string());
                return Ok(());
            }
            Ok(Some(_)) => {}
        }
        console.out(&fmt.format_summary(&summary, compare, current));
        if save {
            write(current)?;
        }
    } else if dc.enabled == Some(true) && !save {
        let labels = orch.list_labels(yaml_path);
        match labels.len() {
            0 => {
                write("baseline")?;
                log.log_info(
                    console,
                    "No baselines found. Saved current run as 'baseline'.",
                );
            }
            1 => {
                let summary =
                    orch.compare_with_baseline(yaml_path, results, current, &labels[0], &config);
                console.out(&fmt.format_summary(&summary, &labels[0], current));
            }
            _ => {
                log.log_info(
                    console,
                    &format!("Available baselines: {}", labels.join(", ")),
                );
                log.log_info(
                    console,
                    "Use --diff-compare <label> to compare against a specific baseline.",
                );
            }
        }
    }
    Ok(())
}

/// `curl-runner diff <label1> <label2> [file]`: compares two stored baselines
/// without running requests. `rest` are the raw args after `diff` (the third
/// one is the file even if it is a flag, as in TS). Exit code `changed > 0`.
pub fn run_subcommand(
    rest: &[String],
    cli: &CliOptions,
    cwd: &Path,
    color: bool,
    console: &mut Console,
) -> i32 {
    let log = Logger::new(OutCfg::from_config(None), color, None);
    let (label1, label2) = (&rest[0], &rest[1]);
    let yaml = match rest.get(2) {
        Some(f) => f.clone(),
        None => {
            let files = find_yaml_files(&[], cli.all == Some(true), cwd);
            match files.len() {
                0 => {
                    log.log_error(
                        console,
                        "No YAML files found. Specify a file: curl-runner diff <label1> <label2> <file.yaml>",
                    );
                    return 1;
                }
                1 => files[0].clone(),
                _ => {
                    log.log_error(
                        console,
                        "Multiple YAML files found. Specify which file to use.",
                    );
                    return 1;
                }
            }
        }
    };
    let dc = GlobalDiffConfig {
        dir: Some(truthy(&cli.diff_dir).unwrap_or("__baselines__").to_string()),
        output_format: Some(truthy(&cli.diff_output).unwrap_or("terminal").to_string()),
        ..Default::default()
    };
    let orch = DiffOrchestrator::new(Some(&dc), cwd);
    let ident = |s: &str| s.to_string();
    let fmt = DiffFormatter::new(
        dc.output_format.as_deref().unwrap_or("terminal"),
        color,
        &ident,
    );
    let config = DiffConfig {
        exclude: Some(Vec::new()),
        match_rules: Some(Default::default()),
        ..Default::default()
    };
    match orch.compare_two_baselines(&yaml, label1, label2, &config) {
        Ok(summary) => {
            console.out(&fmt.format_summary(&summary, label1, label2));
            console.flush();
            i32::from(summary.changed > 0)
        }
        Err(e) => {
            log.log_error(console, &e);
            1
        }
    }
}
