//! Response validation (`core/validation/validators.ts`).

use crate::jsval::{self, JsVal};
use crate::model::{ExpectConfig, HeaderValue, Headers, Json, StatusExpect};
use crate::transport::Response;
use indexmap::IndexMap;
use regex::Regex;
use std::borrow::Cow;
use std::sync::LazyLock;

/// Result of `check`. `errors` are TS's pre-join messages.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct ValidationOutcome {
    pub passed: bool,
    pub errors: Vec<String>,
}

impl ValidationOutcome {
    /// TS `errors.join('; ')`; `None` when passed.
    pub fn error_message(&self) -> Option<String> {
        if self.passed {
            None
        } else {
            Some(self.errors.join("; "))
        }
    }
}

/// TS `validateResponse`. `responseTime` checks `resp.metrics.duration`, which
/// the executor sets to the request's wall time (retries and delays included),
/// as TS validated `result.metrics.duration`.
pub fn check(expect: &ExpectConfig, resp: &Response) -> ValidationOutcome {
    let mut errors = Vec::new();
    if let Some(status) = &expect.status {
        errors.extend(validate_status(resp.status, status));
    }
    if let Some(headers) = &expect.headers {
        errors.extend(validate_headers(&resp.headers, headers));
    }
    if let Some(body) = &expect.body {
        errors.extend(validate_body_properties(Some(&resp.body), body, ""));
    }
    if let Some(rt) = &expect.response_time {
        let ms = resp.metrics.duration;
        if !validate_range_pattern(Some(&number_json(ms)), rt) {
            errors.push(format!(
                "Expected response time to match {rt}ms, got {}ms",
                to_fixed2(ms)
            ));
        }
    }
    if expect.failure == Some(true) && errors.is_empty() {
        return if resp.status >= 400 {
            ValidationOutcome {
                passed: true,
                errors,
            }
        } else {
            ValidationOutcome {
                passed: false,
                errors: vec![format!(
                    "Expected request to fail (4xx/5xx) but got status {}",
                    resp.status
                )],
            }
        };
    }
    ValidationOutcome {
        passed: errors.is_empty(),
        errors,
    }
}

fn number_json(x: f64) -> Json {
    serde_json::Number::from_f64(x).map_or(Json::Null, Json::Number)
}

/// JS `Number.prototype.toFixed(2)`: exact ties round up (Rust rounds half-even).
fn to_fixed2(x: f64) -> String {
    if !x.is_finite() {
        return jsval::num_to_string(x);
    }
    let exact = format!("{:.60}", x.abs());
    let (int, frac) = exact.split_once('.').unwrap_or((&exact, ""));
    let (head, tail) = frac.split_at(2.min(frac.len()));
    let tie = tail.starts_with('5') && tail[1..].bytes().all(|b| b == b'0');
    let sign = if x < 0.0 && exact.bytes().any(|b| b != b'0' && b != b'.') {
        "-"
    } else {
        ""
    };
    if !tie {
        return format!("{sign}{:.2}", x.abs());
    }
    // tie: bump the last kept digit, carrying
    let mut digits: Vec<u8> = format!("{int}{head}").into_bytes();
    let mut i = digits.len();
    loop {
        if i == 0 {
            digits.insert(0, b'1');
            break;
        }
        i -= 1;
        if digits[i] == b'9' {
            digits[i] = b'0';
        } else {
            digits[i] += 1;
            break;
        }
    }
    let s = String::from_utf8(digits).unwrap_or_default();
    let (a, b) = s.split_at(s.len() - 2);
    format!("{sign}{a}.{b}")
}

/// TS `validateStatus`.
pub fn validate_status(actual: u16, expected: &StatusExpect) -> Vec<String> {
    let list: &[u16] = match expected {
        StatusExpect::One(s) => std::slice::from_ref(s),
        StatusExpect::Many(v) => v,
    };
    // TS: `includes(actual || 0)`
    if list.contains(&actual) {
        return vec![];
    }
    let joined: Vec<String> = list.iter().map(u16::to_string).collect();
    vec![format!(
        "Expected status {}, got {actual}",
        joined.join(" or ")
    )]
}

