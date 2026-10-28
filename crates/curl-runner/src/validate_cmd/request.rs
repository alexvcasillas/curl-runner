//! Request rules (TS `validateRequestConfig` and the HTTP-level checks:
//! method, headers, body, formData, timeout, retry, store, redirects).

use super::issues::{
    Crash, Ctx, Fix, FixValue, Res, Severity, as_num, as_str, change_to, entries,
    find_closest_match, has_key, is_identifier, is_object, is_sensitive_value, js_number,
    js_string, not_plain_object, prop, truthy, truthy_prop,
};
use super::{conditions, global, security, url};
use curl_runner_core::jsval::{num_to_string, parse_int, trim};
use curl_runner_core::model::Json;

pub const VALID_METHODS: &[&str] = &["GET", "POST", "PUT", "DELETE", "PATCH", "HEAD", "OPTIONS"];

pub(super) const VALID_RETRY_KEYS: &[&str] = &["count", "delay", "backoff", "retryableStatuses"];
pub const VALID_REQUEST_KEYS: &[&str] = &[
    "name",
    "url",
    "method",
    "headers",
    "params",
    "body",
    "formData",
    "timeout",
    "followRedirects",
    "maxRedirects",
    "auth",
    "proxy",
    "insecure",
    "ssl",
    "output",
    "http2",
    "retry",
    "variables",
    "store",
    "when",
    "expect",
    "snapshot",
    "diff",
    "sourceOutputConfig",
    "sourceFile",
];

/// TS `STANDARD_HEADERS` (lower-case name → canonical casing).
const STANDARD_HEADERS: &[&str] = &[
    "Content-Type",
    "Content-Length",
    "Content-Encoding",
    "Content-Language",
    "Content-Disposition",
    "Accept",
    "Accept-Encoding",
    "Accept-Language",
    "Accept-Charset",
    "Authorization",
    "Cache-Control",
    "Connection",
    "Cookie",
    "Set-Cookie",
    "Host",
    "Origin",
    "Referer",
    "User-Agent",
    "X-Requested-With",
    "X-Forwarded-For",
    "X-Forwarded-Proto",
    "X-API-Key",
    "X-Auth-Token",
    "X-CSRF-Token",
    "X-Request-ID",
    "X-Correlation-ID",
    "ETag",
    "If-Match",
    "If-None-Match",
    "If-Modified-Since",
    "If-Unmodified-Since",
    "Last-Modified",
    "Location",
    "Pragma",
    "Proxy-Authorization",
    "Range",
    "Content-Range",
    "TE",
    "Trailer",
    "Transfer-Encoding",
    "Upgrade",
    "Vary",
    "Via",
    "Warning",
    "WWW-Authenticate",
    "Access-Control-Allow-Origin",
    "Access-Control-Allow-Methods",
    "Access-Control-Allow-Headers",
    "Access-Control-Allow-Credentials",
    "Access-Control-Expose-Headers",
    "Access-Control-Max-Age",
    "Strict-Transport-Security",
    "X-Content-Type-Options",
    "X-Frame-Options",
    "X-XSS-Protection",
];

fn standard_casing(lower: &str) -> Option<&'static str> {
    STANDARD_HEADERS
        .iter()
        .copied()
        .find(|h| h.to_ascii_lowercase() == lower)
}

/// `getHeaderValue`: first header whose name matches case-insensitively.
fn header_value<'a>(headers: &'a Json, name: &str) -> Option<&'a Json> {
    entries(headers)
        .into_iter()
        .find(|(k, _)| k.to_lowercase() == name)
        .map(|(_, v)| v)
}

pub fn validate_request_config(cx: &mut Ctx, req: &Json, path: &str, is_defaults: bool) -> Res {
    // The runner interpolates request objects, not `defaults` blocks.
    let saved = cx.literal_only;
    cx.literal_only |= is_defaults;
    let res = validate_request_fields(cx, req, path, is_defaults);
    cx.literal_only = saved;
    res
}

