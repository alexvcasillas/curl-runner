//! Variable interpolation (`core/interpolator/variable-resolver.ts`).
//!
//! Supports `${VAR}`, `${VAR:default}`, `${A:${B:fallback}}`, `${VAR:upper}`,
//! `${VAR:lower}`, `${store.key}` and the dynamic variables `UUID`,
//! `UUID:short`, `RANDOM:min-max`, `RANDOM:string:n`, `TIMESTAMP`,
//! `CURRENT_TIME`, `DATE:fmt` and `TIME:fmt`.
//!
//! Differences from the TS API, all deliberate:
//! - Environment access is injected (`InterpCtx::env`) instead of reading
//!   `process.env` directly; [`process_env`] is the real implementation.
//! - [`InterpCtx`] borrows its maps instead of owning them.
//!
//! Where TS calls it: `cli.ts` interpolates every parsed request object with
//! `global.variables` + `collection.variables` (no store, env enabled), then
//! merges `defaults`; the file-level `global`/`collection` blocks themselves
//! are not interpolated. `request-executor.ts` re-interpolates each request
//! with the response store before running it (skipped while the store is
//! empty). Object keys are never interpolated; the keys `__proto__`,
//! `prototype` and `constructor` are dropped.

use crate::model::{Json, RequestConfig};
use crate::parser::ParseError;
use chrono::{Datelike, NaiveDateTime, Timelike};
use indexmap::IndexMap;
use rand::RngExt;

/// Values saved from earlier responses (`${store.key}`).
pub type Store = IndexMap<String, String>;

/// Environment lookup, injectable for tests.
pub type EnvLookup<'a> = &'a dyn Fn(&str) -> Option<String>;

/// Real process environment lookup (`std::env::var`, `None` when unset or not
/// valid unicode).
pub fn process_env(name: &str) -> Option<String> {
    std::env::var(name).ok()
}

/// Everything variable resolution needs.
#[derive(Clone, Copy)]
pub struct InterpCtx<'a> {
    /// Static variables (`global.variables` merged with `collection.variables`).
    pub variables: &'a IndexMap<String, String>,
    /// Response store; `None` disables `${store.*}` resolution.
    pub store: Option<&'a Store>,
    /// Fall back to the environment for unknown names.
    pub resolve_env: bool,
    /// Environment lookup used when `resolve_env` is set.
    pub env: EnvLookup<'a>,
}

impl<'a> InterpCtx<'a> {
    /// Context with env resolution enabled (the TS default).
    pub fn new(
        variables: &'a IndexMap<String, String>,
        store: Option<&'a Store>,
        env: EnvLookup<'a>,
    ) -> Self {
        Self {
            variables,
            store,
            resolve_env: true,
            env,
        }
    }
}

/// A `${...}` occurrence: byte range `start..end` and the inner text.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VariableRef {
    pub start: usize,
    pub end: usize,
    pub name: String,
}

const DYNAMIC_PREFIXES: [&str; 4] = ["DATE", "TIME", "UUID", "RANDOM"];
const FORBIDDEN_KEYS: [&str; 3] = ["__proto__", "prototype", "constructor"];

/// Find `${...}` references with brace matching (`${A:${B}}` is one ref).
/// Unterminated `${` yields nothing.
pub fn extract_variables(s: &str) -> Vec<VariableRef> {
    let b = s.as_bytes();
    let mut refs = Vec::new();
    let mut i = 0;
    while i < b.len() {
        if b[i] == b'$' && b.get(i + 1) == Some(&b'{') {
            let start = i;
            i += 2;
            let mut depth = 1;
            let name_start = i;
            while i < b.len() && depth > 0 {
                match b[i] {
                    b'{' => depth += 1,
                    b'}' => depth -= 1,
                    _ => {}
                }
                i += 1;
            }
            if depth == 0 {
                refs.push(VariableRef {
                    start,
                    end: i,
                    name: s[name_start..i - 1].to_string(),
                });
            }
        } else {
            i += 1;
        }
    }
    refs
}

