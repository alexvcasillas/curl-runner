//! Swagger 2.0 to OpenAPI 3.x normalization.

use serde_json::{Map, Value, json};

use super::map::essence;

const METHODS: [&str; 8] = [
    "get", "put", "post", "delete", "options", "head", "patch", "trace",
];

/// Rewrite a Swagger 2.0 document into the 3.x shape. Lenient: anything
/// unexpected is passed through untouched.
pub fn to_v3(mut v: Value) -> Value {
    let Some(root) = v.as_object().cloned() else {
        return v;
    };
    let consumes = media_list(root.get("consumes"));
    let produces = media_list(root.get("produces"));
    let params_src = root.get("parameters").cloned().unwrap_or(Value::Null);

    let mut out = Map::new();
    out.insert("openapi".into(), json!("3.0.0"));
    for k in ["info", "tags", "externalDocs"] {
        if let Some(x) = root.get(k) {
            out.insert(k.into(), x.clone());
        }
    }
    if let Some(host) = root.get("host").and_then(Value::as_str) {
        let scheme = root
            .get("schemes")
            .and_then(Value::as_array)
            .and_then(|a| a.first())
            .and_then(Value::as_str)
            .unwrap_or("https");
        let base = root.get("basePath").and_then(Value::as_str).unwrap_or("");
        out.insert(
            "servers".into(),
            json!([{ "url": format!("{scheme}://{host}{base}") }]),
        );
    } else if let Some(base) = root.get("basePath").and_then(Value::as_str) {
        out.insert("servers".into(), json!([{ "url": base }]));
    }

    if let Some(Value::Object(paths)) = root.get("paths") {
        let mut np = Map::new();
        for (path, item) in paths {
            np.insert(
                path.clone(),
                convert_path_item(item, &consumes, &produces, &params_src),
            );
        }
        out.insert("paths".into(), Value::Object(np));
    }

    let mut components = Map::new();
    if let Some(d) = root.get("definitions") {
        components.insert("schemas".into(), d.clone());
    }
    if let Value::Object(p) = &params_src {
        let np: Map<String, Value> = p
            .iter()
            .map(|(k, p)| (k.clone(), convert_param(p)))
            .collect();
        components.insert("parameters".into(), Value::Object(np));
    }
    if let Some(Value::Object(r)) = root.get("responses") {
        let nr: Map<String, Value> = r
            .iter()
            .map(|(k, r)| (k.clone(), convert_response(r, &produces)))
            .collect();
        components.insert("responses".into(), Value::Object(nr));
    }
    if let Some(Value::Object(s)) = root.get("securityDefinitions") {
        let ns: Map<String, Value> = s
            .iter()
            .map(|(k, s)| (k.clone(), convert_security(s)))
            .collect();
        components.insert("securitySchemes".into(), Value::Object(ns));
    }
    if !components.is_empty() {
        out.insert("components".into(), Value::Object(components));
    }
    if let Some(s) = root.get("security") {
        out.insert("security".into(), s.clone());
    }
    for (k, x) in &root {
        if k.starts_with("x-") {
            out.insert(k.clone(), x.clone());
        }
    }

    v = Value::Object(out);
    rewrite_refs(&mut v);
    v
}

fn media_list(v: Option<&Value>) -> Vec<String> {
    v.and_then(Value::as_array)
        .map(|a| {
            a.iter()
                .filter_map(|x| x.as_str().map(String::from))
                .collect()
        })
        .unwrap_or_default()
}

fn convert_path_item(
    item: &Value,
    consumes: &[String],
    produces: &[String],
    params_src: &Value,
) -> Value {
    let Some(map) = item.as_object() else {
        return item.clone();
    };
    let path_params: &[Value] = map
        .get("parameters")
        .and_then(Value::as_array)
        .map_or(&[], Vec::as_slice);
    let mut out = Map::new();
    for (k, v) in map {
        if METHODS.contains(&k.as_str()) {
            out.insert(
                k.clone(),
                convert_operation(v, consumes, produces, params_src, path_params),
            );
        } else if k != "parameters" {
            out.insert(k.clone(), v.clone());
        }
    }
    Value::Object(out)
}

/// `(in, name)` of a parameter, following a `#/parameters/x` reference.
fn param_key(p: &Value, params_src: &Value) -> Option<(String, String)> {
    let r = match p
        .get("$ref")
        .and_then(Value::as_str)
        .and_then(|r| r.strip_prefix("#/parameters/"))
    {
        Some(name) => params_src.get(name)?,
        None => p,
    };
    Some((
        r.get("in")?.as_str()?.to_string(),
        r.get("name")?.as_str()?.to_string(),
    ))
}

