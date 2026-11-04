//! Request bodies: the preferred media type's sample, form fields and
//! the `Content-Type` header.

use serde_json::{Map, Value};

use super::doc::Resolved;
use super::map::{Mapper, essence, is_secret_param, note, text_field, to_text};
use crate::convert::import::{
    Attention, Field, FieldValue, MappedBody, MappedRequest, PLACEHOLDER_FILE,
};
use crate::convert::param_var;
use crate::convert::postman::map::form_encode;
use crate::model::Json;

/// How deep `allOf` parts nest before the rest are ignored.
const MAX_ALL_OF_DEPTH: usize = 10;

/// A form body's schema, flattened.
struct FormParts<'d> {
    /// In document order; a later `allOf` part's property replaces an
    /// earlier one of the same name.
    props: Vec<(&'d str, &'d Value)>,
    required: Vec<&'d str>,
    example: Json,
    /// The schema declares an object with no fields at all, so there is
    /// no body to send.
    empty: bool,
}

impl<'d> Mapper<'d> {
    pub(super) fn body(&mut self, op: &'d Value, r: &mut MappedRequest) {
        let Some(rb) = op.get("requestBody").and_then(|raw| self.resolve(raw)) else {
            return;
        };
        let Some(content) = rb.get("content").and_then(Value::as_object) else {
            return;
        };
        let find = |want: fn(&str) -> bool| {
            content
                .iter()
                .find(|(k, _)| want(&essence(k)))
                .map(|(k, v)| (k.as_str(), v))
        };
        let is_json = |m: &str| m == "application/json" || m.ends_with("+json");
        let is_text =
            |m: &str| m.starts_with("text/") || m == "application/xml" || m.ends_with("+xml");
        let content_type = if let Some((media, m)) = find(is_json) {
            let value = match self.sampler.example(m) {
                Some(v) => v,
                None => self.schema_sample(m.get("schema")),
            };
            self.document_json(&value);
            r.body = MappedBody::Json(match value {
                Json::Null => Json::Object(Map::new()),
                v => v,
            });
            Some(media)
        } else if let Some((_, m)) = find(|m| m == "multipart/form-data") {
            self.multipart(m, r);
            None
        } else if let Some((media, m)) = find(|m| m == "application/x-www-form-urlencoded") {
            self.urlencoded(m, r).then_some(media)
        } else if let Some((media, m)) = find(is_text) {
            let example = self.sampler.example(m).or_else(|| {
                let schema = self.resolve(m.get("schema")?)?;
                schema.get("example").cloned()
            });
            let text = match example {
                Some(Json::String(s)) => s,
                Some(Json::Null) | None => String::new(),
                Some(other) => other.to_string(),
            };
            self.document_text(&text);
            r.body = MappedBody::Text(text);
            Some(media)
        } else {
            if let Some(first) = content.keys().next() {
                self.skip_body(essence(first));
            }
            None
        };
        if let Some(ct) = content_type {
            r.headers
                .push(text_field("Content-Type", ct.into(), false, None));
        }
    }

    /// A body sample from `schema`, noting a top-level choice of shapes.
    fn schema_sample(&mut self, schema: Option<&'d Value>) -> Json {
        let Some(schema) = schema else {
            return Json::Null;
        };
        if self.sampler.alternatives(schema) > 1 {
            self.tally
                .hit("shapes".into(), self.req, &self.at, true, || {
                    Attention::BodyShapes {
                        count: 1,
                        examples: vec![self.at.clone()],
                    }
                });
        }
        self.sampler.value(schema)
    }

