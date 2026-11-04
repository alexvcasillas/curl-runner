//! OpenAPI 3.0/3.1 and Swagger 2.0 importer.

pub mod doc;
mod swagger2;

pub use doc::{Doc, Resolved, SpecVersion, parse_document};

#[cfg(test)]
mod tests;
