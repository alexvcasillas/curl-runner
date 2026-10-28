use curl::easy::{Easy, List};
use serde_json::Value;
use testkit::TestServer;

struct Resp {
    status: u32,
    headers: Vec<String>,
    body: Vec<u8>,
}

fn request(url: &str, method: &str, headers: &[&str], body: Option<&[u8]>) -> Resp {
    let mut easy = Easy::new();
    easy.url(url).unwrap();
    easy.custom_request(method).unwrap();
    let mut list = List::new();
    for h in headers {
        list.append(h).unwrap();
    }
    easy.http_headers(list).unwrap();
    if let Some(b) = body {
        easy.post_fields_copy(b).unwrap();
    }
    let mut out = Vec::new();
    let mut hdrs = Vec::new();
    {
        let mut t = easy.transfer();
        t.write_function(|d| {
            out.extend_from_slice(d);
            Ok(d.len())
        })
        .unwrap();
        t.header_function(|d| {
            hdrs.push(String::from_utf8_lossy(d).trim().to_string());
            true
        })
        .unwrap();
        t.perform().unwrap();
    }
    Resp {
        status: easy.response_code().unwrap(),
        headers: hdrs,
        body: out,
    }
}

fn get(url: &str) -> Resp {
    request(url, "GET", &[], None)
}

fn json(r: &Resp) -> Value {
    serde_json::from_slice(&r.body).unwrap()
}

