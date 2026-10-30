//! curl <-> YAML conversion (`packages/cli/src/convert`).
//!
//! Pure functions: no file access, no stdout/stderr. The CLI layer reads the
//! inputs, prints the results and warnings and writes `--output`.

mod batch;
mod curl_parser;
mod debug;
mod generator;
mod jsjson;
mod jsutil;
mod normalizer;
mod tokenizer;
mod types;
mod yaml_serializer;
mod yaml_to_ir;

pub use batch::extract_curl_commands;
pub use curl_parser::{parse_curl, parse_tokens};
pub use generator::{generate_curl_from_ir, shell_quote};
/// The map type of [`CurlRunnerIr`] fields, so callers can rebuild them.
pub use jsutil::JsMap;
pub use normalizer::{NormalizeError, normalize};
pub use tokenizer::tokenize;
pub use types::*;
pub use yaml_serializer::{serialize_batch_to_yaml, serialize_to_yaml};
pub use yaml_to_ir::yaml_to_ir;

use crate::model::Json;

/// Stand-in for a scalar entry of `requests` (it has no fields).
static SCALAR: Json = Json::Bool(true);

/// Result of a conversion (`ConvertResult`).
#[derive(Debug, Clone, PartialEq)]
pub struct ConvertResult {
    pub output: String,
    pub warnings: Vec<String>,
    /// `--debug` dump (`JSON.stringify(debug, null, 2)` is the display form).
    pub debug: Option<Json>,
}

/// A conversion failure; the message is what the TS code would have thrown.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("{0}")]
pub struct ConvertError(pub String);

impl From<NormalizeError> for ConvertError {
    fn from(e: NormalizeError) -> Self {
        ConvertError(e.to_string())
    }
}

fn yaml_options(o: &ConvertOptions) -> YamlOptions {
    YamlOptions {
        pretty: o.pretty,
        loss_report: o.loss_report,
    }
}

/// Convert a curl command string to YAML (`convertCurlToYaml`).
pub fn convert_curl_to_yaml(
    cmd: &str,
    options: &ConvertOptions,
) -> Result<ConvertResult, ConvertError> {
    let ast = parse_curl(cmd);
    let ir = normalize(&ast)?;
    let output = serialize_to_yaml(&ir, yaml_options(options));
    let debug = options.debug.then(|| {
        debug::object(vec![
            ("tokens", debug::strings(&tokenize(crate::jsval::trim(cmd)))),
            ("ast", debug::ast(&ast)),
            ("ir", debug::ir(&ir)),
        ])
    });
    Ok(ConvertResult {
        output,
        warnings: ir.warnings,
        debug,
    })
}

/// Convert the text of a shell script to YAML, one request per curl command
/// (`convertFileToYaml`, after the file is read).
pub fn convert_script_to_yaml(
    script: &str,
    options: &ConvertOptions,
) -> Result<ConvertResult, ConvertError> {
    let commands = extract_curl_commands(script);
    if commands.is_empty() {
        return Ok(ConvertResult {
            output: String::new(),
            warnings: vec!["No curl commands found in file".into()],
            debug: None,
        });
    }

    let mut irs = Vec::new();
    let mut warnings = Vec::new();
    for (idx, cmd) in commands.iter().enumerate() {
        let mut ir = normalize(&parse_curl(cmd))?;
        ir.name = Some(format!("request_{}", idx + 1));
        warnings.extend(ir.warnings.iter().cloned());
        irs.push(ir);
    }

    let opts = yaml_options(options);
    if irs.len() == 1 {
        let output = serialize_to_yaml(&irs[0], opts);
        let debug = options
            .debug
            .then(|| debug::object(vec![("ir", debug::ir(&irs[0]))]));
        return Ok(ConvertResult {
            output,
            warnings,
            debug,
        });
    }
    let output = serialize_batch_to_yaml(&irs, opts);
    let debug = options.debug.then(|| {
        debug::object(vec![(
            "ir",
            Json::Array(irs.iter().map(debug::ir).collect()),
        )])
    });
    Ok(ConvertResult {
        output,
        warnings,
        debug,
    })
}

