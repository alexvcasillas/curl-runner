//! Request configuration types (`types/request.ts`).

use super::serde_util::lenient_opt_map;
use super::{DiffSetting, Json, SnapshotSetting};
use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FileAttachment {
    pub file: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub filename: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub content_type: Option<String>,
}

/// Untagged: string | number | boolean | file attachment.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum FormFieldValue {
    Bool(bool),
    Number(serde_json::Number),
    Text(String),
    File(FileAttachment),
}

pub type FormDataConfig = IndexMap<String, FormFieldValue>;
pub type StoreConfig = IndexMap<String, String>;

/// `left`/`operator`/`right` comparison. Operator kept as string
/// (`==`, `!=`, `>`, `<`, `>=`, `<=`, `contains`, `matches`, `exists`, `not-exists`).
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConditionExpression {
    #[serde(default)]
    pub left: String,
    #[serde(default)]
    pub operator: String,
    /// string | number | boolean
    #[serde(
        default,
        deserialize_with = "super::serde_util::keep_null",
        skip_serializing_if = "Option::is_none"
    )]
    pub right: Option<Json>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub case_sensitive: Option<bool>,
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WhenObject {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub all: Option<Vec<ConditionExpression>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub any: Option<Vec<ConditionExpression>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub left: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub operator: Option<String>,
    /// string | number | boolean
    #[serde(
        default,
        deserialize_with = "super::serde_util::keep_null",
        skip_serializing_if = "Option::is_none"
    )]
    pub right: Option<Json>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub case_sensitive: Option<bool>,
}

/// `when:` — string shorthand or object.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum WhenCondition {
    Expr(String),
    Object(WhenObject),
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SslConfig {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub verify: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ca: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cert: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub key: Option<String>,
}

/// `type`: `basic` | `bearer`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AuthConfig {
    #[serde(rename = "type", default)]
    pub auth_type: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub username: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub password: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub token: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RetryConfig {
    /// Required in TS; `None` (key omitted) when absent so merges can tell
    /// "unset" from an explicit `count: 0`.
    #[serde(
        default,
        deserialize_with = "super::serde_util::opt_u64",
        skip_serializing_if = "Option::is_none"
    )]
    pub count: Option<u64>,
    #[serde(
        deserialize_with = "super::serde_util::opt_u64",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub delay: Option<u64>,
    #[serde(
        deserialize_with = "super::serde_util::opt_f64",
        default,
        skip_serializing_if = "Option::is_none",
        serialize_with = "super::serde_util::ser_opt_f64"
    )]
    pub backoff: Option<f64>,
    #[serde(
        deserialize_with = "super::serde_util::opt_vec_u16",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub retryable_statuses: Option<Vec<u16>>,
}

/// `status`: number | number[] (numeric strings accepted, see `serde_util`).
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(untagged)]
pub enum StatusExpect {
    One(u16),
    Many(Vec<u16>),
}

impl<'de> Deserialize<'de> for StatusExpect {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        use super::serde_util::json_to_u16;
        match Json::deserialize(d)? {
            Json::Array(items) => items
                .into_iter()
                .map(json_to_u16)
                .collect::<Result<_, _>>()
                .map(Self::Many),
            other => json_to_u16(other).map(Self::One),
        }
        .map_err(serde::de::Error::custom)
    }
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExpectConfig {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub failure: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub status: Option<StatusExpect>,
    #[serde(
        default,
        deserialize_with = "lenient_opt_map",
        skip_serializing_if = "Option::is_none"
    )]
    pub headers: Option<IndexMap<String, String>>,
    #[serde(
        default,
        deserialize_with = "super::serde_util::keep_null",
        skip_serializing_if = "Option::is_none"
    )]
    pub body: Option<Json>,
    /// e.g. `< 1000`, `> 500, < 2000`
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub response_time: Option<String>,
}

/// Per-file output settings attached to each request. `format`: json|pretty|raw,
/// `prettyLevel`: minimal|standard|detailed.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OutputConfig {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub verbose: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub quiet: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub show_headers: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub show_body: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub show_metrics: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub format: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pretty_level: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub save_to_file: Option<String>,
    /// Internal flag set when dry-run mode is enabled (global `output` only).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub dry_run: Option<bool>,
}

/// `sourceOutputConfig` has the same shape as the global output block.
pub type SourceOutputConfig = OutputConfig;

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RequestConfig {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// Required in TS; empty when absent (so `defaults` blocks can reuse this type).
    #[serde(default)]
    pub url: String,
    /// GET | POST | PUT | DELETE | PATCH | HEAD | OPTIONS
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub method: Option<String>,
    #[serde(
        default,
        deserialize_with = "lenient_opt_map",
        skip_serializing_if = "Option::is_none"
    )]
    pub headers: Option<IndexMap<String, String>>,
    #[serde(
        default,
        deserialize_with = "lenient_opt_map",
        skip_serializing_if = "Option::is_none"
    )]
    pub params: Option<IndexMap<String, String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_file: Option<String>,
    #[serde(
        default,
        deserialize_with = "super::serde_util::keep_null",
        skip_serializing_if = "Option::is_none"
    )]
    pub body: Option<Json>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub form_data: Option<FormDataConfig>,
    #[serde(
        deserialize_with = "super::serde_util::opt_u64",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub timeout: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub follow_redirects: Option<bool>,
    #[serde(
        deserialize_with = "super::serde_util::opt_u64",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub max_redirects: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub auth: Option<AuthConfig>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub proxy: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub insecure: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ssl: Option<SslConfig>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub output: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub http2: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub retry: Option<RetryConfig>,
    #[serde(
        default,
        deserialize_with = "lenient_opt_map",
        skip_serializing_if = "Option::is_none"
    )]
    pub variables: Option<IndexMap<String, String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub store: Option<StoreConfig>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub when: Option<WhenCondition>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub expect: Option<ExpectConfig>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub snapshot: Option<SnapshotSetting>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub diff: Option<DiffSetting>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_output_config: Option<SourceOutputConfig>,
}
