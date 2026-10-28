//! Hand-written CLI argument parser, a faithful port of `core/config/cli-parser.ts`.
//!
//! Quirks preserved from the TS parser (existing CI invocations depend on them):
//! - Unknown flags never error. An unknown `--flag` swallows the following
//!   argument when that argument does not start with `-`.
//! - `--flag=value` is NOT supported (the whole `flag=value` is an unknown key).
//! - A value never starts with `-`, so `--retries -1` leaves `--retries`
//!   without a value and `-1` is parsed as short flags.
//! - Numbers use JS `parseInt`/`parseFloat` prefix semantics (`30ms` is 30).
//!   Where TS stores `NaN` (or an overflowing number) the Rust field stays unset.
//! - Short `-c` is accepted and ignored; `--continue-on-error` is the real flag.
//! - Invalid enum values (`--output-format xml`) are consumed and ignored.

pub use curl_runner_core::config::CliOptions;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Command {
    Run,
    Validate,
    Convert,
    Upgrade,
    /// `init` and `edit` (see `ParsedArgs::rest`, `parse_init_args`, `parse_edit_args`).
    Wizard,
    /// `diff <label1> <label2> [file]` (needs 3+ args, else `diff` is a file).
    Diff,
    Help,
    Version,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ParsedArgs {
    pub command: Command,
    pub options: CliOptions,
    /// All raw args when the command is a subcommand (`rest[0]` is the subcommand
    /// name); for `Diff`, the args after `diff`. Empty otherwise.
    pub rest: Vec<String>,
}

/// The TS parser never fails; this exists for API stability and is currently
/// never constructed.
#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum ArgsError {
    #[error("{0}")]
    Invalid(String),
}

/// Subcommand detection (`detectSubcommand`); only looks at `args[0]`.
fn detect_subcommand(args: &[String]) -> Option<Command> {
    match args.first().map(String::as_str) {
        Some("upgrade") => Some(Command::Upgrade),
        Some("validate") => Some(Command::Validate),
        Some("convert") => Some(Command::Convert),
        Some("diff") if args.len() >= 3 => Some(Command::Diff),
        Some("init") | Some("edit") => Some(Command::Wizard),
        _ => None,
    }
}

/// Mirrors `resolveConfig`'s ordering: subcommands (except `diff`) bypass flag
/// parsing; then help beats version; then `diff`; otherwise a normal run.
pub fn parse(argv: &[String]) -> Result<ParsedArgs, ArgsError> {
    let sub = detect_subcommand(argv);
    if let Some(c) = sub.filter(|c| *c != Command::Diff) {
        return Ok(ParsedArgs {
            command: c,
            options: CliOptions::default(),
            rest: argv.to_vec(),
        });
    }

    let options = parse_cli_args(argv);
    let command = if options.help == Some(true) {
        Command::Help
    } else if options.version == Some(true) {
        Command::Version
    } else if sub == Some(Command::Diff) {
        return Ok(ParsedArgs {
            command: Command::Diff,
            rest: argv[1..].to_vec(),
            options,
        });
    } else {
        Command::Run
    };
    Ok(ParsedArgs {
        command,
        options,
        rest: Vec::new(),
    })
}

/// JS `Number.parseInt(s, 10)` restricted to non-negative results that fit u64.
/// `None` for NaN, negative values and overflow.
fn js_parse_uint(s: &str) -> Option<u64> {
    let s = s.trim_start();
    let s = s.strip_prefix('+').unwrap_or(s);
    let end = s.find(|c: char| !c.is_ascii_digit()).unwrap_or(s.len());
    s[..end].parse().ok()
}

