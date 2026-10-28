//! Normalization: [`CurlAst`] to [`CurlRunnerIr`] (`convert/normalizer.ts`).
//! Infers methods, detects body types, canonicalizes headers, parses query
//! strings.

use super::jsjson;
use super::jsutil::{JsMap, atob, decode_uri_component, encode_uri_component};
use super::types::{
    Auth, Body, CurlAst, CurlRunnerIr, FormField, FormFile, NumText, RetryIr, Source, Ssl,
};
use crate::jsval::trim;

/// Conversion failed where the TS code threw (a malformed `%` escape).
pub type NormalizeError = super::jsutil::UriError;

/// Normalize a [`CurlAst`] (`normalize`).
pub fn normalize(ast: &CurlAst) -> Result<CurlRunnerIr, NormalizeError> {
    let mut warnings: Vec<String> = Vec::new();

    let (url, params) = parse_url(&ast.url)?;
    let mut headers = normalize_headers(ast);
    let auth = detect_auth(ast, &mut headers);
    let (body, form_data) = detect_body(ast);
    // curl labels `-d`/`--data*` text as form data unless told otherwise, while the
    // runner defaults a body to JSON: keep what curl would have sent.
    if matches!(
        body,
        Some(Body::Urlencoded(_) | Body::Raw(_) | Body::Binary(_))
    ) && headers.get("Content-Type").is_none()
    {
        headers.insert(
            "Content-Type",
            "application/x-www-form-urlencoded".to_string(),
        );
    }
    let method = infer_method(ast, body.is_some(), form_data.is_some());
    let ssl = detect_ssl(ast);
    let retry = detect_retry(ast, &mut warnings);

    for f in &ast.unsupported_flags {
        let value = f.value.as_deref().filter(|v| !v.is_empty());
        warnings.push(format!(
            "Unsupported curl flag: {}{}",
            f.flag,
            value.map(|v| format!(" {v}")).unwrap_or_default()
        ));
    }
    if ast.compressed {
        warnings.push(
            "Flag --compressed has no YAML equivalent; curl handles decompression natively".into(),
        );
    }
    if let Some(c) = non_empty(&ast.cookie) {
        warnings.push(format!(
            "Cookie flag (-b {c}) stored as header; manual review recommended"
        ));
        headers.insert("Cookie", c.to_string());
    }
    if let Some(c) = non_empty(&ast.cookie_jar) {
        warnings.push(format!("Cookie jar flag (-c {c}) has no YAML equivalent"));
    }
    if let Some(ua) = non_empty(&ast.user_agent)
        && !truthy(headers.get("User-Agent"))
    {
        headers.insert("User-Agent", ua.to_string());
    }
    if let Some(r) = non_empty(&ast.referer)
        && !truthy(headers.get("Referer"))
    {
        headers.insert("Referer", r.to_string());
    }

    Ok(CurlRunnerIr {
        name: None,
        method,
        url,
        headers,
        params: (!params.is_empty()).then_some(params),
        body,
        form_data,
        auth,
        insecure: ast.insecure,
        follow_redirects: ast.location.then_some(true),
        max_redirects: ast.max_redirs.map(NumText::Num),
        timeout: ast.max_time.map(|t| t * 1000.0),
        proxy: non_empty(&ast.proxy).map(str::to_string),
        output: non_empty(&ast.output).map(str::to_string),
        http2: ast.http2,
        retry,
        ssl,
        source: Source::Curl,
        falsy_content_type: false,
        warnings,
        unsupported_flags: ast.unsupported_flags.clone(),
    })
}

/// `--retry N` / `--retry-delay S` (whole seconds) to `retry`. Anything that
/// cannot be mapped exactly is reported, never dropped silently.
fn detect_retry(ast: &CurlAst, warnings: &mut Vec<String>) -> Option<RetryIr> {
    let whole = |v: &str| {
        let v = trim(v);
        (!v.is_empty() && v.bytes().all(|b| b.is_ascii_digit()))
            .then(|| v.parse::<u64>().ok())
            .flatten()
    };
    let count = ast.retry.as_deref().and_then(|v| {
        let n = whole(v);
        if n.is_none() {
            warnings.push(format!(
                "Unsupported curl flag: --retry {v} (expected a whole number); ignored"
            ));
        }
        n
    });
    let delay = ast.retry_delay.as_deref().and_then(|v| {
        let n = whole(v).and_then(|s| s.checked_mul(1000));
        if n.is_none() {
            warnings.push(format!(
                "Unsupported curl flag: --retry-delay {v} (expected whole seconds); ignored"
            ));
        }
        n
    });
    match (count, delay) {
        (Some(count), delay_ms) => Some(RetryIr { count, delay_ms }),
        (None, Some(_)) => {
            warnings.push("--retry-delay has no effect without --retry; ignored".into());
            None
        }
        (None, None) => None,
    }
}

