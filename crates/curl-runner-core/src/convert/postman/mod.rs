//! Postman collection importer.

pub mod model;

use super::ConvertError;
pub use model::Collection;
pub use model::{
    Auth, Body, Description, Event, Folder, Info, Item, KeyValue, Request, RequestItem, Url,
    Variable,
};

/// Parse a Postman Collection v2.0/v2.1 export.
pub fn parse_collection(json: &str) -> Result<Collection, ConvertError> {
    let not = |e: &dyn std::fmt::Display| ConvertError(format!("Not a Postman collection: {e}"));
    let value: serde_json::Value = serde_json::from_str(json).map_err(|e| not(&e))?;
    if value.get("info").is_none() || value.get("item").is_none() {
        return Err(not(&"missing \"info\" or \"item\""));
    }
    let schema = value["info"]["schema"].as_str().unwrap_or("");
    if !(schema.contains("/v2.0.") || schema.contains("/v2.1.")) {
        return Err(ConvertError(format!(
            "Unsupported Postman collection schema: {schema}. Export it as Collection v2.1 from Postman."
        )));
    }
    serde_json::from_value(value).map_err(|e| not(&e))
}

#[cfg(test)]
mod tests;
