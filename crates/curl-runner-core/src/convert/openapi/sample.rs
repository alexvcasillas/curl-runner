//! Sample values generated from OpenAPI schemas.

use serde_json::{Map, Value};

use super::doc::{Doc, Resolved};
use crate::model::Json;

/// Values per `value()` call after which optional properties are skipped.
const BUDGET: usize = 200;
/// Values per `value()` call after which everything is an empty container or `null`.
const HARD_CAP: usize = 2 * BUDGET;
/// Nesting depth after which a schema gets an empty container.
const MAX_DEPTH: usize = 10;
/// Placeholder path for a file field.
const FILE_PLACEHOLDER: &str = "./pick-a-file";

/// A `$ref` the sampler could not follow.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnresolvedRef {
    /// The `$ref` text.
    pub target: String,
    /// True for a reference to another document, false for a missing local target.
    pub external: bool,
}

#[derive(Clone, Copy, PartialEq)]
enum Kind {
    Object,
    Array,
    Other,
}

/// Generates deterministic sample values from schemas.
pub struct Sampler<'d> {
    doc: &'d Doc,
    stack: Vec<String>,
    count: usize,
    unresolved: Vec<UnresolvedRef>,
    /// Unresolved refs met since the last `take_hits`, once each.
    hits: Vec<UnresolvedRef>,
}

impl<'d> Sampler<'d> {
    pub fn new(doc: &'d Doc) -> Self {
        Self {
            doc,
            stack: Vec::new(),
            count: 0,
            unresolved: Vec::new(),
            hits: Vec::new(),
        }
    }

    /// Every `$ref` that failed to resolve so far, once per target, in first-seen order.
    pub fn unresolved_refs(&self) -> &[UnresolvedRef] {
        &self.unresolved
    }

    /// The unresolved refs met since the last call, once each, in
    /// first-seen order. Lets a caller tell which request met them.
    pub fn take_hits(&mut self) -> Vec<UnresolvedRef> {
        std::mem::take(&mut self.hits)
    }

    /// A sample for `schema`, within the size budget.
    pub fn value(&mut self, schema: &Value) -> Json {
        self.count = 0;
        self.stack.clear();
        self.node(schema, 0)
    }

    /// The holder's `example`, else the first of its `examples`.
    ///
    /// `examples` is a map of example objects (parameters, media types) or a
    /// list of values (3.1 schemas). Example objects may be `$ref`s and give
    /// their value in `value`.
    pub fn example(&mut self, holder: &Value) -> Option<Json> {
        if let Some(v) = holder.get("example") {
            return Some(v.clone());
        }
        match holder.get("examples")? {
            Value::Array(items) => items.first().cloned(),
            Value::Object(map) => {
                let (_, first) = map.iter().next()?;
                match self.doc.resolve(first) {
                    Resolved::Value(v) => v.get("value").cloned(),
                    other => {
                        self.record(other);
                        None
                    }
                }
            }
            _ => None,
        }
    }

    /// How many top-level `oneOf`/`anyOf` options `schema` has, 0 if none.
    pub fn alternatives(&mut self, schema: &Value) -> usize {
        let Resolved::Value(s) = self.doc.resolve(schema) else {
            return 0;
        };
        ["oneOf", "anyOf"]
            .iter()
            .find_map(|k| s.get(*k).and_then(Value::as_array))
            .map_or(0, Vec::len)
    }

    fn record(&mut self, r: Resolved<'_>) {
        let (target, external) = match r {
            Resolved::Missing(t) => (t, false),
            Resolved::External(t) => (t, true),
            Resolved::Value(_) => return,
        };
        let r = UnresolvedRef { target, external };
        if !self.hits.contains(&r) {
            self.hits.push(r.clone());
        }
        if !self.unresolved.contains(&r) {
            self.unresolved.push(r);
        }
    }

    fn node(&mut self, schema: &Value, depth: usize) -> Json {
        self.count += 1;
        let doc = self.doc;
        let mut pushed = false;
        let resolved = match schema.get("$ref").and_then(Value::as_str) {
            Some(r) => match doc.resolve(schema) {
                Resolved::Value(v) => {
                    if self.stack.iter().any(|s| s == r) {
                        return recursion_stop(kind(v));
                    }
                    self.stack.push(r.to_string());
                    pushed = true;
                    v
                }
                other => {
                    self.record(other);
                    return Json::Object(Map::new());
                }
            },
            None => schema,
        };
        let out = if depth > MAX_DEPTH {
            recursion_stop(kind(resolved))
        } else if self.count > HARD_CAP {
            capped(kind(resolved))
        } else {
            self.generate(resolved, depth)
        };
        if pushed {
            self.stack.pop();
        }
        out
    }