fn non_empty(s: &Option<String>) -> Option<&str> {
    s.as_deref().filter(|s| !s.is_empty())
}

fn truthy(s: Option<&String>) -> bool {
    s.is_some_and(|s| !s.is_empty())
}

fn parse_url(raw: &str) -> Result<(String, JsMap<String>), NormalizeError> {
    let mut params = JsMap::new();
    let Some(q) = raw.find('?') else {
        return Ok((raw.to_string(), params));
    };
    let (base, query) = (&raw[..q], &raw[q + 1..]);
    for pair in query.split('&') {
        match pair.find('=').filter(|e| *e > 0) {
            Some(eq) => {
                let key = decode_uri_component(&pair[..eq])?;
                let val = decode_uri_component(&pair[eq + 1..])?;
                params.insert(key, val);
            }
            None if !pair.is_empty() => {
                params.insert(decode_uri_component(pair)?, String::new());
            }
            None => {}
        }
    }
    Ok((base.to_string(), params))
}

fn normalize_headers(ast: &CurlAst) -> JsMap<String> {
    let mut headers = JsMap::new();
    for (k, v) in &ast.headers {
        if k.to_lowercase() == "authorization" {
            continue;
        }
        headers.insert(canonical_header_case(k), v.clone());
    }
    headers
}

/// `content-type` -> `Content-Type`.
fn canonical_header_case(h: &str) -> String {
    h.split('-')
        .map(|part| {
            let mut c = part.chars();
            match c.next() {
                Some(f) => f
                    .to_uppercase()
                    .chain(c.flat_map(char::to_lowercase))
                    .collect(),
                None => String::new(),
            }
        })
        .collect::<Vec<String>>()
        .join("-")
}

fn detect_auth(ast: &CurlAst, headers: &mut JsMap<String>) -> Option<Auth> {
    if let Some(user) = non_empty(&ast.user) {
        return Some(match user.find(':').filter(|c| *c > 0) {
            Some(c) => basic(&user[..c], &user[c + 1..]),
            None => basic(user, ""),
        });
    }

    for (k, v) in &ast.headers {
        if k.to_lowercase() != "authorization" {
            continue;
        }
        let value = trim(v);
        let lower = value.to_lowercase();
        if lower.starts_with("bearer ") {
            return Some(Auth {
                kind: "bearer".into(),
                username: None,
                password: None,
                token: Some(trim(value.get(7..).unwrap_or("")).to_string()),
            });
        }
        if lower.starts_with("basic ") {
            match atob(trim(value.get(6..).unwrap_or(""))) {
                Some(decoded) => {
                    if let Some(c) = decoded.find(':').filter(|c| *c > 0) {
                        return Some(basic(&decoded[..c], &decoded[c + 1..]));
                    }
                }
                None => headers.insert("Authorization", value.to_string()),
            }
        } else {
            headers.insert("Authorization", value.to_string());
        }
    }
    None
}

fn basic(user: &str, pass: &str) -> Auth {
    Auth {
        kind: "basic".into(),
        username: Some(user.to_string()),
        password: Some(pass.to_string()),
        token: None,
    }
}