/// TS `validateHeaders`: expected names are matched lowercased against the
/// (already lowercase) actual names. A multi-value header matches when any
/// value, or the `", "`-joined whole, equals the expectation.
pub fn validate_headers(actual: &Headers, expected: &IndexMap<String, String>) -> Vec<String> {
    let mut errors = Vec::new();
    for (key, value) in expected {
        let actual_value = actual.get(&key.to_lowercase());
        let matched = match actual_value {
            Some(HeaderValue::Many(vs)) => vs.contains(value) || vs.join(", ") == *value,
            Some(HeaderValue::One(s)) => s == value,
            None => false,
        };
        if !matched {
            let display = match actual_value {
                Some(HeaderValue::Many(vs)) => format!(
                    "[{}]",
                    vs.iter()
                        .map(|v| format!("\"{v}\""))
                        .collect::<Vec<_>>()
                        .join(", ")
                ),
                Some(HeaderValue::One(s)) => format!("\"{s}\""),
                None => "\"undefined\"".to_string(),
            };
            errors.push(format!("Expected header {key}=\"{value}\", got {display}"));
        }
    }
    errors
}

fn is_container(v: &Json) -> bool {
    matches!(v, Json::Array(_) | Json::Object(_))
}

/// JS `actual?.[key]` by reference (only `length` is computed).
fn lookup<'a>(actual: Option<&'a Json>, key: &str) -> Option<Cow<'a, Json>> {
    match actual? {
        Json::Object(m) => m.get(key).map(Cow::Borrowed),
        Json::Array(a) => {
            if key == "length" {
                return Some(Cow::Owned(Json::from(a.len())));
            }
            let is_index = key == "0"
                || (!key.starts_with('0')
                    && !key.is_empty()
                    && key.bytes().all(|b| b.is_ascii_digit()));
            if is_index {
                key.parse::<usize>()
                    .ok()
                    .and_then(|i| a.get(i))
                    .map(Cow::Borrowed)
            } else {
                None
            }
        }
        _ => None,
    }
}

/// TS `validateBodyProperties`. `actual == None` is JS `undefined`.
pub fn validate_body_properties(actual: Option<&Json>, expected: &Json, path: &str) -> Vec<String> {
    let shown = if path.is_empty() { "body" } else { path };
    let single = |e: Result<(), String>| e.err().into_iter().collect::<Vec<_>>();

    match expected {
        Json::Array(items) => {
            if let Some(Json::Array(actual_items)) = actual {
                let mut errors = Vec::new();
                for (i, expected_item) in items.iter().enumerate() {
                    let item_path = format!("{shown}[{i}]");
                    let found = actual_items.iter().any(|a| {
                        if is_container(expected_item) {
                            validate_body_properties(Some(a), expected_item, &item_path).is_empty()
                        } else {
                            validate_value(Some(a), expected_item, &item_path).is_ok()
                        }
                    });
                    if !found {
                        errors.push(format!(
                            "Expected {shown} to contain item matching {}, but no match found",
                            jsval::stringify(expected_item)
                        ));
                    }
                }
                return errors;
            }
            single(validate_value(actual, expected, shown))
        }
        Json::Object(props) => {
            let mut errors = Vec::new();
            for (key, expected_value) in props {
                let current = if path.is_empty() {
                    key.clone()
                } else {
                    format!("{path}.{key}")
                };
                let actual_value = match actual {
                    Some(whole @ Json::Array(a)) if is_array_selector(key) => {
                        array_selector(whole, a, key)
                    }
                    other => lookup(other, key),
                };
                if is_container(expected_value) {
                    errors.extend(validate_body_properties(
                        actual_value.as_deref(),
                        expected_value,
                        &current,
                    ));
                } else if let Err(e) =
                    validate_value(actual_value.as_deref(), expected_value, &current)
                {
                    errors.push(e);
                }
            }
            errors
        }
        _ => single(validate_value(actual, expected, shown)),
    }
}

/// JS `a === b` where `b` is an expected scalar from the test file.
fn strict_eq(actual: Option<&Json>, expected: &Json) -> bool {
    match (actual, expected) {
        (Some(Json::Null), Json::Null) => true,
        (Some(Json::Bool(a)), Json::Bool(b)) => a == b,
        (Some(Json::Number(a)), Json::Number(b)) => a.as_f64() == b.as_f64(),
        (Some(Json::String(a)), Json::String(b)) => a == b,
        // objects/arrays are never the same reference
        _ => false,
    }
}

