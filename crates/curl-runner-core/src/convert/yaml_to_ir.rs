//! YAML to IR: a curl-runner request (untyped, as `Bun.YAML.parse` hands it to
//! `yamlToIR`) to a [`CurlRunnerIr`] (`convert/yaml-to-ir.ts`).
//!
//! The TS code does no validation, so values are coerced the way the JS
//! template literals and truthiness checks would.

use super::jsutil::JsMap;
use super::types::{Auth, Body, CurlRunnerIr, FormField, FormFile, NumText, Source, Ssl};
use crate::jsval::JsVal;
use crate::model::Json;

/// JS truthiness.
pub fn truthy(v: Option<&Json>) -> bool {
    match v {
        None | Some(Json::Null) => false,
        Some(Json::Bool(b)) => *b,
        Some(Json::Number(n)) => n.as_f64().is_some_and(|f| f != 0.0 && !f.is_nan()),
        Some(Json::String(s)) => !s.is_empty(),
        Some(_) => true,
    }
}

/// JS `String(v)`.
pub fn js_string(v: &Json) -> String {
    JsVal::from_json(v).to_js_string()
}

fn truthy_string(v: Option<&Json>) -> Option<String> {
    v.filter(|v| truthy(Some(v))).map(js_string)
}

/// `Object.entries(v)` / own properties of `{...v}`: a mapping's entries, or
/// the index-keyed characters of a string and items of a list.
fn object_entries(v: Option<&Json>) -> Vec<(String, Json)> {
    match v {
        Some(Json::Object(o)) => o.iter().map(|(k, v)| (k.clone(), v.clone())).collect(),
        Some(Json::Array(a)) => a
            .iter()
            .enumerate()
            .map(|(i, v)| (i.to_string(), v.clone()))
            .collect(),
        Some(Json::String(s)) => s
            .chars()
            .enumerate()
            .map(|(i, c)| (i.to_string(), Json::String(c.to_string())))
            .collect(),
        _ => Vec::new(),
    }
}

