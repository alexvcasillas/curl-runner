//! Response store extraction (`utils/response-store.ts`).

use crate::interpolate::Store;
use crate::jsval;
use crate::model::{HeaderValue, Json, StoreConfig};
use crate::transport::Response;
use std::borrow::Cow;

/// Keys never traversed or stored (prototype-pollution guard in TS).
const FORBIDDEN_KEYS: [&str; 3] = ["__proto__", "prototype", "constructor"];

fn is_word(s: &str) -> bool {
    !s.is_empty() && s.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'_')
}

/// `^(\w+)\[(\d+)\]$`
fn array_match(part: &str) -> Option<(&str, &str)> {
    let inner = part.strip_suffix(']')?;
    let (key, idx) = inner.split_once('[')?;
    (is_word(key) && !idx.is_empty() && idx.bytes().all(|b| b.is_ascii_digit()))
        .then_some((key, idx))
}

/// Property access by reference; only `length` is computed (owned).
pub(crate) fn prop<'a>(current: &'a Json, key: &str) -> Option<Cow<'a, Json>> {
    match current {
        Json::Object(m) => m.get(key).map(Cow::Borrowed),
        Json::Array(a) if key == "length" => Some(Cow::Owned(Json::from(a.len()))),
        _ => None,
    }
}

/// Walk `path` by reference; only computed values (`length`) are owned.
pub(crate) fn walk<'a>(obj: &'a Json, path: &str) -> Option<Cow<'a, Json>> {
    let mut current: Cow<'a, Json> = Cow::Borrowed(obj);
    for part in path.split('.') {
        // a computed (owned) value is a number: nothing further to traverse
        let Cow::Borrowed(cur) = current else {
            return None;
        };
        if !matches!(cur, Json::Object(_) | Json::Array(_)) {
            return None;
        }
        current = if let Some((key, idx)) = array_match(part) {
            if FORBIDDEN_KEYS.contains(&key) {
                return None;
            }
            let index: usize = idx.parse().ok()?;
            match prop(cur, key)? {
                Cow::Borrowed(Json::Array(a)) => Cow::Borrowed(a.get(index)?),
                _ => return None,
            }
        } else if let (Json::Array(a), true) = (
            cur,
            !part.is_empty() && part.bytes().all(|b| b.is_ascii_digit()),
        ) {
            Cow::Borrowed(a.get(part.parse::<usize>().ok()?)?)
        } else {
            if FORBIDDEN_KEYS.contains(&part) {
                return None;
            }
            prop(cur, part)?
        };
    }
    Some(current)
}

/// TS `getValueByPath`: dot paths with `0` / `[0]` array indices. `None` is
/// `undefined`; JSON `null` is `Some(Json::Null)`. Only the leaf is cloned.
pub fn get_value_by_path(obj: &Json, path: &str) -> Option<Json> {
    walk(obj, path).map(Cow::into_owned)
}

/// TS `valueToString`: undefined/null -> "", objects/arrays -> JSON.
pub fn value_to_string(v: Option<&Json>) -> String {
    match v {
        None | Some(Json::Null) => String::new(),
        Some(Json::String(s)) => s.clone(),
        Some(other) => jsval::stringify(other),
    }
}

/// status/headers/metrics as the TS `responseObj` (body is walked in place).
fn response_rest(resp: &Response) -> Json {
    let headers: serde_json::Map<String, Json> = resp
        .headers
        .iter()
        .map(|(k, v)| {
            let v = match v {
                HeaderValue::One(s) => Json::String(s.clone()),
                HeaderValue::Many(vs) => {
                    Json::Array(vs.iter().cloned().map(Json::String).collect())
                }
            };
            (k.clone(), v)
        })
        .collect();
    let mut obj = serde_json::Map::new();
    obj.insert("status".into(), Json::from(resp.status));
    obj.insert("headers".into(), Json::Object(headers));
    obj.insert(
        "metrics".into(),
        serde_json::to_value(&resp.metrics).unwrap_or(Json::Null),
    );
    Json::Object(obj)
}

