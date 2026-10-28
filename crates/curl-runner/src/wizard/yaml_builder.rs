//! Builds the YAML configuration from wizard answers (`wizard/yaml-builder.ts`).
//!
//! `build_*` turn answers into a config value (key order and the JS
//! truthiness rules of the TS code), `to_yaml_string` is the TS serializer:
//! same layout, same bytes, wherever TS wrote YAML that reads back as the
//! config. Where it did not, this writes text that does (see
//! `crates/CHANGES-FROM-TS.md`).

use super::types::{AuthType, BodyType, ExpectStatus, WizardAnswers};
use curl_runner_core::jsval::{is_js_space, num_to_string};
use curl_runner_core::model::Json;
use curl_runner_core::snapshot::json::js_entries;
use curl_runner_core::yaml_text::{map_key_text, scalar_text};
use serde_json::Map;

/// JS truthiness of a JSON value.
pub fn js_truthy(v: &Json) -> bool {
    match v {
        Json::Null => false,
        Json::Bool(b) => *b,
        Json::Number(n) => n.as_f64().is_some_and(|x| x != 0.0 && !x.is_nan()),
        Json::String(s) => !s.is_empty(),
        Json::Array(_) | Json::Object(_) => true,
    }
}

fn truthy_num(x: Option<f64>) -> Option<f64> {
    x.filter(|n| *n != 0.0 && !n.is_nan())
}

/// A JS number as JSON (whole numbers as integers, like a YAML `5000`).
pub fn num(x: f64) -> Json {
    if x.fract() == 0.0 && x.abs() < 9_007_199_254_740_992.0 {
        return Json::from(x as i64);
    }
    serde_json::Number::from_f64(x).map_or(Json::Null, Json::Number)
}

fn non_empty(m: &Option<indexmap::IndexMap<String, Json>>) -> Option<Json> {
    m.as_ref()
        .filter(|m| !m.is_empty())
        .map(|m| Json::Object(m.iter().map(|(k, v)| (k.clone(), v.clone())).collect()))
}

fn s(v: &str) -> Json {
    Json::String(v.to_string())
}

/// Same data: numbers by value, objects without regard to key order. What a
/// file must read back as after it is rewritten.
pub fn same_data(a: &Json, b: &Json) -> bool {
    match (a, b) {
        (Json::Number(x), Json::Number(y)) => x.as_f64() == y.as_f64(),
        (Json::Array(x), Json::Array(y)) => {
            x.len() == y.len() && x.iter().zip(y).all(|(p, q)| same_data(p, q))
        }
        (Json::Object(x), Json::Object(y)) => {
            x.len() == y.len()
                && x.iter()
                    .all(|(k, v)| y.get(k).is_some_and(|w| same_data(v, w)))
        }
        _ => a == b,
    }
}

