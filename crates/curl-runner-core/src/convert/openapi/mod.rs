//! OpenAPI 3.0/3.1 and Swagger 2.0 importer.

mod auth;
mod body;
pub mod doc;
pub mod map;
pub mod sample;
mod swagger2;

pub use doc::{Doc, Resolved, SpecVersion, parse_document};
pub use map::{api_prefix, map_document};

use super::ConvertError;
use super::import::{Import, plan_import};

/// Parse, map and emit an OpenAPI 3.0/3.1 or Swagger 2.0 document.
pub fn convert_openapi(text: &str) -> Result<Import, ConvertError> {
    let doc = parse_document(text)?;
    let name = doc.root["info"]["title"]
        .as_str()
        .map(str::trim)
        .filter(|t| !t.is_empty())
        .unwrap_or("OpenAPI import")
        .to_string();
    Ok(plan_import(name, map_document(&doc), "OpenAPI"))
}

#[cfg(test)]
mod tests;
