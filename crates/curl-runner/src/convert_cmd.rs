//! `curl-runner convert curl|file|yaml` (`cli.ts` `executeConvert`): reads the
//! input, runs the pure conversion from `curl_runner_core::convert`, prints
//! the YAML or curl command, the warnings and the `--debug` dump.

use crate::colors::Color;
use crate::output::{Console, Logger, OutCfg};
use curl_runner_core::convert::{
    ConvertArgs, ConvertError, ConvertResult, Subcommand, convert_curl_to_yaml,
    convert_script_to_yaml, convert_yaml_to_curl, parse_convert_args,
};
use curl_runner_core::snapshot::js_stringify_pretty;
use std::io;
use std::path::Path;

/// Run the `convert` command; `argv` is the whole argument list (`argv[0]` is
/// `convert`). Returns the exit code.
pub fn run(argv: &[String], color: bool, console: &mut Console) -> i32 {
    let log = Logger::new(OutCfg::from_config(None), color, None);

    if argv.iter().any(|a| a == "--help" || a == "-h") {
        console.out(&help(&log));
        return 0;
    }
    let Some(parsed) = parse_convert_args(argv) else {
        console.out(&help(&log));
        return 1;
    };

    let result = match convert(&parsed) {
        Ok(r) => r,
        Err(msg) => {
            log.log_error(console, &msg);
            return 1;
        }
    };

    if parsed.options.debug
        && let Some(debug) = &result.debug
    {
        console.out(&log.color("DEBUG:", Color::Yellow));
        console.out(&js_stringify_pretty(debug, 2));
        console.out("");
    }

    let output_path = parsed.options.output.as_deref().filter(|o| !o.is_empty());
    if output_path.is_none() {
        for w in &result.warnings {
            log.log_warning(console, w);
        }
    }

    match output_path {
        Some(path) => {
            if let Err(e) = write_output(Path::new(path), &result.output) {
                log.log_error(console, &fs_message(&e, path, "open"));
                return 1;
            }
            log.log_info(console, &format!("Written to {path}"));
        }
        None => console.out(&result.output),
    }
    0
}

fn convert(parsed: &ConvertArgs) -> Result<ConvertResult, String> {
    let to_msg = |e: ConvertError| e.0;
    match parsed.subcommand {
        Subcommand::Curl => convert_curl_to_yaml(&parsed.input, &parsed.options).map_err(to_msg),
        Subcommand::File => {
            convert_script_to_yaml(&read_text(&parsed.input)?, &parsed.options).map_err(to_msg)
        }
        Subcommand::Yaml => convert_yaml_to_curl(&read_text(&parsed.input)?).map_err(to_msg),
    }
}

fn read_text(path: &str) -> Result<String, String> {
    match std::fs::read(path) {
        Ok(bytes) => Ok(String::from_utf8_lossy(&bytes).into_owned()),
        Err(e) if Path::new(path).is_dir() => {
            let _ = e;
            Err("Directories cannot be read like files".into())
        }
        Err(e) => Err(fs_message(&e, path, "open")),
    }
}

/// Bun's `Bun.write`: creates missing parent directories.
fn write_output(path: &Path, content: &str) -> io::Result<()> {
    if let Some(parent) = path.parent().filter(|p| !p.as_os_str().is_empty()) {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(path, content)
}

/// The text of the Node/Bun filesystem error for the common failures.
fn fs_message(e: &io::Error, path: &str, op: &str) -> String {
    if Path::new(path).is_dir() {
        return format!("EISDIR: illegal operation on a directory, {op} '{path}'");
    }
    match e.kind() {
        io::ErrorKind::NotFound => format!("ENOENT: no such file or directory, {op} '{path}'"),
        io::ErrorKind::PermissionDenied => format!("EACCES: permission denied, {op} '{path}'"),
        _ => e.to_string(),
    }
}

/// `showConvertHelp` (printed with `console.log`, so with a trailing newline
/// after the template's own).
fn help(log: &Logger) -> String {
    let y = |s: &str| log.color(s, Color::Yellow);
    format!(
        "\n{}\n\n{}\n  curl-runner convert curl \"<curl command>\"    Convert curl → YAML\n  curl-runner convert file <script.sh>         Convert shell script → YAML\n  curl-runner convert yaml <test.yaml>         Convert YAML → curl\n\n{}\n  -o, --output <file>   Write output to file\n  --pretty              Pretty-print output (default for YAML)\n  --loss-report         Show warnings for unsupported features (default)\n  --debug               Show token stream, AST, and IR\n  -h, --help            Show this help\n\n{}\n  # Convert inline curl to YAML\n  curl-runner convert curl \"curl -X POST https://api.example.com -d '{{\"name\":\"Alex\"}}'\"\n\n  # Convert and save to file\n  curl-runner convert curl \"curl https://api.example.com\" -o test.yaml\n\n  # Convert shell script (batch)\n  curl-runner convert file scripts/api-tests.sh\n\n  # Convert YAML back to curl\n  curl-runner convert yaml test.yaml\n\n  # Debug mode\n  curl-runner convert curl \"curl https://example.com\" --debug\n",
        log.color("CURL RUNNER - CONVERT", Color::Bright),
        y("USAGE:"),
        y("OPTIONS:"),
        y("EXAMPLES:"),
    )
}
