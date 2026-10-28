//! YAML serializer: [`CurlRunnerIr`] to curl-runner YAML text
//! (`convert/yaml-serializer.ts`). Hand-rolled like the TS one: stable key
//! order, its own quoting rules, loss comments. The text must stay
//! byte-identical, so this does not go through a YAML library.

use super::jsutil::locale_compare;
use super::types::{Body, CurlRunnerIr, FormField, YamlOptions};
use crate::jsval::num_to_string;
use crate::model::Json;
use crate::snapshot::js_stringify;
use crate::yaml_text::{yaml_key, yaml_str};

/// Serialize one IR (`serializeToYaml`).
pub fn serialize_to_yaml(ir: &CurlRunnerIr, options: YamlOptions) -> String {
    let pretty = options.pretty.unwrap_or(true);
    let mut lines: Vec<String> = Vec::new();

    if options.loss_report.unwrap_or(true) && !ir.warnings.is_empty() {
        for w in &ir.warnings {
            lines.push(format!("# Warning: {w}"));
        }
        lines.push(String::new());
    }

    lines.push("request:".into());
    if let Some(n) = ir.name.as_deref().filter(|n| !n.is_empty()) {
        lines.push(format!("  name: {}", yaml_str(n)));
    }
    request_fields(&mut lines, ir, 2, pretty);
    finish(lines)
}

/// Serialize several IRs as a `requests:` collection (`serializeBatchToYaml`).
pub fn serialize_batch_to_yaml(irs: &[CurlRunnerIr], options: YamlOptions) -> String {
    let pretty = options.pretty.unwrap_or(true);
    let mut lines: Vec<String> = Vec::new();

    if options.loss_report.unwrap_or(true) {
        let all: Vec<&String> = irs.iter().flat_map(|ir| &ir.warnings).collect();
        if !all.is_empty() {
            for w in all {
                lines.push(format!("# Warning: {w}"));
            }
            lines.push(String::new());
        }
    }

    lines.push("requests:".into());
    for (idx, ir) in irs.iter().enumerate() {
        if idx > 0 {
            lines.push(String::new());
        }
        match ir.name.as_deref().filter(|n| !n.is_empty()) {
            Some(n) => lines.push(format!("  - name: {}", yaml_str(n))),
            None => lines.push(format!("  - name: request_{}", idx + 1)),
        }
        request_fields(&mut lines, ir, 4, pretty);
    }
    finish(lines)
}

fn finish(lines: Vec<String>) -> String {
    format!("{}\n", lines.join("\n"))
}

/// Everything from `method:` on, with the keys indented by `base` spaces.
fn request_fields(lines: &mut Vec<String>, ir: &CurlRunnerIr, base: usize, pretty: bool) {
    let p0 = " ".repeat(base);
    let p1 = " ".repeat(base + 2);
    let p2 = " ".repeat(base + 4);

    lines.push(format!("{p0}method: {}", ir.method));
    lines.push(format!("{p0}url: {}", yaml_str(&ir.url)));

    if let Some(params) = ir.params.as_ref().filter(|p| !p.is_empty()) {
        lines.push(format!("{p0}params:"));
        for (k, v) in sorted_entries(params.entries()) {
            lines.push(format!("{p1}{}: {}", yaml_key(k), yaml_str(v)));
        }
    }

    if !ir.headers.is_empty() {
        lines.push(format!("{p0}headers:"));
        for (k, v) in sorted_entries(ir.headers.entries()) {
            lines.push(format!("{p1}{}: {}", yaml_key(k), yaml_str(v)));
        }
    }

    if let Some(a) = &ir.auth {
        let or_empty = |s: &Option<String>| s.clone().unwrap_or_default();
        lines.push(format!("{p0}auth:"));
        lines.push(format!("{p1}type: {}", a.kind));
        match a.kind.as_str() {
            "basic" => {
                lines.push(format!(
                    "{p1}username: {}",
                    yaml_str(&or_empty(&a.username))
                ));
                lines.push(format!(
                    "{p1}password: {}",
                    yaml_str(&or_empty(&a.password))
                ));
            }
            "bearer" => lines.push(format!("{p1}token: {}", yaml_str(&or_empty(&a.token)))),
            _ => {}
        }
    }

    if let Some(body) = &ir.body {
        lines.push(format!("{p0}body:"));
        match body {
            Body::Json(c) if pretty => lines.extend(serialize_json_to_yaml(c, base + 2)),
            Body::Json(c) => lines.push(format!("{p1}json: {}", js_stringify(c))),
            Body::Urlencoded(c) => lines.push(format!("{p1}form: {}", yaml_str(c))),
            Body::Binary(c) => lines.push(format!("{p1}binary: {}", yaml_str(c))),
            Body::Raw(c) => lines.push(format!("{p1}raw: {}", yaml_str(c))),
        }
    }

    if let Some(form) = ir.form_data.as_ref().filter(|f| !f.is_empty()) {
        lines.push(format!("{p0}formData:"));
        for (k, v) in sorted_entries(form.entries()) {
            match v {
                FormField::Text(s) => lines.push(format!("{p1}{}: {}", yaml_key(k), yaml_str(s))),
                FormField::File(f) => {
                    lines.push(format!("{p1}{}:", yaml_key(k)));
                    lines.push(format!("{p2}file: {}", yaml_str(&f.file)));
                    if let Some(n) = f.filename.as_deref().filter(|n| !n.is_empty()) {
                        lines.push(format!("{p2}filename: {}", yaml_str(n)));
                    }
                    if let Some(t) = f.content_type.as_deref().filter(|t| !t.is_empty()) {
                        lines.push(format!("{p2}contentType: {}", yaml_str(t)));
                    }
                }
            }
        }
    }

    if let Some(t) = ir.timeout {
        lines.push(format!("{p0}timeout: {}", num_to_string(t)));
    }
    if ir.follow_redirects == Some(true) {
        lines.push(format!("{p0}followRedirects: true"));
    }
    if let Some(m) = &ir.max_redirects {
        lines.push(format!("{p0}maxRedirects: {}", m.to_js_string()));
    }
    if let Some(p) = ir.proxy.as_deref().filter(|p| !p.is_empty()) {
        lines.push(format!("{p0}proxy: {}", yaml_str(p)));
    }
    if ir.insecure {
        lines.push(format!("{p0}insecure: true"));
    }
    if ir.http2 {
        lines.push(format!("{p0}http2: true"));
    }
    if let Some(o) = ir.output.as_deref().filter(|o| !o.is_empty()) {
        lines.push(format!("{p0}output: {}", yaml_str(o)));
    }

    if let Some(ssl) = &ir.ssl {
        lines.push(format!("{p0}ssl:"));
        for (k, v) in [("ca", &ssl.ca), ("cert", &ssl.cert), ("key", &ssl.key)] {
            if let Some(v) = v.as_deref().filter(|v| !v.is_empty()) {
                lines.push(format!("{p1}{k}: {}", yaml_str(v)));
            }
        }
    }
}

