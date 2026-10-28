//! Loads an existing YAML file into wizard answers (edit mode)
//! (`wizard/parser.ts`).

use super::types::{AuthType, BodyType, ExpectStatus, WizardAnswers};
use super::yaml_builder::js_truthy;
use curl_runner_core::jsval::num_to_string;
use curl_runner_core::model::Json;
use curl_runner_core::parser;
use indexmap::IndexMap;
use std::path::Path;

/// Where the edited request sits in the document (TS looks at `request`, then
/// the first of `requests`, then the first of `collection.requests`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RequestLoc {
    Request,
    Requests,
    Collection,
}

impl RequestLoc {
    fn find(root: &Json) -> Option<RequestLoc> {
        let first = |v: Option<&Json>| v.and_then(Json::as_array).is_some_and(|a| !a.is_empty());
        if root.get("request").is_some_and(js_truthy) {
            Some(RequestLoc::Request)
        } else if first(root.get("requests")) {
            Some(RequestLoc::Requests)
        } else if first(root.get("collection").and_then(|c| c.get("requests"))) {
            Some(RequestLoc::Collection)
        } else {
            None
        }
    }

    pub fn get(self, root: &Json) -> Option<&Json> {
        match self {
            RequestLoc::Request => root.get("request"),
            RequestLoc::Requests => root.get("requests")?.get(0),
            RequestLoc::Collection => root.get("collection")?.get("requests")?.get(0),
        }
    }

    pub fn get_mut(self, root: &mut Json) -> Option<&mut Json> {
        match self {
            RequestLoc::Request => root.get_mut("request"),
            RequestLoc::Requests => root.get_mut("requests")?.get_mut(0),
            RequestLoc::Collection => root.get_mut("collection")?.get_mut("requests")?.get_mut(0),
        }
    }
}

pub const NO_REQUEST: &str = "No request found in YAML file";

/// A parsed document and its request, found by the TS rules.
pub struct Loaded {
    pub root: Json,
    pub loc: RequestLoc,
    pub answers: WizardAnswers,
}

/// Parse `content` (any YAML) and find the request. A document that is not
/// one mapping, or has no request (or an edited request that is not a
/// mapping), is an error.
#[cfg(test)]
pub fn load_str(content: &str) -> Result<Loaded, String> {
    let root = parser::parse_raw(content).map_err(|e| e.to_string())?;
    from_root(root)
}

fn from_root(root: Json) -> Result<Loaded, String> {
    if !root.is_object() {
        return Err(format!(
            "{NO_REQUEST} (the file must be a single YAML mapping document)"
        ));
    }
    let loc = RequestLoc::find(&root).ok_or_else(|| NO_REQUEST.to_string())?;
    let request = loc.get(&root).filter(|r| r.is_object());
    let answers = request_to_answers(request.ok_or_else(|| NO_REQUEST.to_string())?);
    Ok(Loaded { root, loc, answers })
}

/// `loadYamlFile`, keeping the document so an edit can write the rest back.
pub fn load_file(path: &Path) -> Result<Loaded, String> {
    from_root(parser::load_raw(path).map_err(|e| e.to_string())?)
}

/// `loadYamlFile`.
#[cfg(test)]
pub fn load_yaml_file(path: &Path) -> Result<WizardAnswers, String> {
    load_file(path).map(|l| l.answers)
}

/// `fileExists`.
pub fn file_exists(path: &Path) -> bool {
    path.exists()
}

/// A scalar read as the string the wizard shows (`token: 123` is `"123"`).
fn text(v: Option<&Json>) -> Option<String> {
    match v? {
        Json::String(s) => Some(s.clone()),
        Json::Number(n) => Some(num_to_string(n.as_f64()?)),
        Json::Bool(b) => Some(b.to_string()),
        _ => None,
    }
}

fn truthy_text(v: Option<&Json>) -> Option<String> {
    v.filter(|v| js_truthy(v)).and_then(|v| text(Some(v)))
}

fn truthy_num(v: Option<&Json>) -> Option<f64> {
    v.filter(|v| js_truthy(v)).and_then(Json::as_f64)
}

fn truthy_true(v: Option<&Json>) -> Option<bool> {
    v.and_then(Json::as_bool).filter(|b| *b)
}

fn entries(v: Option<&Json>) -> Option<IndexMap<String, Json>> {
    let m = v?.as_object().filter(|m| !m.is_empty())?;
    Some(m.iter().map(|(k, v)| (k.clone(), v.clone())).collect())
}