/// Convert the text of a curl-runner YAML file to curl commands
/// (`convertYamlToCurl`, after the file is read).
pub fn convert_yaml_to_curl(content: &str) -> Result<ConvertResult, ConvertError> {
    let root = crate::parser::parse_raw_opt(content)
        .map_err(|e| ConvertError(format!("YAML Parse error: {e}")))?;

    // `yaml.request` on a null document is a TypeError in TS.
    if root.is_null() {
        return Err(ConvertError(
            "null is not an object (evaluating 'yaml.request')".into(),
        ));
    }

    let mut requests: Vec<&Json> = Vec::new();
    if yaml_to_ir::truthy(root.get("request")) {
        requests.extend(root.get("request"));
    }
    spread(root.get("requests"), &mut requests)?;
    spread(
        root.get("collection").and_then(|c| c.get("requests")),
        &mut requests,
    )?;

    if requests.is_empty() {
        return Ok(ConvertResult {
            output: String::new(),
            warnings: vec!["No requests found in YAML file".into()],
            debug: None,
        });
    }

    let mut warnings = Vec::new();
    let mut commands = Vec::new();
    for req in requests {
        let ir = if req.is_object() || req.is_null() {
            yaml_to_ir(req).map_err(ConvertError)?
        } else {
            // A scalar request has no fields: it fails at the missing url.
            yaml_to_ir(&Json::Object(Default::default())).map_err(ConvertError)?
        };
        warnings.extend(ir.warnings.iter().cloned());
        commands.push(generate_checked(&ir, req)?);
    }
    Ok(ConvertResult {
        output: commands.join("\n\n"),
        warnings,
        debug: None,
    })
}

/// `requests.push(...v)` for a truthy `v`: arrays spread, a string spreads into
/// its characters (each a request without fields), anything else throws.
fn spread<'a>(v: Option<&'a Json>, out: &mut Vec<&'a Json>) -> Result<(), ConvertError> {
    if !yaml_to_ir::truthy(v) {
        return Ok(());
    }
    match v {
        Some(Json::Array(a)) => out.extend(a.iter()),
        Some(Json::String(s)) => out.extend(s.chars().map(|_| &SCALAR)),
        _ => {
            return Err(ConvertError(
                "Spread syntax requires ...iterable[Symbol.iterator] to be a function".into(),
            ));
        }
    }
    Ok(())
}

/// `shellQuote(v)` on a value that is not a string throws in TS (arrays have
/// their own `includes`, so they pass).
fn quote_error(v: Option<&Json>) -> Option<ConvertError> {
    match v {
        Some(Json::Number(_) | Json::Bool(_) | Json::Object(_)) if yaml_to_ir::truthy(v) => {
            Some(ConvertError(
                "s.includes is not a function. (In 's.includes(\"'\")', 's.includes' is undefined)"
                    .into(),
            ))
        }
        _ => None,
    }
}

/// `generateCurlFromIR`, failing where it throws on a non-string option
/// (`proxy`, `output`, `ssl.*`) or `url`, checked in the order it reaches them.
fn generate_checked(ir: &CurlRunnerIr, req: &Json) -> Result<String, ConvertError> {
    let ssl = req.get("ssl");
    for v in [
        req.get("proxy"),
        req.get("output"),
        ssl.and_then(|s| s.get("ca")),
        ssl.and_then(|s| s.get("cert")),
        ssl.and_then(|s| s.get("key")),
    ] {
        if let Some(e) = quote_error(v) {
            return Err(e);
        }
    }
    let has_params = ir.params.as_ref().is_some_and(|p| !p.is_empty());
    let msg = match req.get("url") {
        None | Some(Json::Null) => {
            let what = if req.get("url").is_none() {
                "undefined"
            } else {
                "null"
            };
            let target = if has_params {
                "url.includes"
            } else {
                "s.includes"
            };
            format!("{what} is not an object (evaluating '{target}')")
        }
        Some(Json::Number(_) | Json::Bool(_) | Json::Object(_)) => {
            if has_params {
                "url.includes is not a function. (In 'url.includes(\"?\")', 'url.includes' is undefined)".to_string()
            } else {
                "s.includes is not a function. (In 's.includes(\"'\")', 's.includes' is undefined)"
                    .to_string()
            }
        }
        _ => return Ok(generate_curl_from_ir(ir)),
    };
    Err(ConvertError(msg))
}

/// Parse the arguments after `convert` (`parseConvertArgs`); `args[0]` is
/// `convert`. `None` shows the help.
pub fn parse_convert_args(args: &[String]) -> Option<ConvertArgs> {
    if args.len() < 3 {
        return None;
    }
    let subcommand = match args[1].as_str() {
        "curl" => Subcommand::Curl,
        "file" => Subcommand::File,
        "yaml" => Subcommand::Yaml,
        _ => return None,
    };
    let mut options = ConvertOptions::default();
    let mut i = 3;
    while i < args.len() {
        match args[i].as_str() {
            "--output" | "-o" => {
                i += 1;
                options.output = args.get(i).cloned();
            }
            "--pretty" => options.pretty = Some(true),
            "--normalize" => options.normalize = true,
            "--loss-report" => options.loss_report = Some(true),
            "--batch" => options.batch = true,
            "--env-detect" => options.env_detect = true,
            "--debug" => options.debug = true,
            _ => {}
        }
        i += 1;
    }
    Some(ConvertArgs {
        subcommand,
        input: args[2].clone(),
        options,
    })
}

#[cfg(test)]
mod tests;