/// A JSON value as the `json:` entry of a body, `indent` spaces deep.
fn serialize_json_to_yaml(value: &Json, indent: usize) -> Vec<String> {
    let pad = " ".repeat(indent);
    match value {
        Json::Null => vec![format!("{pad}json: null")],
        Json::Array(a) if a.is_empty() => vec![format!("{pad}json: []")],
        Json::Array(a)
            if a.iter()
                .all(|v| !matches!(v, Json::Array(_) | Json::Object(_))) =>
        {
            let items: Vec<String> = a.iter().map(js_stringify).collect();
            vec![format!("{pad}json: [{}]", items.join(", "))]
        }
        Json::Array(_) => vec![format!("{pad}json: {}", js_stringify(value))],
        Json::Object(o) if o.is_empty() => vec![format!("{pad}json: {{}}")],
        Json::Object(o) => {
            let mut lines = vec![format!("{pad}json:")];
            for (key, v) in crate::snapshot::json::js_entries(o) {
                let rendered = match v {
                    Json::Null => "null".to_string(),
                    Json::Object(_) | Json::Array(_) => js_stringify(v),
                    other => yaml_value(other),
                };
                lines.push(format!("{pad}  {}: {rendered}", yaml_key(key)));
            }
            lines
        }
        other => vec![format!("{pad}json: {}", js_stringify(other))],
    }
}

/// A YAML primitive (string, number or boolean).
fn yaml_value(v: &Json) -> String {
    match v {
        Json::String(s) => yaml_str(s),
        Json::Number(_) | Json::Bool(_) => js_stringify(v),
        other => js_stringify(other),
    }
}

fn sorted_entries<V>(mut entries: Vec<(&String, V)>) -> Vec<(&String, V)> {
    entries.sort_by(|(a, _), (b, _)| locale_compare(a, b));
    entries
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::convert::curl_parser::parse_curl;
    use crate::convert::normalizer::normalize;

    fn y(cmd: &str) -> String {
        serialize_to_yaml(
            &normalize(&parse_curl(cmd)).unwrap(),
            YamlOptions::default(),
        )
    }

    #[test]
    fn produces_valid_yaml_structure() {
        let s = y("curl -X POST -d '{\"name\":\"Alex\"}' https://api.example.com");
        assert!(s.contains("request:"));
        assert!(s.contains("method: POST"));
        assert!(s.contains("url: https://api.example.com"));
    }

    #[test]
    fn includes_headers_auth_body_and_warnings() {
        assert!(
            y("curl -H \"Accept: text/plain\" https://example.com")
                .contains("headers:\n    Accept: text/plain")
        );
        let s = y("curl -u admin:pass https://example.com");
        for want in ["auth:", "type: basic", "username: admin", "password: pass"] {
            assert!(s.contains(want), "{want} in {s}");
        }
        let s = y("curl -d '{\"name\":\"Alex\"}' https://example.com");
        assert!(s.contains("body:") && s.contains("json:") && s.contains("name: Alex"));
        assert!(y("curl --compressed https://example.com").contains("# Warning:"));
    }

    #[test]
    fn batch_serializer() {
        let mut a = normalize(&parse_curl("curl https://example.com/a")).unwrap();
        let mut b = normalize(&parse_curl("curl https://example.com/b")).unwrap();
        a.name = Some("first".into());
        b.name = Some("second".into());
        let s = serialize_batch_to_yaml(&[a, b], YamlOptions::default());
        assert!(s.contains("requests:") && s.contains("name: first") && s.contains("name: second"));
    }

    #[test]
    fn quoting_rules() {
        assert_eq!(yaml_str(""), "\"\"");
        assert_eq!(yaml_str("a: b"), "\"a: b\"");
        assert_eq!(yaml_str("http://x"), "http://x");
        assert_eq!(yaml_str("True"), "\"True\"");
        assert_eq!(yaml_str("-12.5"), "\"-12.5\"");
        assert_eq!(yaml_str("1.2.3"), "1.2.3");
        assert_eq!(yaml_str("a\"b\\"), "a\"b\\");
        assert_eq!(yaml_str("{a\"b\\"), "\"{a\\\"b\\\\\"");
        assert_eq!(yaml_key("a.b"), "\"a.b\"");
        assert_eq!(yaml_key("Content-Type"), "Content-Type");
    }
}