/// JS `Number.parseFloat`: longest valid decimal prefix.
fn js_parse_float(s: &str) -> Option<f64> {
    let s = s.trim_start();
    let b = s.as_bytes();
    let mut i = 0;
    if i < b.len() && (b[i] == b'+' || b[i] == b'-') {
        i += 1;
    }
    let digits_start = i;
    while i < b.len() && b[i].is_ascii_digit() {
        i += 1;
    }
    let mut mantissa_digits = i - digits_start;
    if i < b.len() && b[i] == b'.' {
        let mut j = i + 1;
        while j < b.len() && b[j].is_ascii_digit() {
            j += 1;
        }
        mantissa_digits += j - i - 1;
        if mantissa_digits > 0 {
            i = j;
        }
    }
    if mantissa_digits == 0 {
        return None;
    }
    if i < b.len() && (b[i] == b'e' || b[i] == b'E') {
        let mut j = i + 1;
        if j < b.len() && (b[j] == b'+' || b[j] == b'-') {
            j += 1;
        }
        let exp_start = j;
        while j < b.len() && b[j].is_ascii_digit() {
            j += 1;
        }
        if j > exp_start {
            i = j;
        }
    }
    s[..i].parse().ok()
}

/// Port of `parseCliArgs`.
pub fn parse_cli_args(args: &[String]) -> CliOptions {
    let mut o = CliOptions::default();
    let mut i = 0;
    while i < args.len() {
        let arg = &args[i];
        let next = args.get(i + 1).map(String::as_str);
        let has_next = next.is_some_and(|n| !n.starts_with('-'));
        if let Some(key) = arg.strip_prefix("--") {
            i += parse_long(key, next, has_next, &mut o);
        } else if let Some(flags) = arg.strip_prefix('-') {
            i += parse_short(flags, next, has_next, &mut o);
        } else {
            o.files.push(arg.clone());
        }
        i += 1;
    }
    o
}

fn parse_long(key: &str, next: Option<&str>, has_next: bool, o: &mut CliOptions) -> usize {
    let t = Some(true);
    match key {
        "help" => o.help = t,
        "version" => o.version = t,
        "all" => o.all = t,
        "dry-run" => o.dry_run = t,
        "http2" => o.http2 = t,
        "connection-pool" => o.connection_pool = t,
        "quiet" => o.quiet = t,
        "show-headers" => o.show_headers = t,
        "show-body" => o.show_body = t,
        "show-metrics" => o.show_metrics = t,
        "no-retry" => o.no_retry = t,
        "watch" => o.watch = t,
        "watch-clear" => o.watch_clear = t,
        "no-watch-clear" => o.watch_clear = Some(false),
        "profile-histogram" => o.profile_histogram = t,
        "snapshot" => o.snapshot = t,
        "update-snapshots" => o.snapshot_update = Some("all".into()),
        "update-failing" => o.snapshot_update = Some("failing".into()),
        "ci-snapshot" => o.snapshot_ci = t,
        "diff" => o.diff = t,
        "diff-save" => o.diff_save = t,
        "strict-exit" => o.strict_exit = t,
        "no-redact" => o.no_redact = t,
        "allow-path" => o.allow_path = t,
        _ => return parse_long_value(key, next, has_next, o),
    }
    0
}

