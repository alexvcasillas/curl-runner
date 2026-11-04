//! OpenAPI 3.0/3.1 and Swagger 2.0 importer.

mod auth;
mod body;
pub mod doc;
pub mod map;
pub mod sample;
mod swagger2;

pub use doc::{Doc, Resolved, SpecVersion, parse_document};
pub use map::{api_prefix, map_document};

#[cfg(test)]
mod tests;