/// The `in` of a parameter, following a `#/parameters/x` reference.
fn param_in<'a>(p: &'a Value, params_src: &'a Value) -> Option<&'a str> {
    if let Some(name) = p
        .get("$ref")
        .and_then(Value::as_str)
        .and_then(|r| r.strip_prefix("#/parameters/"))
    {
        return params_src.get(name)?.get("in")?.as_str();
    }
    p.get("in")?.as_str()
}

fn convert_operation(
    op: &Value,
    doc_consumes: &[String],
    produces: &[String],
    params_src: &Value,
    path_params: &[Value],
) -> Value {
    let Some(map) = op.as_object() else {
        return op.clone();
    };
    let own = media_list(map.get("consumes"));
    let consumes = if own.is_empty() { doc_consumes } else { &own };
    let own_produces = media_list(map.get("produces"));
    let produces = if own_produces.is_empty() {
        produces
    } else {
        &own_produces
    };

    let mut params = Vec::new();
    let mut body: Option<Value> = None;
    let mut form: Vec<(String, Value, bool)> = Vec::new();
    let own_params: &[Value] = map
        .get("parameters")
        .and_then(Value::as_array)
        .map_or(&[], Vec::as_slice);
    let own_keys: Vec<_> = own_params
        .iter()
        .filter_map(|p| param_key(p, params_src))
        .collect();
    let inherited = path_params
        .iter()
        .filter(|p| param_key(p, params_src).is_none_or(|k| !own_keys.contains(&k)));
    for p in inherited.chain(own_params) {
        let resolved = match p
            .get("$ref")
            .and_then(Value::as_str)
            .and_then(|r| r.strip_prefix("#/parameters/"))
        {
            Some(name) => params_src.get(name).unwrap_or(p),
            None => p,
        };
        match param_in(p, params_src) {
            Some("body") => body = Some(body_param(resolved, consumes)),
            Some("formData") => {
                let name = resolved
                    .get("name")
                    .and_then(Value::as_str)
                    .unwrap_or("")
                    .to_string();
                let required = resolved
                    .get("required")
                    .and_then(Value::as_bool)
                    .unwrap_or(false);
                form.push((name, form_schema(resolved), required));
            }
            _ => params.push(convert_param(p)),
        }
    }
    if body.is_none() && !form.is_empty() {
        // Every form type `consumes` lists, so the mapper picks by its own
        // preference. A file needs multipart.
        let file = form
            .iter()
            .any(|(_, s, _)| s.get("format").and_then(Value::as_str) == Some("binary"));
        let mut media: Vec<&str> = consumes
            .iter()
            .map(String::as_str)
            .filter(|c| match essence(c).as_str() {
                MULTIPART => true,
                URLENCODED => !file,
                _ => false,
            })
            .collect();
        if media.is_empty() {
            media.push(if file { MULTIPART } else { URLENCODED });
        }
        let mut props = Map::new();
        let mut required = Vec::new();
        for (name, schema, req) in form {
            if req {
                required.push(Value::String(name.clone()));
            }
            props.insert(name, schema);
        }
        let mut schema = json!({ "type": "object", "properties": Value::Object(props) });
        if !required.is_empty() {
            schema["required"] = Value::Array(required);
        }
        body = Some(json!({ "content": content(&media, &schema) }));
    }

    let mut out = Map::new();
    for (k, v) in map {
        match k.as_str() {
            "parameters" => {
                out.insert(k.clone(), Value::Array(std::mem::take(&mut params)));
            }
            "consumes" | "produces" | "schemes" => {}
            "responses" => {
                if let Value::Object(r) = v {
                    let nr: Map<String, Value> = r
                        .iter()
                        .map(|(c, r)| (c.clone(), convert_response(r, produces)))
                        .collect();
                    out.insert(k.clone(), Value::Object(nr));
                }
            }
            _ => {
                out.insert(k.clone(), v.clone());
            }
        }
    }
    if !params.is_empty() {
        out.insert("parameters".into(), Value::Array(params));
    }
    if let Some(b) = body {
        out.insert("requestBody".into(), b);
    }
    Value::Object(out)
}

const MULTIPART: &str = "multipart/form-data";
const URLENCODED: &str = "application/x-www-form-urlencoded";

/// The same schema under every media type in `media`.
fn content(media: &[&str], schema: &Value) -> Value {
    let map: Map<String, Value> = media
        .iter()
        .map(|m| ((*m).to_string(), json!({ "schema": schema })))
        .collect();
    Value::Object(map)
}

/// The body schema under every `consumes` media type, so the mapper picks
/// by its own preference rather than the document's order.
fn body_param(p: &Value, consumes: &[String]) -> Value {
    let media: Vec<&str> = if consumes.is_empty() {
        vec!["application/json"]
    } else {
        consumes.iter().map(String::as_str).collect()
    };
    let mut rb = Map::new();
    if let Some(d) = p.get("description") {
        rb.insert("description".into(), d.clone());
    }
    if let Some(r) = p.get("required") {
        rb.insert("required".into(), r.clone());
    }
    let schema = p.get("schema").cloned().unwrap_or_else(|| json!({}));
    rb.insert("content".into(), content(&media, &schema));
    Value::Object(rb)
}