/// TS `validateValue`. `Err` carries the TS error message.
pub fn validate_value(actual: Option<&Json>, expected: &Json, path: &str) -> Result<(), String> {
    let got = || jsval::stringify_opt(actual);
    if let Json::String(s) = expected {
        match s.as_str() {
            "$absent" => {
                return match actual {
                    Some(_) => Err(format!("Expected {path} to be absent, got {}", got())),
                    None => Ok(()),
                };
            }
            "$exists" => {
                return match actual {
                    None => Err(format!("Expected {path} to exist, but it was absent")),
                    Some(_) => Ok(()),
                };
            }
            "*" => return Ok(()),
            _ => {}
        }
    }
    if let Json::Array(options) = expected {
        let matched = options.iter().any(|opt| match opt {
            Json::String(s) if s == "*" => true,
            Json::String(s) if is_regex_pattern(s) => validate_regex_pattern(actual, s),
            Json::String(s) if is_range_pattern(s) => validate_range_pattern(actual, s),
            other => strict_eq(actual, other),
        });
        return if matched {
            Ok(())
        } else {
            Err(format!(
                "Expected {path} to match one of {}, got {}",
                jsval::stringify(expected),
                got()
            ))
        };
    }
    if let Json::String(s) = expected {
        if is_regex_pattern(s) {
            return if validate_regex_pattern(actual, s) {
                Ok(())
            } else {
                Err(format!(
                    "Expected {path} to match pattern {s}, got {}",
                    got()
                ))
            };
        }
        if is_range_pattern(s) {
            return if validate_range_pattern(actual, s) {
                Ok(())
            } else {
                Err(format!("Expected {path} to match range {s}, got {}", got()))
            };
        }
    }
    if matches!(expected, Json::Null) || matches!(expected, Json::String(s) if s == "null") {
        return match actual {
            Some(Json::Null) => Ok(()),
            _ => Err(format!("Expected {path} to be null, got {}", got())),
        };
    }
    if strict_eq(actual, expected) {
        Ok(())
    } else {
        Err(format!(
            "Expected {path} to be {}, got {}",
            jsval::stringify(expected),
            got()
        ))
    }
}

/// TS `isRegexPattern`.
pub fn is_regex_pattern(p: &str) -> bool {
    p.starts_with('^')
        || p.ends_with('$')
        || p.contains("\\d")
        || p.contains("\\w")
        || p.contains("\\s")
}

/// TS `validateRegexPattern`: `new RegExp(p).test(String(actual))`, invalid -> false.
pub fn validate_regex_pattern(actual: Option<&Json>, pattern: &str) -> bool {
    let s = JsVal::from_opt(actual).to_js_string();
    jsval::regex_test(pattern, &s, false).unwrap_or(false)
}

const WS: &str = r"[ \t\n\r\x0B\x0C\x{a0}\x{1680}\x{2000}-\x{200a}\x{2028}\x{2029}\x{202f}\x{205f}\x{3000}\x{feff}]*";
const OP: &str = r"(?:>=?|<=?|>|<)";

static RANGE_PATTERN: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(&format!(
        r"^{OP}{WS}[0-9.\-]+(?:{WS},{WS}{OP}{WS}[0-9.\-]+)*$"
    ))
    .expect("range regex")
});
static DASH_RANGE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(&format!(r"^([0-9.\-]+){WS}-{WS}([0-9.\-]+)$")).expect("dash range regex")
});
static CONDITION: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(&format!(r"^({OP}){WS}([0-9.\-]+)$")).expect("condition regex"));
static SLICE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"slice\(([0-9]+)(?:,([0-9]+))?\)").expect("slice regex"));

/// TS `isRangePattern`.
pub fn is_range_pattern(p: &str) -> bool {
    RANGE_PATTERN.is_match(p)
}

/// TS `validateRangePattern` (also used for `responseTime`).
pub fn validate_range_pattern(actual: Option<&Json>, pattern: &str) -> bool {
    let num = JsVal::from_opt(actual).to_number();
    if num.is_nan() {
        return false;
    }
    if let Some(c) = DASH_RANGE.captures(pattern) {
        let min = jsval::str_to_number(&c[1]);
        let max = jsval::str_to_number(&c[2]);
        return num >= min && num <= max;
    }
    pattern.split(',').map(jsval::trim).all(|cond| {
        let Some(c) = CONDITION.captures(cond) else {
            return false;
        };
        let target = jsval::str_to_number(&c[2]);
        match &c[1] {
            ">" => num > target,
            ">=" => num >= target,
            "<" => num < target,
            "<=" => num <= target,
            _ => false,
        }
    })
}

/// TS `isArraySelector`.
pub fn is_array_selector(key: &str) -> bool {
    let bracketed = key.len() >= 2
        && key.starts_with('[')
        && key.ends_with(']')
        && !key
            .chars()
            .any(|c| matches!(c, '\n' | '\r' | '\u{2028}' | '\u{2029}'));
    bracketed || key == "*" || key.starts_with("slice(")
}

