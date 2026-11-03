//! Postman requests turned into an ordered, format-neutral model. The
//! emitter writes it as YAML; this module decides what each part becomes.

use super::model::{
    Auth, Body, Collection, Description, Event, Folder, Item, KeyValue, RequestItem, Url,
};
use super::report::{Attention, ImportReport};
use crate::convert::{SECRET_PARAMS, header_var, param_var, secret_header};
use serde_json::Value;
use std::fmt;

/// A whole collection, mapped.
#[derive(Debug)]
pub struct Mapped {
    pub requests: Vec<MappedRequest>,
    pub variables: Vec<ImportedVar>,
    pub description: Option<String>,
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

/// Path of a file field Postman's export left without one.
const PLACEHOLDER_FILE: &str = "./pick-a-file";

/// Names that curl-runner reads as dynamic variables.
const RESERVED_PREFIXES: [&str; 6] = [
    "DATE",
    "TIME",
    "UUID",
    "RANDOM",
    "CURRENT_TIME",
    "TIMESTAMP",
];

/// A Postman variable name as curl-runner can use it: only
/// `[A-Za-z0-9_]`, and `_VAR` after a name curl-runner treats specially.
pub fn var_name(name: &str) -> String {
    let mut out: String = name
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '_' {
                c
            } else {
                '_'
            }
        })
        .collect();
    if name.starts_with("store.") || RESERVED_PREFIXES.iter().any(|p| out.starts_with(p)) {
        out.push_str("_VAR");
    }
    out
}

pub fn map_collection(c: &Collection) -> Mapped {
    let mut m = Mapper {
        collection_auth: c.auth.as_ref(),
        ..Mapper::default()
    };
    for v in &c.variable {
        if v.key.is_empty() {
            continue;
        }
        let name = m.rename(&v.key);
        if m.vars.iter().any(|x| x.name == name) {
            continue;
        }
        let value = match &v.value {
            None | Some(Value::Null) => String::new(),
            Some(Value::String(s)) => s.clone(),
            Some(other) => other.to_string(),
        };
        m.vars.push(ImportedVar {
            name,
            value,
            secret: v.secret,
        });
    }
    m.scripts(&c.event, String::new());
    m.walk(&c.item, &mut Vec::new());
    let description = c
        .info
        .description
        .as_ref()
        .map(|d| d.text().trim().to_string())
        .filter(|d| !d.is_empty());
    m.finish(description)
}

/// Which scrubbing rule a field's value follows.
#[derive(Clone, Copy)]
enum Kind {
    Header,
    /// Query params and form text fields.
    Param,
}

#[derive(Default)]
struct Mapper<'c> {
    collection_auth: Option<&'c Auth>,
    vars: Vec<ImportedVar>,
    /// Variables a credential references.
    secret_names: Vec<String>,
    /// Referenced variables, in first-use order, path variables left out.
    refs: Vec<String>,
    attention: Vec<Attention>,
    folders: usize,
    requests: Vec<MappedRequest>,
    /// Path of the request being mapped.
    at: String,
    req_refs: Vec<String>,
}

impl<'c> Mapper<'c> {
    /// One `Script` per event that has code.
    fn scripts(&mut self, events: &[Event], at: String) {
        for e in events.iter().filter(|e| e.has_code()) {
            self.attention.push(Attention::Script {
                at: at.clone(),
                listen: e.listen.clone(),
            });
        }
    }

    fn walk(&mut self, items: &'c [Item], stack: &mut Vec<&'c Folder>) {
        for item in items {
            match item {
                Item::Folder(f) => {
                    self.folders += 1;
                    stack.push(f);
                    let at = stack.iter().map(|f| f.name.as_str()).collect::<Vec<_>>();
                    self.scripts(&f.event, at.join("/"));
                    self.walk(&f.item, stack);
                    stack.pop();
                }
                Item::Request(r) => {
                    let mapped = self.request(r, stack);
                    self.requests.push(mapped);
                }
            }
        }
    }

