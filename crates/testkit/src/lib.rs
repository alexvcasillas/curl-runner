//! Shared test helpers: a local HTTP server exposing deterministic endpoints.
//!
//! Endpoints: `/echo`, `/status/{code}`, `/delay/{ms}`, `/redirect/{n}`,
//! `/multipart`, `/headers/multi`, `/binary`, `/flaky/{id}/{failures}`,
//! `/json/{fixture}`.
//!
//! Example-suite mocks (used by the parity harness, `parity/`): `/jp/...`
//! mimics jsonplaceholder (`posts`, `users`, `comments`, `albums`, `todos`),
//! `/httpbin/{get,post,headers}` mimics httpbin, and `/api/test/...` mimics the
//! local example API (`auth/basic`, `auth/bearer`, `headers`, `ip`,
//! `user-agent` (a `curl/<ver>` UA is echoed as `curl`), `post`, `get-only`, `status/{code}`, `delay/{ms}`,
//! `basic-auth/{user}/{pass}`).

mod http;

use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

use serde_json::{Map, Value, json};

pub use http::{Request, Response};

type Hits = Arc<Mutex<HashMap<String, u32>>>;

/// A background HTTP server bound to 127.0.0.1. Stops and releases its port on drop.
pub struct TestServer {
    pub base_url: String,
    listener: http::Listener,
}

impl TestServer {
    /// Start on a random free port.
    pub fn start() -> TestServer {
        Self::start_on(0)
    }

    /// Start on a specific port (0 = random).
    pub fn start_on(port: u16) -> TestServer {
        let hits: Hits = Arc::default();
        let listener = http::Listener::start(port, move |req| handle(&req, &hits));
        TestServer {
            base_url: format!("http://127.0.0.1:{}", listener.port),
            listener,
        }
    }
}

impl TestServer {
    /// Start on a random port with a custom handler (for tests that need
    /// their own routes, e.g. a fake npm registry or GitHub releases).
    pub fn with_handler<H>(handler: H) -> TestServer
    where
        H: Fn(Request) -> Response + Send + Sync + 'static,
    {
        let listener = http::Listener::start(0, handler);
        TestServer {
            base_url: format!("http://127.0.0.1:{}", listener.port),
            listener,
        }
    }
}

impl Drop for TestServer {
    fn drop(&mut self) {
        self.listener.stop();
    }
}

fn json_response(status: u16, v: &Value) -> Response {
    Response::from_data(v.to_string().into_bytes())
        .with_status_code(status)
        .with_header("Content-Type", "application/json")
}

fn bad_request(msg: &str) -> Response {
    json_response(400, &json!({ "error": msg }))
}