fn detect_body(ast: &CurlAst) -> (Option<Body>, Option<JsMap<FormField>>) {
    if !ast.form.is_empty() {
        let mut form = JsMap::new();
        for entry in &ast.form {
            parse_form_entry(entry, &mut form);
        }
        for entry in &ast.form_string {
            if let Some(eq) = entry.find('=').filter(|e| *e > 0) {
                form.insert(&entry[..eq], FormField::Text(entry[eq + 1..].to_string()));
            }
        }
        return (None, Some(form));
    }

    if !ast.data_urlencode.is_empty() {
        let parts: Vec<String> = ast
            .data_urlencode
            .iter()
            .map(|d| encode_uri_component(d))
            .collect();
        return (Some(Body::Urlencoded(parts.join("&"))), None);
    }

    if let Some(first) = ast.data_binary.first() {
        if first.starts_with('@') {
            return (Some(Body::Binary(first.clone())), None);
        }
        return (Some(Body::Raw(ast.data_binary.concat())), None);
    }

    let all: Vec<&str> = ast
        .data
        .iter()
        .chain(&ast.data_raw)
        .map(String::as_str)
        .collect();
    if !all.is_empty() {
        let combined = all.join("&");
        if let Some(json) = jsjson::parse(&combined) {
            return (Some(Body::Json(json)), None);
        }
        if looks_like_form_urlencoded(&combined) {
            return (Some(Body::Urlencoded(combined)), None);
        }
        return (Some(Body::Raw(combined)), None);
    }
    (None, None)
}

fn parse_form_entry(entry: &str, form: &mut JsMap<FormField>) {
    let Some(eq) = entry.find('=').filter(|e| *e > 0) else {
        return;
    };
    let key = &entry[..eq];
    let value = &entry[eq + 1..];

    let Some(spec) = value.strip_prefix('@') else {
        form.insert(key, FormField::Text(value.to_string()));
        return;
    };
    let mut parts = spec.split(';');
    let mut file = FormFile {
        file: parts.next().unwrap_or("").to_string(),
        filename: None,
        content_type: None,
        type_first: false,
    };
    let mut assigned_any = false;
    for part in parts {
        // JS `const [k, v] = part.split('=')`: only the first two pieces.
        let mut kv = part.split('=');
        let k = kv.next().unwrap_or("");
        let v = kv.next().map(str::to_string);
        match k {
            "filename" => {
                file.filename = v;
                assigned_any = true;
            }
            "type" => {
                file.content_type = v;
                if !assigned_any {
                    file.type_first = true;
                }
                assigned_any = true;
            }
            _ => {}
        }
    }
    form.insert(key, FormField::File(file));
}

/// `/^[^=&]+=[^&]*(&[^=&]+=[^&]*)*$/`.
fn looks_like_form_urlencoded(s: &str) -> bool {
    s.split('&').all(|seg| seg.find('=').is_some_and(|e| e > 0))
}

fn infer_method(ast: &CurlAst, has_body: bool, has_form: bool) -> String {
    if let Some(m) = non_empty(&ast.method) {
        return m.to_string();
    }
    if ast.head {
        return "HEAD".into();
    }
    if ast.get {
        return "GET".into();
    }
    if has_body || has_form {
        return "POST".into();
    }
    "GET".into()
}

