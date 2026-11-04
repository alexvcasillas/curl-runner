//! `curl-runner convert curl|file|yaml` (`cli.ts` `executeConvert`): reads the
//! input, runs the pure conversion from `curl_runner_core::convert`, prints
//! the YAML or curl command, the warnings and the `--debug` dump.

use crate::colors::Color;
use crate::output::{Console, Logger, OutCfg};
use curl_runner_core::convert::import::{Attention, Import, slug, write_import};
use curl_runner_core::convert::openapi::convert_openapi;
use curl_runner_core::convert::postman::convert_postman;
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
        match argv.get(1).map(String::as_str) {
            Some("postman") => {
                console.out(&postman_help(&log));
                return 0;
            }
            Some("openapi") => {
                console.out(&openapi_help(&log));
                return 0;
            }
            _ => {}
        }
        console.out(&help(&log));
        return 0;
    }
    let Some(parsed) = parse_convert_args(argv) else {
        console.out(&help(&log));
        return 1;
    };

    match parsed.subcommand {
        Subcommand::Postman => {
            return run_import(&parsed, ImportKind::POSTMAN, &log, console);
        }
        Subcommand::OpenApi => {
            return run_import(&parsed, ImportKind::OPENAPI, &log, console);
        }
        _ => {}
    }

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

/// What differs between the import subcommands: the converter and the folder
/// used when the document has no usable name.
struct ImportKind {
    convert: fn(&str) -> Result<Import, ConvertError>,
    fallback_dir: &'static str,
}

impl ImportKind {
    const POSTMAN: Self = Self {
        convert: convert_postman,
        fallback_dir: "postman-import",
    };
    const OPENAPI: Self = Self {
        convert: convert_openapi,
        fallback_dir: "openapi-import",
    };
}

/// `convert postman|openapi <file> [-o <dir>]`: writes request files and a
/// `.env` into the directory and reports what needs the user's attention.
fn run_import(parsed: &ConvertArgs, kind: ImportKind, log: &Logger, console: &mut Console) -> i32 {
    let import =
        match read_text(&parsed.input).and_then(|text| (kind.convert)(&text).map_err(|e| e.0)) {
            Ok(i) => i,
            Err(msg) => {
                log.log_error(console, &msg);
                return 1;
            }
        };
    let dir = match parsed.options.output.as_deref().filter(|o| !o.is_empty()) {
        Some(o) => o.to_string(),
        None => slug(&import.name, kind.fallback_dir),
    };
    let outcome = match write_import(
        Path::new(&dir),
        Path::new(""),
        &import,
        "set it in .env.local",
    ) {
        Ok(o) => o,
        Err(e) => {
            // Names the file and the real cause.
            log.log_error(console, &e.to_string());
            return 1;
        }
    };

    let folders = import.report.folders;
    console.err(&format!(
        "Converted {} request{} in {} folder{} to {dir}",
        import.report.requests,
        plural(import.report.requests),
        folders,
        plural(folders),
    ));
    // The CLI never writes a secret's value, so every secret is listed, not
    // only the ones the import found empty.
    let mut items: Vec<Attention> = import.report.attention.clone();
    for v in import.variables.iter().filter(|v| v.secret) {
        let covered = items
            .iter()
            .any(|a| matches!(a, Attention::SetSecret { var } if *var == v.name));
        if !covered {
            items.push(Attention::SetSecret {
                var: v.name.clone(),
            });
        }
    }
    items.extend(
        outcome
            .kept_env
            .iter()
            .map(|var| Attention::KeptEnv { var: var.clone() }),
    );
    let rank = |a: &Attention| match a {
        Attention::PickFile { .. } => 0,
        Attention::SetSecret { .. } => 1,
        _ => 2,
    };
    items.sort_by_key(rank);
    let first_secret = items.iter().find_map(|a| match a {
        Attention::SetSecret { var } => Some(var.clone()),
        _ => None,
    });
    if !items.is_empty() {
        console.err("Needs your attention:");
        for item in &items {
            console.err(&format!("  - {item}"));
        }
    }
    console.err(&format!(
        "Run them from {dir} so curl-runner loads its .env."
    ));
    if let Some(var) = first_secret {
        console.err(&format!("Set the secrets above in {dir}/.env.local."));
        console.err(&format!(
            "A secret left unset is sent as written, like ${{{var}}}."
        ));
    }
    0
}

