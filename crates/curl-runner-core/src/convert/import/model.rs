//! The format-neutral model an importer maps into and the emitter writes.

use super::report::ImportReport;
use serde_json::Value;
use std::fmt;

/// A whole collection, mapped.
#[derive(Debug)]
pub struct Mapped {
    pub requests: Vec<MappedRequest>,
    pub variables: Vec<ImportedVar>,
    pub description: Option<String>,
    /// `.env` comment lines, each written above its variable's key when
    /// the import adds that key. In order; a variable may have several.
    pub env_comments: Vec<(String, String)>,
    pub report: ImportReport,
}

#[derive(Debug, Clone, PartialEq)]
pub struct MappedRequest {
    /// Raw folder names from the collection root down.
    pub folders: Vec<String>,
    pub name: String,
    /// Description lines: the request's first, then its folders' from
    /// nearest to farthest.
    pub comments: Vec<String>,
    pub method: String,
    pub url: String,
    pub headers: Vec<Field>,
    pub params: Vec<Field>,
    pub body: MappedBody,
    pub auth: Option<MappedAuth>,
    /// Path variables, for the file's `global.variables`.
    pub file_vars: Vec<(String, String)>,
    /// Status the request is expected to return, written as `expect.status`.
    pub expect_status: Option<u16>,
}

impl MappedRequest {
    /// Folders and name joined with `/`, as the report names it.
    pub fn path(&self) -> String {
        let mut p = self.folders.join("/");
        if !p.is_empty() {
            p.push('/');
        }
        p.push_str(&self.name);
        p
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Field {
    pub key: String,
    pub value: FieldValue,
    pub disabled: bool,
    pub note: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum FieldValue {
    Text(String),
    File { path: String, placeholder: bool },
}

#[derive(Debug, Clone, PartialEq)]
pub enum MappedBody {
    None,
    Json(Value),
    Text(String),
    Form(Vec<Field>),
}

#[derive(Debug, Clone, PartialEq)]
pub enum MappedAuth {
    Bearer(String),
    Basic { username: String, password: String },
}

/// A variable for `.env`. Debug hides the value.
#[derive(Clone, PartialEq)]
pub struct ImportedVar {
    pub name: String,
    pub value: String,
    pub secret: bool,
}

impl fmt::Debug for ImportedVar {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ImportedVar")
            .field("name", &self.name)
            .field("value", &"<hidden>")
            .field("secret", &self.secret)
            .finish()
    }
}