fn form_schema(p: &Value) -> Value {
    let mut s = type_fields(p);
    if s.get("type").and_then(Value::as_str) == Some("file") {
        s = json!({ "type": "string", "format": "binary" });
    }
    if let Some(d) = p.get("description") {
        s["description"] = d.clone();
    }
    s
}

const SCHEMA_KEYS: [&str; 15] = [
    "type",
    "format",
    "items",
    "enum",
    "default",
    "minimum",
    "maximum",
    "exclusiveMinimum",
    "exclusiveMaximum",
    "minLength",
    "maxLength",
    "pattern",
    "minItems",
    "maxItems",
    "uniqueItems",
];

fn type_fields(p: &Value) -> Value {
    let mut s = Map::new();
    for k in SCHEMA_KEYS {
        if let Some(v) = p.get(k) {
            s.insert(k.into(), v.clone());
        }
    }
    Value::Object(s)
}

/// Non-body parameter: inline `type`/`items`/... move under `schema`.
fn convert_param(p: &Value) -> Value {
    let Some(map) = p.as_object() else {
        return p.clone();
    };
    if map.contains_key("$ref") || map.contains_key("schema") {
        return p.clone();
    }
    let mut out = Map::new();
    for (k, v) in map {
        if !SCHEMA_KEYS.contains(&k.as_str()) && k != "collectionFormat" {
            out.insert(k.clone(), v.clone());
        }
    }
    out.insert("schema".into(), type_fields(p));
    match map.get("collectionFormat").and_then(Value::as_str) {
        Some("multi") => {
            out.insert("style".into(), json!("form"));
            out.insert("explode".into(), json!(true));
        }
        Some("ssv") => {
            out.insert("style".into(), json!("spaceDelimited"));
        }
        Some("pipes") => {
            out.insert("style".into(), json!("pipeDelimited"));
        }
        Some("tsv") => {
            out.insert("x-collectionFormat".into(), json!("tsv"));
        }
        Some("csv") => {
            out.insert("style".into(), json!("form"));
            out.insert("explode".into(), json!(false));
        }
        _ => {}
    }
    Value::Object(out)
}

fn convert_response(r: &Value, produces: &[String]) -> Value {
    let Some(map) = r.as_object() else {
        return r.clone();
    };
    let mut out = Map::new();
    for (k, v) in map {
        if k == "schema" {
            let media = produces.first().map_or("application/json", String::as_str);
            out.insert("content".into(), json!({ media: { "schema": v.clone() } }));
        } else if k != "examples" {
            out.insert(k.clone(), v.clone());
        }
    }
    Value::Object(out)
}

fn convert_security(s: &Value) -> Value {
    match s.get("type").and_then(Value::as_str) {
        Some("basic") => {
            let mut o = json!({ "type": "http", "scheme": "basic" });
            if let Some(d) = s.get("description") {
                o["description"] = d.clone();
            }
            o
        }
        Some("oauth2") => {
            let scopes = s.get("scopes").cloned().unwrap_or_else(|| json!({}));
            let mut flow = Map::new();
            for k in ["authorizationUrl", "tokenUrl"] {
                if let Some(u) = s.get(k) {
                    flow.insert(k.into(), u.clone());
                }
            }
            flow.insert("scopes".into(), scopes);
            let name = match s.get("flow").and_then(Value::as_str) {
                Some("implicit") => "implicit",
                Some("password") => "password",
                Some("application") => "clientCredentials",
                _ => "authorizationCode",
            };
            let mut o = json!({ "type": "oauth2", "flows": { name: Value::Object(flow) } });
            if let Some(d) = s.get("description") {
                o["description"] = d.clone();
            }
            o
        }
        _ => s.clone(),
    }
}

/// Point every `#/definitions/...`-style `$ref` at its 3.x location.
fn rewrite_refs(v: &mut Value) {
    const MAP: [(&str, &str); 4] = [
        ("#/definitions/", "#/components/schemas/"),
        ("#/parameters/", "#/components/parameters/"),
        ("#/responses/", "#/components/responses/"),
        ("#/securityDefinitions/", "#/components/securitySchemes/"),
    ];
    match v {
        Value::Object(m) => {
            for (k, x) in m.iter_mut() {
                if k == "$ref"
                    && let Value::String(s) = x
                {
                    for (from, to) in MAP {
                        if let Some(rest) = s.strip_prefix(from) {
                            *s = format!("{to}{rest}");
                            break;
                        }
                    }
                } else {
                    rewrite_refs(x);
                }
            }
        }
        Value::Array(a) => a.iter_mut().for_each(rewrite_refs),
        _ => {}
    }
}