fn plural(n: usize) -> &'static str {
    if n == 1 { "" } else { "s" }
}

fn convert(parsed: &ConvertArgs) -> Result<ConvertResult, String> {
    let to_msg = |e: ConvertError| e.0;
    match parsed.subcommand {
        Subcommand::Curl => convert_curl_to_yaml(&parsed.input, &parsed.options).map_err(to_msg),
        Subcommand::File => {
            convert_script_to_yaml(&read_text(&parsed.input)?, &parsed.options).map_err(to_msg)
        }
        Subcommand::Yaml => convert_yaml_to_curl(&read_text(&parsed.input)?).map_err(to_msg),
        Subcommand::Postman | Subcommand::OpenApi => unreachable!("handled by run_import"),
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
        "\n{}\n\n{}\n  curl-runner convert curl \"<curl command>\"    Convert curl → YAML\n  curl-runner convert file <script.sh>         Convert shell script → YAML\n  curl-runner convert yaml <test.yaml>         Convert YAML → curl\n  curl-runner convert postman <collection>     Convert Postman collection → YAML files\n  curl-runner convert openapi <spec>           Convert OpenAPI/Swagger spec → YAML files\n\n{}\n  -o, --output <file>   Write output to file\n  --pretty              Pretty-print output (default for YAML)\n  --loss-report         Show warnings for unsupported features (default)\n  --debug               Show token stream, AST, and IR\n  -h, --help            Show this help\n\n{}\n  # Convert inline curl to YAML\n  curl-runner convert curl \"curl -X POST https://api.example.com -d '{{\"name\":\"Alex\"}}'\"\n\n  # Convert and save to file\n  curl-runner convert curl \"curl https://api.example.com\" -o test.yaml\n\n  # Convert shell script (batch)\n  curl-runner convert file scripts/api-tests.sh\n\n  # Convert YAML back to curl\n  curl-runner convert yaml test.yaml\n\n  # Debug mode\n  curl-runner convert curl \"curl https://example.com\" --debug\n",
        log.color("CURL RUNNER - CONVERT", Color::Bright),
        y("USAGE:"),
        y("OPTIONS:"),
        y("EXAMPLES:"),
    )
}

/// Help for `convert postman --help`. The shared help is pinned to the
/// TypeScript goldens, so Postman has its own.
fn postman_help(log: &Logger) -> String {
    let y = |s: &str| log.color(s, Color::Yellow);
    format!(
        "\n{}\n\n{}\n  curl-runner convert postman <collection.json> [-o <dir>]\n\n{}\n  -o, --output <dir>   Write the files into this directory (default: the collection's name)\n  -h, --help           Show this help\n",
        log.color("CURL RUNNER - CONVERT POSTMAN", Color::Bright),
        y("USAGE:"),
        y("OPTIONS:"),
    )
}

/// Help for `convert openapi --help`.
fn openapi_help(log: &Logger) -> String {
    let y = |s: &str| log.color(s, Color::Yellow);
    format!(
        "\n{}\n\n{}\n  curl-runner convert openapi <spec.json|spec.yaml> [-o <dir>]\n\n{}\n  -o, --output <dir>   Write the files into this directory (default: the API's title)\n  -h, --help           Show this help\n",
        log.color("CURL RUNNER - CONVERT OPENAPI", Color::Bright),
        y("USAGE:"),
        y("OPTIONS:"),
    )
}