/// `requestToAnswers`.
pub fn request_to_answers(request: &Json) -> WizardAnswers {
    let get = |k: &str| request.get(k);
    let mut a = WizardAnswers {
        url: text(get("url")).unwrap_or_default(),
        method: truthy_text(get("method")).unwrap_or_else(|| "GET".into()),
        ..Default::default()
    };

    a.name = truthy_text(get("name"));
    a.headers = entries(get("headers"));
    a.params = entries(get("params"));

    // Body
    if let Some(body) = get("body") {
        if matches!(body, Json::Object(_) | Json::Array(_)) {
            a.body_type = Some(BodyType::Json);
        } else {
            a.body_type = Some(BodyType::Raw);
        }
        a.body = Some(body.clone());
    } else if let Some(Json::Object(f)) = get("formData").filter(|v| js_truthy(v)) {
        a.body_type = Some(BodyType::Form);
        a.form_data = Some(f.iter().map(|(k, v)| (k.clone(), v.clone())).collect());
    } else {
        a.body_type = Some(BodyType::None);
    }

    // Auth
    match get("auth").filter(|v| js_truthy(v)) {
        Some(auth) => match auth.get("type").and_then(Json::as_str) {
            Some("bearer") => {
                a.auth_type = Some(AuthType::Bearer);
                a.token = text(auth.get("token"));
            }
            Some("basic") => {
                a.auth_type = Some(AuthType::Basic);
                a.username = text(auth.get("username"));
                a.password = text(auth.get("password"));
            }
            _ => {}
        },
        None => a.auth_type = Some(AuthType::None),
    }

    // Advanced
    a.timeout = truthy_num(get("timeout"));
    a.follow_redirects = get("followRedirects").and_then(Json::as_bool);
    a.max_redirects = truthy_num(get("maxRedirects"));
    a.insecure = truthy_true(get("insecure"));
    a.http2 = truthy_true(get("http2"));

    // Retry
    if let Some(retry) = get("retry").filter(|v| js_truthy(v)) {
        a.retry_enabled = Some(true);
        a.retry_count = retry.get("count").and_then(Json::as_f64);
        a.retry_delay = retry.get("delay").and_then(Json::as_f64);
        a.retry_backoff = retry.get("backoff").and_then(Json::as_f64);
    }

    // Validation
    if let Some(expect) = get("expect") {
        a.expect_status = match expect.get("status").filter(|v| js_truthy(v)) {
            Some(Json::Number(n)) => n.as_f64().map(ExpectStatus::One),
            Some(Json::Array(list)) => list
                .iter()
                .map(Json::as_f64)
                .collect::<Option<Vec<_>>>()
                .map(ExpectStatus::Many),
            _ => None,
        };
        a.expect_response_time = truthy_text(expect.get("responseTime"));
    }

    a.output_file = truthy_text(get("output"));
    a
}