    fn generate(&mut self, s: &Value, depth: usize) -> Json {
        if let Some(v) = literal(s) {
            return v;
        }
        if let Some(parts) = s.get("allOf").and_then(Value::as_array) {
            return self.merge(s, parts, depth);
        }
        for key in ["oneOf", "anyOf"] {
            if let Some(first) = s.get(key).and_then(Value::as_array).and_then(|a| a.first()) {
                return self.node(first, depth + 1);
            }
        }
        match type_name(s).as_deref() {
            Some("object") => self.object(s, depth),
            Some("array") => self.array(s, depth),
            Some("string") => string(s),
            Some("integer" | "number") => s
                .get("minimum")
                .filter(|m| m.is_number())
                .cloned()
                .unwrap_or_else(|| Json::from(1)),
            Some("boolean") => Json::Bool(true),
            Some(_) => Json::Null,
            None if s.get("properties").is_some() => self.object(s, depth),
            None if s.get("items").is_some() => self.array(s, depth),
            None => Json::Null,
        }
    }

    fn merge(&mut self, s: &Value, parts: &[Value], depth: usize) -> Json {
        let mut merged = Map::new();
        let mut first_other = None;
        for part in parts {
            match self.node(part, depth + 1) {
                Json::Object(m) => merged.extend(m),
                other => {
                    first_other.get_or_insert(other);
                }
            }
        }
        if s.get("properties").is_some()
            && let Json::Object(m) = self.object(s, depth)
        {
            merged.extend(m);
        }
        match first_other {
            Some(other) if merged.is_empty() => other,
            _ => Json::Object(merged),
        }
    }

    fn object(&mut self, s: &Value, depth: usize) -> Json {
        let mut out = Map::new();
        let Some(props) = s.get("properties").and_then(Value::as_object) else {
            return Json::Object(out);
        };
        let required: Vec<&str> = s
            .get("required")
            .and_then(Value::as_array)
            .map(|a| a.iter().filter_map(Value::as_str).collect())
            .unwrap_or_default();
        for (name, prop) in props {
            if self.count > BUDGET && !required.contains(&name.as_str()) {
                continue;
            }
            out.insert(name.clone(), self.node(prop, depth + 1));
        }
        Json::Object(out)
    }

    fn array(&mut self, s: &Value, depth: usize) -> Json {
        match s.get("items") {
            Some(items) => Json::Array(vec![self.node(items, depth + 1)]),
            None => Json::Array(Vec::new()),
        }
    }
}

/// `example`, `examples[0]`, `default`, `const`, `enum[0]`, in that order.
fn literal(s: &Value) -> Option<Json> {
    s.get("example")
        .or_else(|| s.get("examples").and_then(Value::as_array)?.first())
        .or_else(|| s.get("default"))
        .or_else(|| s.get("const"))
        .or_else(|| s.get("enum").and_then(Value::as_array)?.first())
        .cloned()
}

/// The schema's type, ignoring `"null"` in a 3.1 type array.
fn type_name(s: &Value) -> Option<String> {
    match s.get("type")? {
        Value::String(t) => Some(t.clone()),
        Value::Array(ts) => ts
            .iter()
            .filter_map(Value::as_str)
            .find(|t| *t != "null")
            .map(str::to_string),
        _ => None,
    }
}

fn string(s: &Value) -> Json {
    let text = match s.get("format").and_then(Value::as_str) {
        Some("date-time") => "2024-01-01T00:00:00Z",
        Some("date") => "2024-01-01",
        Some("email") => "user@example.com",
        Some("uuid") => "00000000-0000-4000-8000-000000000000",
        Some("uri" | "url") => "https://example.com",
        Some("binary") => FILE_PLACEHOLDER,
        _ => "string",
    };
    Json::String(text.to_string())
}

fn kind(s: &Value) -> Kind {
    match type_name(s).as_deref() {
        Some("array") => Kind::Array,
        Some("object") => Kind::Object,
        None if s.get("items").is_some() => Kind::Array,
        None if s.get("properties").is_some() => Kind::Object,
        _ => Kind::Other,
    }
}

/// A repeated `$ref` or too deep: `[]` for arrays, `{}` otherwise.
fn recursion_stop(k: Kind) -> Json {
    match k {
        Kind::Array => Json::Array(Vec::new()),
        _ => Json::Object(Map::new()),
    }
}

/// Past the hard cap: an empty container, or `null` for scalars.
fn capped(k: Kind) -> Json {
    match k {
        Kind::Array => Json::Array(Vec::new()),
        Kind::Object => Json::Object(Map::new()),
        Kind::Other => Json::Null,
    }
}