#[test]
fn echo_returns_request_details() {
    let s = TestServer::start();
    let r = request(
        &format!("{}/echo?a=1&b=two", s.base_url),
        "POST",
        &["X-Custom: Yes", "Content-Type: application/json"],
        Some(br#"{"k":1}"#),
    );
    assert_eq!(r.status, 200);
    let v = json(&r);
    assert_eq!(v["method"], "POST");
    assert_eq!(v["path"], "/echo");
    assert_eq!(v["query"]["a"], "1");
    assert_eq!(v["query"]["b"], "two");
    assert_eq!(v["headers"]["x-custom"], "Yes");
    assert_eq!(v["body"]["k"], 1);
}

#[test]
fn echo_non_json_body_is_string() {
    let s = TestServer::start();
    let r = request(&format!("{}/echo", s.base_url), "PUT", &[], Some(b"hello"));
    assert_eq!(json(&r)["body"], "hello");
    assert_eq!(json(&r)["method"], "PUT");
}

#[test]
fn echo_supports_all_methods() {
    let s = TestServer::start();
    for m in ["GET", "POST", "PUT", "PATCH", "DELETE"] {
        let r = request(&format!("{}/echo", s.base_url), m, &[], None);
        assert_eq!(r.status, 200, "{m}");
        assert_eq!(json(&r)["method"], m);
    }
}

#[test]
fn status_returns_code() {
    let s = TestServer::start();
    let r = get(&format!("{}/status/404", s.base_url));
    assert_eq!(r.status, 404);
    assert_eq!(json(&r)["status"], 404);
    assert_eq!(get(&format!("{}/status/201", s.base_url)).status, 201);
}

#[test]
fn delay_sleeps() {
    let s = TestServer::start();
    let t = std::time::Instant::now();
    let r = get(&format!("{}/delay/200", s.base_url));
    assert_eq!(r.status, 200);
    assert!(t.elapsed().as_millis() >= 200);
}

#[test]
fn concurrent_delays_run_in_parallel() {
    let s = TestServer::start();
    let t = std::time::Instant::now();
    let hs: Vec<_> = (0..6)
        .map(|_| {
            let u = format!("{}/delay/300", s.base_url);
            std::thread::spawn(move || get(&u).status)
        })
        .collect();
    for h in hs {
        assert_eq!(h.join().unwrap(), 200);
    }
    assert!(t.elapsed().as_millis() < 1200);
}

#[test]
fn redirect_chain_ends_at_echo() {
    let s = TestServer::start();
    let mut easy = Easy::new();
    easy.url(&format!("{}/redirect/3", s.base_url)).unwrap();
    easy.follow_location(true).unwrap();
    let mut out = Vec::new();
    {
        let mut t = easy.transfer();
        t.write_function(|d| {
            out.extend_from_slice(d);
            Ok(d.len())
        })
        .unwrap();
        t.perform().unwrap();
    }
    assert_eq!(easy.response_code().unwrap(), 200);
    assert_eq!(easy.redirect_count().unwrap(), 3);
    let v: Value = serde_json::from_slice(&out).unwrap();
    assert_eq!(v["path"], "/echo");
}

#[test]
fn redirect_is_302_with_location() {
    let s = TestServer::start();
    let r = get(&format!("{}/redirect/2", s.base_url));
    assert_eq!(r.status, 302);
    assert!(
        r.headers
            .iter()
            .any(|h| h.eq_ignore_ascii_case("location: /redirect/1"))
    );
}

#[test]
fn multipart_lists_parts() {
    let s = TestServer::start();
    let mut easy = Easy::new();
    easy.url(&format!("{}/multipart", s.base_url)).unwrap();
    let mut form = curl::easy::Form::new();
    form.part("field").contents(b"abc").add().unwrap();
    form.part("file")
        .buffer("a.txt", b"hello world".to_vec())
        .content_type("text/plain")
        .add()
        .unwrap();
    easy.httppost(form).unwrap();
    let mut out = Vec::new();
    {
        let mut t = easy.transfer();
        t.write_function(|d| {
            out.extend_from_slice(d);
            Ok(d.len())
        })
        .unwrap();
        t.perform().unwrap();
    }
    assert_eq!(easy.response_code().unwrap(), 200);
    let v: Value = serde_json::from_slice(&out).unwrap();
    let parts = v.as_array().unwrap();
    assert_eq!(parts.len(), 2);
    assert_eq!(parts[0]["name"], "field");
    assert_eq!(parts[0]["size"], 3);
    assert_eq!(parts[1]["name"], "file");
    assert_eq!(parts[1]["filename"], "a.txt");
    assert_eq!(parts[1]["contentType"], "text/plain");
    assert_eq!(parts[1]["size"], 11);
}

#[test]
fn multi_set_cookie() {
    let s = TestServer::start();
    let r = get(&format!("{}/headers/multi", s.base_url));
    let n = r
        .headers
        .iter()
        .filter(|h| h.to_ascii_lowercase().starts_with("set-cookie:"))
        .count();
    assert_eq!(n, 2);
}

#[test]
fn binary_is_256_bytes() {
    let s = TestServer::start();
    let r = get(&format!("{}/binary", s.base_url));
    assert_eq!(r.status, 200);
    assert!(
        r.headers
            .iter()
            .any(|h| h.eq_ignore_ascii_case("content-type: application/octet-stream"))
    );
    assert_eq!(r.body, (0..=255u8).collect::<Vec<_>>());
}

#[test]
fn flaky_returns_503_then_200() {
    let s = TestServer::start();
    let u = format!("{}/flaky/abc/2", s.base_url);
    assert_eq!(get(&u).status, 503);
    assert_eq!(get(&u).status, 503);
    assert_eq!(get(&u).status, 200);
    assert_eq!(get(&u).status, 200);
    // independent per id
    assert_eq!(get(&format!("{}/flaky/other/1", s.base_url)).status, 503);
}

#[test]
fn json_fixtures() {
    let s = TestServer::start();
    let r = get(&format!("{}/json/user", s.base_url));
    assert_eq!(r.status, 200);
    let v = json(&r);
    assert_eq!(v["id"], 1);
    assert_eq!(v["name"], "Test User");
    assert_eq!(get(&format!("{}/json/nope", s.base_url)).status, 404);
}

#[test]
fn unknown_path_is_404() {
    let s = TestServer::start();
    assert_eq!(get(&format!("{}/nope", s.base_url)).status, 404);
}

#[test]
fn drop_releases_port() {
    let s = TestServer::start();
    let url = s.base_url.clone();
    let port: u16 = url.rsplit(':').next().unwrap().parse().unwrap();
    drop(s);
    // Listener close may lag slightly; retry until connect is refused.
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(2);
    while std::net::TcpStream::connect(("127.0.0.1", port)).is_ok() {
        assert!(std::time::Instant::now() < deadline, "port still accepting");
        std::thread::sleep(std::time::Duration::from_millis(20));
    }
}

#[test]
fn malformed_params_return_400() {
    let s = TestServer::start();
    for p in [
        "status/abc",
        "status/99",
        "status/600",
        "status/0",
        "delay/abc",
        "redirect/abc",
        "flaky/x/abc",
    ] {
        assert_eq!(get(&format!("{}/{p}", s.base_url)).status, 400, "{p}");
    }
    assert_eq!(get(&format!("{}/status/599", s.base_url)).status, 599);
}

#[test]
fn jp_mock_mimics_jsonplaceholder() {
    let s = TestServer::start();
    let b = &s.base_url;
    let posts = json(&get(&format!("{b}/jp/posts")));
    assert_eq!(posts.as_array().unwrap().len(), 100);
    assert_eq!(posts[99]["id"], 100);
    let p = json(&get(&format!("{b}/jp/posts/1")));
    assert_eq!((p["id"].clone(), p["userId"].clone()), (1.into(), 1.into()));
    assert_eq!(get(&format!("{b}/jp/posts/999999")).status, 404);
    let r = request(
        &format!("{b}/jp/posts"),
        "POST",
        &[],
        Some(br#"{"title":"t"}"#),
    );
    assert_eq!(r.status, 201);
    assert_eq!(
        (json(&r)["id"].clone(), json(&r)["title"].clone()),
        (101.into(), "t".into())
    );
    assert_eq!(json(&get(&format!("{b}/jp/users/1")))["username"], "Bret");
    assert_eq!(
        json(&get(&format!("{b}/jp/users/2/todos")))
            .as_array()
            .unwrap()
            .len(),
        3
    );
    assert_eq!(
        json(&get(&format!("{b}/jp/posts/3/comments")))[0]["postId"],
        3
    );
}

#[test]
fn api_test_mock_endpoints() {
    let s = TestServer::start();
    let b = &s.base_url;
    assert_eq!(get(&format!("{b}/api/test/auth/basic")).status, 401);
    let r = request(
        &format!("{b}/api/test/basic-auth/user/pass"),
        "GET",
        &["Authorization: Basic dXNlcjpwYXNz"],
        None,
    );
    assert_eq!(r.status, 200);
    let r = request(
        &format!("{b}/api/test/auth/bearer"),
        "GET",
        &["Authorization: Bearer x"],
        None,
    );
    assert_eq!(r.status, 200);
    assert_eq!(get(&format!("{b}/api/test/status/500")).status, 500);
    assert_eq!(get(&format!("{b}/api/test/get-only")).status, 200);
    assert_eq!(
        request(&format!("{b}/api/test/get-only"), "POST", &[], Some(b"x")).status,
        405
    );
    let h = json(&request(
        &format!("{b}/api/test/headers"),
        "GET",
        &["X-A: 1"],
        None,
    ));
    assert_eq!(h["headers"]["x-a"], "1");
    let u = json(&request(
        &format!("{b}/api/test/user-agent"),
        "GET",
        &["User-Agent: curl/8.22.0-DEV"],
        None,
    ));
    assert_eq!(u["user-agent"], "curl");
    assert_eq!(
        json(&get(&format!("{b}/httpbin/get")))["origin"],
        "127.0.0.1"
    );
    let r = request(
        &format!("{b}/httpbin/post"),
        "POST",
        &["Content-Type: application/json"],
        Some(br#"{"a":1}"#),
    );
    assert_eq!(json(&r)["json"]["a"], 1);
}

/// Regression: tiny_http 0.12 served each keep-alive connection on a pool of
/// 4 threads and could lose the wake-up for a burst of new connections, so
/// with more than 4 idle keep-alive connections held open (libcurl's
/// connection cache) the extra requests were never answered (the parity
/// harness's intermittent "parallel hang"). A fresh server per round makes
/// the burst hit a cold pool; the watchdog fails instead of hanging.
#[test]
fn burst_of_keep_alive_connections_is_fully_served() {
    use std::io::{Read, Write};
    use std::net::TcpStream;
    use std::sync::{Arc, Barrier, mpsc};
    use std::time::Duration;

    const CONNS: usize = 8;
    for round in 0..20 {
        let s = TestServer::start();
        let addr = s.base_url.trim_start_matches("http://").to_string();
        let barrier = Arc::new(Barrier::new(CONNS));
        let (tx, rx) = mpsc::channel();
        let mut keep = Vec::new();
        for _ in 0..CONNS {
            let (addr, barrier, tx) = (addr.clone(), barrier.clone(), tx.clone());
            let (done_tx, done_rx) = mpsc::channel::<()>();
            keep.push(done_tx);
            std::thread::spawn(move || {
                let mut c = TcpStream::connect(&addr).unwrap();
                c.set_read_timeout(Some(Duration::from_secs(10))).unwrap();
                barrier.wait();
                c.write_all(b"GET /json/empty HTTP/1.1\r\nHost: x\r\n\r\n")
                    .unwrap();
                let mut buf = Vec::new();
                let mut chunk = [0u8; 512];
                while !buf.ends_with(b"\r\n\r\n{}") {
                    match c.read(&mut chunk) {
                        Ok(0) | Err(_) => break,
                        Ok(n) => buf.extend_from_slice(&chunk[..n]),
                    }
                }
                let _ = tx.send(buf.starts_with(b"HTTP/1.1 200"));
                // Keep the connection open (idle, keep-alive) until the round ends.
                let _ = done_rx.recv_timeout(Duration::from_secs(10));
            });
        }
        for i in 0..CONNS {
            let ok = rx
                .recv_timeout(Duration::from_secs(5))
                .unwrap_or_else(|_| panic!("round {round}: response {i} never arrived (hang)"));
            assert!(ok, "round {round}: bad response");
        }
        drop(keep);
    }
}