/// `buildRequestConfig`.
pub fn build_request_config(a: &WizardAnswers) -> Json {
    let mut c = Map::new();
    c.insert("url".into(), s(&a.url));

    if let Some(n) = a.name.as_deref().filter(|n| !n.is_empty()) {
        c.insert("name".into(), s(n));
    }
    // Method (only when not GET)
    if !a.method.is_empty() && a.method != "GET" {
        c.insert("method".into(), s(&a.method));
    }
    if let Some(h) = non_empty(&a.headers) {
        c.insert("headers".into(), h);
    }
    if let Some(p) = non_empty(&a.params) {
        c.insert("params".into(), p);
    }

    // Body
    match a.body_type {
        Some(BodyType::Json | BodyType::Raw) => {
            if let Some(b) = a.body.as_ref().filter(|b| js_truthy(b)) {
                c.insert("body".into(), b.clone());
            }
        }
        Some(BodyType::Form) => {
            if let Some(f) = &a.form_data {
                c.insert(
                    "formData".into(),
                    Json::Object(f.iter().map(|(k, v)| (k.clone(), v.clone())).collect()),
                );
            }
        }
        _ => {}
    }

    // Auth
    match a.auth_type {
        Some(AuthType::Basic) if a.username.as_deref().is_some_and(|u| !u.is_empty()) => {
            let mut auth = Map::new();
            auth.insert("type".into(), s("basic"));
            auth.insert(
                "username".into(),
                s(a.username.as_deref().unwrap_or_default()),
            );
            auth.insert(
                "password".into(),
                s(a.password.as_deref().unwrap_or_default()),
            );
            c.insert("auth".into(), Json::Object(auth));
        }
        Some(AuthType::Bearer) if a.token.as_deref().is_some_and(|t| !t.is_empty()) => {
            let mut auth = Map::new();
            auth.insert("type".into(), s("bearer"));
            auth.insert("token".into(), s(a.token.as_deref().unwrap_or_default()));
            c.insert("auth".into(), Json::Object(auth));
        }
        _ => {}
    }

    if let Some(t) = truthy_num(a.timeout) {
        c.insert("timeout".into(), num(t));
    }
    if a.follow_redirects == Some(false) {
        c.insert("followRedirects".into(), Json::Bool(false));
    }
    if let Some(m) = truthy_num(a.max_redirects) {
        c.insert("maxRedirects".into(), num(m));
    }
    if a.insecure == Some(true) {
        c.insert("insecure".into(), Json::Bool(true));
    }
    if a.http2 == Some(true) {
        c.insert("http2".into(), Json::Bool(true));
    }

    // Retry
    if a.retry_enabled == Some(true)
        && let Some(count) = truthy_num(a.retry_count)
    {
        let mut r = Map::new();
        r.insert("count".into(), num(count));
        if let Some(d) = truthy_num(a.retry_delay) {
            r.insert("delay".into(), num(d));
        }
        if let Some(b) = truthy_num(a.retry_backoff) {
            r.insert("backoff".into(), num(b));
        }
        c.insert("retry".into(), Json::Object(r));
    }

    // Validation (expect)
    let status = a.expect_status.as_ref().filter(|s| match s {
        ExpectStatus::One(n) => truthy_num(Some(*n)).is_some(),
        ExpectStatus::Many(_) => true,
    });
    let time = a.expect_response_time.as_deref().filter(|t| !t.is_empty());
    if status.is_some() || time.is_some() {
        let mut e = Map::new();
        match status {
            Some(ExpectStatus::One(n)) => {
                e.insert("status".into(), num(*n));
            }
            Some(ExpectStatus::Many(v)) => {
                e.insert(
                    "status".into(),
                    Json::Array(v.iter().map(|n| num(*n)).collect()),
                );
            }
            None => {}
        }
        if let Some(t) = time {
            e.insert("responseTime".into(), s(t));
        }
        c.insert("expect".into(), Json::Object(e));
    }

    if let Some(o) = a.output_file.as_deref().filter(|o| !o.is_empty()) {
        c.insert("output".into(), s(o));
    }
    Json::Object(c)
}

/// `buildYamlConfig`: the request under a `request` key.
pub fn build_yaml_config(answers: &WizardAnswers) -> Json {
    let mut m = Map::new();
    m.insert("request".into(), build_request_config(answers));
    Json::Object(m)
}

/// `toYamlString`.
pub fn to_yaml_string(config: &Json) -> String {
    serialize(config, 0)
}

/// The strings TS writes quoted (`JSON.stringify`): a `:` `#` or newline,
/// a leading or trailing space, a leading digit, `true`/`false`/`null`/
/// `yes`/`no` in any case. Anything else TS wrote plain; `scalar_text` also
/// quotes what would not read back (`*/*`, `~`, `- x`, the empty string, ...).
fn ts_quotes(s: &str) -> bool {
    s.contains([':', '#', '\n'])
        || s.starts_with(' ')
        || s.ends_with(' ')
        || s.starts_with(|c: char| c.is_ascii_digit())
        || matches!(
            s.to_lowercase().as_str(),
            "true" | "false" | "null" | "yes" | "no"
        )
}

fn scalar(v: &Json, flow: bool) -> String {
    match v {
        Json::Null => "null".into(),
        Json::Bool(b) => b.to_string(),
        Json::Number(n) => num_to_string(n.as_f64().unwrap_or(f64::NAN)),
        Json::String(s) => scalar_text(s, ts_quotes(s), flow),
        Json::Array(_) | Json::Object(_) => serialize(v, 0),
    }
}

fn is_container(v: &Json) -> bool {
    matches!(v, Json::Array(_) | Json::Object(_))
}

/// An array written as `[a, b]` on one line (TS: every item is a primitive).
fn is_simple_array(a: &[Json]) -> bool {
    !a.is_empty() && !a.iter().any(is_container)
}

