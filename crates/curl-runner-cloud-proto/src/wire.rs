use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::bundle::BundleFile;
use crate::error::ErrorCode;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Source {
    Git,
    Cloud,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Access {
    Off,
    Public,
    Restricted,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Role {
    Owner,
    Member,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Plan {
    Free,
    Team,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct Input {
    pub name: String,
    pub description: Option<String>,
    pub secret: bool,
    pub required: bool,
    pub locked: bool,
    pub default: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct GitMeta {
    pub commit: Option<String>,
    #[serde(rename = "ref")]
    pub git_ref: Option<String>,
    pub repo_url: Option<String>,
}

/// Sent gzipped (`Content-Encoding: gzip`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct PublishRequest {
    pub files: Vec<BundleFile>,
    pub hash: String,
    pub inputs: Vec<Input>,
    pub name: Option<String>,
    pub description: Option<String>,
    pub git: Option<GitMeta>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PublishStatus {
    Created,
    Unchanged,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct FileChanges {
    pub added: Vec<String>,
    pub changed: Vec<String>,
    pub removed: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct PublishResponse {
    pub collection: String,
    pub version: u32,
    pub status: PublishStatus,
    pub changed: FileChanges,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct VersionSummary {
    pub number: u32,
    pub hash: String,
    pub published_at: DateTime<Utc>,
    pub publisher: String,
    pub git: Option<GitMeta>,
    pub file_count: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct SharedInfo {
    pub team: String,
    /// The team's slug, which the studio deep link needs. Absent from older servers.
    #[serde(default)]
    pub team_slug: String,
    pub collection: String,
    pub name: String,
    pub description: Option<String>,
    pub version: u32,
    pub updated_at: DateTime<Utc>,
    pub request_count: u32,
    pub access: Access,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct Snapshot {
    pub info: SharedInfo,
    pub files: Vec<BundleFile>,
    pub inputs: Vec<Input>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SignInMethod {
    Email,
    Github,
    Google,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SignupMode {
    Open,
    Domains,
    Closed,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct ServerInfo {
    pub version: String,
    pub sign_in: Vec<SignInMethod>,
    pub signup: SignupMode,
    pub billing: bool,
    /// The dashboard's public URL. Absent from servers that predate it.
    #[serde(default)]
    pub app_url: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct ErrorBody {
    pub error: ErrorCode,
    pub message: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn publish_request_roundtrip() {
        let req = PublishRequest {
            files: vec![BundleFile {
                path: "a.yaml".into(),
                text: "x".into(),
            }],
            hash: "h".into(),
            inputs: vec![],
            name: None,
            description: None,
            git: Some(GitMeta {
                commit: Some("abc".into()),
                git_ref: Some("main".into()),
                repo_url: None,
            }),
        };
        let v = serde_json::to_value(&req).unwrap();
        assert_eq!(v["git"]["ref"], "main");
        assert!(v["git"].get("git_ref").is_none());
        let back: PublishRequest = serde_json::from_value(v).unwrap();
        assert_eq!(back, req);
    }
}