/// `formatDate`: replaces the first `YYYY`, `MM`, `DD` (in that order).
pub fn format_date(dt: &NaiveDateTime, format: &str) -> String {
    format
        .replacen("YYYY", &dt.year().to_string(), 1)
        .replacen("MM", &format!("{:02}", dt.month()), 1)
        .replacen("DD", &format!("{:02}", dt.day()), 1)
}

/// `formatTime`: replaces the first `HH`, `mm`, `ss` (in that order).
pub fn format_time(dt: &NaiveDateTime, format: &str) -> String {
    format
        .replacen("HH", &format!("{:02}", dt.hour()), 1)
        .replacen("mm", &format!("{:02}", dt.minute()), 1)
        .replacen("ss", &format!("{:02}", dt.second()), 1)
}

fn all_digits(s: &str) -> bool {
    !s.is_empty() && s.bytes().all(|c| c.is_ascii_digit())
}

/// Dynamic variables; `None` when `name` is not one.
pub fn resolve_dynamic_variable(name: &str) -> Option<String> {
    if name == "UUID" {
        return Some(uuid::Uuid::new_v4().to_string());
    }
    if name == "UUID:short" {
        return Some(uuid::Uuid::new_v4().to_string()[..8].to_string());
    }
    if let Some(rest) = name.strip_prefix("RANDOM:") {
        if let Some((min, max)) = rest.split_once('-')
            && all_digits(min)
            && all_digits(max)
        {
            let min: f64 = min.parse().ok()?;
            let max: f64 = max.parse().ok()?;
            let r: f64 = rand::rng().random();
            return Some(format!("{}", (r * (max - min + 1.0)).floor() + min));
        }
        if let Some(len) = rest.strip_prefix("string:")
            && all_digits(len)
        {
            const CHARS: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789";
            let len: usize = len.parse().ok()?;
            let mut rng = rand::rng();
            return Some(
                (0..len)
                    .map(|_| CHARS[rng.random_range(0..CHARS.len())] as char)
                    .collect(),
            );
        }
    }
    if name == "CURRENT_TIME" || name == "TIMESTAMP" {
        return Some(chrono::Utc::now().timestamp_millis().to_string());
    }
    if let Some(fmt) = name.strip_prefix("DATE:") {
        return Some(format_date(&chrono::Local::now().naive_local(), fmt));
    }
    if let Some(fmt) = name.strip_prefix("TIME:") {
        return Some(format_time(&chrono::Local::now().naive_local(), fmt));
    }
    None
}

fn lookup_env(name: &str, ctx: &InterpCtx) -> Option<String> {
    if ctx.resolve_env {
        (ctx.env)(name)
    } else {
        None
    }
}

/// Resolve one reference: store, transforms, defaults, dynamic, static, env.
/// `None` means unresolved (the caller keeps the original text).
pub fn resolve_variable(name: &str, ctx: &InterpCtx) -> Option<String> {
    // 1. store
    if let (Some(key), Some(store)) = (name.strip_prefix("store."), ctx.store) {
        return store.get(key).cloned();
    }

    // 2. transforms (`^([^:]+):(upper|lower)$`)
    if let Some((base, transform)) = name.split_once(':')
        && !base.is_empty()
        && (transform == "upper" || transform == "lower")
    {
        let value = ctx
            .variables
            .get(base)
            .cloned()
            .or_else(|| lookup_env(base, ctx));
        // JS truthiness: an empty string does not transform.
        return match value {
            Some(v) if !v.is_empty() => Some(if transform == "upper" {
                v.to_uppercase()
            } else {
                v.to_lowercase()
            }),
            _ => None,
        };
    }

    // 3. default values
    if let Some((actual, default)) = name.split_once(':')
        && !DYNAMIC_PREFIXES.contains(&actual)
    {
        if let Some(v) = resolve_variable(actual, ctx) {
            return Some(v);
        }
        if let Some(inner) = default.strip_prefix("${") {
            let nested = inner.strip_suffix('}').unwrap_or(inner);
            return Some(resolve_variable(nested, ctx).unwrap_or_else(|| default.to_string()));
        }
        return Some(default.to_string());
    }

    // 4. dynamic
    if let Some(v) = resolve_dynamic_variable(name) {
        return Some(v);
    }

    // 5. static
    if let Some(v) = ctx.variables.get(name) {
        return Some(v.clone());
    }

    // 6. environment
    lookup_env(name, ctx)
}

