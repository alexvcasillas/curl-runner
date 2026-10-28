//! Global and collection configuration (`types/global.ts`).

use super::serde_util::{lenient_opt_map, lenient_opt_string, ser_opt_f64};
use super::{
    GlobalDiffConfig, GlobalSnapshotConfig, OutputConfig, ProfileConfig, RequestConfig, SslConfig,
    WatchConfig,
};
use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CollectionConfig {
    #[serde(default)]
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(
        default,
        deserialize_with = "lenient_opt_map",
        skip_serializing_if = "Option::is_none"
    )]
    pub variables: Option<IndexMap<String, String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub defaults: Option<RequestConfig>,
    #[serde(default)]
    pub requests: Vec<RequestConfig>,
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CiExitConfig {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub strict_exit: Option<bool>,
    #[serde(
        deserialize_with = "crate::model::serde_util::opt_u64",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub fail_on: Option<u64>,
    #[serde(
        deserialize_with = "crate::model::serde_util::opt_f64",
        default,
        skip_serializing_if = "Option::is_none",
        serialize_with = "ser_opt_f64"
    )]
    pub fail_on_percentage: Option<f64>,
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConnectionPoolConfig {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub enabled: Option<bool>,
    #[serde(
        deserialize_with = "crate::model::serde_util::opt_u64",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub max_streams_per_host: Option<u64>,
    #[serde(
        deserialize_with = "crate::model::serde_util::opt_u64",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub keepalive_time: Option<u64>,
    #[serde(
        deserialize_with = "crate::model::serde_util::opt_u64",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub connect_timeout: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EnvConfig {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub environment: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub redact_secrets: Option<bool>,
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SecurityConfig {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub allowed_protocols: Option<Vec<String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub allow_paths: Option<bool>,
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GlobalConfig {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub env: Option<EnvConfig>,
    /// `sequential` | `parallel`
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub execution: Option<String>,
    #[serde(
        deserialize_with = "crate::model::serde_util::opt_u64",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub max_concurrency: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub continue_on_error: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub dry_run: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub http2: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub connection_pool: Option<ConnectionPoolConfig>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ci: Option<CiExitConfig>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ssl: Option<SslConfig>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub watch: Option<WatchConfig>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub profile: Option<ProfileConfig>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub snapshot: Option<GlobalSnapshotConfig>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub diff: Option<GlobalDiffConfig>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub security: Option<SecurityConfig>,
    #[serde(
        default,
        deserialize_with = "lenient_opt_map",
        skip_serializing_if = "Option::is_none"
    )]
    pub variables: Option<IndexMap<String, String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub output: Option<OutputConfig>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub defaults: Option<RequestConfig>,
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct YamlFile {
    #[serde(
        default,
        deserialize_with = "lenient_opt_string",
        skip_serializing_if = "Option::is_none"
    )]
    pub version: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub global: Option<GlobalConfig>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub collection: Option<CollectionConfig>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub requests: Option<Vec<RequestConfig>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub request: Option<RequestConfig>,
}
