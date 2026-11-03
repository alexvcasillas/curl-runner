//! Serde types for Postman Collection v2.0/v2.1. Everything defaults and
//! unknown fields are ignored, so real exports parse as-is.

use serde::{Deserialize, Deserializer};
use serde_json::{Map, Value};
use std::fmt;

#[derive(Debug, Default, Deserialize)]
#[serde(default)]
pub struct Collection {
    pub info: Info,
    pub item: Vec<Item>,
    pub auth: Option<Auth>,
    pub variable: Vec<Variable>,
    pub event: Vec<Event>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default)]
pub struct Info {
    pub name: String,
    pub schema: String,
    pub description: Option<Description>,
}

/// A folder when it has `item`, otherwise a request.
#[derive(Debug, Deserialize)]
#[serde(untagged)]
#[allow(clippy::large_enum_variant)] // parsed once, read-only; boxing would only add noise
pub enum Item {
    Folder(Folder),
    Request(RequestItem),
}

#[derive(Debug, Deserialize)]
pub struct Folder {
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub description: Option<Description>,
    pub item: Vec<Item>,
    #[serde(default)]
    pub auth: Option<Auth>,
    #[serde(default)]
    pub event: Vec<Event>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default)]
pub struct RequestItem {
    pub name: String,
    #[serde(deserialize_with = "de_request")]
    pub request: Request,
    pub event: Vec<Event>,
    pub response: Vec<Value>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default)]
pub struct Request {
    pub method: String,
    pub header: Vec<KeyValue>,
    pub url: Url,
    pub body: Option<Body>,
    pub auth: Option<Auth>,
    pub description: Option<Description>,
}

/// Postman also allows `request` to be a bare URL string.
fn de_request<'de, D: Deserializer<'de>>(d: D) -> Result<Request, D::Error> {
    #[derive(Deserialize)]
    #[serde(untagged)]
    enum Raw {
        Url(String),
        Full(Box<Request>),
    }
    Ok(match Raw::deserialize(d)? {
        Raw::Url(u) => Request {
            url: Url::Raw(u),
            ..Request::default()
        },
        Raw::Full(r) => *r,
    })
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
pub enum Url {
    Raw(String),
    Object {
        #[serde(default)]
        raw: Option<String>,
        #[serde(default)]
        host: Option<Value>,
        #[serde(default)]
        path: Option<Value>,
        #[serde(default)]
        query: Vec<KeyValue>,
        #[serde(default)]
        variable: Vec<KeyValue>,
    },
}

impl Default for Url {
    fn default() -> Self {
        Url::Raw(String::new())
    }
}

#[derive(Debug, Default, Deserialize)]
#[serde(default)]
pub struct KeyValue {
    pub key: String,
    #[serde(deserialize_with = "de_lenient_string")]
    pub value: Option<String>,
    pub disabled: bool,
    pub description: Option<Description>,
    /// `type` in the JSON: `"file"` or `"text"`.
    #[serde(rename = "type")]
    pub kind: Option<String>,
    pub src: Option<Value>,
}

/// Values are strings in practice, but numbers and booleans do occur.
fn de_lenient_string<'de, D: Deserializer<'de>>(d: D) -> Result<Option<String>, D::Error> {
    Ok(match Option::<Value>::deserialize(d)? {
        None | Some(Value::Null) => None,
        Some(Value::String(s)) => Some(s),
        Some(other) => Some(other.to_string()),
    })
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
pub enum Description {
    Text(String),
    Rich {
        #[serde(default)]
        content: String,
    },
}

impl Description {
    pub fn text(&self) -> &str {
        match self {
            Description::Text(s) => s,
            Description::Rich { content } => content,
        }
    }
}

#[derive(Debug, Default, Deserialize)]
#[serde(default)]
pub struct Body {
    pub mode: String,
    pub raw: Option<String>,
    pub urlencoded: Vec<KeyValue>,
    pub formdata: Vec<KeyValue>,
    pub graphql: Option<Value>,
    pub options: Option<Value>,
}

#[derive(Debug, Default, Deserialize)]
pub struct Auth {
    #[serde(rename = "type", default)]
    pub kind: String,
    #[serde(flatten)]
    pub rest: Map<String, Value>,
}

impl Auth {
    /// The settings Postman keeps under the auth's own type: a
    /// `[{key, value}]` array in v2.1, a `{key: value}` object in v2.0.
    pub fn params(&self) -> Vec<(String, String)> {
        let text = |v: Option<&Value>| match v {
            Some(Value::String(s)) => s.clone(),
            Some(Value::Null) | None => String::new(),
            Some(other) => other.to_string(),
        };
        match self.rest.get(&self.kind) {
            Some(Value::Array(items)) => items
                .iter()
                .filter_map(|p| Some((p.get("key")?.as_str()?.to_string(), text(p.get("value")))))
                .collect(),
            Some(Value::Object(map)) => map
                .iter()
                .map(|(k, v)| (k.clone(), text(Some(v))))
                .collect(),
            _ => Vec::new(),
        }
    }
}

#[derive(Default, Deserialize)]
#[serde(default)]
pub struct Variable {
    pub key: String,
    pub value: Option<Value>,
    pub secret: bool,
}

impl fmt::Debug for Variable {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Variable")
            .field("key", &self.key)
            .field("value", &"<hidden>")
            .field("secret", &self.secret)
            .finish()
    }
}

#[derive(Debug, Default, Deserialize)]
#[serde(default)]
pub struct Event {
    pub listen: String,
    pub script: Option<Script>,
}

impl Event {
    /// The script has something besides whitespace.
    pub fn has_code(&self) -> bool {
        self.script
            .as_ref()
            .is_some_and(|s| s.exec.iter().any(|l| !l.trim().is_empty()))
    }
}

#[derive(Debug, Default, Deserialize)]
#[serde(default)]
pub struct Script {
    /// One string or a list of lines in the JSON.
    #[serde(deserialize_with = "de_lines")]
    pub exec: Vec<String>,
}

fn de_lines<'de, D: Deserializer<'de>>(d: D) -> Result<Vec<String>, D::Error> {
    Ok(match Option::<Value>::deserialize(d)? {
        Some(Value::String(s)) => vec![s],
        Some(Value::Array(a)) => a
            .into_iter()
            .filter_map(|v| match v {
                Value::String(s) => Some(s),
                _ => None,
            })
            .collect(),
        _ => Vec::new(),
    })
}
