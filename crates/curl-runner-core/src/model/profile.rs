//! Performance profiling types (`types/profile.ts`) and watch config (`types/watch.ts`).

use super::serde_util::{ser_f64, ser_vec_f64};
use super::{ExecutionResult, RequestConfig};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProfileConfig {
    #[serde(default, deserialize_with = "crate::model::serde_util::u64_or_default")]
    pub iterations: u64,
    #[serde(
        deserialize_with = "crate::model::serde_util::opt_u64",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub warmup: Option<u64>,
    #[serde(
        deserialize_with = "crate::model::serde_util::opt_u64",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub concurrency: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub histogram: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub export_file: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProfileStats {
    pub iterations: u64,
    pub warmup: u64,
    #[serde(serialize_with = "ser_f64")]
    pub min: f64,
    #[serde(serialize_with = "ser_f64")]
    pub max: f64,
    #[serde(serialize_with = "ser_f64")]
    pub mean: f64,
    #[serde(serialize_with = "ser_f64")]
    pub median: f64,
    #[serde(serialize_with = "ser_f64")]
    pub p50: f64,
    #[serde(serialize_with = "ser_f64")]
    pub p95: f64,
    #[serde(serialize_with = "ser_f64")]
    pub p99: f64,
    #[serde(serialize_with = "ser_f64")]
    pub std_dev: f64,
    pub failures: u64,
    #[serde(serialize_with = "ser_f64")]
    pub failure_rate: f64,
    #[serde(serialize_with = "ser_vec_f64")]
    pub timings: Vec<f64>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProfileResult {
    pub request: RequestConfig,
    pub stats: ProfileStats,
    pub iterations: Vec<ExecutionResult>,
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WatchConfig {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub enabled: Option<bool>,
    #[serde(
        deserialize_with = "crate::model::serde_util::opt_u64",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub debounce: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub clear: Option<bool>,
}