/// `serializeToYaml`.
fn serialize(v: &Json, indent: usize) -> String {
    let spaces = "  ".repeat(indent);
    match v {
        Json::Array(a) if a.is_empty() => "[]".into(),
        Json::Array(a) if is_simple_array(a) => {
            let items: Vec<String> = a.iter().map(|i| scalar(i, true)).collect();
            format!("[{}]", items.join(", "))
        }
        Json::Array(a) => a
            .iter()
            .map(|item| {
                let text = serialize(item, indent + 1);
                let text = if is_container(item) {
                    text.trim_start_matches(is_js_space)
                } else {
                    text.as_str()
                };
                format!("{spaces}- {text}")
            })
            .collect::<Vec<_>>()
            .join("\n"),
        Json::Object(m) if m.is_empty() => "{}".into(),
        Json::Object(m) => js_entries(m)
            .into_iter()
            .map(|(k, val)| {
                let key = map_key_text(k);
                match val {
                    Json::Object(o) if !o.is_empty() => {
                        format!("{spaces}{key}:\n{}", serialize(val, indent + 1))
                    }
                    // TS wrote these after `key:` on a line of their own
                    // (`key:\n{}`, or `key:   - a: 1` for a one-line list),
                    // which is not YAML that reads back.
                    Json::Array(a) if !a.is_empty() && !is_simple_array(a) => {
                        format!("{spaces}{key}:\n{}", serialize(val, indent + 1))
                    }
                    _ => format!("{spaces}{key}: {}", serialize(val, indent + 1)),
                }
            })
            .collect::<Vec<_>>()
            .join("\n"),
        scalar_value => scalar(scalar_value, false),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use curl_runner_core::parser::parse_raw;
    use serde_json::json;
    use std::path::{Path, PathBuf};

    fn answers(v: Json) -> WizardAnswers {
        serde_json::from_value(v).unwrap()
    }

    fn cfg(v: Json) -> Json {
        build_request_config(&answers(v))
    }

    // ---- yaml-builder.test.ts: buildRequestConfig ----

    #[test]
    fn minimal_config_with_just_url() {
        assert_eq!(
            cfg(json!({"url": "https://api.example.com", "method": "GET"})),
            json!({"url": "https://api.example.com"})
        );
    }

    #[test]
    fn includes_method_when_not_get() {
        let c = cfg(json!({"url": "https://api.example.com", "method": "POST"}));
        assert_eq!(c["method"], "POST");
    }

    #[test]
    fn includes_name_when_provided() {
        let c = cfg(json!({"url": "u", "method": "GET", "name": "My Request"}));
        assert_eq!(c["name"], "My Request");
    }

    #[test]
    fn includes_headers_when_provided() {
        let h = json!({"Content-Type": "application/json", "Authorization": "Bearer token"});
        let c = cfg(json!({"url": "u", "method": "GET", "headers": h}));
        assert_eq!(c["headers"], h);
    }

    #[test]
    fn includes_json_body() {
        let c = cfg(json!({"url": "u", "method": "POST", "bodyType": "json",
            "body": {"name": "test", "value": 123}}));
        assert_eq!(c["body"], json!({"name": "test", "value": 123}));
    }

    #[test]
    fn includes_form_data() {
        let f = json!({"field1": "value1", "file": {"file": "./upload.txt"}});
        let c = cfg(json!({"url": "u", "method": "POST", "bodyType": "form", "formData": f}));
        assert_eq!(c["formData"], f);
    }

    #[test]
    fn includes_raw_body() {
        let c = cfg(json!({"url": "u", "method": "POST", "bodyType": "raw",
            "body": "plain text content"}));
        assert_eq!(c["body"], "plain text content");
    }

    #[test]
    fn includes_bearer_auth() {
        let c =
            cfg(json!({"url": "u", "method": "GET", "authType": "bearer", "token": "my-token"}));
        assert_eq!(c["auth"], json!({"type": "bearer", "token": "my-token"}));
    }

    #[test]
    fn includes_basic_auth() {
        let c = cfg(json!({"url": "u", "method": "GET", "authType": "basic",
            "username": "user", "password": "pass"}));
        assert_eq!(
            c["auth"],
            json!({"type": "basic", "username": "user", "password": "pass"})
        );
    }

    #[test]
    fn includes_timeout() {
        assert_eq!(
            cfg(json!({"url": "u", "method": "GET", "timeout": 5000}))["timeout"],
            5000
        );
    }

    #[test]
    fn includes_follow_redirects_when_false() {
        let c = cfg(json!({"url": "u", "method": "GET", "followRedirects": false}));
        assert_eq!(c["followRedirects"], false);
    }

    #[test]
    fn includes_insecure_flag() {
        assert_eq!(
            cfg(json!({"url": "u", "method": "GET", "insecure": true}))["insecure"],
            true
        );
    }

    #[test]
    fn includes_http2_flag() {
        assert_eq!(
            cfg(json!({"url": "u", "method": "GET", "http2": true}))["http2"],
            true
        );
    }

    #[test]
    fn includes_retry_config() {
        let c = cfg(json!({"url": "u", "method": "GET", "retryEnabled": true,
            "retryCount": 3, "retryDelay": 1000, "retryBackoff": 2}));
        assert_eq!(c["retry"], json!({"count": 3, "delay": 1000, "backoff": 2}));
    }

    #[test]
    fn includes_expect_validation() {
        let c = cfg(json!({"url": "u", "method": "GET", "expectStatus": 200,
            "expectResponseTime": "< 2000"}));
        assert_eq!(
            c["expect"],
            json!({"status": 200, "responseTime": "< 2000"})
        );
    }

    #[test]
    fn handles_array_of_status_codes() {
        let c = cfg(json!({"url": "u", "method": "GET", "expectStatus": [200, 201]}));
        assert_eq!(c["expect"]["status"], json!([200, 201]));
    }

    // ---- yaml-builder.test.ts: buildYamlConfig ----

    #[test]
    fn wraps_config_in_request_key() {
        let c = build_yaml_config(&answers(
            json!({"url": "https://api.example.com", "method": "GET"}),
        ));
        assert_eq!(c["request"]["url"], "https://api.example.com");
    }

    // ---- key order and truthiness (TS object insertion order) ----

    #[test]
    fn key_order_is_the_ts_one() {
        let c = cfg(json!({
            "url": "u", "method": "PUT", "name": "n", "headers": {"A": "b"}, "params": {"p": "1"},
            "bodyType": "json", "body": {"a": 1}, "authType": "bearer", "token": "t",
            "timeout": 1, "followRedirects": false, "maxRedirects": 2, "insecure": true,
            "http2": true, "retryEnabled": true, "retryCount": 1, "retryDelay": 2,
            "retryBackoff": 3, "expectStatus": 200, "expectResponseTime": "< 1",
            "outputFile": "o.json"
        }));
        let keys: Vec<&str> = c.as_object().unwrap().keys().map(String::as_str).collect();
        assert_eq!(
            keys,
            [
                "url",
                "name",
                "method",
                "headers",
                "params",
                "body",
                "auth",
                "timeout",
                "followRedirects",
                "maxRedirects",
                "insecure",
                "http2",
                "retry",
                "expect",
                "output"
            ]
        );
        assert_eq!(
            c["retry"].as_object().unwrap().keys().collect::<Vec<_>>(),
            ["count", "delay", "backoff"]
        );
    }

    #[test]
    fn falsy_values_are_dropped_like_js() {
        let c = cfg(json!({
            "url": "u", "method": "POST", "name": "", "headers": {}, "params": {},
            "bodyType": "json", "body": 0, "timeout": 0, "maxRedirects": 0, "insecure": false,
            "http2": false, "retryEnabled": true, "retryCount": 0, "expectResponseTime": "",
            "outputFile": ""
        }));
        assert_eq!(c, json!({"url": "u", "method": "POST"}));
        // an empty object body and empty form data are truthy in JS
        let c = cfg(json!({"url": "u", "bodyType": "json", "body": {}}));
        assert_eq!(c["body"], json!({}));
        let c = cfg(json!({"url": "u", "bodyType": "form", "formData": {}}));
        assert_eq!(c["formData"], json!({}));
        assert!(js_truthy(&json!([])) && !js_truthy(&json!(null)) && !js_truthy(&json!("")));
    }

    #[test]
    fn auth_needs_its_credential_and_basic_password_defaults_empty() {
        assert_eq!(
            cfg(json!({"url": "u", "authType": "bearer"})),
            json!({"url": "u"})
        );
        assert_eq!(
            cfg(json!({"url": "u", "authType": "basic", "password": "p"})),
            json!({"url": "u"})
        );
        let c = cfg(json!({"url": "u", "authType": "basic", "username": "me"}));
        assert_eq!(c["auth"]["password"], "");
    }

    #[test]
    fn body_none_and_retry_disabled_are_ignored() {
        let c = cfg(json!({"url": "u", "bodyType": "none", "body": {"a": 1},
            "retryEnabled": false, "retryCount": 5}));
        assert_eq!(c, json!({"url": "u"}));
    }

    // ---- toYamlString (yaml-builder.test.ts) ----

    fn yaml(v: Json) -> String {
        to_yaml_string(&build_yaml_config(&answers(v)))
    }

    #[test]
    fn serializes_simple_config() {
        let y = yaml(json!({"url": "https://api.example.com", "method": "GET"}));
        assert!(y.contains("request:"));
        assert!(y.contains("url: \"https://api.example.com\""));
    }

    #[test]
    fn serializes_post_with_body() {
        let y = yaml(json!({"url": "https://api.example.com", "method": "POST",
            "bodyType": "json", "body": {"name": "test"}}));
        assert!(y.contains("method: POST") && y.contains("body:") && y.contains("name: test"));
    }

    #[test]
    fn quotes_strings_with_special_characters() {
        let y = yaml(json!({"url": "u", "method": "POST", "bodyType": "json",
            "body": {"message": "hello: world"}}));
        assert!(y.contains("\"hello: world\""));
    }

    #[test]
    fn serializes_arrays_inline_for_simple_values() {
        let y = yaml(json!({"url": "u", "method": "GET", "expectStatus": [200, 201]}));
        assert!(y.contains("[200, 201]"));
    }

    #[test]
    fn output_has_no_trailing_newline_like_ts() {
        assert_eq!(
            yaml(json!({"url": "a", "method": "GET"})),
            "request:\n  url: a"
        );
    }

    // ---- goldens recorded from the TS implementation ----

    fn golden_dir() -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/golden/wizard")
    }

    /// `{...template.defaults, ...answers}` as JSON.
    fn case_answers(case: &Json) -> WizardAnswers {
        let mut merged = match case.get("template").and_then(Json::as_str) {
            Some(key) => {
                let t = super::super::types::WizardTemplate::from_key(key).unwrap();
                serde_json::to_value(super::super::templates::get_template(t).defaults).unwrap()
            }
            None => json!({}),
        };
        for (k, v) in case["answers"].as_object().unwrap() {
            merged[k] = v.clone();
        }
        answers(merged)
    }

    #[test]
    fn goldens_match_ts_bytes_or_read_back_where_ts_could_not() {
        let dir = golden_dir();
        let cases: Vec<Json> =
            serde_json::from_slice(&std::fs::read(dir.join("cases.json")).unwrap()).unwrap();
        assert!(cases.len() > 70);
        let mut deviating = Vec::new();
        for case in &cases {
            let name = case["name"].as_str().unwrap();
            let a = case_answers(case);
            let config = build_yaml_config(&a);
            let got = to_yaml_string(&config);
            let want = std::fs::read(dir.join(format!("{name}.yaml"))).unwrap();
            let intended: Json =
                serde_json::from_slice(&std::fs::read(dir.join(format!("{name}.json"))).unwrap())
                    .unwrap();
            assert!(
                same_data(&config["request"], &intended["request"]),
                "{name}: config differs"
            );
            // Whatever we write must parse back to the intended config.
            let back = parse_raw(&got).unwrap_or_else(|e| panic!("{name}: {e}\n{got}"));
            assert!(
                same_data(&back, &config),
                "{name}: reads back as {back}\n{got}"
            );
            if case["tsRoundTrips"].as_bool().unwrap() {
                assert_eq!(got.as_bytes(), want.as_slice(), "{name}");
            } else {
                assert_ne!(got.as_bytes(), want.as_slice(), "{name}: TS text was fine?");
                deviating.push(name.to_string());
            }
        }
        // 18 as of the recorded goldens: only inputs where TS wrote broken YAML.
        assert_eq!(deviating.len(), 18, "{deviating:?}");
    }

    #[test]
    fn broken_ts_outputs_are_fixed_locally() {
        // single object item in a list: TS wrote `items:   - id: 1`
        let y = yaml(json!({"url": "u", "bodyType": "json", "method": "POST",
            "body": {"items": [{"id": 1}]}}));
        assert!(y.contains("items:\n"), "{y}");
        // empty containers: TS wrote `a:\n{}`
        let y = yaml(json!({"url": "u", "bodyType": "json", "method": "POST",
            "body": {"a": {}, "b": []}}));
        assert!(y.contains("a: {}") && y.contains("b: []"), "{y}");
        // empty password: TS wrote `password: ` (null)
        let y = yaml(json!({"url": "u", "authType": "basic", "username": "x"}));
        assert!(y.contains("password: \"\""), "{y}");
    }

    #[test]
    fn integer_like_keys_come_first_like_js() {
        let y = yaml(json!({"url": "u", "method": "POST", "bodyType": "json",
            "body": {"b": 1, "2": "x", "a": 2, "1": "y"}}));
        assert!(
            y.contains("body:\n    1: y\n    2: x\n    b: 1\n    a: 2"),
            "{y}"
        );
    }
}