fn handle(req: &Request, hits: &Hits) -> Response {
    let method = req.method.clone();
    let url = req.url.clone();
    let (path, query_str) = match url.split_once('?') {
        Some((p, q)) => (p.to_string(), q.to_string()),
        None => (url.clone(), String::new()),
    };
    let body = &req.body;
    let segs: Vec<&str> = path.trim_matches('/').split('/').collect();

    match segs.as_slice() {
        ["echo"] => {
            let mut headers = Map::new();
            for (k, v) in &req.headers {
                headers.insert(k.to_ascii_lowercase(), Value::String(v.clone()));
            }
            let mut query = Map::new();
            for (k, v) in url::form_urlencoded::parse(query_str.as_bytes()) {
                query.insert(k.into_owned(), Value::String(v.into_owned()));
            }
            json_response(
                200,
                &json!({
                    "method": method, "path": path, "query": query,
                    "headers": headers, "body": parse_body(body),
                }),
            )
        }
        ["status", code] => match code.parse::<u16>() {
            Ok(c) if (100..=599).contains(&c) => json_response(c, &json!({ "status": c })),
            _ => bad_request("bad status"),
        },
        ["delay", ms] => match ms.parse::<u64>() {
            Ok(ms) => {
                thread::sleep(Duration::from_millis(ms));
                json_response(200, &json!({ "delayed": ms }))
            }
            Err(_) => bad_request("bad delay"),
        },
        ["redirect", n] => match n.parse::<u32>() {
            Err(_) => bad_request("bad redirect count"),
            Ok(n) => {
                let loc = if n <= 1 {
                    "/echo".to_string()
                } else {
                    format!("/redirect/{}", n - 1)
                };
                Response::from_data(Vec::new())
                    .with_status_code(302)
                    .with_header("Location", &loc)
            }
        },
        ["multipart"] => {
            let ct = req.header("content-type").unwrap_or_default();
            json_response(200, &Value::Array(parse_multipart(ct, body)))
        }
        ["headers", "multi"] => json_response(200, &json!({ "ok": true }))
            .with_header("Set-Cookie", "a=1; Path=/")
            .with_header("Set-Cookie", "b=2; Path=/"),
        ["binary"] => Response::from_data((0..=255u8).collect::<Vec<_>>())
            .with_header("Content-Type", "application/octet-stream"),
        ["flaky", id, failures] => {
            let Ok(failures) = failures.parse::<u32>() else {
                return bad_request("bad failures");
            };
            let n = {
                let mut m = hits.lock().unwrap();
                let c = m.entry((*id).to_string()).or_insert(0);
                *c += 1;
                *c
            };
            if n <= failures {
                json_response(503, &json!({ "error": "flaky", "attempt": n }))
            } else {
                json_response(200, &json!({ "ok": true, "attempt": n }))
            }
        }
        ["json", fixture] => match fixture_body(fixture) {
            Some(v) => json_response(200, &v),
            None => json_response(404, &json!({ "error": "unknown fixture" })),
        },
        ["jp", rest @ ..] => jp_route(&method, rest, body),
        ["httpbin", what @ ("get" | "post" | "headers")] => {
            let mut headers = Map::new();
            for (k, v) in &req.headers {
                headers.insert(canonical_header(k), Value::String(mock_header_value(k, v)));
            }
            json_response(
                200,
                &json!({ "headers": headers, "json": parse_body(body), "origin": "127.0.0.1",
                         "path": format!("/{what}") }),
            )
        }
        ["api", "test", rest @ ..] => api_test_route(req, &method, rest, body),
        _ => json_response(404, &json!({ "error": "not found" })),
    }
}

/// Body is parsed as JSON when valid, otherwise returned as a plain string
/// (empty body yields an empty string).
fn parse_body(body: &[u8]) -> Value {
    let s = String::from_utf8_lossy(body).into_owned();
    serde_json::from_str(&s).unwrap_or(Value::String(s))
}

fn fixture_body(name: &str) -> Option<Value> {
    Some(match name {
        "user" => {
            json!({ "id": 1, "name": "Test User", "email": "test@example.com", "active": true })
        }
        "users" => json!([{ "id": 1, "name": "Test User" }, { "id": 2, "name": "Other User" }]),
        "nested" => json!({ "data": { "items": [{ "id": 1 }, { "id": 2 }], "total": 2 } }),
        "empty" => json!({}),
        _ => return None,
    })
}

fn find(hay: &[u8], needle: &[u8], from: usize) -> Option<usize> {
    if needle.is_empty() || hay.len() < needle.len() || from > hay.len() - needle.len() {
        return None;
    }
    (from..=hay.len() - needle.len()).find(|&i| &hay[i..i + needle.len()] == needle)
}

fn parse_multipart(content_type: &str, body: &[u8]) -> Vec<Value> {
    let Some(boundary) = content_type
        .split(';')
        .filter_map(|p| p.trim().strip_prefix("boundary="))
        .next()
        .map(|b| b.trim_matches('"').to_string())
    else {
        return Vec::new();
    };
    let delim = format!("--{boundary}").into_bytes();
    let mut parts = Vec::new();
    let mut pos = match find(body, &delim, 0) {
        Some(p) => p + delim.len(),
        None => return parts,
    };
    while !body[pos..].starts_with(b"--") {
        let start = pos + 2; // skip CRLF after boundary
        let Some(hend) = find(body, b"\r\n\r\n", start) else {
            break;
        };
        let head = String::from_utf8_lossy(&body[start..hend]).into_owned();
        let data_start = hend + 4;
        let Some(next) = find(body, &delim, data_start) else {
            break;
        };
        let size = next.saturating_sub(2).saturating_sub(data_start); // strip CRLF before delimiter
        let (mut name, mut filename, mut ctype) = (Value::Null, Value::Null, Value::Null);
        for line in head.lines() {
            let lower = line.to_ascii_lowercase();
            if lower.starts_with("content-disposition:") {
                name = disp_param(line, "name").map_or(Value::Null, Value::String);
                filename = disp_param(line, "filename").map_or(Value::Null, Value::String);
            } else if lower.starts_with("content-type:") {
                ctype = Value::String(line["content-type:".len()..].trim().to_string());
            }
        }
        parts.push(
            json!({ "name": name, "filename": filename, "contentType": ctype, "size": size }),
        );
        pos = next + delim.len();
    }
    parts
}

