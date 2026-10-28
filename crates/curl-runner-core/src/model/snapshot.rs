//! Snapshot testing types (`types/snapshot.ts`).

use super::Json;
use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

/// `include` entries: body | status | headers. `match`: path -> `*` | `regex:pattern`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SnapshotConfig {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub enabled: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub include: Option<Vec<String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub exclude: Option<Vec<String>>,
    #[serde(default, rename = "match", skip_serializing_if = "Option::is_none")]
    pub match_rules: Option<IndexMap<String, String>>,
}

/// Request-level `snapshot:` — `true`/`false` or detailed config.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum SnapshotSetting {
    Bool(bool),
    Config(SnapshotConfig),
}

/// `GlobalSnapshotConfig` (extends `SnapshotConfig`). `updateMode`: none|all|failing.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GlobalSnapshotConfig {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub enabled: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub include: Option<Vec<String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub exclude: Option<Vec<String>>,
    #[serde(default, rename = "match", skip_serializing_if = "Option::is_none")]
    pub match_rules: Option<IndexMap<String, String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub dir: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub update_mode: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ci: Option<bool>,
}

/// Field order is the TS key order (`hash`, `updatedAt`, then the included
/// parts): the hash and the snapshot file bytes depend on it. `status` and
/// header values are JSON because stored files are read as untyped JS values
/// (a hand-edited `status: "200"` must compare as a string, not vanish).
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Snapshot {
    #[serde(default)]
    pub hash: String,
    #[serde(default)]
    pub updated_at: String,
    #[serde(
        default,
        deserialize_with = "super::serde_util::keep_null",
        skip_serializing_if = "Option::is_none"
    )]
    pub status: Option<Json>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub headers: Option<IndexMap<String, Json>>,
    #[serde(
        default,
        deserialize_with = "super::serde_util::keep_null",
        skip_serializing_if = "Option::is_none"
    )]
    pub body: Option<Json>,
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SnapshotFile {
    pub version: u64,
    pub snapshots: IndexMap<String, Snapshot>,
}

/// `type`: added | removed | changed | type_mismatch.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SnapshotDiff {
    pub path: String,
    #[serde(
        default,
        deserialize_with = "super::serde_util::keep_null",
        skip_serializing_if = "Option::is_none"
    )]
    pub expected: Option<Json>,
    #[serde(
        default,
        deserialize_with = "super::serde_util::keep_null",
        skip_serializing_if = "Option::is_none"
    )]
    pub received: Option<Json>,
    #[serde(rename = "type")]
    pub diff_type: String,
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SnapshotCompareResult {
    #[serde(rename = "match")]
    pub matches: bool,
    pub is_new: bool,
    pub updated: bool,
    pub differences: Vec<SnapshotDiff>,
}