fn validate_request_fields(cx: &mut Ctx, req: &Json, path: &str, is_defaults: bool) -> Res {
    cx.check_unknown_keys(req, VALID_REQUEST_KEYS, path)?;

    if let Some(name) = truthy_prop(req, "name") {
        let entry = format!("{path}:{}", js_string(name));
        if !cx.request_names.contains(&entry) {
            cx.request_names.push(entry);
        }
    }

    let url_v = truthy_prop(req, "url");
    if !is_defaults && url_v.is_none() {
        cx.add(&format!("{path}.url"), Severity::Error, "URL is required");
    }
    if let Some(u) = url_v {
        url::validate_url(cx, u, &format!("{path}.url"));
    }

    if let Some(m) = truthy_prop(req, "method") {
        validate_method(cx, m, &format!("{path}.method"))?;
    }

    let body = prop(req, "body");
    if let Some(h) = truthy_prop(req, "headers") {
        validate_headers(cx, h, &format!("{path}.headers"), req);
    } else if body.is_some() {
        cx.add(
            &format!("{path}.headers"),
            Severity::Info,
            "Body present but no Content-Type header; consider adding one",
        );
    }

    if truthy_prop(req, "params").is_some_and(not_plain_object) {
        cx.add(
            &format!("{path}.params"),
            Severity::Error,
            "Params must be an object",
        );
    }

    if body.is_some() && prop(req, "formData").is_some() {
        cx.add(
            path,
            Severity::Error,
            "Cannot use both \"body\" and \"formData\"",
        );
    }
    if let Some(b) = body {
        validate_body(cx, b, &format!("{path}.body"), req)?;
    }
    if let Some(fd) = truthy_prop(req, "formData") {
        validate_form_data(cx, fd, &format!("{path}.formData"));
    }

    if let Some(t) = prop(req, "timeout") {
        validate_timeout(cx, t, &format!("{path}.timeout"));
    }
    if let Some(a) = truthy_prop(req, "auth") {
        security::validate_auth(cx, a, &format!("{path}.auth"))?;
    }
    if let Some(s) = truthy_prop(req, "ssl") {
        security::validate_ssl(cx, s, &format!("{path}.ssl"))?;
    }
    if let Some(insecure) = prop(req, "insecure") {
        cx.boolean_field(req, "insecure", path);
        if insecure == &Json::Bool(true) {
            cx.add(
                &format!("{path}.insecure"),
                Severity::Warning,
                "Insecure mode disables SSL verification",
            );
        }
    }
    if let Some(p) = truthy_prop(req, "proxy") {
        url::validate_proxy_url(cx, p, &format!("{path}.proxy"));
    }
    if let Some(r) = truthy_prop(req, "retry") {
        validate_retry(cx, r, &format!("{path}.retry"), prop(req, "method"))?;
    }
    if let Some(w) = truthy_prop(req, "when") {
        conditions::validate_when(cx, w, &format!("{path}.when"))?;
    }
    if let Some(e) = truthy_prop(req, "expect") {
        conditions::validate_expect(cx, e, &format!("{path}.expect"))?;
    }
    if let Some(s) = truthy_prop(req, "store") {
        validate_store(cx, s, &format!("{path}.store"));
    }

    if let Some(mr) = prop(req, "maxRedirects") {
        let p = format!("{path}.maxRedirects");
        if cx.not_num_at_least(mr, 0.0) {
            let m = cx.num_msg(mr, "Must be a non-negative number");
            cx.add(&p, Severity::Error, m);
        } else if as_num(mr).is_some_and(|n| n > 50.0) {
            cx.add(&p, Severity::Warning, "Very high redirect limit");
        }
    }

    cx.boolean_field(req, "followRedirects", path);
    cx.boolean_field(req, "http2", path);

    if let Some(s) = prop(req, "snapshot").filter(|v| is_object(v)) {
        global::validate_snapshot(
            cx,
            s,
            &format!("{path}.snapshot"),
            global::VALID_REQUEST_SNAPSHOT_KEYS,
        )?;
    }
    if let Some(d) = prop(req, "diff").filter(|v| is_object(v)) {
        global::validate_diff(
            cx,
            d,
            &format!("{path}.diff"),
            global::VALID_REQUEST_DIFF_KEYS,
        )?;
    }
    Ok(())
}

fn validate_method(cx: &mut Ctx, method: &Json, path: &str) -> Res {
    let method = as_str(method).ok_or_else(|| {
        Crash(
            "method.toUpperCase is not a function. (In 'method.toUpperCase()', 'method.toUpperCase' is undefined)"
                .into(),
        )
    })?;
    let upper = method.to_uppercase();
    if !VALID_METHODS.contains(&upper.as_str()) {
        let suggestion = find_closest_match(&upper, VALID_METHODS);
        cx.add_fix(
            path,
            Severity::Error,
            format!(
                "Invalid method \"{method}\". Must be: {}",
                VALID_METHODS.join(", ")
            ),
            suggestion.map(change_to),
        );
    } else if method != upper {
        cx.add_fix(
            path,
            Severity::Warning,
            format!("Method should be uppercase: \"{upper}\""),
            Some(change_to(&upper)),
        );
    }
    Ok(())
}