/// TS `getArrayValue` by reference: `whole` is the array as a `Json` (returned
/// as-is for `*`); only `slice(..)` builds a new value.
fn array_selector<'a>(whole: &'a Json, array: &'a [Json], selector: &str) -> Option<Cow<'a, Json>> {
    if selector == "*" {
        return Some(Cow::Borrowed(whole));
    }
    if selector.starts_with('[') && selector.ends_with(']') && selector.len() >= 2 {
        let index = &selector[1..selector.len() - 1];
        if index == "*" {
            return Some(Cow::Borrowed(whole));
        }
        let n = jsval::str_to_number(index);
        if !n.is_nan() {
            let i = if n >= 0.0 { n } else { array.len() as f64 + n };
            if i.fract() == 0.0 && i >= 0.0 && i < array.len() as f64 {
                return array.get(i as usize).map(Cow::Borrowed);
            }
            return None;
        }
    }
    if selector.starts_with("slice(")
        && let Some(c) = SLICE.captures(selector)
    {
        let len = array.len() as f64;
        let start = jsval::str_to_number(&c[1]).min(len) as usize;
        let end = c
            .get(2)
            .map_or(len, |m| jsval::str_to_number(m.as_str()).min(len)) as usize;
        let slice = if start < end { &array[start..end] } else { &[] };
        return Some(Cow::Owned(Json::Array(slice.to_vec())));
    }
    None
}