/// Interpolate every `${...}` in `s`; unresolved references are kept as-is.
pub fn interpolate_str(s: &str, ctx: &InterpCtx) -> String {
    let refs = extract_variables(s);
    let mut out = String::with_capacity(s.len());
    let mut last = 0;
    for r in &refs {
        out.push_str(&s[last..r.start]);
        match resolve_variable(&r.name, ctx) {
            Some(v) => out.push_str(&v),
            None => out.push_str(&s[r.start..r.end]),
        }
        last = r.end;
    }
    out.push_str(&s[last..]);
    out
}

/// Interpolate string values anywhere in a JSON tree (keys are untouched,
/// forbidden keys are dropped). Whole-string replacements stay strings.
pub fn interpolate_json(value: &Json, ctx: &InterpCtx) -> Json {
    match value {
        Json::String(s) => Json::String(interpolate_str(s, ctx)),
        Json::Array(a) => Json::Array(a.iter().map(|v| interpolate_json(v, ctx)).collect()),
        Json::Object(m) => Json::Object(
            m.iter()
                .filter(|(k, _)| !FORBIDDEN_KEYS.contains(&k.as_str()))
                .map(|(k, v)| (k.clone(), interpolate_json(v, ctx)))
                .collect(),
        ),
        other => other.clone(),
    }
}

/// Per-request stage used at execution time. Like the TS executor, a request
/// is returned untouched when `ctx.store` is `Some` but empty. Fails when the
/// interpolated values no longer fit the typed request (e.g. a numeric field
/// that now holds a non-numeric string); the error names the field.
pub fn interpolate_request(
    req: &RequestConfig,
    ctx: &InterpCtx,
) -> Result<RequestConfig, ParseError> {
    if ctx.store.is_some_and(|s| s.is_empty()) {
        return Ok(req.clone());
    }
    let raw = serde_json::to_value(req).map_err(|e| ParseError::Invalid {
        field: "request".into(),
        message: e.to_string(),
    })?;
    interpolate_request_json(&raw, ctx)
}

