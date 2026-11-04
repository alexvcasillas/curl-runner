//! Parsing and `$ref` resolution.

use serde_json::{Map, Value};

use super::swagger2;
use crate::convert::ConvertError;

/// Maximum `$ref` chain length before a reference counts as unresolvable.
const MAX_HOPS: usize = 32;

/// The specification version of the source document.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SpecVersion {
    Swagger2,
    OpenApi30,
    OpenApi31,
}

/// A parsed document. `root` always has the 3.x shape.
#[derive(Debug, Clone)]
pub struct Doc {
    pub root: Value,
    pub version: SpecVersion,
}

/// Outcome of following a `$ref` chain.
#[derive(Debug, Clone, PartialEq)]
pub enum Resolved<'a> {
    Value(&'a Value),
    /// A local reference with no target, or a cycle. Holds the `$ref` text.
    Missing(String),
    /// A reference to another document. Holds the `$ref` text.
    External(String),
}

/// Parse JSON or YAML (detected from the content) into a [`Doc`].
pub fn parse_document(text: &str) -> Result<Doc, ConvertError> {
    let value = if text.trim_start().starts_with('{') {
        serde_json::from_str::<Value>(text).map_err(not_openapi)?
    } else {
        let y = serde_yaml_ng::from_str::<serde_yaml_ng::Value>(text).map_err(not_openapi)?;
        yaml_to_json(y)
    };
    let version = detect_version(&value)?;
    let root = match version {
        SpecVersion::Swagger2 => swagger2::to_v3(value),
        _ => value,
    };
    Ok(Doc { root, version })
}

fn not_openapi(e: impl std::fmt::Display) -> ConvertError {
    ConvertError(format!("Not an OpenAPI document: {e}"))
}

fn detect_version(v: &Value) -> Result<SpecVersion, ConvertError> {
    let (key, found) = match (v.get("openapi"), v.get("swagger")) {
        (Some(f), _) => ("openapi", f),
        (None, Some(f)) => ("swagger", f),
        _ => {
            return Err(ConvertError(
                "Not an OpenAPI document: no \"openapi\" or \"swagger\" field".into(),
            ));
        }
    };
    let text = match found {
        Value::String(s) => s.clone(),
        Value::Number(n) => match n.as_f64() {
            Some(f) if n.is_f64() && f.fract() == 0.0 => format!("{f:.1}"),
            _ => n.to_string(),
        },
        other => other.to_string(),
    };
    let version = match key {
        "swagger" if text == "2.0" => Some(SpecVersion::Swagger2),
        "openapi" if text == "3.0" || text.starts_with("3.0.") => Some(SpecVersion::OpenApi30),
        "openapi" if text == "3.1" || text.starts_with("3.1.") => Some(SpecVersion::OpenApi31),
        _ => None,
    };
    version.ok_or_else(|| {
        ConvertError(format!(
            "Unsupported OpenAPI version: {text}. Use OpenAPI 3.0, 3.1 or Swagger 2.0."
        ))
    })
}

/// Convert YAML to JSON. Non-string keys (`200:`) become string keys.
fn yaml_to_json(v: serde_yaml_ng::Value) -> Value {
    use serde_yaml_ng::Value as Y;
    match v {
        Y::Null => Value::Null,
        Y::Bool(b) => Value::Bool(b),
        Y::Number(n) => serde_json::to_value(n).unwrap_or(Value::Null),
        Y::String(s) => Value::String(s),
        Y::Sequence(s) => Value::Array(s.into_iter().map(yaml_to_json).collect()),
        Y::Mapping(m) => {
            let mut out = Map::new();
            for (k, v) in m {
                out.insert(key_string(k), yaml_to_json(v));
            }
            Value::Object(out)
        }
        Y::Tagged(t) => yaml_to_json(t.value),
    }
}

fn key_string(k: serde_yaml_ng::Value) -> String {
    use serde_yaml_ng::Value as Y;
    match k {
        Y::String(s) => s,
        Y::Bool(b) => b.to_string(),
        Y::Number(n) => n.to_string(),
        Y::Null => "null".into(),
        Y::Tagged(t) => key_string(t.value),
        other => serde_yaml_ng::to_string(&other)
            .map(|s| s.trim().to_string())
            .unwrap_or_default(),
    }
}

impl Doc {
    /// Follow `$ref` chains from `v`. Returns `v` itself when it isn't a reference.
    pub fn resolve<'a>(&'a self, v: &'a Value) -> Resolved<'a> {
        let mut cur = v;
        let mut last = String::new();
        for _ in 0..=MAX_HOPS {
            let Some(r) = cur.get("$ref").and_then(Value::as_str) else {
                return Resolved::Value(cur);
            };
            last = r.to_string();
            let Some(pointer) = r.strip_prefix('#') else {
                return Resolved::External(last);
            };
            match self.lookup(pointer) {
                Some(next) => cur = next,
                None => return Resolved::Missing(last),
            }
        }
        Resolved::Missing(last)
    }

    fn lookup(&self, pointer: &str) -> Option<&Value> {
        let mut cur = &self.root;
        if pointer.is_empty() {
            return Some(cur);
        }
        for raw in pointer.strip_prefix('/')?.split('/') {
            let token = percent_decode(raw).replace("~1", "/").replace("~0", "~");
            cur = match cur {
                Value::Object(m) => m.get(&token)?,
                Value::Array(a) => a.get(token.parse::<usize>().ok()?)?,
                _ => return None,
            };
        }
        Some(cur)
    }
}

fn percent_decode(s: &str) -> String {
    if !s.contains('%') {
        return s.to_string();
    }
    let b = s.as_bytes();
    let mut out = Vec::with_capacity(b.len());
    let mut i = 0;
    while i < b.len() {
        if b[i] == b'%'
            && let Some(hex) = s.get(i + 1..i + 3)
            && let Ok(byte) = u8::from_str_radix(hex, 16)
        {
            out.push(byte);
            i += 3;
        } else {
            out.push(b[i]);
            i += 1;
        }
    }
    String::from_utf8_lossy(&out).into_owned()
}