/// TS `getArrayValue`; `None` is `undefined`.
pub fn get_array_value(array: &[Json], selector: &str) -> Option<Json> {
    let whole = Json::Array(array.to_vec());
    let Json::Array(a) = &whole else { return None };
    array_selector(&whole, a, selector).map(Cow::into_owned)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{HeaderValue, Metrics};
    use serde_json::json;

    fn resp(status: u16) -> Response {
        Response {
            status,
            headers: Headers::new(),
            header_history: vec![],
            body: Json::Null,
            metrics: Metrics::default(),
        }
    }
    fn resp_h(status: u16, h: &[(&str, &str)]) -> Response {
        let mut r = resp(status);
        for (k, v) in h {
            r.headers.insert((*k).into(), HeaderValue::One((*v).into()));
        }
        r
    }
    fn resp_b(status: u16, body: Json) -> Response {
        Response {
            body,
            ..resp(status)
        }
    }
    fn resp_ms(ms: f64) -> Response {
        let mut r = resp(200);
        r.metrics.duration = ms;
        r
    }
    fn ex(v: Json) -> ExpectConfig {
        serde_json::from_value(v).unwrap()
    }
    fn hm(pairs: &[(&str, &str)]) -> indexmap::IndexMap<String, String> {
        pairs
            .iter()
            .map(|(k, v)| ((*k).into(), (*v).into()))
            .collect()
    }
    fn vb(a: Json, e: Json) -> Vec<String> {
        validate_body_properties(Some(&a), &e, "")
    }
    fn vv(a: Json, e: Json) -> bool {
        validate_value(Some(&a), &e, "path").is_ok()
    }

    // validateResponse
    #[test]
    fn empty_expect_passes() {
        let o = check(&ex(json!({})), &resp(200));
        assert!(o.passed);
        assert!(o.errors.is_empty());
        assert_eq!(o.error_message(), None);
    }
    #[test]
    fn status_match_and_mismatch() {
        assert!(check(&ex(json!({"status": 200})), &resp(200)).passed);
        let o = check(&ex(json!({"status": 200})), &resp(404));
        assert!(!o.passed);
        assert_eq!(o.errors, vec!["Expected status 200, got 404"]);
    }
    #[test]
    fn headers_body_pass() {
        let r = resp_h(200, &[("content-type", "application/json")]);
        assert!(
            check(
                &ex(json!({"headers": {"content-type": "application/json"}})),
                &r
            )
            .passed
        );
        let r = resp_b(200, json!({"name": "test"}));
        assert!(check(&ex(json!({"body": {"name": "test"}})), &r).passed);
    }
    #[test]
    fn response_time() {
        assert!(check(&ex(json!({"responseTime": "<= 100"})), &resp_ms(50.0)).passed);
        let o = check(&ex(json!({"responseTime": "<= 100"})), &resp_ms(150.0));
        assert!(!o.passed);
        assert_eq!(
            o.errors,
            vec!["Expected response time to match <= 100ms, got 150.00ms"]
        );
        assert!(
            check(
                &ex(json!({"responseTime": "> 500, < 2000"})),
                &resp_ms(900.0)
            )
            .passed
        );
        assert!(
            !check(
                &ex(json!({"responseTime": "> 500, < 2000"})),
                &resp_ms(2500.0)
            )
            .passed
        );
        assert!(check(&ex(json!({"responseTime": "< 1000"})), &resp_ms(12.5)).passed);
    }
    #[test]
    fn response_time_to_fixed_rounds_ties_up() {
        let o = check(&ex(json!({"responseTime": "> 5"})), &resp_ms(0.125));
        assert_eq!(
            o.errors,
            vec!["Expected response time to match > 5ms, got 0.13ms"]
        );
        let o = check(&ex(json!({"responseTime": "< 5"})), &resp_ms(1234.5678));
        assert_eq!(
            o.errors,
            vec!["Expected response time to match < 5ms, got 1234.57ms"]
        );
    }
    #[test]
    fn errors_joined_in_order() {
        let r = resp_b(404, json!({"a": 1}));
        let o = check(&ex(json!({"status": 200, "body": {"a": 2}})), &r);
        assert_eq!(
            o.error_message().unwrap(),
            "Expected status 200, got 404; Expected a to be 2, got 1"
        );
    }
    #[test]
    fn failure_expectation() {
        assert!(check(&ex(json!({"failure": true, "status": 404})), &resp(404)).passed);
        assert!(check(&ex(json!({"failure": true, "status": 500})), &resp(500)).passed);
        let o = check(&ex(json!({"failure": true})), &resp(200));
        assert!(!o.passed);
        assert_eq!(
            o.errors,
            vec!["Expected request to fail (4xx/5xx) but got status 200"]
        );
        let o = check(&ex(json!({"failure": true, "status": 404})), &resp(500));
        assert!(!o.passed);
        assert_eq!(o.errors, vec!["Expected status 404, got 500"]);
    }

    // validateStatus
    #[test]
    fn status_single_and_many() {
        assert!(validate_status(200, &StatusExpect::One(200)).is_empty());
        assert_eq!(validate_status(404, &StatusExpect::One(200)).len(), 1);
        let m = StatusExpect::Many(vec![200, 201]);
        assert!(validate_status(200, &m).is_empty());
        assert!(validate_status(201, &m).is_empty());
        assert_eq!(
            validate_status(404, &m),
            vec!["Expected status 200 or 201, got 404"]
        );
    }

    // validateHeaders
    fn hdrs(pairs: &[(&str, HeaderValue)]) -> Headers {
        pairs
            .iter()
            .map(|(k, v)| ((*k).to_string(), v.clone()))
            .collect()
    }
    fn one(s: &str) -> HeaderValue {
        HeaderValue::One(s.into())
    }
    fn many(v: &[&str]) -> HeaderValue {
        HeaderValue::Many(v.iter().map(|s| (*s).into()).collect())
    }
    #[test]
    fn headers_match() {
        let a = hdrs(&[("content-type", one("application/json"))]);
        assert!(validate_headers(&a, &hm(&[("content-type", "application/json")])).is_empty());
        assert!(validate_headers(&a, &hm(&[("Content-Type", "application/json")])).is_empty());
        assert!(validate_headers(&a, &hm(&[("CONTENT-TYPE", "application/json")])).is_empty());
    }
    #[test]
    fn headers_mismatch_and_missing() {
        let a = hdrs(&[("content-type", one("text/html"))]);
        let e = validate_headers(&a, &hm(&[("content-type", "application/json")]));
        assert_eq!(
            e,
            vec![r#"Expected header content-type="application/json", got "text/html""#]
        );
        let e = validate_headers(&Headers::new(), &hm(&[("x-custom", "value")]));
        assert_eq!(
            e,
            vec![r#"Expected header x-custom="value", got "undefined""#]
        );
    }
    #[test]
    fn headers_multi_value() {
        let a = hdrs(&[("set-cookie", many(&["a=1", "b=2"]))]);
        assert!(validate_headers(&a, &hm(&[("set-cookie", "b=2")])).is_empty());
        let a2 = hdrs(&[("cache-control", many(&["no-store", "no-cache"]))]);
        assert!(validate_headers(&a2, &hm(&[("cache-control", "no-store, no-cache")])).is_empty());
        let e = validate_headers(&a, &hm(&[("set-cookie", "c=3")]));
        assert_eq!(
            e,
            vec![r#"Expected header set-cookie="c=3", got ["a=1", "b=2"]"#]
        );
    }

    #[test]
    fn headers_non_string_expectations_are_stringified() {
        // TS compares `number === string` and always fails; Rust stringifies first.
        let e = ex(json!({"headers": {"x-count": 5, "x-flag": true}}));
        let r = resp_h(200, &[("x-count", "5"), ("x-flag", "true")]);
        assert!(check(&e, &r).passed);
        let r = resp_h(200, &[("x-count", "6"), ("x-flag", "true")]);
        assert!(!check(&e, &r).passed);
    }

    // validateBodyProperties
    #[test]
    fn body_objects() {
        assert!(vb(json!({"name": "test"}), json!({"name": "test"})).is_empty());
        assert!(
            vb(
                json!({"user": {"name": "test"}}),
                json!({"user": {"name": "test"}})
            )
            .is_empty()
        );
        // partial object match: extra actual keys are fine
        assert!(vb(json!({"a": 1, "b": 2}), json!({"a": 1})).is_empty());
        let e = vb(json!({"name": "actual"}), json!({"name": "expected"}));
        assert_eq!(e, vec![r#"Expected name to be "expected", got "actual""#]);
    }
    #[test]
    fn body_primitive_and_one_of() {
        assert!(vb(json!("hello"), json!("hello")).is_empty());
        assert!(vb(json!(1), json!([1, 2, 3])).is_empty());
        assert!(vb(json!(2), json!([1, 2, 3])).is_empty());
        let e = vb(json!(5), json!([1, 2, 3]));
        assert_eq!(e, vec!["Expected body to match one of [1,2,3], got 5"]);
    }
    #[test]
    fn body_array_containment() {
        let actual =
            json!([{"type": "paragraph", "text": "Hello"}, {"type": "heading", "text": "World"}]);
        assert!(vb(actual.clone(), json!([{"type": "paragraph"}])).is_empty());
        assert!(
            vb(
                actual.clone(),
                json!([{"type": "paragraph"}, {"type": "heading"}])
            )
            .is_empty()
        );
        let e = vb(
            json!([{"type": "paragraph", "text": "Hello"}]),
            json!([{"type": "heading"}]),
        );
        assert_eq!(
            e,
            vec![
                r#"Expected body to contain item matching {"type":"heading"}, but no match found"#
            ]
        );
        assert!(vb(json!([1, 2, 3]), json!([1, 2])).is_empty());
        let e = vb(json!([1, 2, 3]), json!([9]));
        assert_eq!(
            e,
            vec!["Expected body to contain item matching 9, but no match found"]
        );
        assert!(
            vb(
                json!({"content": [{"type": "paragraph"}, {"type": "heading"}]}),
                json!({"content": [{"type": "paragraph"}]})
            )
            .is_empty()
        );
        let e = vb(json!({"content": [1]}), json!({"content": [9]}));
        assert_eq!(
            e,
            vec!["Expected content to contain item matching 9, but no match found"]
        );
    }
    #[test]
    fn body_array_selectors() {
        assert!(
            vb(
                json!([{"id": 1}, {"id": 2}]),
                json!({"[0]": {"id": 1}, "[-1]": {"id": 2}})
            )
            .is_empty()
        );
        assert!(vb(json!([1, 2, 3]), json!({"slice(0,2)": [1, 2]})).is_empty());
        let e = vb(json!([1, 2]), json!({"[5]": 1}));
        assert_eq!(e, vec!["Expected [5] to be 1, got undefined"]);
    }
    #[test]
    fn body_absent_exists() {
        assert!(
            vb(
                json!({"data": {"foo": 1}}),
                json!({"data": {"cssStyles": "$absent"}})
            )
            .is_empty()
        );
        let e = vb(
            json!({"data": {"cssStyles": null}}),
            json!({"data": {"cssStyles": "$absent"}}),
        );
        assert_eq!(e, vec!["Expected data.cssStyles to be absent, got null"]);
        assert!(
            vb(
                json!({"data": {"token": "abc"}}),
                json!({"data": {"token": "$exists"}})
            )
            .is_empty()
        );
        let e = vb(json!({"data": {}}), json!({"data": {"token": "$exists"}}));
        assert_eq!(e, vec!["Expected data.token to exist, but it was absent"]);
    }
    #[test]
    fn body_missing_path_and_null_expected() {
        let e = vb(json!({}), json!({"a": {"b": 1}}));
        assert_eq!(e, vec!["Expected a.b to be 1, got undefined"]);
        assert_eq!(vb(Json::Null, json!(null)), Vec::<String>::new());
        assert_eq!(
            vb(json!(1), json!(null)),
            vec!["Expected body to be null, got 1"]
        );
    }
    #[test]
    fn body_numbers_compare_by_value() {
        // YAML 1.0 parses as float; TS has a single number type
        assert!(vb(json!({"n": 1}), json!({"n": 1.0})).is_empty());
        let e = vb(json!({"n": 2}), json!({"n": 1.0}));
        assert_eq!(e, vec!["Expected n to be 1, got 2"]);
    }
    #[test]
    fn body_array_length_and_index_keys() {
        assert!(vb(json!({"a": [1, 2]}), json!({"a": {"length": 2}})).is_empty());
        assert!(vb(json!([7, 8]), json!({"1": 8})).is_empty());
    }

    // validateValue
    #[test]
    fn value_wildcard() {
        assert!(vv(json!("anything"), json!("*")));
        assert!(vv(json!(123), json!("*")));
        assert!(vv(Json::Null, json!("*")));
        assert!(validate_value(None, &json!("*"), "p").is_ok());
    }
    #[test]
    fn value_absent() {
        assert!(validate_value(None, &json!("$absent"), "path").is_ok());
        assert!(!vv(json!("value"), json!("$absent")));
        assert!(!vv(Json::Null, json!("$absent")));
        let e = validate_value(Some(&json!("value")), &json!("$absent"), "myField").unwrap_err();
        assert_eq!(e, r#"Expected myField to be absent, got "value""#);
    }
    #[test]
    fn value_exists() {
        assert!(vv(json!("value"), json!("$exists")));
        assert!(vv(json!(0), json!("$exists")));
        assert!(vv(Json::Null, json!("$exists")));
        assert!(validate_value(None, &json!("$exists"), "path").is_err());
    }
    #[test]
    fn value_exact() {
        assert!(vv(json!("test"), json!("test")));
        assert!(vv(json!(42), json!(42)));
        assert!(vv(json!(true), json!(true)));
        let e = validate_value(Some(&json!("actual")), &json!("expected"), "myField").unwrap_err();
        assert_eq!(e, r#"Expected myField to be "expected", got "actual""#);
        // strict equality: no coercion
        assert!(!vv(json!("1"), json!(1)));
        assert!(!vv(json!(1), json!("1")));
    }
    #[test]
    fn value_null() {
        assert!(vv(Json::Null, json!(null)));
        assert!(vv(Json::Null, json!("null")));
        assert!(!vv(json!("not null"), json!(null)));
        let e = validate_value(Some(&json!("x")), &json!("null"), "p").unwrap_err();
        assert_eq!(e, r#"Expected p to be null, got "x""#);
    }
    #[test]
    fn value_arrays() {
        assert!(vv(json!("a"), json!(["a", "b", "c"])));
        assert!(vv(json!("b"), json!(["a", "b", "c"])));
        assert!(!vv(json!("d"), json!(["a", "b", "c"])));
        assert!(vv(json!("anything"), json!(["*"])));
        assert!(vv(json!("test123"), json!(["^test\\d+$"])));
        assert!(vv(json!(50), json!([">= 0", "<= 100"])));
        let e = validate_value(Some(&json!("d")), &json!(["a", "b"]), "p").unwrap_err();
        assert_eq!(e, r#"Expected p to match one of ["a","b"], got "d""#);
    }
    #[test]
    fn value_regex_and_range() {
        assert!(vv(json!("test123"), json!("^test\\d+$")));
        assert!(!vv(json!("other"), json!("^test\\d+$")));
        let e = validate_value(Some(&json!("other")), &json!("^test\\d+$"), "p").unwrap_err();
        assert_eq!(e, r#"Expected p to match pattern ^test\d+$, got "other""#);
        assert!(vv(json!(50), json!(">= 0")));
        assert!(!vv(json!(-1), json!(">= 0")));
        let e = validate_value(Some(&json!(-1)), &json!(">= 0"), "p").unwrap_err();
        assert_eq!(e, "Expected p to match range >= 0, got -1");
    }
    #[test]
    fn regex_with_lookaround() {
        assert!(vv(json!("user"), json!("^(?!admin).*$")));
        assert!(!vv(json!("admin1"), json!("^(?!admin).*$")));
    }

    // patterns
    #[test]
    fn regex_detection() {
        for p in ["^test", "test$", "\\d+", "\\w+", "\\s+"] {
            assert!(is_regex_pattern(p), "{p}");
        }
        for p in ["hello", "C++", "really?", "*-", "[]", "[a-z]"] {
            assert!(!is_regex_pattern(p), "{p}");
        }
    }
    #[test]
    fn regex_validation() {
        let s = |s: &str| json!(s);
        assert!(validate_regex_pattern(Some(&s("test123")), "^test\\d+$"));
        assert!(validate_regex_pattern(Some(&s("hello world")), "world"));
        assert!(!validate_regex_pattern(Some(&s("test")), "^\\d+$"));
        assert!(!validate_regex_pattern(Some(&s("test")), "[invalid"));
        assert!(validate_regex_pattern(Some(&json!(123)), "\\d+"));
        assert!(validate_regex_pattern(Some(&json!(true)), "true"));
        assert!(validate_regex_pattern(None, "undefined"));
        let large = "abc".repeat(50_000);
        assert!(validate_regex_pattern(Some(&s(&large)), "^abc"));
        // cap: text beyond 100k chars is never seen
        let big = format!("{}Z", "a".repeat(100_000));
        assert!(!validate_regex_pattern(Some(&s(&big)), "Z$"));
    }
    #[test]
    fn range_detection() {
        for p in [">= 0", "<= 100", "> 10", "< 50", ">= 0, <= 100"] {
            assert!(is_range_pattern(p), "{p}");
        }
        for p in ["100", "hello", "92998-3874"] {
            assert!(!is_range_pattern(p), "{p}");
        }
    }
    #[test]
    fn range_validation() {
        let n = |v: f64| Some(json!(v));
        assert!(validate_range_pattern(n(50.0).as_ref(), "> 10"));
        assert!(!validate_range_pattern(n(10.0).as_ref(), "> 10"));
        assert!(validate_range_pattern(n(10.0).as_ref(), ">= 10"));
        assert!(!validate_range_pattern(n(9.0).as_ref(), ">= 10"));
        assert!(validate_range_pattern(n(5.0).as_ref(), "< 10"));
        assert!(!validate_range_pattern(n(10.0).as_ref(), "< 10"));
        assert!(validate_range_pattern(n(10.0).as_ref(), "<= 10"));
        assert!(!validate_range_pattern(n(11.0).as_ref(), "<= 10"));
        assert!(validate_range_pattern(n(50.0).as_ref(), ">= 0, <= 100"));
        assert!(!validate_range_pattern(n(-1.0).as_ref(), ">= 0, <= 100"));
        assert!(!validate_range_pattern(n(101.0).as_ref(), ">= 0, <= 100"));
        assert!(!validate_range_pattern(Some(&json!("hello")), ">= 0"));
        assert!(validate_range_pattern(n(-5.0).as_ref(), ">= -10"));
        assert!(!validate_range_pattern(n(-15.0).as_ref(), ">= -10"));
        assert!(validate_range_pattern(n(3.25).as_ref(), ">= 3.0, <= 4.0"));
        // numeric strings coerce; dash range; absent is NaN
        assert!(validate_range_pattern(Some(&json!("42")), "> 10"));
        assert!(validate_range_pattern(n(20.0).as_ref(), "10 - 50"));
        assert!(validate_range_pattern(n(20.0).as_ref(), "10-50"));
        assert!(!validate_range_pattern(n(60.0).as_ref(), "10-50"));
        assert!(!validate_range_pattern(None, ">= 0"));
        // plain number without operator never matches
        assert!(!validate_range_pattern(n(100.0).as_ref(), "100"));
    }

    // selectors
    #[test]
    fn selectors() {
        for k in ["[0]", "[-1]", "[*]", "*", "slice(0,3)"] {
            assert!(is_array_selector(k), "{k}");
        }
        assert!(!is_array_selector("name"));
        assert!(!is_array_selector("user_id"));
        let a: Vec<Json> = ["a", "b", "c", "d", "e"].iter().map(|s| json!(s)).collect();
        let arr = Json::Array(a.clone());
        assert_eq!(get_array_value(&a, "*"), Some(arr.clone()));
        assert_eq!(get_array_value(&a, "[0]"), Some(json!("a")));
        assert_eq!(get_array_value(&a, "[2]"), Some(json!("c")));
        assert_eq!(get_array_value(&a, "[-1]"), Some(json!("e")));
        assert_eq!(get_array_value(&a, "[-2]"), Some(json!("d")));
        assert_eq!(get_array_value(&a, "[*]"), Some(arr));
        assert_eq!(get_array_value(&a, "slice(0,2)"), Some(json!(["a", "b"])));
        assert_eq!(get_array_value(&a, "slice(1,3)"), Some(json!(["b", "c"])));
        assert_eq!(
            get_array_value(&a, "slice(2)"),
            Some(json!(["c", "d", "e"]))
        );
        assert_eq!(get_array_value(&a, "invalid"), None);
        assert_eq!(get_array_value(&a, "[9]"), None);
        assert_eq!(get_array_value(&a, "[-9]"), None);
        assert_eq!(get_array_value(&a, "[]"), Some(json!("a")));
    }
}