/// Resolve `path` against the response without cloning the body.
fn resolve<'a>(resp: &'a Response, rest: &'a Json, path: &str) -> Option<Cow<'a, Json>> {
    let (first, tail) = match path.split_once('.') {
        Some((f, t)) => (f, Some(t)),
        None => (path, None),
    };
    let (name, index) = match array_match(first) {
        Some((k, i)) => (k, Some(i)),
        None => (first, None),
    };
    if name != "body" {
        return walk(rest, path);
    }
    let base: &Json = match index {
        None => &resp.body,
        Some(i) => match &resp.body {
            Json::Array(a) => a.get(i.parse::<usize>().ok()?)?,
            _ => return None,
        },
    };
    match tail {
        None => Some(Cow::Borrowed(base)),
        Some(t) => walk(base, t),
    }
}

/// TS `extractStoreValues`, merged into `into` (new values override).
pub fn extract(cfg: &StoreConfig, resp: &Response, into: &mut Store) {
    let rest = response_rest(resp);
    for (var, path) in cfg {
        if FORBIDDEN_KEYS.contains(&var.as_str()) {
            continue;
        }
        let value = resolve(resp, &rest, path);
        into.insert(var.clone(), value_to_string(value.as_deref()));
    }
}

/// TS `mergeStoreContext`.
pub fn merge(existing: &Store, new: &Store) -> Store {
    let mut out = existing.clone();
    out.extend(new.iter().map(|(k, v)| (k.clone(), v.clone())));
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{HeaderValue, Headers, Metrics};
    use serde_json::json;

    fn obj() -> Json {
        json!({
            "status": 200,
            "body": {"id": 123, "user": {"name": "John", "email": "john@example.com"},
                     "items": [{"id": 1, "name": "Item 1"}, {"id": 2, "name": "Item 2"}]},
            "headers": {"content-type": "application/json", "x-request-id": "abc123"}
        })
    }
    fn g(p: &str) -> Option<Json> {
        get_value_by_path(&obj(), p)
    }

    #[test]
    fn path_top_nested_header() {
        assert_eq!(g("status"), Some(json!(200)));
        assert_eq!(g("body.id"), Some(json!(123)));
        assert_eq!(g("body.user.name"), Some(json!("John")));
        assert_eq!(g("body.user.email"), Some(json!("john@example.com")));
        assert_eq!(g("headers.content-type"), Some(json!("application/json")));
        assert_eq!(g("headers.x-request-id"), Some(json!("abc123")));
    }
    #[test]
    fn path_array_index_forms() {
        assert_eq!(g("body.items.0.id"), Some(json!(1)));
        assert_eq!(g("body.items.1.name"), Some(json!("Item 2")));
        assert_eq!(g("body.items[0].id"), Some(json!(1)));
        assert_eq!(g("body.items[1].name"), Some(json!("Item 2")));
        assert_eq!(g("body.items[9].id"), None);
        assert_eq!(g("body.user[0]"), None);
        assert_eq!(g("body.items.length"), Some(json!(2)));
    }
    #[test]
    fn path_missing_and_primitives() {
        assert_eq!(g("body.nonexistent"), None);
        assert_eq!(g("body.user.age"), None);
        assert_eq!(g("nonexistent.path"), None);
        assert_eq!(get_value_by_path(&Json::Null, "any.path"), None);
        assert_eq!(get_value_by_path(&json!("string"), "length"), None);
        assert_eq!(get_value_by_path(&json!(123), "toString"), None);
        // null leaf is a value, not missing
        assert_eq!(
            get_value_by_path(&json!({"a": null}), "a"),
            Some(Json::Null)
        );
    }
    #[test]
    fn path_forbidden_keys() {
        let o = json!({"a": {"b": 1}, "__proto__": {"x": 1}, "constructor": {"name": "n"}});
        assert_eq!(get_value_by_path(&o, "__proto__.polluted"), None);
        assert_eq!(get_value_by_path(&o, "__proto__.x"), None);
        assert_eq!(get_value_by_path(&o, "prototype.x"), None);
        assert_eq!(get_value_by_path(&o, "constructor.name"), None);
        assert_eq!(get_value_by_path(&o, "constructor[0]"), None);
    }

    #[test]
    fn to_string_values() {
        assert_eq!(value_to_string(Some(&json!("hello"))), "hello");
        assert_eq!(value_to_string(Some(&json!(123))), "123");
        assert_eq!(value_to_string(Some(&json!(45.67))), "45.67");
        assert_eq!(value_to_string(Some(&json!(1.0))), "1");
        assert_eq!(value_to_string(Some(&json!(true))), "true");
        assert_eq!(value_to_string(Some(&json!(false))), "false");
        assert_eq!(value_to_string(Some(&Json::Null)), "");
        assert_eq!(value_to_string(None), "");
        assert_eq!(value_to_string(Some(&json!({"a": 1}))), r#"{"a":1}"#);
        assert_eq!(value_to_string(Some(&json!([1, 2, 3]))), "[1,2,3]");
    }

    fn resp() -> Response {
        let mut headers = Headers::new();
        headers.insert(
            "content-type".into(),
            HeaderValue::One("application/json".into()),
        );
        headers.insert("x-request-id".into(), HeaderValue::One("req-12345".into()));
        headers.insert(
            "set-cookie".into(),
            HeaderValue::Many(vec!["a=1".into(), "b=2".into()]),
        );
        Response {
            status: 201,
            headers,
            header_history: vec![],
            body: json!({"id": 456, "data": {"token": "jwt-token-here", "user": {"id": 789, "name": "Test User"}}}),
            metrics: Metrics {
                duration: 150.0,
                size: Some(1024.0),
                ..Default::default()
            },
        }
    }
    fn cfg(pairs: &[(&str, &str)]) -> StoreConfig {
        pairs
            .iter()
            .map(|(k, v)| ((*k).into(), (*v).into()))
            .collect()
    }
    fn ext(pairs: &[(&str, &str)]) -> Store {
        let mut s = Store::new();
        extract(&cfg(pairs), &resp(), &mut s);
        s
    }

    #[test]
    fn extract_status_body_headers_metrics() {
        let s = ext(&[
            ("statusCode", "status"),
            ("userId", "body.id"),
            ("token", "body.data.token"),
            ("userName", "body.data.user.name"),
            ("contentType", "headers.content-type"),
            ("requestId", "headers.x-request-id"),
            ("duration", "metrics.duration"),
            ("size", "metrics.size"),
        ]);
        assert_eq!(s["statusCode"], "201");
        assert_eq!(s["userId"], "456");
        assert_eq!(s["token"], "jwt-token-here");
        assert_eq!(s["userName"], "Test User");
        assert_eq!(s["contentType"], "application/json");
        assert_eq!(s["requestId"], "req-12345");
        assert_eq!(s["duration"], "150");
        assert_eq!(s["size"], "1024");
    }
    #[test]
    fn extract_body_root_and_index_without_cloning_paths() {
        let mut r = resp();
        r.body = json!([{"id": 1}, {"id": 2}]);
        let mut s = Store::new();
        extract(
            &cfg(&[
                ("all", "body"),
                ("second", "body[1].id"),
                ("len", "body.length"),
                ("bad", "body[5].id"),
            ]),
            &r,
            &mut s,
        );
        assert_eq!(s["all"], r#"[{"id":1},{"id":2}]"#);
        assert_eq!(s["second"], "2");
        assert_eq!(s["len"], "2");
        assert_eq!(s["bad"], "");
    }
    #[test]
    fn extract_missing_is_empty_string() {
        let s = ext(&[("missing", "body.nonexistent")]);
        assert_eq!(s["missing"], "");
    }
    #[test]
    fn extract_objects_and_multi_headers() {
        let s = ext(&[
            ("user", "body.data.user"),
            ("cookies", "headers.set-cookie"),
        ]);
        assert_eq!(s["user"], r#"{"id":789,"name":"Test User"}"#);
        assert_eq!(s["cookies"], r#"["a=1","b=2"]"#);
    }
    #[test]
    fn extract_overrides_and_skips_forbidden() {
        let mut s = Store::new();
        s.insert("a".into(), "old".into());
        s.insert("keep".into(), "k".into());
        extract(
            &cfg(&[
                ("a", "status"),
                ("__proto__", "body.id"),
                ("constructor", "status"),
                ("prototype", "status"),
            ]),
            &resp(),
            &mut s,
        );
        assert_eq!(s["a"], "201");
        assert_eq!(s["keep"], "k");
        assert_eq!(s.len(), 2);
        assert_eq!(s.keys().next().unwrap(), "a");
    }
    #[test]
    fn merge_overrides_without_mutation() {
        let a = cfg(&[("a", "1"), ("b", "2")]);
        let b = cfg(&[("b", "new"), ("c", "3")]);
        let m = merge(&a, &b);
        assert_eq!(m, cfg(&[("a", "1"), ("b", "new"), ("c", "3")]));
        assert_eq!(a, cfg(&[("a", "1"), ("b", "2")]));
        assert_eq!(b, cfg(&[("b", "new"), ("c", "3")]));
        assert!(Store::new().is_empty());
    }
}