fn validate_headers(cx: &mut Ctx, headers: &Json, path: &str, req: &Json) {
    if not_plain_object(headers) {
        cx.add(path, Severity::Error, "Headers must be an object");
        return;
    }
    let mut seen: Vec<(String, String)> = Vec::new();
    for (key, value) in entries(headers) {
        let lower = key.to_lowercase();
        let p = format!("{path}.{key}");
        if let Some(std) = standard_casing(&lower)
            && key != std
        {
            cx.add_fix(
                &p,
                Severity::Info,
                format!("Header \"{key}\" should be \"{std}\" for consistency"),
                Some(Fix {
                    description: format!("Rename to \"{std}\""),
                    value: FixValue::Rename {
                        to: std.into(),
                        value: Some(value.clone()),
                    },
                }),
            );
        }
        if let Some((_, first)) = seen.iter().find(|(l, _)| *l == lower) {
            cx.add(
                &p,
                Severity::Warning,
                format!("Duplicate header \"{key}\" (case-insensitive match with \"{first}\")"),
            );
        }
        match seen.iter_mut().find(|(l, _)| *l == lower) {
            Some(slot) => slot.1 = key.clone(),
            None => seen.push((lower, key.clone())),
        }
        if !value.is_string() && !value.is_number() {
            cx.add(
                &p,
                Severity::Error,
                "Header value must be a string or number",
            );
        }
        if let Some(v) = value.as_str()
            && is_sensitive_value(key, v)
        {
            cx.add(
                &p,
                Severity::Warning,
                "Potentially sensitive header value. Consider using variables like \"${env.TOKEN}\"",
            );
        }
    }

    if prop(req, "body").is_some() && !truthy(header_value(headers, "content-type")) {
        cx.add(
            path,
            Severity::Info,
            "Body present but no Content-Type header; consider adding one",
        );
    }
    if truthy_prop(req, "auth").is_some() && truthy(header_value(headers, "authorization")) {
        cx.add(
            path,
            Severity::Warning,
            "Both auth block and Authorization header present; auth block may be ignored",
        );
    }
}

fn validate_body(cx: &mut Ctx, body: &Json, path: &str, req: &Json) -> Res {
    let method = truthy_prop(req, "method")
        .and_then(as_str)
        .unwrap_or("GET")
        .to_uppercase();
    if ["GET", "HEAD", "OPTIONS"].contains(&method.as_str()) {
        cx.add(
            path,
            Severity::Warning,
            format!("Body with {method} method may be ignored by some servers"),
        );
    }

    let Some(body) = body.as_str() else {
        return Ok(());
    };
    let trimmed = trim(body);
    let content_type = truthy_prop(req, "headers").and_then(|h| header_value(h, "content-type"));
    let is_json_ct = match content_type {
        None | Some(Json::Null) => false,
        Some(Json::String(s)) => s.to_lowercase().contains("json"),
        Some(_) => return Err(Crash(CONTENT_TYPE_CRASH.into())),
    };
    let looks_like_json = (trimmed.starts_with('{') && trimmed.ends_with('}'))
        || (trimmed.starts_with('[') && trimmed.ends_with(']'));
    if (looks_like_json || is_json_ct)
        && serde_json::from_str::<serde::de::IgnoredAny>(trimmed).is_err()
    {
        cx.add(
            path,
            Severity::Warning,
            "Body looks like JSON but is invalid; check syntax",
        );
    }
    Ok(())
}

/// What Bun reports for `contentType?.toLowerCase()` on a non-string header.
const CONTENT_TYPE_CRASH: &str = "(request.headers ? this.getHeaderValue(request.headers, \"content-type\") : null)?.toLowerCase is not a function. (In '(request.headers ? this.getHeaderValue(request.headers, \"content-type\") : null)?.toLowerCase()', '(request.headers ? this.getHeaderValue(request.headers, \"content-type\") : null)?.toLowerCase' is undefined)";

fn validate_form_data(cx: &mut Ctx, fd: &Json, path: &str) {
    if not_plain_object(fd) {
        cx.add(path, Severity::Error, "FormData must be an object");
        return;
    }
    for (key, value) in entries(fd) {
        if !(value.is_object() || value.is_array()) || !has_key(value, "file") {
            continue;
        }
        let fp = format!("{path}.{key}.file");
        match prop(value, "file").and_then(as_str) {
            Some(f) => cx.file_path(f, &fp),
            None => cx.add(&fp, Severity::Error, "File path must be a string"),
        }
        if truthy_prop(value, "contentType").is_some_and(|v| !v.is_string()) {
            cx.add(
                &format!("{path}.{key}.contentType"),
                Severity::Error,
                "contentType must be a string",
            );
        }
        if truthy_prop(value, "filename").is_some_and(|v| !v.is_string()) {
            cx.add(
                &format!("{path}.{key}.filename"),
                Severity::Error,
                "filename must be a string",
            );
        }
    }
}