// parser.test.ts
#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn conv(v: Json) -> WizardAnswers {
        request_to_answers(&v)
    }

    #[test]
    fn converts_minimal_request() {
        let a = conv(json!({"url": "https://api.example.com"}));
        assert_eq!(a.url, "https://api.example.com");
        assert_eq!(a.method, "GET");
    }

    #[test]
    fn converts_request_with_method() {
        assert_eq!(conv(json!({"url": "u", "method": "POST"})).method, "POST");
    }

    #[test]
    fn converts_request_with_name() {
        assert_eq!(
            conv(json!({"url": "u", "name": "My Request"}))
                .name
                .as_deref(),
            Some("My Request")
        );
    }

    #[test]
    fn converts_request_with_headers() {
        let a = conv(json!({"url": "u", "headers": {"Content-Type": "application/json"}}));
        let h = a.headers.unwrap();
        assert_eq!(h.len(), 1);
        assert_eq!(h["Content-Type"], "application/json");
    }

    #[test]
    fn converts_request_with_json_body() {
        let a = conv(json!({"url": "u", "body": {"key": "value"}}));
        assert_eq!(a.body_type, Some(BodyType::Json));
        assert_eq!(a.body, Some(json!({"key": "value"})));
    }

    #[test]
    fn converts_request_with_string_body() {
        let a = conv(json!({"url": "u", "body": "plain text"}));
        assert_eq!(a.body_type, Some(BodyType::Raw));
        assert_eq!(a.body, Some(json!("plain text")));
    }

    #[test]
    fn converts_request_with_form_data() {
        let a = conv(json!({"url": "u", "formData": {"field": "value"}}));
        assert_eq!(a.body_type, Some(BodyType::Form));
        assert_eq!(a.form_data.unwrap()["field"], "value");
    }

    #[test]
    fn no_body_is_none() {
        assert_eq!(conv(json!({"url": "u"})).body_type, Some(BodyType::None));
    }

    #[test]
    fn converts_request_with_bearer_auth() {
        let a = conv(json!({"url": "u", "auth": {"type": "bearer", "token": "my-token"}}));
        assert_eq!(a.auth_type, Some(AuthType::Bearer));
        assert_eq!(a.token.as_deref(), Some("my-token"));
    }

    #[test]
    fn converts_request_with_basic_auth() {
        let a = conv(
            json!({"url": "u", "auth": {"type": "basic", "username": "user", "password": "pass"}}),
        );
        assert_eq!(a.auth_type, Some(AuthType::Basic));
        assert_eq!(a.username.as_deref(), Some("user"));
        assert_eq!(a.password.as_deref(), Some("pass"));
    }

    #[test]
    fn no_auth_is_none_and_unknown_auth_type_is_unset() {
        assert_eq!(conv(json!({"url": "u"})).auth_type, Some(AuthType::None));
        assert_eq!(
            conv(json!({"url": "u", "auth": {"type": "digest"}})).auth_type,
            None
        );
    }

    #[test]
    fn converts_request_with_timeout() {
        assert_eq!(
            conv(json!({"url": "u", "timeout": 5000})).timeout,
            Some(5000.0)
        );
    }

    #[test]
    fn converts_request_with_retry_config() {
        let a = conv(json!({"url": "u", "retry": {"count": 3, "delay": 1000, "backoff": 2}}));
        assert_eq!(a.retry_enabled, Some(true));
        assert_eq!(a.retry_count, Some(3.0));
        assert_eq!(a.retry_delay, Some(1000.0));
        assert_eq!(a.retry_backoff, Some(2.0));
    }

    #[test]
    fn converts_request_with_expect_validation() {
        let a = conv(json!({"url": "u", "expect": {"status": 200, "responseTime": "< 2000"}}));
        assert_eq!(a.expect_status, Some(ExpectStatus::One(200.0)));
        assert_eq!(a.expect_response_time.as_deref(), Some("< 2000"));
        let a = conv(json!({"url": "u", "expect": {"status": [200, 201]}}));
        assert_eq!(
            a.expect_status,
            Some(ExpectStatus::Many(vec![200.0, 201.0]))
        );
    }

    #[test]
    fn converts_insecure_http2_redirect_output() {
        let a = conv(
            json!({"url": "u", "insecure": true, "http2": true, "followRedirects": false,
            "maxRedirects": 4, "output": "o.json"}),
        );
        assert_eq!(a.insecure, Some(true));
        assert_eq!(a.http2, Some(true));
        assert_eq!(a.follow_redirects, Some(false));
        assert_eq!(a.max_redirects, Some(4.0));
        assert_eq!(a.output_file.as_deref(), Some("o.json"));
    }

    fn write(dir: &Path, name: &str, content: &str) -> std::path::PathBuf {
        let p = dir.join(name);
        std::fs::write(&p, content).unwrap();
        p
    }

    #[test]
    fn loads_single_request_yaml() {
        let d = tempfile::tempdir().unwrap();
        let p = write(
            d.path(),
            "single-request.yaml",
            "\nrequest:\n  url: https://api.example.com\n  method: POST\n  headers:\n    Content-Type: application/json\n  body:\n    name: test\n",
        );
        let a = load_yaml_file(&p).unwrap();
        assert_eq!(a.url, "https://api.example.com");
        assert_eq!(a.method, "POST");
        assert_eq!(a.headers.unwrap()["Content-Type"], "application/json");
    }

    #[test]
    fn loads_first_request_from_requests_array() {
        let d = tempfile::tempdir().unwrap();
        let p = write(
            d.path(),
            "multi.yaml",
            "\nrequests:\n  - url: https://api.example.com/first\n    method: GET\n  - url: https://api.example.com/second\n    method: POST\n",
        );
        let a = load_yaml_file(&p).unwrap();
        assert_eq!(a.url, "https://api.example.com/first");
        assert_eq!(a.method, "GET");
    }

    #[test]
    fn loads_first_request_from_collection() {
        let d = tempfile::tempdir().unwrap();
        let p = write(
            d.path(),
            "collection.yaml",
            "\ncollection:\n  name: Test Collection\n  requests:\n    - url: https://api.example.com/collection\n      method: PUT\n",
        );
        let a = load_yaml_file(&p).unwrap();
        assert_eq!(a.url, "https://api.example.com/collection");
        assert_eq!(a.method, "PUT");
    }

    #[test]
    fn errors_for_file_with_no_requests() {
        let d = tempfile::tempdir().unwrap();
        let p = write(
            d.path(),
            "empty.yaml",
            "\nglobal:\n  variables:\n    BASE_URL: https://api.example.com\n",
        );
        let e = load_yaml_file(&p).unwrap_err();
        assert!(e.contains("No request found"), "{e}");
    }

    #[test]
    fn errors_for_non_mapping_roots_and_bad_yaml() {
        for src in [
            "- a\n- b\n",
            "just text\n",
            "---\na: 1\n---\nb: 2\n",
            "a: [\n",
            "",
        ] {
            let e = load_str(src).err().unwrap_or_default();
            assert!(!e.is_empty(), "{src:?} should not load");
        }
        assert!(load_str("request: hi\n").is_err());
        assert!(load_str("requests: []\n").is_err());
    }

    #[test]
    fn file_exists_reports_presence() {
        let d = tempfile::tempdir().unwrap();
        let p = write(d.path(), "exists.yaml", "test");
        assert!(file_exists(&p));
        assert!(!file_exists(&d.path().join("does-not-exist.yaml")));
    }
}
