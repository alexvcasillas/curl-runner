//! Response diffing / baseline types (`types/diff.ts`).

use super::Json;
use super::serde_util::{ser_f64, ser_opt_f64};
use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DiffConfig {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub enabled: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub exclude: Option<Vec<String>>,
    #[serde(default, rename = "match", skip_serializing_if = "Option::is_none")]
    pub match_rules: Option<IndexMap<String, String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub include_timings: Option<bool>,
}

/// Request-level `diff:` — `true`/`false` or detailed config.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum DiffSetting {
    Bool(bool),
    Config(DiffConfig),
}

/// `GlobalDiffConfig` (extends `DiffConfig`). `outputFormat`: terminal|json|markdown.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GlobalDiffConfig {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub enabled: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub exclude: Option<Vec<String>>,
    #[serde(default, rename = "match", skip_serializing_if = "Option::is_none")]
    pub match_rules: Option<IndexMap<String, String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub include_timings: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub dir: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub compare_with: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub save: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub output_format: Option<String>,
}

/// One stored baseline. Fields are in TS key order (`hash`, `capturedAt`,
/// `status`, `headers`, `body`, `timing`) so the file and the hash match. Stored
/// files are read like untyped JS objects, so `status` and header values keep
/// whatever JSON type they have (`"200"` differs from `200`).
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Baseline {
    #[serde(default)]
    pub hash: String,
    #[serde(default)]
    pub captured_at: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub status: Option<Json>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub headers: Option<IndexMap<String, Json>>,
    #[serde(
        default,
        deserialize_with = "super::serde_util::keep_null",
        skip_serializing_if = "Option::is_none"
    )]
    pub body: Option<Json>,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        serialize_with = "ser_opt_f64"
    )]
    pub timing: Option<f64>,
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BaselineFile {
    pub version: u64,
    pub label: String,
    pub captured_at: String,
    pub baselines: IndexMap<String, Baseline>,
}

/// `type`: added | removed | changed | type_mismatch.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ResponseDiff {
    pub path: String,
    #[serde(
        default,
        deserialize_with = "super::serde_util::keep_null",
        skip_serializing_if = "Option::is_none"
    )]
    pub baseline: Option<Json>,
    #[serde(
        default,
        deserialize_with = "super::serde_util::keep_null",
        skip_serializing_if = "Option::is_none"
    )]
    pub current: Option<Json>,
    #[serde(rename = "type")]
    pub diff_type: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TimingDiff {
    #[serde(serialize_with = "ser_f64")]
    pub baseline: f64,
    #[serde(serialize_with = "ser_f64")]
    pub current: f64,
    #[serde(serialize_with = "ser_f64")]
    pub change_percent: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DiffCompareResult {
    pub request_name: String,
    pub has_differences: bool,
    pub is_new_baseline: bool,
    pub baseline_label: String,
    pub current_label: String,
    pub differences: Vec<ResponseDiff>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub timing_diff: Option<TimingDiff>,
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DiffSummary {
    pub total_requests: u64,
    pub unchanged: u64,
    pub changed: u64,
    pub new_baselines: u64,
    pub results: Vec<DiffCompareResult>,
}