fn validate_timeout(cx: &mut Ctx, timeout: &Json, path: &str) {
    if cx.interpolated(timeout) {
        return;
    }
    let Some(t) = as_num(timeout) else {
        if let Some(s) = timeout.as_str() {
            let parsed = parse_int(s);
            if parsed > 0.0 {
                cx.add_fix(
                    path,
                    Severity::Warning,
                    "Timeout should be a number, not string",
                    Some(convert_to(parsed)),
                );
                return;
            }
        }
        let m = cx.num_msg(timeout, "Timeout must be a positive number");
        cx.add(path, Severity::Error, m);
        return;
    };
    if t <= 0.0 {
        cx.add(path, Severity::Error, "Timeout must be a positive number");
    } else if t < 100.0 {
        cx.add(
            path,
            Severity::Warning,
            "Very short timeout (<100ms) may cause failures",
        );
    } else if t > 300_000.0 {
        cx.add(
            path,
            Severity::Warning,
            "Very long timeout (>5min) - is this intentional?",
        );
    }
}

/// `{ description: 'Convert to N', apply: () => N }` for a `parseInt` result.
pub fn convert_to(n: f64) -> Fix {
    Fix {
        description: format!("Convert to {}", num_to_string(n)),
        value: FixValue::Set(
            serde_json::Number::from_f64(n)
                .map(|x| {
                    if n.fract() == 0.0 && n.abs() < 9.007_199_254_740_992e15 {
                        Json::from(n as i64)
                    } else {
                        Json::Number(x)
                    }
                })
                .unwrap_or(Json::Null),
        ),
    }
}

fn validate_retry(cx: &mut Ctx, retry: &Json, path: &str, method: Option<&Json>) -> Res {
    cx.check_unknown_keys(retry, VALID_RETRY_KEYS, path)?;

    let count = prop(retry, "count");
    let cp = format!("{path}.count");
    match count {
        None => cx.add(&cp, Severity::Error, "Retry count is required"),
        Some(c) if cx.not_num_at_least(c, 0.0) => {
            let m = cx.num_msg(c, "Retry count must be a non-negative number");
            cx.add(&cp, Severity::Error, m);
        }
        Some(c) if as_num(c).is_some_and(|n| n > 10.0) => cx.add(
            &cp,
            Severity::Warning,
            "High retry count (>10) may cause long delays",
        ),
        Some(_) => {}
    }

    if let Some(d) = prop(retry, "delay").filter(|d| cx.not_num_at_least(d, 0.0)) {
        let m = cx.num_msg(d, "Retry delay must be a non-negative number");
        cx.add(&format!("{path}.delay"), Severity::Error, m);
    }

    if let Some(b) = prop(retry, "backoff") {
        let bp = format!("{path}.backoff");
        if cx.not_num_at_least(b, 1.0) {
            let m = cx.num_msg(b, "Retry backoff must be >= 1");
            cx.add(&bp, Severity::Error, m);
        } else if as_num(b).is_some_and(|n| n > 5.0) {
            cx.add(
                &bp,
                Severity::Warning,
                "High backoff multiplier may cause very long delays",
            );
        }
    }

    if let Some(m) = method.filter(|m| truthy(Some(m))).and_then(as_str)
        && ["POST", "PATCH"].contains(&m.to_uppercase().as_str())
        && count.is_some_and(|c| truthy(Some(c)) && js_number(c) > 0.0)
    {
        cx.add(
            path,
            Severity::Info,
            format!("Retrying {m} requests may cause duplicate operations"),
        );
    }
    Ok(())
}

fn validate_store(cx: &mut Ctx, store: &Json, path: &str) {
    if not_plain_object(store) {
        cx.add(path, Severity::Error, "Store must be an object");
        return;
    }
    const VALID: [&str; 4] = ["body", "status", "headers", "responseTime"];
    for (key, value) in entries(store) {
        let p = format!("{path}.{key}");
        if !is_identifier(key) {
            cx.add(
                &p,
                Severity::Warning,
                "Store variable names should use alphanumeric characters and underscores",
            );
        }
        match value.as_str() {
            None => cx.add(&p, Severity::Error, "Store path must be a string"),
            Some(v) => {
                let first = v.split('.').next().unwrap_or_default();
                if !VALID.contains(&first) {
                    cx.add(
                        &p,
                        Severity::Warning,
                        format!("Store path should start with: {}", VALID.join(", ")),
                    );
                }
            }
        }
    }
}