    fn request(&mut self, item: &RequestItem, folders: &[&'c Folder]) -> MappedRequest {
        let r = &item.request;
        let mut out = MappedRequest {
            folders: folders.iter().map(|f| f.name.clone()).collect(),
            name: item.name.clone(),
            comments: Vec::new(),
            method: if r.method.is_empty() {
                "GET".into()
            } else {
                r.method.to_ascii_uppercase()
            },
            url: String::new(),
            headers: Vec::new(),
            params: Vec::new(),
            body: MappedBody::None,
            auth: None,
            file_vars: Vec::new(),
        };
        self.at = out.path();
        self.req_refs.clear();
        self.scripts(&item.event, self.at.clone());
        if !item.response.is_empty() {
            self.attention.push(Attention::Examples {
                at: self.at.clone(),
                count: item.response.len(),
            });
        }
        out.comments = description_lines(r.description.as_ref());
        for f in folders.iter().rev() {
            out.comments
                .extend(description_lines(f.description.as_ref()));
        }

        let chosen = r
            .auth
            .iter()
            .chain(folders.iter().rev().filter_map(|f| f.auth.as_ref()))
            .chain(self.collection_auth)
            .find(|a| !a.kind.is_empty() && a.kind != "inherit");
        out.auth = chosen.and_then(|a| self.auth(a, &mut out.headers, &mut out.params));

        let (url, query, file_vars) = self.url(&r.url);
        out.url = url;
        out.params.extend(query);
        out.file_vars = file_vars;
        for kv in &r.header {
            let field = self.kv_field(kv, Kind::Header);
            out.headers.push(field);
        }
        let (body, content_type) = self.body(r.body.as_ref(), &mut out.comments);
        out.body = body;
        if let Some(ct) = content_type
            && !out
                .headers
                .iter()
                .any(|h| !h.disabled && h.key.eq_ignore_ascii_case("content-type"))
        {
            out.headers.push(Field {
                key: "Content-Type".into(),
                value: FieldValue::Text(ct.into()),
                disabled: false,
                note: None,
            });
        }

        for name in std::mem::take(&mut self.req_refs) {
            if !out.file_vars.iter().any(|(n, _)| *n == name) && !self.refs.contains(&name) {
                self.refs.push(name);
            }
        }
        out
    }

    /// Bearer and basic come back as auth; an API key is added to
    /// `headers` or `params`.
    fn auth(
        &mut self,
        a: &Auth,
        headers: &mut Vec<Field>,
        params: &mut Vec<Field>,
    ) -> Option<MappedAuth> {
        let p = a.params();
        let get = |k: &str| {
            p.iter()
                .find(|(key, _)| key == k)
                .map(|(_, v)| v.as_str())
                .unwrap_or("")
        };
        match a.kind.as_str() {
            "noauth" => None,
            "apikey" => {
                let key = get("key");
                if key.is_empty() {
                    return None;
                }
                let (base, list) = if get("in") == "query" {
                    (param_var(key), params)
                } else {
                    (header_var(key), headers)
                };
                let value = self.credential(&base, get("value"));
                list.push(Field {
                    key: self.subst(key),
                    value: FieldValue::Text(value),
                    disabled: false,
                    note: None,
                });
                None
            }
            "bearer" => Some(MappedAuth::Bearer(
                self.credential("API_TOKEN", get("token")),
            )),
            "basic" => Some(MappedAuth::Basic {
                username: self.credential("API_USER", get("username")),
                password: self.credential("API_PASSWORD", get("password")),
            }),
            other => {
                self.attention.push(Attention::UnsupportedAuth {
                    at: self.at.clone(),
                    kind: other.to_string(),
                });
                None
            }
        }
    }

    /// The URL without its query, the query as params, and path variables.
    fn url(&mut self, u: &Url) -> (String, Vec<Field>, Vec<(String, String)>) {
        let (raw, query, vars): (String, &[KeyValue], &[KeyValue]) = match u {
            Url::Raw(s) => (s.clone(), &[], &[]),
            Url::Object {
                raw,
                host,
                path,
                query,
                variable,
            } => {
                let raw = match raw.as_deref() {
                    Some(r) if !r.is_empty() => r.to_string(),
                    _ => {
                        let host = join(host.as_ref(), ".");
                        let path = join(path.as_ref(), "/");
                        if path.is_empty() || path.starts_with('/') {
                            format!("{host}{path}")
                        } else {
                            format!("{host}/{path}")
                        }
                    }
                };
                (raw, query, variable)
            }
        };
        let (base, qs) = match raw.split_once('?') {
            Some((b, q)) => (b, Some(q)),
            None => (raw.as_str(), None),
        };
        let (base, file_vars) = self.path_vars(base, vars);
        let url = self.subst(&base);
        let params = if !query.is_empty() {
            query
                .iter()
                .map(|kv| self.kv_field(kv, Kind::Param))
                .collect()
        } else {
            qs.unwrap_or("")
                .split('&')
                .filter(|p| !p.is_empty())
                .map(|pair| {
                    let (k, v) = pair.split_once('=').unwrap_or((pair, ""));
                    self.field(k, v, false, None, Kind::Param)
                })
                .collect()
        };
        (url, params, file_vars)
    }

    /// `:id` path segments as `${id}`, with each value from `vars`.
    fn path_vars(&mut self, url: &str, vars: &[KeyValue]) -> (String, Vec<(String, String)>) {
        let after_scheme = url.find("://").map_or(0, |i| i + 3);
        let Some(slash) = url[after_scheme..].find('/').map(|i| after_scheme + i) else {
            return (url.to_string(), Vec::new());
        };
        let mut file_vars = Vec::new();
        let mut segments = Vec::new();
        for seg in url[slash..].split('/') {
            let Some(rest) = seg.strip_prefix(':') else {
                segments.push(seg.to_string());
                continue;
            };
            let end = rest
                .find(|c: char| !(c.is_ascii_alphanumeric() || c == '_' || c == '-'))
                .unwrap_or(rest.len());
            if end == 0 {
                segments.push(seg.to_string());
                continue;
            }
            let raw_name = &rest[..end];
            let name = self.rename(raw_name);
            let value = vars
                .iter()
                .find(|v| v.key == raw_name)
                .and_then(|v| v.value.as_deref())
                .unwrap_or("");
            let value = self.subst(value);
            if !file_vars.iter().any(|(n, _)| *n == name) {
                file_vars.push((name.clone(), value));
            }
            segments.push(format!("${{{name}}}{}", &rest[end..]));
        }
        (
            format!("{}{}", &url[..slash], segments.join("/")),
            file_vars,
        )
    }

    fn body(
        &mut self,
        body: Option<&Body>,
        comments: &mut Vec<String>,
    ) -> (MappedBody, Option<&'static str>) {
        let Some(b) = body else {
            return (MappedBody::None, None);
        };
        match b.mode.as_str() {
            "" => (MappedBody::None, None),
            "raw" => {
                let raw = b.raw.as_deref().unwrap_or("");
                if raw.is_empty() {
                    return (MappedBody::None, None);
                }
                let language = b
                    .options
                    .as_ref()
                    .and_then(|o| o.get("raw")?.get("language")?.as_str())
                    .unwrap_or("");
                let content_type = match language {
                    "json" => Some("application/json"),
                    "xml" => Some("application/xml"),
                    "html" => Some("text/html"),
                    "text" => Some("text/plain"),
                    "javascript" => Some("application/javascript"),
                    _ => None,
                };
                let text = self.subst(raw);
                let body = match serde_json::from_str::<Value>(&text) {
                    Ok(v @ (Value::Object(_) | Value::Array(_))) => MappedBody::Json(v),
                    _ => MappedBody::Text(text),
                };
                (body, content_type)
            }
            "urlencoded" => {
                let mut parts = Vec::new();
                for kv in &b.urlencoded {
                    let key = self.subst(&kv.key);
                    let value = self.value(&kv.key, kv.value.as_deref().unwrap_or(""), Kind::Param);
                    if kv.disabled {
                        comments.push(format!("Disabled form field: {key}={value}"));
                    } else {
                        parts.push(format!("{}={}", form_encode(&key), form_encode(&value)));
                    }
                }
                if parts.is_empty() {
                    (MappedBody::None, None)
                } else {
                    (
                        MappedBody::Text(parts.join("&")),
                        Some("application/x-www-form-urlencoded"),
                    )
                }
            }
            "formdata" => {
                let mut fields = Vec::new();
                for kv in &b.formdata {
                    let key = self.subst(&kv.key);
                    let value = if kv.kind.as_deref() == Some("file") {
                        let src = match &kv.src {
                            Some(Value::String(s)) => Some(s.as_str()),
                            Some(Value::Array(a)) => a.iter().find_map(Value::as_str),
                            _ => None,
                        }
                        .filter(|s| !s.is_empty());
                        if src.is_none() && !kv.disabled {
                            self.attention.push(Attention::PickFile {
                                request: self.at.clone(),
                                field: key.clone(),
                            });
                        }
                        FieldValue::File {
                            path: src.unwrap_or(PLACEHOLDER_FILE).to_string(),
                            placeholder: src.is_none(),
                        }
                    } else {
                        FieldValue::Text(self.value(
                            &kv.key,
                            kv.value.as_deref().unwrap_or(""),
                            Kind::Param,
                        ))
                    };
                    fields.push(Field {
                        key,
                        value,
                        disabled: kv.disabled,
                        note: note(kv.description.as_ref()),
                    });
                }
                if fields.is_empty() {
                    (MappedBody::None, None)
                } else {
                    (MappedBody::Form(fields), None)
                }
            }
            "graphql" => {
                let g = b.graphql.as_ref();
                let query = g
                    .and_then(|g| g.get("query"))
                    .and_then(Value::as_str)
                    .unwrap_or("");
                let query = self.subst(query);
                let variables = match g.and_then(|g| g.get("variables")) {
                    Some(Value::String(s)) if !s.trim().is_empty() => {
                        let s = self.subst(s);
                        serde_json::from_str(&s).unwrap_or(Value::String(s))
                    }
                    Some(v @ (Value::Object(_) | Value::Array(_))) => v.clone(),
                    _ => Value::Object(Default::default()),
                };
                (
                    MappedBody::Json(serde_json::json!({ "query": query, "variables": variables })),
                    Some("application/json"),
                )
            }
            mode => {
                self.attention.push(Attention::UnsupportedBody {
                    at: self.at.clone(),
                    mode: mode.to_string(),
                });
                (MappedBody::None, None)
            }
        }
    }

    fn kv_field(&mut self, kv: &KeyValue, kind: Kind) -> Field {
        self.field(
            &kv.key,
            kv.value.as_deref().unwrap_or(""),
            kv.disabled,
            kv.description.as_ref(),
            kind,
        )
    }

    /// A header or param, with a literal secret value moved into a variable.
    fn field(
        &mut self,
        key: &str,
        value: &str,
        disabled: bool,
        description: Option<&Description>,
        kind: Kind,
    ) -> Field {
        let value = self.value(key, value, kind);
        Field {
            key: self.subst(key),
            value: FieldValue::Text(value),
            disabled,
            note: note(description),
        }
    }

    /// `value` with its variables replaced, or a `${VAR}` holding it when
    /// `key` names a secret.
    fn value(&mut self, key: &str, value: &str, kind: Kind) -> String {
        let secret = match kind {
            Kind::Header => secret_header(key),
            Kind::Param => SECRET_PARAMS.iter().any(|p| p.eq_ignore_ascii_case(key)),
        };
        if !secret || value.is_empty() {
            return self.subst(value);
        }
        if value.contains("{{") {
            return self.credential("", value);
        }
        match kind {
            Kind::Param => self.secret_literal(&param_var(key), value),
            Kind::Header if key.eq_ignore_ascii_case("authorization") => {
                // Keep the scheme: `Bearer abc` → `Bearer ${API_TOKEN}`.
                match value.split_once(' ') {
                    Some((scheme, cred)) => {
                        format!("{scheme} {}", self.secret_literal("API_TOKEN", cred.trim()))
                    }
                    None => self.secret_literal("API_TOKEN", value),
                }
            }
            Kind::Header => self.secret_literal(&header_var(key), value),
        }
    }

    /// A credential: its variables become secrets, and a literal becomes a
    /// new secret variable named `base`.
    fn credential(&mut self, base: &str, value: &str) -> String {
        if !value.contains("{{") {
            return self.secret_literal(base, value);
        }
        replace_refs(value, |inner| {
            if !inner.starts_with('$') {
                let name = var_name(inner);
                if !self.secret_names.contains(&name) {
                    self.secret_names.push(name);
                }
            }
            None
        });
        self.subst(value)
    }

    /// `${base}` (or `${base_2}`, … when a variable already holds another
    /// value), with `value` stored as a secret.
    fn secret_literal(&mut self, base: &str, value: &str) -> String {
        let name = (1..)
            .map(|n| {
                if n == 1 {
                    base.to_string()
                } else {
                    format!("{base}_{n}")
                }
            })
            .find(|n| {
                self.vars
                    .iter()
                    .find(|v| v.name == *n)
                    .is_none_or(|v| v.value == value)
            })
            .expect("an unbounded range finds a free name");
        match self.vars.iter_mut().find(|v| v.name == name) {
            Some(v) => v.secret = true,
            None => self.vars.push(ImportedVar {
                name: name.clone(),
                value: value.to_string(),
                secret: true,
            }),
        }
        format!("${{{name}}}")
    }

    /// `{{x}}` as `${x'}`; Postman's dynamic variables as curl-runner's.
    fn subst(&mut self, s: &str) -> String {
        replace_refs(s, |inner| self.reference(inner))
    }

    fn reference(&mut self, inner: &str) -> Option<String> {
        if let Some(dynamic) = inner.strip_prefix('$') {
            let to = match dynamic {
                "guid" | "randomUUID" => "UUID",
                // `$timestamp` is in seconds, curl-runner's TIMESTAMP in
                // milliseconds: it stays as written.
                "randomInt" => "RANDOM:0-1000",
                _ => {
                    let a = Attention::DynamicVar {
                        at: self.at.clone(),
                        name: inner.to_string(),
                    };
                    if !self.attention.contains(&a) {
                        self.attention.push(a);
                    }
                    return None;
                }
            };
            return Some(format!("${{{to}}}"));
        }
        let name = self.rename(inner);
        if !self.req_refs.contains(&name) {
            self.req_refs.push(name.clone());
        }
        Some(format!("${{{name}}}"))
    }

    /// `var_name`, reporting each rename once.
    fn rename(&mut self, from: &str) -> String {
        let to = var_name(from);
        if to != from
            && !self
                .attention
                .iter()
                .any(|a| matches!(a, Attention::Renamed { from: f, .. } if f == from))
        {
            self.attention.push(Attention::Renamed {
                from: from.to_string(),
                to: to.clone(),
            });
        }
        to
    }

    fn finish(mut self, description: Option<String>) -> Mapped {
        for name in std::mem::take(&mut self.secret_names) {
            match self.vars.iter_mut().find(|v| v.name == name) {
                Some(v) => v.secret = true,
                None => self.vars.push(ImportedVar {
                    name,
                    value: String::new(),
                    secret: true,
                }),
            }
        }
        for name in &self.refs {
            if !self.vars.iter().any(|v| v.name == *name) {
                self.attention
                    .push(Attention::SetVariable { var: name.clone() });
            }
        }
        for v in &self.vars {
            if v.secret && v.value.is_empty() {
                self.attention.push(Attention::SetSecret {
                    var: v.name.clone(),
                });
            }
        }
        Mapped {
            report: ImportReport {
                folders: self.folders,
                requests: self.requests.len(),
                attention: self.attention,
            },
            requests: self.requests,
            variables: self.vars,
            description,
        }
    }
}

/// `s` with each `{{inner}}` replaced by `f(inner)`, or kept when `f`
/// gives `None`. `inner` is trimmed; `{{}}` is kept as written.
fn replace_refs(s: &str, mut f: impl FnMut(&str) -> Option<String>) -> String {
    let mut out = String::with_capacity(s.len());
    let mut rest = s;
    while let Some(open) = rest.find("{{") {
        let Some(len) = rest[open + 2..].find("}}") else {
            break;
        };
        let whole = &rest[open..open + len + 4];
        let inner = rest[open + 2..open + 2 + len].trim();
        out.push_str(&rest[..open]);
        match (!inner.is_empty()).then(|| f(inner)).flatten() {
            Some(r) => out.push_str(&r),
            None => out.push_str(whole),
        }
        rest = &rest[open + len + 4..];
    }
    out.push_str(rest);
    out
}

/// Form-encodes `s`, leaving `${…}` references as written.
fn form_encode(s: &str) -> String {
    let mut out = String::new();
    let mut rest = s;
    while let Some(i) = rest.find("${") {
        let Some(j) = rest[i..].find('}') else {
            break;
        };
        out.extend(form_urlencoded::byte_serialize(&rest.as_bytes()[..i]));
        out.push_str(&rest[i..i + j + 1]);
        rest = &rest[i + j + 1..];
    }
    out.extend(form_urlencoded::byte_serialize(rest.as_bytes()));
    out
}

/// `host` or `path` from a URL object: a string, or parts joined by `sep`.
fn join(v: Option<&Value>, sep: &str) -> String {
    match v {
        Some(Value::String(s)) => s.clone(),
        Some(Value::Array(parts)) => parts
            .iter()
            .filter_map(|p| p.as_str().or_else(|| p.get("value")?.as_str()))
            .collect::<Vec<_>>()
            .join(sep),
        _ => String::new(),
    }
}

fn description_lines(d: Option<&Description>) -> Vec<String> {
    d.map(|d| {
        d.text()
            .lines()
            .map(str::trim_end)
            .filter(|l| !l.trim().is_empty())
            .map(str::to_string)
            .collect()
    })
    .unwrap_or_default()
}

/// A field's description on one line.
fn note(d: Option<&Description>) -> Option<String> {
    let text = d?.text().split_whitespace().collect::<Vec<_>>().join(" ");
    (!text.is_empty()).then_some(text)
}
