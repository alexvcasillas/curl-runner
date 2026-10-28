//! Execution result types (`types/execution.ts`).

use super::serde_util::{ser_f64, ser_opt_f64};
use super::{DiffCompareResult, Json, RequestConfig, SnapshotCompareResult};
use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

/// Header value: a single string or (for repeated headers) a list.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum HeaderValue {
    One(String),
    Many(Vec<String>),
}

pub type Headers = IndexMap<String, HeaderValue>;

/// One response header block (informational, redirect, final); final block last.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HeaderBlock {
    pub status: u16,
    pub headers: Headers,
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Metrics {
    #[serde(serialize_with = "ser_f64")]
    pub duration: f64,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        serialize_with = "ser_opt_f64"
    )]
    pub size: Option<f64>,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        serialize_with = "ser_opt_f64"
    )]
    pub dns_lookup: Option<f64>,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        serialize_with = "ser_opt_f64"
    )]
    pub tcp_connection: Option<f64>,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        serialize_with = "ser_opt_f64"
    )]
    pub tls_handshake: Option<f64>,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        serialize_with = "ser_opt_f64"
    )]
    pub first_byte: Option<f64>,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        serialize_with = "ser_opt_f64"
    )]
    pub download: Option<f64>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExecutionResult {
    pub request: RequestConfig,
    pub success: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub status: Option<u16>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub headers: Option<Headers>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub header_history: Option<Vec<HeaderBlock>>,
    #[serde(
        default,
        deserialize_with = "super::serde_util::keep_null",
        skip_serializing_if = "Option::is_none"
    )]
    pub body: Option<Json>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub dry_run: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub metrics: Option<Metrics>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub snapshot_result: Option<SnapshotCompareResult>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub diff_result: Option<DiffCompareResult>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub skipped: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub skip_reason: Option<String>,
}

impl ExecutionResult {
    pub fn new(request: RequestConfig, success: bool) -> Self {
        Self {
            request,
            success,
            status: None,
            headers: None,
            header_history: None,
            body: None,
            error: None,
            dry_run: None,
            metrics: None,
            snapshot_result: None,
            diff_result: None,
            skipped: None,
            skip_reason: None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExecutionSummary {
    pub total: u64,
    pub successful: u64,
    pub failed: u64,
    pub skipped: u64,
    #[serde(serialize_with = "ser_f64")]
    pub duration: f64,
    pub results: Vec<ExecutionResult>,
}

/// Values stored between sequential requests (`${store.name}`).
pub type ResponseStoreContext = IndexMap<String, String>;