/// `yamlToIR`. `Err` carries the message of the TypeError the TS code throws
/// for a `null` entry.
pub fn yaml_to_ir(config: &Json) -> Result<CurlRunnerIr, String> {
    if config.is_null() {
        return Err("null is not an object (evaluating 'config.headers')".into());
    }
    let get = |k: &str| config.get(k);
    let mut warnings = Vec::new();

    let mut headers = JsMap::new();
    let mut falsy_content_type = false;
    if truthy(get("headers")) {
        for (k, v) in object_entries(get("headers")) {
            headers.insert(k.clone(), js_string(&v));
        }
        falsy_content_type = object_entries(get("headers"))
            .iter()
            .any(|(k, v)| k == "Content-Type" && !truthy(Some(v)));
    }

    let mut ir = CurlRunnerIr {
        name: truthy_string(get("name")),
        method: truthy_string(get("method")).unwrap_or_else(|| "GET".into()),
        // A missing `url` is `undefined`; the generator throws when it reaches it.
        url: get("url").map(js_string).unwrap_or_default(),
        headers,
        params: None,
        body: None,
        form_data: None,
        auth: None,
        insecure: false,
        follow_redirects: None,
        max_redirects: None,
        timeout: None,
        proxy: None,
        output: None,
        http2: false,
        ssl: None,
        source: Source::Yaml,
        falsy_content_type,
        warnings: Vec::new(),
        unsupported_flags: Vec::new(),
    };

    if truthy(get("params")) {
        let mut p = JsMap::new();
        for (k, v) in object_entries(get("params")) {
            p.insert_own(k, js_string(&v));
        }
        ir.params = Some(p);
    }

    if truthy(get("auth")) {
        let a = get("auth");
        let field = |k: &str| a.and_then(|a| a.get(k));
        ir.auth = Some(Auth {
            kind: field("type")
                .map(js_string)
                .unwrap_or_else(|| "undefined".into()),
            username: truthy_string(field("username")),
            password: truthy_string(field("password")),
            token: truthy_string(field("token")),
        });
    }

    if let Some(body) = get("body") {
        ir.body = Some(match body {
            Json::Object(o) => {
                if let Some(j) = o.get("json") {
                    Body::Json(j.clone())
                } else if let Some(f) = o.get("form") {
                    Body::Urlencoded(js_string(f))
                } else if let Some(r) = o.get("raw") {
                    Body::Raw(js_string(r))
                } else if let Some(b) = o.get("binary") {
                    Body::Binary(js_string(b))
                } else {
                    Body::Json(body.clone())
                }
            }
            other => Body::Json(other.clone()),
        });
    }

    if truthy(get("formData")) {
        let mut form = JsMap::new();
        for (k, v) in object_entries(get("formData")) {
            let field = match &v {
                Json::Object(o) if o.contains_key("file") => FormField::File(FormFile {
                    file: o.get("file").map(js_string).unwrap_or_default(),
                    filename: truthy_string(o.get("filename")),
                    content_type: truthy_string(o.get("contentType")),
                    type_first: false,
                }),
                other => FormField::Text(js_string(other)),
            };
            form.insert(k, field);
        }
        ir.form_data = Some(form);
    }

    if truthy(get("timeout")) {
        ir.timeout = get("timeout").map(|v| JsVal::from_json(v).to_number());
    }
    if let Some(f) = get("followRedirects") {
        ir.follow_redirects = Some(truthy(Some(f)));
    }
    if truthy(get("maxRedirects")) {
        ir.max_redirects = get("maxRedirects").map(|v| match v {
            Json::Number(n) => NumText::Num(n.as_f64().unwrap_or(f64::NAN)),
            other => NumText::Text(js_string(other)),
        });
    }
    ir.proxy = truthy_string(get("proxy"));
    ir.insecure = truthy(get("insecure"));
    ir.http2 = truthy(get("http2"));
    ir.output = truthy_string(get("output"));

    if truthy(get("ssl")) {
        let s = get("ssl");
        let field = |k: &str| truthy_string(s.and_then(|s| s.get(k)));
        ir.ssl = Some(Ssl {
            ca: field("ca"),
            cert: field("cert"),
            key: field("key"),
        });
    }

    for (key, msg) in [
        ("expect", "expect block has no curl equivalent"),
        ("store", "store block has no curl equivalent"),
        ("when", "when condition has no curl equivalent"),
        ("retry", "retry config simplified in curl output"),
        ("snapshot", "snapshot config has no curl equivalent"),
        ("diff", "diff config has no curl equivalent"),
    ] {
        if truthy(get(key)) {
            warnings.push(msg.to_string());
        }
    }
    ir.warnings = warnings;
    Ok(ir)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::convert::generator::generate_curl_from_ir;
    use serde_json::json;

    fn curl(v: Json) -> String {
        generate_curl_from_ir(&yaml_to_ir(&v).unwrap())
    }

    #[test]
    fn simple_request() {
        assert!(
            curl(json!({"url": "https://api.example.com", "method": "GET"}))
                .contains("https://api.example.com")
        );
    }

    #[test]
    fn post_with_json_body() {
        let c = curl(json!({
            "url": "https://api.example.com", "method": "POST",
            "headers": {"Content-Type": "application/json"},
            "body": {"name": "Alex", "age": 30}
        }));
        assert!(c.contains("-X POST"));
        assert!(c.contains("\"name\":\"Alex\""));
    }

    #[test]
    fn with_auth_and_params() {
        assert!(curl(json!({"url": "https://a.com", "method": "GET", "auth": {"type": "bearer", "token": "mytoken"}})).contains("Bearer mytoken"));
        let c = curl(
            json!({"url": "https://api.example.com/search", "params": {"q": "test", "page": "1"}}),
        );
        assert!(c.contains("q=test") && c.contains("page=1"));
    }

    #[test]
    fn yaml_only_features_generate_warnings() {
        let ir = yaml_to_ir(&json!({"url": "https://a.com", "expect": {"status": 200}, "store": {"userId": "body.id"}})).unwrap();
        assert!(
            ir.warnings
                .contains(&"expect block has no curl equivalent".to_string())
        );
        assert!(
            ir.warnings
                .contains(&"store block has no curl equivalent".to_string())
        );
    }

    #[test]
    fn null_entry_is_a_type_error() {
        assert_eq!(
            yaml_to_ir(&Json::Null).unwrap_err(),
            "null is not an object (evaluating 'config.headers')"
        );
    }
}
