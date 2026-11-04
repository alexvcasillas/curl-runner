//! Parse and plan a Postman collection's files.

use super::{map_collection, parse_collection};
use crate::convert::ConvertError;
use crate::convert::import::{Import, plan_import};

/// Parse, map and emit a Postman collection.
pub fn convert_postman(json: &str) -> Result<Import, ConvertError> {
    let collection = parse_collection(json)?;
    let name = collection.info.name.clone();
    Ok(plan_import(name, map_collection(&collection), "Postman"))
}