    /// The media type's object schema: its properties (`allOf` parts
    /// merged), which are required, and the example's values when it has
    /// one. `None` when the schema is a `$ref` that can't be followed,
    /// which is already reported.
    fn form_parts(&mut self, m: &'d Value) -> Option<FormParts<'d>> {
        let mut parts = FormParts {
            props: Vec::new(),
            required: Vec::new(),
            example: self.sampler.example(m).unwrap_or(Json::Null),
            empty: false,
        };
        if let Some(raw) = m.get("schema") {
            let schema = self.resolve(raw)?;
            parts.empty = no_fields(schema);
            self.collect_props(schema, 0, &mut parts);
        }
        Some(parts)
    }

    fn collect_props(&mut self, schema: &'d Value, depth: usize, out: &mut FormParts<'d>) {
        if depth > MAX_ALL_OF_DEPTH {
            return;
        }
        let all_of = schema.get("allOf").and_then(Value::as_array);
        for part in all_of.into_iter().flatten() {
            if let Some(part) = self.resolve(part) {
                self.collect_props(part, depth + 1, out);
            }
        }
        let props = schema.get("properties").and_then(Value::as_object);
        for (name, prop) in props.into_iter().flatten() {
            match out.props.iter_mut().find(|(n, _)| n == name) {
                Some(slot) => slot.1 = prop,
                None => out.props.push((name.as_str(), prop)),
            }
        }
        let required = schema.get("required").and_then(Value::as_array);
        out.required
            .extend(required.into_iter().flatten().filter_map(Value::as_str));
    }

    /// Report a body left out, once per request and media type.
    fn skip_body(&mut self, media: String) {
        self.tally
            .hit(format!("body\0{media}"), self.req, &self.at, true, || {
                Attention::SkippedBody {
                    media: media.clone(),
                    count: 1,
                    examples: vec![self.at.clone()],
                }
            });
    }

    /// A form field's text: from the example, else sampled, with a secret
    /// name's value moved into a variable.
    fn form_text(&mut self, name: &str, prop: &'d Value, example: &Json) -> String {
        let value = match example.get(name) {
            Some(v) => v.clone(),
            None => self.sampler.value(prop),
        };
        let text = to_text(&value);
        if is_secret_param(name) {
            self.credential(&param_var(name))
        } else {
            self.document_text(&text);
            text
        }
    }

    fn multipart(&mut self, m: &'d Value, r: &mut MappedRequest) {
        let Some(FormParts {
            props,
            required,
            example,
            empty,
        }) = self.form_parts(m)
        else {
            return;
        };
        let mut fields = Vec::new();
        for (name, raw) in props {
            let disabled = !required.contains(&name);
            let prop = self.resolve(raw);
            let value = if prop.is_some_and(|p| self.is_file(p)) {
                if !disabled {
                    self.tally.push(Attention::PickFile {
                        request: self.at.clone(),
                        field: name.to_string(),
                    });
                }
                FieldValue::File {
                    path: PLACEHOLDER_FILE.into(),
                    placeholder: true,
                }
            } else {
                FieldValue::Text(self.form_text(name, raw, &example))
            };
            fields.push(Field {
                key: name.to_string(),
                value,
                disabled,
                note: note(prop.and_then(|p| p.get("description"))),
            });
        }
        if !fields.is_empty() {
            r.body = MappedBody::Form(fields);
        } else if !empty {
            self.skip_body("multipart/form-data".into());
        }
    }

    /// A file field: `format: binary`, a 3.1 `contentMediaType`, or an
    /// array of either.
    fn is_file(&self, p: &Value) -> bool {
        if p.get("format").and_then(Value::as_str) == Some("binary")
            || p.get("contentMediaType").is_some()
        {
            return true;
        }
        let items = p.get("items").map(|i| self.doc.resolve(i));
        matches!(items, Some(Resolved::Value(i)) if i.get("format").and_then(Value::as_str) == Some("binary") || i.get("contentMediaType").is_some())
    }

    /// The required properties as a urlencoded body; the optional ones by
    /// name in a comment. False when there's no body to send.
    fn urlencoded(&mut self, m: &'d Value, r: &mut MappedRequest) -> bool {
        let Some(FormParts {
            props,
            required,
            example,
            empty,
        }) = self.form_parts(m)
        else {
            return false;
        };
        if empty {
            return false;
        }
        if props.is_empty() {
            self.skip_body("application/x-www-form-urlencoded".into());
            return false;
        }
        let mut parts = Vec::new();
        let mut optional = Vec::new();
        for (name, prop) in props {
            if required.contains(&name) {
                let value = self.form_text(name, prop, &example);
                parts.push(format!("{}={}", form_encode(name), form_encode(&value)));
            } else {
                optional.push(name);
            }
        }
        if !optional.is_empty() {
            r.comments
                .push(format!("Optional form fields: {}", optional.join(", ")));
        }
        if parts.is_empty() {
            return false;
        }
        r.body = MappedBody::Text(parts.join("&"));
        true
    }
}

/// An object schema with no fields: no or empty `properties`, no
/// `allOf`/`oneOf`/`anyOf`, and `additionalProperties` false or absent.
fn no_fields(s: &Value) -> bool {
    let object = s.get("type").is_none_or(|t| t == "object");
    let no_props = s
        .get("properties")
        .is_none_or(|p| p.as_object().is_some_and(Map::is_empty));
    let closed = s
        .get("additionalProperties")
        .is_none_or(|a| *a == Value::Bool(false));
    object
        && no_props
        && closed
        && ["allOf", "oneOf", "anyOf"]
            .iter()
            .all(|k| s.get(*k).is_none())
}