fn detect_ssl(ast: &CurlAst) -> Option<Ssl> {
    let (ca, cert, key) = (
        non_empty(&ast.cacert),
        non_empty(&ast.cert),
        non_empty(&ast.key),
    );
    if ca.is_none() && cert.is_none() && key.is_none() {
        return None;
    }
    Some(Ssl {
        ca: ca.map(str::to_string),
        cert: cert.map(str::to_string),
        key: key.map(str::to_string),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::convert::curl_parser::parse_curl;

    fn n(s: &str) -> CurlRunnerIr {
        normalize(&parse_curl(s)).unwrap()
    }

    #[test]
    fn simple_get_preserves_url() {
        let ir = n("curl https://example.com");
        assert_eq!(ir.method, "GET");
        assert_eq!(ir.url, "https://example.com");
        assert_eq!(ir.source, Source::Curl);
    }

    #[test]
    fn method_inference() {
        assert_eq!(n("curl -d \"data\" https://example.com").method, "POST");
        assert_eq!(
            n("curl -X PUT -d \"data\" https://example.com").method,
            "PUT"
        );
        assert_eq!(n("curl -G -d \"q=test\" https://example.com").method, "GET");
        assert_eq!(n("curl -I https://example.com").method, "HEAD");
    }

    #[test]
    fn extracts_query_params_from_url() {
        let ir = n("curl https://example.com/api?key=val&page=2");
        assert_eq!(ir.url, "https://example.com/api");
        let p = ir.params.unwrap();
        assert_eq!(p.get("key").map(String::as_str), Some("val"));
        assert_eq!(p.get("page").map(String::as_str), Some("2"));
    }

    #[test]
    fn malformed_escape_is_an_error() {
        assert!(normalize(&parse_curl("curl https://a.com/?a=%E0%A4%A")).is_err());
    }

    #[test]
    fn body_detection() {
        let ir = n("curl -d '{\"name\":\"Alex\",\"age\":30}' https://example.com");
        assert_eq!(
            ir.body,
            Some(Body::Json(serde_json::json!({"name": "Alex", "age": 30})))
        );
        assert_eq!(
            n("curl -d \"name=Alex&age=30\" https://example.com").body,
            Some(Body::Urlencoded("name=Alex&age=30".into()))
        );
        assert!(matches!(
            n("curl -d \"hello world\" https://example.com").body,
            Some(Body::Raw(_))
        ));
    }

    #[test]
    fn auth_detection() {
        let a = n("curl -u admin:secret https://example.com").auth.unwrap();
        assert_eq!(
            (
                a.kind.as_str(),
                a.username.as_deref(),
                a.password.as_deref()
            ),
            ("basic", Some("admin"), Some("secret"))
        );
        let b = n("curl -H \"Authorization: Bearer tok123\" https://example.com")
            .auth
            .unwrap();
        assert_eq!(
            (b.kind.as_str(), b.token.as_deref()),
            ("bearer", Some("tok123"))
        );
    }

    #[test]
    fn normalizes_header_casing() {
        let ir = n("curl -H \"content-type: text/plain\" https://example.com");
        assert_eq!(
            ir.headers.get("Content-Type").map(String::as_str),
            Some("text/plain")
        );
    }

    #[test]
    fn form_data_with_file_and_metadata() {
        let ir = n("curl -F \"file=@test.png\" -F \"name=test\" https://example.com");
        let f = ir.form_data.unwrap();
        assert_eq!(
            f.get("file"),
            Some(&FormField::File(FormFile {
                file: "test.png".into(),
                filename: None,
                content_type: None,
                type_first: false
            }))
        );
        assert_eq!(f.get("name"), Some(&FormField::Text("test".into())));

        let ir =
            n("curl -F \"file=@test.png;filename=photo.png;type=image/png\" https://example.com");
        let Some(FormField::File(file)) = ir.form_data.unwrap().get("file").cloned() else {
            panic!("expected a file field");
        };
        assert_eq!(file.filename.as_deref(), Some("photo.png"));
        assert_eq!(file.content_type.as_deref(), Some("image/png"));
    }

    #[test]
    fn flags_map_to_fields() {
        assert!(n("curl -k https://example.com").insecure);
        let r = n("curl -L --max-redirs 5 https://example.com");
        assert_eq!(
            (r.follow_redirects, r.max_redirects),
            (Some(true), Some(NumText::Num(5.0)))
        );
        assert_eq!(
            n("curl --max-time 30 https://example.com").timeout,
            Some(30000.0)
        );
        assert_eq!(
            n("curl -x http://proxy:8080 https://example.com")
                .proxy
                .as_deref(),
            Some("http://proxy:8080")
        );
        assert!(n("curl --http2 https://example.com").http2);
        let s = n("curl --cacert ca.pem --cert client.pem --key key.pem https://example.com")
            .ssl
            .unwrap();
        assert_eq!(
            (s.ca.as_deref(), s.cert.as_deref(), s.key.as_deref()),
            (Some("ca.pem"), Some("client.pem"), Some("key.pem"))
        );
    }

    #[test]
    fn warnings_and_header_flags() {
        let ir = n("curl --compressed https://example.com");
        assert!(ir.warnings.iter().any(|w| w.contains("compressed")));
        assert_eq!(
            n("curl -A \"MyApp/1.0\" https://example.com")
                .headers
                .get("User-Agent")
                .map(String::as_str),
            Some("MyApp/1.0")
        );
        assert_eq!(
            n("curl -e \"https://ref.com\" https://example.com")
                .headers
                .get("Referer")
                .map(String::as_str),
            Some("https://ref.com")
        );
        let ir = n("curl -b \"session=abc\" https://example.com");
        assert_eq!(
            ir.headers.get("Cookie").map(String::as_str),
            Some("session=abc")
        );
        assert!(ir.warnings.iter().any(|w| w.contains("Cookie")));
    }
}