fn parse_long_value(key: &str, next: Option<&str>, has_next: bool, o: &mut CliOptions) -> usize {
    let (true, Some(v)) = (has_next, next) else {
        // Boolean-like flags usable without a value.
        match key {
            "verbose" => o.verbose = Some(true),
            "continue-on-error" => o.continue_on_error = Some(true),
            _ => {}
        }
        return 0;
    };
    let s = || Some(v.to_string());
    let pick = |allowed: &[&str]| allowed.contains(&v).then(|| v.to_string());
    match key {
        "env" => o.env = s(),
        "execution" => o.execution = pick(&["sequential", "parallel"]).or(o.execution.take()),
        "max-concurrent" => {
            if let Some(n) = js_parse_uint(v).filter(|n| *n > 0) {
                o.max_concurrency = Some(n);
            }
        }
        "max-streams" => {
            if let Some(n) = js_parse_uint(v).filter(|n| *n > 0) {
                o.max_streams = Some(n);
            }
        }
        "keepalive-time" => o.keepalive_time = js_parse_uint(v),
        "connect-timeout" => o.connect_timeout = js_parse_uint(v),
        "verbose" => o.verbose = Some(v == "true"),
        "output" => o.output = s(),
        "output-format" => {
            o.output_format = pick(&["json", "pretty", "raw"]).or(o.output_format.take())
        }
        "pretty-level" => {
            o.pretty_level = pick(&["minimal", "standard", "detailed"]).or(o.pretty_level.take())
        }
        "timeout" => o.timeout = js_parse_uint(v),
        "retries" => o.retries = js_parse_uint(v),
        "retry-delay" => o.retry_delay = js_parse_uint(v),
        "continue-on-error" => o.continue_on_error = Some(v == "true"),
        "watch-debounce" => o.watch_debounce = js_parse_uint(v),
        "profile" => o.profile = js_parse_uint(v),
        "profile-warmup" => o.profile_warmup = js_parse_uint(v),
        "profile-concurrency" => o.profile_concurrency = js_parse_uint(v),
        "profile-export" => o.profile_export = s(),
        "snapshot-dir" => o.snapshot_dir = s(),
        "diff-label" => o.diff_label = s(),
        "diff-compare" => o.diff_compare = s(),
        "diff-dir" => o.diff_dir = s(),
        "diff-output" => {
            o.diff_output = pick(&["terminal", "json", "markdown"]).or(o.diff_output.take())
        }
        "fail-on" => o.fail_on = js_parse_uint(v),
        "fail-on-percentage" => {
            if let Some(p) = js_parse_float(v).filter(|p| (0.0..=100.0).contains(p)) {
                o.fail_on_percentage = Some(p);
            }
        }
        "allow-protocol" => o
            .allow_protocols
            .get_or_insert_with(Vec::new)
            .push(v.to_lowercase()),
        // Unknown flag with a value: the value is skipped.
        _ => {}
    }
    1
}