/// Interpolate a raw request object and deserialize it as a `RequestConfig`.
/// Errors name the offending field (e.g. `timeout` when `"${store.t}"`
/// resolved to a non-numeric string).
pub fn interpolate_request_json(raw: &Json, ctx: &InterpCtx) -> Result<RequestConfig, ParseError> {
    serde_path_to_error::deserialize(interpolate_json(raw, ctx)).map_err(|e| ParseError::Invalid {
        field: e.path().to_string(),
        message: e.inner().to_string(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::NaiveDate;
    use serde_json::json;

    fn vars(pairs: &[(&str, &str)]) -> IndexMap<String, String> {
        pairs
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect()
    }

    fn no_env(_: &str) -> Option<String> {
        None
    }

    /// Interpolate with variables, optional store, no env.
    fn interp(s: &str, v: &[(&str, &str)], store: Option<&[(&str, &str)]>) -> String {
        let v = vars(v);
        let store = store.map(vars);
        interpolate_str(s, &InterpCtx::new(&v, store.as_ref(), &no_env))
    }

    fn resolve(name: &str, v: &[(&str, &str)], store: Option<&[(&str, &str)]>) -> Option<String> {
        let v = vars(v);
        let store = store.map(vars);
        resolve_variable(name, &InterpCtx::new(&v, store.as_ref(), &no_env))
    }

    fn is_uuid(s: &str) -> bool {
        let p: Vec<_> = s.split('-').collect();
        p.iter().map(|x| x.len()).eq([8, 4, 4, 4, 12])
            && s.chars().all(|c| c == '-' || c.is_ascii_hexdigit())
    }

    // ---- interpolate (variable-resolver.test.ts) ----

    #[test]
    fn basic_interpolation() {
        assert_eq!(interp("hello world", &[], None), "hello world");
        assert_eq!(interp("${VAR}", &[("VAR", "value")], None), "value");
        assert_eq!(
            interp("hello ${NAME}!", &[("NAME", "world")], None),
            "hello world!"
        );
        assert_eq!(
            interp(
                "${GREETING} ${NAME}!",
                &[("GREETING", "Hello"), ("NAME", "World")],
                None
            ),
            "Hello World!"
        );
        assert_eq!(interp("${UNKNOWN}", &[], None), "${UNKNOWN}");
    }

    #[test]
    fn unresolved_placeholder_left_intact() {
        assert_eq!(interp("${NOPE}/x", &[], None), "${NOPE}/x");
    }

    #[test]
    fn json_nested_objects_and_arrays() {
        let v = vars(&[
            ("BASE_URL", "https://api.example.com"),
            ("TOKEN", "secret"),
            ("A", "1"),
        ]);
        let ctx = InterpCtx::new(&v, None, &no_env);
        let input = json!({
            "url": "${BASE_URL}/api",
            "headers": {"Authorization": "Bearer ${TOKEN}"},
            "ids": ["${A}", "${A}"],
            "n": 42, "b": true, "z": null
        });
        assert_eq!(
            interpolate_json(&input, &ctx),
            json!({
                "url": "https://api.example.com/api",
                "headers": {"Authorization": "Bearer secret"},
                "ids": ["1", "1"],
                "n": 42, "b": true, "z": null
            })
        );
    }

    #[test]
    fn json_keys_are_not_interpolated_and_forbidden_keys_dropped() {
        let v = vars(&[("K", "x")]);
        let ctx = InterpCtx::new(&v, None, &no_env);
        let input = json!({"${K}": "${K}", "__proto__": {"p": 1}, "prototype": 1,
                           "constructor": "c", "safe": "ok"});
        assert_eq!(
            interpolate_json(&input, &ctx),
            json!({"${K}": "x", "safe": "ok"})
        );
    }

    #[test]
    fn whole_string_replacement_stays_a_string() {
        let v = vars(&[("PORT", "8080")]);
        let ctx = InterpCtx::new(&v, None, &no_env);
        assert_eq!(interpolate_json(&json!("${PORT}"), &ctx), json!("8080"));
    }

    // ---- store context ----

    #[test]
    fn store_variables() {
        let st = [("userId", "123"), ("postId", "456")];
        assert_eq!(interp("${store.userId}", &[], Some(&st)), "123");
        assert_eq!(
            interp(
                "/users/${store.userId}/posts/${store.postId}",
                &[],
                Some(&st)
            ),
            "/users/123/posts/456"
        );
        assert_eq!(
            interp("${store.missing}", &[], Some(&[])),
            "${store.missing}"
        );
        assert_eq!(
            interp(
                "${BASE}/users/${store.userId}",
                &[("BASE", "https://api.example.com")],
                Some(&st)
            ),
            "https://api.example.com/users/123"
        );
    }

    #[test]
    fn store_json_nested_and_arrays() {
        let v = IndexMap::new();
        let st = vars(&[("token", "jwt"), ("id1", "1"), ("id2", "2")]);
        let ctx = InterpCtx::new(&v, Some(&st), &no_env);
        let input = json!({
            "headers": {"Authorization": "Bearer ${store.token}"},
            "ids": ["${store.id1}", "${store.id2}"],
            "body": {"userId": "${store.id1}", "name": "Test User"}
        });
        assert_eq!(
            interpolate_json(&input, &ctx),
            json!({
                "headers": {"Authorization": "Bearer jwt"},
                "ids": ["1", "2"],
                "body": {"userId": "1", "name": "Test User"}
            })
        );
    }

    #[test]
    fn store_resolves_without_store_context_falls_through() {
        // No store: `store.x` is looked up as a plain static variable.
        assert_eq!(
            interp("${store.x}", &[("store.x", "static")], None),
            "static"
        );
        assert_eq!(interp("${store.x}", &[], None), "${store.x}");
    }

    #[test]
    fn store_takes_priority_over_static() {
        assert_eq!(
            resolve(
                "store.userId",
                &[("store.userId", "static")],
                Some(&[("userId", "store")])
            ),
            Some("store".into())
        );
        assert_eq!(resolve("store.missing", &[], Some(&[])), None);
    }

    #[test]
    fn store_with_dynamic_variables() {
        let st = [("token", "my-token")];
        assert!(is_uuid(&interp("${UUID}", &[], Some(&st))));
        assert_eq!(
            interp("Bearer ${store.token}", &[], Some(&st)),
            "Bearer my-token"
        );
    }

    // ---- transforms ----

    #[test]
    fn string_transforms() {
        assert_eq!(
            interp("${ENV:upper}", &[("ENV", "production")], None),
            "PRODUCTION"
        );
        assert_eq!(
            interp("${RESOURCE:lower}", &[("RESOURCE", "USERS")], None),
            "users"
        );
        assert_eq!(
            interp(
                "${BASE_URL}/${RESOURCE:lower}",
                &[
                    ("BASE_URL", "https://api.example.com"),
                    ("RESOURCE", "Users")
                ],
                None
            ),
            "https://api.example.com/users"
        );
        assert_eq!(interp("${MISSING:upper}", &[], None), "${MISSING:upper}");
        // empty value is falsy in TS, so the reference is left intact
        assert_eq!(interp("${E:upper}", &[("E", "")], None), "${E:upper}");
    }

    // ---- defaults ----

    #[test]
    fn default_values() {
        assert_eq!(interp("${TIMEOUT:5000}", &[], None), "5000");
        assert_eq!(
            interp("${TIMEOUT:5000}", &[("TIMEOUT", "10000")], None),
            "10000"
        );
        assert_eq!(interp("${OPTIONAL:}", &[], None), "");
        assert_eq!(
            interp("${URL:https://api.example.com/v1}", &[], None),
            "https://api.example.com/v1"
        );
        assert_eq!(
            interp(
                "${BASE_URL:https://api.example.com}/${VERSION:v1}/users",
                &[],
                None
            ),
            "https://api.example.com/v1/users"
        );
        assert_eq!(
            interp(
                "${BASE_URL}/api/${VERSION:v1}/users",
                &[("BASE_URL", "https://prod.example.com")],
                None
            ),
            "https://prod.example.com/api/v1/users"
        );
    }

    #[test]
    fn nested_defaults() {
        assert_eq!(
            interp("${DB_HOST:${FALLBACK:localhost}}", &[], None),
            "localhost"
        );
        assert_eq!(
            interp(
                "${DB_HOST:${FALLBACK:localhost}}",
                &[("FALLBACK", "backup-host")],
                None
            ),
            "backup-host"
        );
        // inner reference with no value and no default keeps the raw text
        assert_eq!(interp("${A:${B}}", &[], None), "${B}");
    }

    // ---- dynamic ----

    #[test]
    fn dynamic_variables_shape() {
        assert!(is_uuid(&interp("${UUID}", &[], None)));
        let short = interp("${UUID:short}", &[], None);
        assert!(short.len() == 8 && short.chars().all(|c| c.is_ascii_hexdigit()));
        for _ in 0..200 {
            let n: u32 = interp("${RANDOM:1-100}", &[], None).parse().unwrap();
            assert!((1..=100).contains(&n));
        }
        let s = interp("${RANDOM:string:16}", &[], None);
        assert!(s.len() == 16 && s.chars().all(|c| c.is_ascii_alphanumeric()));
        assert!(
            interp("${TIMESTAMP}", &[], None)
                .bytes()
                .all(|c| c.is_ascii_digit())
        );
        assert!(
            interp("${CURRENT_TIME}", &[], None)
                .bytes()
                .all(|c| c.is_ascii_digit())
        );
        let d = interp("${DATE:YYYY-MM-DD}", &[], None);
        assert!(NaiveDate::parse_from_str(&d, "%Y-%m-%d").is_ok(), "{d}");
        let t = interp("${TIME:HH:mm:ss}", &[], None);
        assert!(
            chrono::NaiveTime::parse_from_str(&t, "%H:%M:%S").is_ok(),
            "{t}"
        );
    }

    #[test]
    fn multiple_dynamic_variables_in_one_object() {
        let v = IndexMap::new();
        let ctx = InterpCtx::new(&v, None, &no_env);
        let out = interpolate_json(
            &json!({"a": "${UUID:short}", "b": "${RANDOM:1-1000}", "c": "${RANDOM:string:10}"}),
            &ctx,
        );
        assert_eq!(out["a"].as_str().unwrap().len(), 8);
        assert!(out["b"].as_str().unwrap().parse::<u32>().is_ok());
        assert_eq!(out["c"].as_str().unwrap().len(), 10);
    }

    #[test]
    fn resolve_dynamic_unknown_is_none() {
        assert_eq!(resolve_dynamic_variable("UNKNOWN"), None);
        assert_eq!(resolve_dynamic_variable("RANDOM:abc"), None);
    }

    #[test]
    fn dynamic_prefix_with_unknown_suffix_is_unresolved() {
        assert_eq!(interp("${RANDOM:abc}", &[], None), "${RANDOM:abc}");
    }

    // ---- resolveVariable ----

    #[test]
    fn resolve_variable_basics() {
        assert_eq!(
            resolve("BASE_URL", &[("BASE_URL", "https://example.com")], None),
            Some("https://example.com".into())
        );
        assert_eq!(resolve("UNKNOWN", &[], None), None);
        assert_eq!(
            resolve("store.userId", &[], Some(&[("userId", "123")])),
            Some("123".into())
        );
        assert_eq!(
            resolve("ENV:upper", &[("ENV", "production")], None),
            Some("PRODUCTION".into())
        );
        assert_eq!(resolve("TIMEOUT:5000", &[], None), Some("5000".into()));
        assert_eq!(
            resolve("TIMEOUT:5000", &[("TIMEOUT", "10000")], None),
            Some("10000".into())
        );
    }

    // ---- environment ----

    #[test]
    fn env_is_last_resort_and_toggleable() {
        let env = |n: &str| match n {
            "HOME_X" => Some("/home/x".to_string()),
            "SHARED" => Some("env".to_string()),
            "EMPTY" => Some(String::new()),
            _ => None,
        };
        let v = vars(&[("SHARED", "static")]);
        let ctx = InterpCtx::new(&v, None, &env);
        assert_eq!(interpolate_str("${HOME_X}", &ctx), "/home/x");
        assert_eq!(interpolate_str("${SHARED}", &ctx), "static");
        assert_eq!(interpolate_str("a${EMPTY}b", &ctx), "ab");
        assert_eq!(interpolate_str("${HOME_X:upper}", &ctx), "/HOME/X");
        assert_eq!(interpolate_str("${NOPE:fallback}", &ctx), "fallback");
        let off = InterpCtx {
            resolve_env: false,
            ..ctx
        };
        assert_eq!(interpolate_str("${HOME_X}", &off), "${HOME_X}");
        assert_eq!(interpolate_str("${HOME_X:d}", &off), "d");
    }

    #[test]
    fn process_env_reads_real_environment() {
        assert!(process_env("PATH").is_some());
        assert_eq!(process_env("CURL_RUNNER_SURELY_NOT_SET_4"), None);
    }

    // ---- extractVariables ----

    #[test]
    fn extract_variables_cases() {
        let r = extract_variables("${VAR}");
        assert_eq!(
            r,
            vec![VariableRef {
                start: 0,
                end: 6,
                name: "VAR".into()
            }]
        );
        let r = extract_variables("${A} and ${B}");
        assert_eq!((r[0].name.as_str(), r[1].name.as_str()), ("A", "B"));
        let r = extract_variables("${VAR:${DEFAULT}}");
        assert_eq!(r.len(), 1);
        assert_eq!(r[0].name, "VAR:${DEFAULT}");
        assert!(extract_variables("no variables here").is_empty());
        assert!(extract_variables("${incomplete").is_empty());
    }

    #[test]
    fn non_ascii_text_around_refs() {
        assert_eq!(
            interp("héllo ${A} wörld ✓", &[("A", "ü")], None),
            "héllo ü wörld ✓"
        );
    }

    // ---- formatDate / formatTime ----

    #[test]
    fn format_date_cases() {
        let d = NaiveDate::from_ymd_opt(2024, 1, 15)
            .unwrap()
            .and_hms_opt(0, 0, 0)
            .unwrap();
        assert_eq!(format_date(&d, "YYYY-MM-DD"), "2024-01-15");
        let d = NaiveDate::from_ymd_opt(2024, 12, 25)
            .unwrap()
            .and_hms_opt(0, 0, 0)
            .unwrap();
        assert_eq!(format_date(&d, "DD/MM/YYYY"), "25/12/2024");
        let d = NaiveDate::from_ymd_opt(2024, 6, 10)
            .unwrap()
            .and_hms_opt(0, 0, 0)
            .unwrap();
        assert_eq!(format_date(&d, "YYYY"), "2024");
        assert_eq!(format_date(&d, "MM-DD"), "06-10");
    }

    #[test]
    fn format_time_cases() {
        let d = |h, m, s| {
            NaiveDate::from_ymd_opt(2024, 1, 1)
                .unwrap()
                .and_hms_opt(h, m, s)
                .unwrap()
        };
        assert_eq!(format_time(&d(14, 30, 45), "HH:mm:ss"), "14:30:45");
        assert_eq!(format_time(&d(9, 5, 0), "HH:mm"), "09:05");
        assert_eq!(format_time(&d(1, 2, 3), "HH:mm:ss"), "01:02:03");
    }

    // ---- interpolate_request ----

    #[test]
    fn request_interpolation_uses_store() {
        let req: RequestConfig = serde_json::from_value(json!({
            "url": "https://x/${store.id}",
            "method": "POST",
            "headers": {"Authorization": "Bearer ${store.token}"},
            "body": {"id": "${store.id}", "n": 5},
            "timeout": 3000,
            "expect": {"status": [200, 201]}
        }))
        .unwrap();
        let v = IndexMap::new();
        let st = vars(&[("id", "7"), ("token", "t")]);
        let out = interpolate_request(&req, &InterpCtx::new(&v, Some(&st), &no_env)).unwrap();
        assert_eq!(out.url, "https://x/7");
        assert_eq!(out.headers.unwrap()["Authorization"], "Bearer t");
        assert_eq!(out.body.unwrap(), json!({"id": "7", "n": 5}));
        assert_eq!(out.timeout, Some(3000));
        assert_eq!(
            out.expect.unwrap().status,
            Some(crate::model::StatusExpect::Many(vec![200, 201]))
        );
    }

    #[test]
    fn request_untouched_when_store_is_empty() {
        let req = RequestConfig {
            url: "${UUID}".into(),
            ..Default::default()
        };
        let v = IndexMap::new();
        let st = Store::new();
        let out = interpolate_request(&req, &InterpCtx::new(&v, Some(&st), &no_env)).unwrap();
        assert_eq!(out, req);
    }

    #[test]
    fn request_static_variables_without_store() {
        let req = RequestConfig {
            url: "${BASE}/u".into(),
            ..Default::default()
        };
        let v = vars(&[("BASE", "http://h")]);
        let out = interpolate_request(&req, &InterpCtx::new(&v, None, &no_env)).unwrap();
        assert_eq!(out.url, "http://h/u");
    }

    #[test]
    fn request_with_invalid_typed_value_errors_naming_field() {
        let req = RequestConfig {
            url: "http://h".into(),
            timeout: Some(5),
            ..Default::default()
        };
        // Simulate a placeholder in a typed field by interpolating raw JSON.
        let mut raw = serde_json::to_value(&req).unwrap();
        raw["timeout"] = json!("${store.t}");
        let v = IndexMap::new();
        let st = vars(&[("t", "abc")]);
        let ctx = InterpCtx::new(&v, Some(&st), &no_env);
        let err = interpolate_request_json(&raw, &ctx).unwrap_err();
        assert!(
            matches!(&err, ParseError::Invalid { field, .. } if field == "timeout"),
            "{err}"
        );
        // and resolves fine when the value is numeric
        let ok = vars(&[("t", "30")]);
        let ctx = InterpCtx::new(&v, Some(&ok), &no_env);
        assert_eq!(
            interpolate_request_json(&raw, &ctx).unwrap().timeout,
            Some(30)
        );
    }
}