fn disp_param(line: &str, key: &str) -> Option<String> {
    let pat = format!("{key}=\"");
    let mut from = 0;
    while let Some(i) = line[from..].find(&pat) {
        let at = from + i;
        // avoid matching `filename="` when looking for `name="`
        if at == 0 || matches!(line.as_bytes()[at - 1], b' ' | b';') {
            let rest = &line[at + pat.len()..];
            return rest.find('"').map(|e| rest[..e].to_string());
        }
        from = at + pat.len();
    }
    None
}

/// `curl/8.7.1` and `curl/8.22.0-DEV` both echo as `curl`, so the mock
/// endpoints give the same bytes for any curl build (parity harness).
fn mock_header_value(name: &str, value: &str) -> String {
    if name.eq_ignore_ascii_case("user-agent") && value.starts_with("curl/") {
        "curl".to_string()
    } else {
        value.to_string()
    }
}

fn canonical_header(name: &str) -> String {
    name.split('-')
        .map(|p| {
            let mut c = p.chars();
            c.next()
                .map(|f| f.to_ascii_uppercase().to_string() + &c.as_str().to_ascii_lowercase())
                .unwrap_or_default()
        })
        .collect::<Vec<_>>()
        .join("-")
}

fn header_value(req: &Request, name: &str) -> Option<String> {
    req.header(name).map(str::to_string)
}

fn api_test_route(req: &Request, method: &str, rest: &[&str], body: &[u8]) -> Response {
    match rest {
        ["auth", "basic"] => match header_value(req, "authorization") {
            Some(a) if a.starts_with("Basic ") => {
                json_response(200, &json!({ "authenticated": true }))
            }
            _ => json_response(401, &json!({ "authenticated": false })),
        },
        ["auth", "bearer"] => match header_value(req, "authorization") {
            Some(a) if a.starts_with("Bearer ") => {
                json_response(200, &json!({ "authenticated": true }))
            }
            _ => json_response(401, &json!({ "authenticated": false })),
        },
        ["basic-auth", user, pass] => {
            let want = format!("{user}:{pass}");
            let ok = header_value(req, "authorization")
                .and_then(|a| a.strip_prefix("Basic ").map(str::to_string))
                .is_some_and(|b| b64_decode(&b).as_deref() == Some(want.as_str()));
            if ok {
                json_response(200, &json!({ "authenticated": true, "user": user }))
            } else {
                json_response(401, &json!({ "authenticated": false }))
            }
        }
        ["headers"] => {
            let mut headers = Map::new();
            for (k, v) in &req.headers {
                headers.insert(
                    k.to_ascii_lowercase(),
                    Value::String(mock_header_value(k, v)),
                );
            }
            json_response(200, &json!({ "headers": headers }))
        }
        ["ip"] => json_response(200, &json!({ "origin": "127.0.0.1" })),
        ["user-agent"] => json_response(
            200,
            &json!({ "user-agent": mock_header_value("user-agent", &header_value(req, "user-agent").unwrap_or_default()) }),
        ),
        ["post"] => json_response(200, &json!({ "method": method, "json": parse_body(body) })),
        ["get-only"] if method == "GET" => json_response(200, &json!({ "ok": true })),
        ["get-only"] => json_response(405, &json!({ "error": "method not allowed" })),
        ["status", code] => match code.parse::<u16>() {
            Ok(c) if (100..=599).contains(&c) => json_response(c, &json!({ "status": c })),
            _ => bad_request("bad status"),
        },
        ["delay", ms] => match ms.parse::<u64>() {
            Ok(ms) => {
                thread::sleep(Duration::from_millis(ms));
                json_response(200, &json!({ "delayed": ms }))
            }
            Err(_) => bad_request("bad delay"),
        },
        _ => json_response(404, &json!({ "error": "not found" })),
    }
}