fn parse_short(flags: &str, next: Option<&str>, has_next: bool, o: &mut CliOptions) -> usize {
    let mut consumed = 0;
    let value = if has_next { next } else { None };
    for f in flags.chars() {
        match f {
            'h' => o.help = Some(true),
            'v' => o.verbose = Some(true),
            'p' => o.execution = Some("parallel".into()),
            'c' => o.continue_on_error = Some(true),
            'q' => o.quiet = Some(true),
            'w' => o.watch = Some(true),
            's' => o.snapshot = Some(true),
            'u' => o.snapshot_update = Some("all".into()),
            'd' => o.diff = Some(true),
            'n' => o.dry_run = Some(true),
            'o' => {
                if let Some(v) = value {
                    o.output = Some(v.to_string());
                    consumed = 1;
                }
            }
            'P' => {
                if let Some(v) = value {
                    o.profile = js_parse_uint(v);
                    consumed = 1;
                }
            }
            'e' => {
                if let Some(v) = value {
                    o.env = Some(v.to_string());
                    consumed = 1;
                }
            }
            // `-c` (and any unknown short flag) is accepted and ignored.
            _ => {}
        }
    }
    consumed
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct InitOptions {
    pub wizard: bool,
    pub url: Option<String>,
    pub output_path: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct EditOptions {
    pub file: String,
    pub output_path: Option<String>,
}

/// Port of `parseInitArgs`; `args[0]` is the subcommand name.
pub fn parse_init_args(args: &[String]) -> InitOptions {
    let mut o = InitOptions::default();
    let mut i = 1;
    while i < args.len() {
        let a = args[i].as_str();
        let next = args
            .get(i + 1)
            .filter(|n| !n.is_empty() && !n.starts_with('-'));
        if a == "--wizard" || a == "-w" {
            o.wizard = true;
        } else if a == "-o" || a == "--output" {
            if let Some(n) = next {
                o.output_path = Some(n.clone());
                i += 1;
            }
        } else if !a.starts_with('-') && o.url.is_none() {
            o.url = Some(a.to_string());
        }
        i += 1;
    }
    o
}

/// Port of `parseEditArgs`; `args[0]` is the subcommand name.
pub fn parse_edit_args(args: &[String]) -> EditOptions {
    let mut o = EditOptions::default();
    let mut i = 1;
    while i < args.len() {
        let a = args[i].as_str();
        let next = args
            .get(i + 1)
            .filter(|n| !n.is_empty() && !n.starts_with('-'));
        if a == "-o" || a == "--output" {
            if let Some(n) = next {
                o.output_path = Some(n.clone());
                i += 1;
            }
        } else if !a.starts_with('-') && o.file.is_empty() {
            o.file = a.to_string();
        }
        i += 1;
    }
    o
}

#[cfg(test)]
mod tests {
    use super::*;

    fn p(a: &[&str]) -> CliOptions {
        parse_cli_args(&a.iter().map(|s| s.to_string()).collect::<Vec<_>>())
    }
    fn pa(a: &[&str]) -> ParsedArgs {
        parse(&a.iter().map(|s| s.to_string()).collect::<Vec<_>>()).unwrap()
    }
    fn sv(a: &[&str]) -> Vec<String> {
        a.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn files_and_empty() {
        assert!(p(&[]).files.is_empty());
        assert_eq!(
            p(&["api.yaml", "test.yaml"]).files,
            sv(&["api.yaml", "test.yaml"])
        );
    }

    #[test]
    fn boolean_flags() {
        let t = Some(true);
        assert_eq!(p(&["--help"]).help, t);
        assert_eq!(p(&["--version"]).version, t);
        assert_eq!(p(&["--all"]).all, t);
        assert_eq!(p(&["--dry-run"]).dry_run, t);
        assert_eq!(p(&["--http2"]).http2, t);
        assert_eq!(p(&["--connection-pool"]).connection_pool, t);
        assert_eq!(p(&["--quiet"]).quiet, t);
        assert_eq!(p(&["--show-headers"]).show_headers, t);
        assert_eq!(p(&["--show-body"]).show_body, t);
        assert_eq!(p(&["--show-metrics"]).show_metrics, t);
        assert_eq!(p(&["--no-retry"]).no_retry, t);
        assert_eq!(p(&["--watch"]).watch, t);
        assert_eq!(p(&["--watch-clear"]).watch_clear, t);
        assert_eq!(p(&["--no-watch-clear"]).watch_clear, Some(false));
        assert_eq!(p(&["--profile-histogram"]).profile_histogram, t);
        assert_eq!(p(&["--snapshot"]).snapshot, t);
        assert_eq!(
            p(&["--update-snapshots"]).snapshot_update.as_deref(),
            Some("all")
        );
        assert_eq!(
            p(&["--update-failing"]).snapshot_update.as_deref(),
            Some("failing")
        );
        assert_eq!(p(&["--ci-snapshot"]).snapshot_ci, t);
        assert_eq!(p(&["--diff"]).diff, t);
        assert_eq!(p(&["--diff-save"]).diff_save, t);
        assert_eq!(p(&["--strict-exit"]).strict_exit, t);
        assert_eq!(p(&["--no-redact"]).no_redact, t);
        assert_eq!(p(&["--allow-path"]).allow_path, t);
        assert_eq!(p(&["--verbose"]).verbose, t);
        assert_eq!(p(&["--continue-on-error"]).continue_on_error, t);
    }

    #[test]
    fn value_flags() {
        assert_eq!(
            p(&["--execution", "sequential"]).execution.as_deref(),
            Some("sequential")
        );
        assert_eq!(
            p(&["--execution", "parallel"]).execution.as_deref(),
            Some("parallel")
        );
        assert_eq!(p(&["--execution", "bogus"]).execution, None);
        assert_eq!(p(&["--max-concurrent", "5"]).max_concurrency, Some(5));
        assert_eq!(p(&["--max-concurrent", "0"]).max_concurrency, None);
        assert_eq!(p(&["--max-concurrent", "-1"]).max_concurrency, None);
        assert_eq!(p(&["--max-streams", "7"]).max_streams, Some(7));
        assert_eq!(p(&["--max-streams", "0"]).max_streams, None);
        assert_eq!(p(&["--keepalive-time", "9"]).keepalive_time, Some(9));
        assert_eq!(p(&["--connect-timeout", "4"]).connect_timeout, Some(4));
        assert_eq!(p(&["--verbose", "true"]).verbose, Some(true));
        assert_eq!(p(&["--verbose", "false"]).verbose, Some(false));
        assert_eq!(
            p(&["--continue-on-error", "true"]).continue_on_error,
            Some(true)
        );
        assert_eq!(
            p(&["--continue-on-error", "false"]).continue_on_error,
            Some(false)
        );
        assert_eq!(p(&["--output", "o.json"]).output.as_deref(), Some("o.json"));
        for f in ["json", "pretty", "raw"] {
            assert_eq!(p(&["--output-format", f]).output_format.as_deref(), Some(f));
        }
        assert_eq!(p(&["--output-format", "xml"]).output_format, None);
        for l in ["minimal", "standard", "detailed"] {
            assert_eq!(p(&["--pretty-level", l]).pretty_level.as_deref(), Some(l));
        }
        assert_eq!(p(&["--timeout", "30"]).timeout, Some(30));
        assert_eq!(p(&["--retries", "3"]).retries, Some(3));
        assert_eq!(p(&["--retry-delay", "50"]).retry_delay, Some(50));
        assert_eq!(p(&["--watch-debounce", "500"]).watch_debounce, Some(500));
        assert_eq!(p(&["--profile", "100"]).profile, Some(100));
        assert_eq!(p(&["--profile-warmup", "5"]).profile_warmup, Some(5));
        assert_eq!(
            p(&["--profile-concurrency", "10"]).profile_concurrency,
            Some(10)
        );
        assert_eq!(
            p(&["--profile-export", "r.csv"]).profile_export.as_deref(),
            Some("r.csv")
        );
        assert_eq!(
            p(&["--snapshot-dir", "s"]).snapshot_dir.as_deref(),
            Some("s")
        );
        assert_eq!(p(&["--diff-label", "l"]).diff_label.as_deref(), Some("l"));
        assert_eq!(
            p(&["--diff-compare", "c"]).diff_compare.as_deref(),
            Some("c")
        );
        assert_eq!(p(&["--diff-dir", "d"]).diff_dir.as_deref(), Some("d"));
        for f in ["terminal", "json", "markdown"] {
            assert_eq!(p(&["--diff-output", f]).diff_output.as_deref(), Some(f));
        }
        assert_eq!(p(&["--diff-output", "html"]).diff_output, None);
        assert_eq!(p(&["--fail-on", "2"]).fail_on, Some(2));
        assert_eq!(
            p(&["--fail-on-percentage", "12.5"]).fail_on_percentage,
            Some(12.5)
        );
        assert_eq!(p(&["--fail-on-percentage", "101"]).fail_on_percentage, None);
        assert_eq!(p(&["--fail-on-percentage", "abc"]).fail_on_percentage, None);
        assert_eq!(p(&["--env", "staging"]).env.as_deref(), Some("staging"));
    }

    #[test]
    fn allow_protocol_repeatable_and_lowercased() {
        let o = p(&["--allow-protocol", "FTP", "--allow-protocol", "sftp"]);
        assert_eq!(o.allow_protocols, Some(sv(&["ftp", "sftp"])));
    }

    #[test]
    fn invalid_enum_keeps_earlier_valid_value() {
        let o = p(&["--output-format", "json", "--output-format", "xml"]);
        assert_eq!(o.output_format.as_deref(), Some("json"));
    }

    #[test]
    fn js_number_prefix_semantics() {
        assert_eq!(p(&["--timeout", "30ms"]).timeout, Some(30));
        assert_eq!(p(&["--timeout", "12.7"]).timeout, Some(12));
        assert_eq!(p(&["--timeout", "abc"]).timeout, None);
        assert_eq!(p(&["--timeout", "99999999999999999999999"]).timeout, None);
        assert_eq!(
            p(&["--fail-on-percentage", "5e1"]).fail_on_percentage,
            Some(50.0)
        );
        assert_eq!(
            p(&["--fail-on-percentage", ".5x"]).fail_on_percentage,
            Some(0.5)
        );
    }

    #[test]
    fn short_flags() {
        assert_eq!(p(&["-h"]).help, Some(true));
        assert_eq!(p(&["-v"]).verbose, Some(true));
        assert_eq!(p(&["-p"]).execution.as_deref(), Some("parallel"));
        assert_eq!(p(&["-q"]).quiet, Some(true));
        assert_eq!(p(&["-w"]).watch, Some(true));
        assert_eq!(p(&["-s"]).snapshot, Some(true));
        assert_eq!(p(&["-u"]).snapshot_update.as_deref(), Some("all"));
        assert_eq!(p(&["-d"]).diff, Some(true));
        assert_eq!(p(&["-n"]).dry_run, Some(true));
        assert_eq!(p(&["-o", "out.json"]).output.as_deref(), Some("out.json"));
        assert_eq!(p(&["-P", "50"]).profile, Some(50));
        assert_eq!(p(&["-e", "prod"]).env.as_deref(), Some("prod"));
        let o = p(&["-vq"]);
        assert_eq!((o.verbose, o.quiet), (Some(true), Some(true)));
        let o = p(&["-pws"]);
        assert_eq!(o.execution.as_deref(), Some("parallel"));
        assert_eq!((o.watch, o.snapshot), (Some(true), Some(true)));
    }

    #[test]
    fn short_c_is_continue_on_error() {
        assert_eq!(p(&["-c"]).continue_on_error, Some(true));
        let o = p(&["-vc"]);
        assert_eq!((o.verbose, o.continue_on_error), (Some(true), Some(true)));
    }

    #[test]
    fn short_value_flag_without_value_is_ignored() {
        assert_eq!(p(&["-o"]).output, None);
        assert_eq!(p(&["-o", "-v"]).output, None);
    }

    #[test]
    fn unknown_flags_are_ignored_not_errors() {
        assert!(parse(&sv(&["--bogus", "-Z"])).is_ok());
        let o = p(&["--bogus", "-Z", "a.yaml"]);
        assert_eq!(o.files, sv(&["a.yaml"]));
        assert_eq!(
            o,
            CliOptions {
                files: sv(&["a.yaml"]),
                ..Default::default()
            }
        );
    }

    #[test]
    fn unknown_long_flag_swallows_following_value() {
        let o = p(&["--bogus", "a.yaml", "b.yaml"]);
        assert_eq!(o.files, sv(&["b.yaml"]));
    }

    #[test]
    fn equals_syntax_is_not_supported() {
        let o = p(&["--timeout=5000", "a.yaml"]);
        assert_eq!(o.timeout, None);
        // Unknown key `timeout=5000` swallows `a.yaml`.
        assert!(o.files.is_empty());
        assert_eq!(p(&["--quiet=true"]).quiet, None);
    }

    #[test]
    fn negative_number_is_not_a_value() {
        let o = p(&["--retries", "-1"]);
        assert_eq!(o.retries, None);
    }

    #[test]
    fn mixed_arguments() {
        let o = p(&["api.yaml", "--dry-run", "test.yaml", "--quiet"]);
        assert_eq!(o.files, sv(&["api.yaml", "test.yaml"]));
        assert_eq!((o.dry_run, o.quiet), (Some(true), Some(true)));
        let o = p(&[
            "api.yaml",
            "--execution",
            "parallel",
            "--max-concurrent",
            "5",
            "--timeout",
            "30",
            "-v",
            "--http2",
        ]);
        assert_eq!(o.files, sv(&["api.yaml"]));
        assert_eq!(o.execution.as_deref(), Some("parallel"));
        assert_eq!(o.max_concurrency, Some(5));
        assert_eq!(o.timeout, Some(30));
        assert_eq!((o.verbose, o.http2), (Some(true), Some(true)));
    }

    #[test]
    fn ci_style_invocation() {
        let a = pa(&[
            "a.yaml",
            "b.yaml",
            "--continue-on-error",
            "--strict-exit",
            "--retries",
            "3",
            "--retry-delay",
            "5000",
            "--timeout",
            "30000",
            "--output-format",
            "pretty",
            "--pretty-level",
            "minimal",
            "--output",
            "curl-runner-results.json",
        ]);
        assert_eq!(a.command, Command::Run);
        let o = a.options;
        assert_eq!(o.files, sv(&["a.yaml", "b.yaml"]));
        assert_eq!(o.continue_on_error, Some(true));
        assert_eq!(o.strict_exit, Some(true));
        assert_eq!(o.retries, Some(3));
        assert_eq!(o.retry_delay, Some(5000));
        assert_eq!(o.timeout, Some(30000));
        assert_eq!(o.output_format.as_deref(), Some("pretty"));
        assert_eq!(o.pretty_level.as_deref(), Some("minimal"));
        assert_eq!(o.output.as_deref(), Some("curl-runner-results.json"));
        assert_eq!(o.verbose, None);
        assert_eq!(o.quiet, None);
    }

    #[test]
    fn commands() {
        assert_eq!(pa(&[]).command, Command::Run);
        assert_eq!(pa(&["a.yaml"]).command, Command::Run);
        assert_eq!(pa(&["upgrade"]).command, Command::Upgrade);
        assert_eq!(
            pa(&["upgrade", "--dry-run"]).rest,
            sv(&["upgrade", "--dry-run"])
        );
        assert_eq!(pa(&["validate", "-q"]).command, Command::Validate);
        assert_eq!(pa(&["convert", "curl", "x"]).command, Command::Convert);
        assert_eq!(pa(&["init"]).command, Command::Wizard);
        assert_eq!(pa(&["edit", "a.yaml"]).command, Command::Wizard);
        assert_eq!(pa(&["--help"]).command, Command::Help);
        assert_eq!(pa(&["-h"]).command, Command::Help);
        assert_eq!(pa(&["--version"]).command, Command::Version);
        assert_eq!(pa(&["--help", "--version"]).command, Command::Help);
        // Subcommand args are not flag-parsed: `--help` after `upgrade` stays raw.
        assert_eq!(pa(&["upgrade", "--help"]).command, Command::Upgrade);
    }

    #[test]
    fn diff_subcommand_needs_three_args() {
        let a = pa(&["diff", "staging", "prod"]);
        assert_eq!(a.command, Command::Diff);
        assert_eq!(a.rest, sv(&["staging", "prod"]));
        assert_eq!(
            pa(&["diff", "staging", "prod", "--help"]).command,
            Command::Help
        );
        let a = pa(&["diff", "x"]);
        assert_eq!(a.command, Command::Run);
        assert_eq!(a.options.files, sv(&["diff", "x"]));
    }

    #[test]
    fn init_and_edit_args() {
        let o = parse_init_args(&sv(&["init", "https://x.io", "-w", "-o", "api.yaml"]));
        assert_eq!(
            o,
            InitOptions {
                wizard: true,
                url: Some("https://x.io".into()),
                output_path: Some("api.yaml".into())
            }
        );
        assert!(parse_init_args(&sv(&["init", "--wizard"])).wizard);
        let e = parse_edit_args(&sv(&["edit", "api.yaml", "--output", "new.yaml"]));
        assert_eq!(e.file, "api.yaml");
        assert_eq!(e.output_path.as_deref(), Some("new.yaml"));
        assert_eq!(parse_edit_args(&sv(&["edit"])).file, "");
    }

    #[test]
    fn quiet_round_trip_through_resolve() {
        use curl_runner_core::config::resolve;
        use curl_runner_core::model::GlobalConfig;
        for flag in ["--quiet", "-q"] {
            let a = pa(&[flag]);
            let r = resolve(GlobalConfig::default(), None, &|_| None, &a.options);
            let o = r.output.unwrap();
            assert_eq!(o.quiet, Some(true));
            assert_eq!(o.verbose, Some(false));
        }
    }
}
