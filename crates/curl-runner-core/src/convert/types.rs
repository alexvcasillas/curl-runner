//! Types for the curl <-> YAML conversion engine (`convert/types.ts`).

use super::jsutil::JsMap;
use crate::model::Json;

#[derive(Debug, Clone, PartialEq)]
pub struct UnsupportedFlag {
    pub flag: String,
    pub value: Option<String>,
}

/// Parsed curl command. `order` records the sequence in which the TS parser
/// first assigned each property (it decides the key order of the debug dump).
#[derive(Debug, Clone, Default, PartialEq)]
pub struct CurlAst {
    pub url: String,
    pub method: Option<String>,
    pub headers: Vec<(String, String)>,
    pub data: Vec<String>,
    pub data_urlencode: Vec<String>,
    pub data_raw: Vec<String>,
    pub data_binary: Vec<String>,
    pub form: Vec<String>,
    pub form_string: Vec<String>,
    pub user: Option<String>,
    pub get: bool,
    pub head: bool,
    pub compressed: bool,
    pub insecure: bool,
    pub location: bool,
    pub max_redirs: Option<f64>,
    pub max_time: Option<f64>,
    pub proxy: Option<String>,
    pub output: Option<String>,
    pub http2: bool,
    pub cacert: Option<String>,
    pub cert: Option<String>,
    pub key: Option<String>,
    pub cookie: Option<String>,
    pub cookie_jar: Option<String>,
    pub user_agent: Option<String>,
    pub referer: Option<String>,
    pub silent: bool,
    pub verbose: bool,
    /// Raw `--retry` / `--retry-delay` values, validated by the normalizer.
    pub retry: Option<String>,
    pub retry_delay: Option<String>,
    pub unsupported_flags: Vec<UnsupportedFlag>,
    pub order: Vec<&'static str>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Body {
    Json(Json),
    Urlencoded(String),
    Raw(String),
    Binary(String),
}

impl Body {
    pub fn type_name(&self) -> &'static str {
        match self {
            Body::Json(_) => "json",
            Body::Urlencoded(_) => "urlencoded",
            Body::Raw(_) => "raw",
            Body::Binary(_) => "binary",
        }
    }
}

/// A `--max-redirs` value: a number from curl, or whatever a YAML file holds
/// (`maxRedirects: "x"` is printed as written).
#[derive(Debug, Clone, PartialEq)]
pub enum NumText {
    Num(f64),
    Text(String),
}

impl NumText {
    /// JS `String(v)`.
    pub fn to_js_string(&self) -> String {
        match self {
            NumText::Num(n) => crate::jsval::num_to_string(*n),
            NumText::Text(t) => t.clone(),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Auth {
    /// `basic` or `bearer` (a YAML file can carry anything).
    pub kind: String,
    pub username: Option<String>,
    pub password: Option<String>,
    pub token: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct FormFile {
    pub file: String,
    pub filename: Option<String>,
    pub content_type: Option<String>,
    /// `type=` was assigned before `filename=` (debug dump key order).
    pub type_first: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub enum FormField {
    Text(String),
    File(FormFile),
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct Ssl {
    pub ca: Option<String>,
    pub cert: Option<String>,
    pub key: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Source {
    Curl,
    Yaml,
}

/// The `retry` block: `--retry` / `--retry-delay` and the runner settings
/// that reproduce curl's retry timing and status set.
#[derive(Debug, Clone, PartialEq)]
pub struct RetryIr {
    pub count: u64,
    /// Milliseconds (curl takes whole seconds).
    pub delay_ms: Option<u64>,
    pub backoff: Option<f64>,
    pub statuses: Option<Vec<u16>>,
}

/// Statuses curl retries on (`--retry`): the runner's default set lacks 408.
pub const CURL_RETRY_STATUSES: [u16; 6] = [408, 429, 500, 502, 503, 504];

/// Intermediate representation shared by both converters.
#[derive(Debug, Clone, PartialEq)]
pub struct CurlRunnerIr {
    pub name: Option<String>,
    pub method: String,
    pub url: String,
    pub headers: JsMap<String>,
    pub params: Option<JsMap<String>>,
    pub body: Option<Body>,
    pub form_data: Option<JsMap<FormField>>,
    pub auth: Option<Auth>,
    pub insecure: bool,
    pub follow_redirects: Option<bool>,
    pub max_redirects: Option<NumText>,
    pub timeout: Option<f64>,
    pub proxy: Option<String>,
    pub output: Option<String>,
    pub http2: bool,
    pub retry: Option<RetryIr>,
    pub ssl: Option<Ssl>,
    pub source: Source,
    /// A YAML `Content-Type` header whose value is falsy (`null`, `0`,
    /// `false`): the TS generator then adds its own JSON content type, but the
    /// header is still printed (as `null`, `0`, ...).
    pub falsy_content_type: bool,
    pub warnings: Vec<String>,
    pub unsupported_flags: Vec<UnsupportedFlag>,
}

/// Convert command options (`ConvertOptions`).
#[derive(Debug, Clone, Default, PartialEq)]
pub struct ConvertOptions {
    pub output: Option<String>,
    pub pretty: Option<bool>,
    pub normalize: bool,
    pub loss_report: Option<bool>,
    pub batch: bool,
    pub env_detect: bool,
    pub debug: bool,
}

/// Serializer options.
#[derive(Debug, Clone, Copy, Default)]
pub struct YamlOptions {
    pub pretty: Option<bool>,
    pub loss_report: Option<bool>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Subcommand {
    Curl,
    File,
    Yaml,
}

/// Parsed `convert` arguments (`ConvertArgs`).
#[derive(Debug, Clone, PartialEq)]
pub struct ConvertArgs {
    pub subcommand: Subcommand,
    pub input: String,
    pub options: ConvertOptions,
}