fn b64_decode(s: &str) -> Option<String> {
    let val = |c: u8| match c {
        b'A'..=b'Z' => Some(c - b'A'),
        b'a'..=b'z' => Some(c - b'a' + 26),
        b'0'..=b'9' => Some(c - b'0' + 52),
        b'+' => Some(62),
        b'/' => Some(63),
        _ => None,
    };
    let mut out = Vec::new();
    let (mut acc, mut bits) = (0u32, 0u32);
    for c in s.bytes().take_while(|&c| c != b'=') {
        acc = (acc << 6) | u32::from(val(c)?);
        bits += 6;
        if bits >= 8 {
            bits -= 8;
            out.push((acc >> bits) as u8);
            acc &= (1 << bits) - 1;
        }
    }
    String::from_utf8(out).ok()
}

const USERNAMES: [&str; 10] = [
    "Bret",
    "Antonette",
    "Samantha",
    "Karianne",
    "Kamren",
    "Leopoldo_Corkery",
    "Elwyn.Skiles",
    "Maxime_Nienow",
    "Delphine",
    "Moriah.Stanton",
];

fn jp_post(id: u64) -> Value {
    json!({ "userId": (id - 1) / 10 + 1, "id": id, "title": format!("post title {id}"),
            "body": format!("post body {id}") })
}

fn jp_user(id: u64) -> Value {
    let name = USERNAMES[(id as usize - 1) % 10];
    json!({ "id": id, "name": format!("User {id}"), "username": name,
            "email": format!("user{id}@example.com") })
}

/// jsonplaceholder look-alike: 100 posts, 10 users.
fn jp_route(method: &str, rest: &[&str], body: &[u8]) -> Response {
    let id_of = |s: &str| s.parse::<u64>().ok().filter(|n| *n >= 1);
    let not_found = || json_response(404, &json!({}));
    let sent = parse_body(body);
    let created = |id: u64| {
        let mut m = sent.as_object().cloned().unwrap_or_default();
        m.insert("id".into(), json!(id));
        Value::Object(m)
    };
    match (method, rest) {
        ("GET", ["posts"]) => json_response(200, &Value::Array((1..=100).map(jp_post).collect())),
        ("POST", ["posts"]) => json_response(201, &created(101)),
        ("GET", ["posts", id]) => match id_of(id).filter(|n| *n <= 100) {
            Some(n) => json_response(200, &jp_post(n)),
            None => not_found(),
        },
        ("PUT" | "PATCH", ["posts", id]) => match id_of(id).filter(|n| *n <= 100) {
            Some(n) => json_response(200, &created(n)),
            None => not_found(),
        },
        ("DELETE", ["posts", id]) if id_of(id).is_some_and(|n| n <= 100) => {
            json_response(200, &json!({}))
        }
        ("GET", ["posts", id, "comments"]) if id_of(id).is_some_and(|n| n <= 100) => {
            json_response(
                200,
                &Value::Array(
                    (1..=5)
                        .map(|i| json!({ "postId": id_of(id), "id": i, "email": format!("c{i}@example.com") }))
                        .collect(),
                ),
            )
        }
        ("GET", ["comments"]) => json_response(
            200,
            &Value::Array((1..=5).map(|i| json!({ "postId": 1, "id": i })).collect()),
        ),
        ("GET", ["users"]) => json_response(200, &Value::Array((1..=10).map(jp_user).collect())),
        ("POST", ["users"]) => json_response(201, &created(11)),
        ("GET", ["users", id]) => match id_of(id).filter(|n| *n <= 10) {
            Some(n) => json_response(200, &jp_user(n)),
            None => not_found(),
        },
        ("GET", ["users", id, kind @ ("albums" | "todos")]) if id_of(id).is_some_and(|n| n <= 10) => {
            json_response(
                200,
                &Value::Array(
                    (1..=3)
                        .map(|i| json!({ "userId": id_of(id), "id": i, "title": format!("{kind} {i}") }))
                        .collect(),
                ),
            )
        }
        _ => not_found(),
    }
}
