//! The `cloud:` block of the collection's config file.

use std::path::Path;

use curl_runner_cloud_proto::Input;
use serde::Deserialize;

use crate::config::CONFIG_FILE_PATHS;
use crate::model::Json;
use crate::parser::{self, ParseError};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CloudConfig {
    pub collection: String,
    pub name: Option<String>,
    pub description: Option<String>,
    pub inputs: Vec<Input>,
}

#[derive(Debug, thiserror::Error)]
pub enum CloudConfigError {
    #[error(transparent)]
    Parse(#[from] ParseError),
    #[error("invalid cloud.collection `{0}`: use lowercase letters, digits and dashes (max 63)")]
    BadSlug(String),
    #[error("invalid cloud block: {0}")]
    Invalid(String),
}

#[derive(Deserialize)]
struct RawCloud {
    collection: String,
    name: Option<String>,
    description: Option<String>,
    #[serde(default)]
    inputs: Vec<RawInput>,
}

#[derive(Deserialize)]
struct RawInput {
    name: String,
    description: Option<String>,
    secret: Option<bool>,
    required: Option<bool>,
    locked: Option<bool>,
    default: Option<Json>,
}

/// `^[a-z0-9][a-z0-9-]{0,62}$`
fn valid_slug(s: &str) -> bool {
    let b = s.as_bytes();
    let ok = |c: u8| c.is_ascii_lowercase() || c.is_ascii_digit();
    (1..=63).contains(&b.len()) && ok(b[0]) && b.iter().all(|&c| ok(c) || c == b'-')
}

fn scalar_text(v: &Json) -> Option<String> {
    match v {
        Json::String(s) => Some(s.clone()),
        Json::Number(n) => Some(n.to_string()),
        Json::Bool(b) => Some(b.to_string()),
        _ => None,
    }
}

/// The `cloud:` block of the first config file in the order of
/// [`CONFIG_FILE_PATHS`] that exists, or `None` when that file has none.
pub fn load_cloud_config(dir: &Path) -> Result<Option<CloudConfig>, CloudConfigError> {
    let Some(path) = CONFIG_FILE_PATHS
        .iter()
        .map(|n| dir.join(n))
        .find(|p| p.is_file())
    else {
        return Ok(None);
    };
    let doc = parser::load_raw(&path)?;
    let Some(block) = doc.get("cloud").filter(|v| !v.is_null()) else {
        return Ok(None);
    };
    let raw: RawCloud = serde_path_to_error::deserialize(block.clone())
        .map_err(|e| CloudConfigError::Invalid(format!("{}: {}", e.path(), e.inner())))?;
    if !valid_slug(&raw.collection) {
        return Err(CloudConfigError::BadSlug(raw.collection));
    }
    let variable = |name: &str| {
        ["collection", "global"]
            .iter()
            .find_map(|s| doc.get(s)?.get("variables")?.get(name))
            .and_then(scalar_text)
    };
    let mut inputs = Vec::with_capacity(raw.inputs.len());
    for i in raw.inputs {
        let secret = i.secret.unwrap_or_else(|| i.name.starts_with("SECRET_"));
        let default = match i.default {
            Some(v) => Some(scalar_text(&v).ok_or_else(|| {
                CloudConfigError::Invalid(format!("input {}: default must be a scalar", i.name))
            })?),
            None if !secret => variable(&i.name),
            None => None,
        };
        inputs.push(Input {
            name: i.name,
            description: i.description,
            secret,
            required: i.required.unwrap_or(false),
            locked: i.locked.unwrap_or(false),
            default,
        });
    }
    Ok(Some(CloudConfig {
        collection: raw.collection,
        name: raw.name,
        description: raw.description,
        inputs,
    }))
}
