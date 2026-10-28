//! Transport tests against the in-process `testkit::TestServer`.

use super::*;
use crate::model::{AuthConfig, ConnectionPoolConfig, FileAttachment, FormFieldValue, HeaderValue};
use indexmap::IndexMap;
use serde_json::json;
use std::path::PathBuf;
use testkit::TestServer;

fn req(url: String) -> RequestConfig {
    RequestConfig {
        url,
        ..Default::default()
    }
}

/// The filesystem root holding the temp dir (`/`, or `C:\` on Windows): an
/// absolute cwd that every tempdir path lies inside.
fn root() -> PathBuf {
    std::env::temp_dir()
        .ancestors()
        .last()
        .unwrap()
        .to_path_buf()
}

fn go(r: &RequestConfig) -> Result<Response, TransportError> {
    send(r, &GlobalConfig::default(), &root())
}

#[test]
fn get_echo_returns_status_headers_body_and_metrics() {
    let srv = TestServer::start();
    let r = RequestConfig {
        params: Some(IndexMap::from([("q".to_string(), "a b".to_string())])),
        headers: Some(IndexMap::from([("X-Test".to_string(), "1".to_string())])),
        ..req(format!("{}/echo", srv.base_url))
    };
    let resp = go(&r).unwrap();
    assert_eq!(resp.status, 200);
    assert_eq!(
        resp.headers["content-type"],
        HeaderValue::One("application/json".into())
    );
    assert_eq!(resp.header_history.len(), 1);
    assert_eq!(resp.body["method"], "GET");
    assert_eq!(resp.body["query"]["q"], "a b");
    assert_eq!(resp.body["headers"]["x-test"], "1");
    assert!(
        resp.body["headers"]["user-agent"]
            .as_str()
            .unwrap()
            .starts_with("curl/")
    );
    let m = &resp.metrics;
    assert!(m.duration > 0.0);
    assert_eq!(m.download, Some(m.duration));
    assert!(m.size.unwrap() > 0.0);
    assert!(m.first_byte.unwrap() <= m.duration);
}

#[test]
fn posts_json_body() {
    let srv = TestServer::start();
    let r = RequestConfig {
        method: Some("POST".into()),
        body: Some(json!({"b": 1, "a": [true, null]})),
        ..req(format!("{}/echo", srv.base_url))
    };
    let resp = go(&r).unwrap();
    assert_eq!(resp.body["method"], "POST");
    assert_eq!(resp.body["headers"]["content-type"], "application/json");
    assert_eq!(resp.body["body"], json!({"b": 1, "a": [true, null]}));
}

#[test]
fn posts_string_body_raw_and_keeps_user_content_type() {
    let srv = TestServer::start();
    let r = RequestConfig {
        method: Some("PUT".into()),
        headers: Some(IndexMap::from([(
            "Content-Type".to_string(),
            "text/plain".to_string(),
        )])),
        body: Some(json!("@/etc/passwd")),
        ..req(format!("{}/echo", srv.base_url))
    };
    let resp = go(&r).unwrap();
    assert_eq!(resp.body["method"], "PUT");
    assert_eq!(resp.body["headers"]["content-type"], "text/plain");
    assert_eq!(resp.body["body"], "@/etc/passwd");
}

#[test]
fn null_body_sends_nothing() {
    let srv = TestServer::start();
    let r = RequestConfig {
        method: Some("POST".into()),
        body: Some(Json::Null),
        ..req(format!("{}/echo", srv.base_url))
    };
    let resp = go(&r).unwrap();
    assert_eq!(resp.body["body"], "");
    assert!(resp.body["headers"].get("content-type").is_none());
}

#[test]
fn basic_and_bearer_auth() {
    let srv = TestServer::start();
    let basic = RequestConfig {
        auth: Some(AuthConfig {
            auth_type: "basic".into(),
            username: Some("user".into()),
            password: Some("pass".into()),
            token: None,
        }),
        ..req(format!("{}/echo", srv.base_url))
    };
    assert_eq!(
        go(&basic).unwrap().body["headers"]["authorization"],
        "Basic dXNlcjpwYXNz"
    );
    let bearer = RequestConfig {
        auth: Some(AuthConfig {
            auth_type: "bearer".into(),
            token: Some("abc".into()),
            ..Default::default()
        }),
        ..req(format!("{}/echo", srv.base_url))
    };
    assert_eq!(
        go(&bearer).unwrap().body["headers"]["authorization"],
        "Bearer abc"
    );
}

#[test]
fn multipart_upload_with_filename_and_type() {
    let srv = TestServer::start();
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("a.txt"), b"hello").unwrap();
    let form: IndexMap<String, FormFieldValue> = IndexMap::from([
        ("title".to_string(), FormFieldValue::Text("It's @x".into())),
        ("n".to_string(), FormFieldValue::Number(5.into())),
        (
            "doc".to_string(),
            FormFieldValue::File(FileAttachment {
                file: "a.txt".into(),
                filename: Some("custom.txt".into()),
                content_type: Some("text/x-custom".into()),
            }),
        ),
    ]);
    let r = RequestConfig {
        method: Some("POST".into()),
        form_data: Some(form),
        body: Some(json!({"ignored": true})),
        ..req(format!("{}/multipart", srv.base_url))
    };
    let resp = send(&r, &GlobalConfig::default(), dir.path()).unwrap();
    assert_eq!(
        resp.body,
        json!([
            {"name": "title", "filename": null, "contentType": null, "size": 7},
            {"name": "n", "filename": null, "contentType": null, "size": 1},
            {"name": "doc", "filename": "custom.txt", "contentType": "text/x-custom", "size": 5},
        ])
    );
}

#[test]
fn missing_form_file_is_an_error() {
    let r = RequestConfig {
        form_data: Some(IndexMap::from([(
            "doc".to_string(),
            FormFieldValue::File(FileAttachment {
                file: "nope.txt".into(),
                ..Default::default()
            }),
        )])),
        ..req("http://127.0.0.1:1/".into())
    };
    let err = build_easy(&r, &GlobalConfig::default(), &root())
        .err()
        .unwrap();
    assert_eq!(err.message, "File(s) not found: doc: nope.txt");
}

#[test]
fn repeated_headers_become_array() {
    let srv = TestServer::start();
    let resp = go(&req(format!("{}/headers/multi", srv.base_url))).unwrap();
    assert_eq!(
        resp.headers["set-cookie"],
        HeaderValue::Many(vec!["a=1; Path=/".into(), "b=2; Path=/".into()])
    );
}

#[test]
fn redirect_history_final_last() {
    let srv = TestServer::start();
    let resp = go(&req(format!("{}/redirect/2", srv.base_url))).unwrap();
    assert_eq!(resp.status, 200);
    let statuses: Vec<u16> = resp.header_history.iter().map(|b| b.status).collect();
    assert_eq!(statuses, [302, 302, 200]);
    assert_eq!(resp.headers, resp.header_history[2].headers);
    assert_eq!(resp.body["path"], "/echo");
}

#[test]
fn redirects_not_followed_when_disabled() {
    let srv = TestServer::start();
    let r = RequestConfig {
        follow_redirects: Some(false),
        ..req(format!("{}/redirect/2", srv.base_url))
    };
    let resp = go(&r).unwrap();
    assert_eq!(resp.status, 302);
    assert_eq!(resp.header_history.len(), 1);
    assert_eq!(
        resp.headers["location"],
        HeaderValue::One("/redirect/1".into())
    );
}

#[test]
fn max_redirects_exceeded_is_an_error() {
    let srv = TestServer::start();
    let r = RequestConfig {
        max_redirects: Some(1),
        ..req(format!("{}/redirect/3", srv.base_url))
    };
    let err = go(&r).unwrap_err();
    assert_eq!(err.code, Some(47));
    assert!(err.message.starts_with("curl: (47) "), "{}", err.message);
}

#[test]
fn binary_body_lossy_string() {
    let srv = TestServer::start();
    let resp = go(&req(format!("{}/binary", srv.base_url))).unwrap();
    assert_eq!(
        resp.body,
        Json::String(super::response::tests::lossy_0_to_255())
    );
    assert_eq!(resp.metrics.size, Some(256.0));
}

#[test]
fn status_codes_pass_through() {
    let srv = TestServer::start();
    let resp = go(&req(format!("{}/status/503", srv.base_url))).unwrap();
    assert_eq!(resp.status, 503);
    assert_eq!(resp.body, json!({"status": 503}));
}

#[test]
fn head_request_does_not_hang() {
    let srv = TestServer::start();
    let r = RequestConfig {
        method: Some("HEAD".into()),
        timeout: Some(2000),
        ..req(format!("{}/echo", srv.base_url))
    };
    let resp = go(&r).unwrap();
    assert_eq!(resp.status, 200);
    assert_eq!(resp.body, json!(""));
}

#[test]
fn timeout_is_milliseconds() {
    let srv = TestServer::start();
    let short = RequestConfig {
        timeout: Some(100),
        ..req(format!("{}/delay/500", srv.base_url))
    };
    let err = go(&short).unwrap_err();
    assert_eq!(err.code, Some(28));
    assert_eq!(err.message, "Request timeout after 100ms");

    let long = RequestConfig {
        timeout: Some(2000),
        ..req(format!("{}/delay/500", srv.base_url))
    };
    assert_eq!(go(&long).unwrap().status, 200);
}

#[test]
fn connection_refused_is_transport_error() {
    let err = go(&req("http://127.0.0.1:1/".into())).unwrap_err();
    assert_eq!(err.code, Some(7));
    assert!(err.message.starts_with("curl: (7) "), "{}", err.message);
}

#[test]
fn output_writes_body_to_file() {
    let srv = TestServer::start();
    let dir = tempfile::tempdir().unwrap();
    let out: PathBuf = dir.path().join("out.json");
    let r = RequestConfig {
        output: Some(out.to_string_lossy().into_owned()),
        ..req(format!("{}/json/user", srv.base_url))
    };
    let resp = go(&r).unwrap();
    assert_eq!(resp.body, json!(""));
    let saved: Json = serde_json::from_slice(&std::fs::read(&out).unwrap()).unwrap();
    assert_eq!(saved["id"], 1);
}

// Not performed: tiny_http mishandles the h2c `Upgrade` libcurl sends for
// HTTP/2 over cleartext (it stops reading the connection), so a live transfer
// here would hang. Option setting itself must succeed.
#[test]
fn connection_pool_and_http2_options_apply() {
    let g = GlobalConfig {
        connection_pool: Some(ConnectionPoolConfig {
            enabled: Some(true),
            keepalive_time: Some(10),
            connect_timeout: Some(5),
            ..Default::default()
        }),
        ..Default::default()
    };
    let r = RequestConfig {
        http2: Some(true),
        ..req("https://example.invalid/".into())
    };
    assert!(build_easy(&r, &g, &root()).is_ok());
}

#[test]
fn timeout_error_messages() {
    let e = curl::Error::new(28);
    let g = GlobalConfig::default();
    let timed = RequestConfig {
        timeout: Some(5000),
        ..req("http://x".into())
    };
    assert_eq!(
        TransportError::from_curl(&e, &timed, &g).message,
        "Request timeout after 5000ms"
    );
    assert_eq!(
        TransportError::from_curl(&e, &req("http://x".into()), &g).message,
        "curl: (28) Timeout was reached"
    );
}

#[test]
fn build_easy_then_finish_without_send() {
    let srv = TestServer::start();
    let r = req(format!("{}/json/users", srv.base_url));
    let multi = curl::multi::Multi::new();
    let h = multi
        .add2(build_easy(&r, &GlobalConfig::default(), &root()).unwrap())
        .unwrap();
    while multi.perform().unwrap() > 0 {
        multi
            .wait(&mut [], std::time::Duration::from_millis(50))
            .unwrap();
    }
    let resp = finish(multi.remove2(h).unwrap()).unwrap();
    assert_eq!(resp.status, 200);
    assert_eq!(resp.body[1]["name"], "Other User");
}

#[test]
fn empty_strings_are_absent() {
    let srv = TestServer::start();
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("a.txt"), b"hi").unwrap();
    let r = RequestConfig {
        method: Some("POST".into()),
        proxy: Some(String::new()),
        output: Some(String::new()),
        ssl: Some(crate::model::SslConfig {
            ca: Some(String::new()),
            cert: Some(String::new()),
            key: Some(String::new()),
            verify: None,
        }),
        form_data: Some(IndexMap::from([(
            "doc".to_string(),
            FormFieldValue::File(FileAttachment {
                file: "a.txt".into(),
                filename: Some(String::new()),
                content_type: Some(String::new()),
            }),
        )])),
        ..req(format!("{}/multipart", srv.base_url))
    };
    let resp = send(&r, &GlobalConfig::default(), dir.path()).unwrap();
    // Body stays in memory (no `-o ""`), filename defaults to the basename,
    // content type is libcurl's guess.
    assert_eq!(resp.body[0]["filename"], "a.txt");
    assert_eq!(resp.body[0]["contentType"], "text/plain");
    assert_eq!(resp.body[0]["size"], 2);

    let cmd = to_curl_command(&r, &GlobalConfig::default());
    for flag in [
        " -x ",
        " -o ",
        "--cacert",
        "--cert",
        "--key",
        "filename=",
        "type=",
    ] {
        assert!(!cmd.contains(flag), "{flag} in {cmd}");
    }
    assert!(cmd.contains("-F doc=@a.txt "), "{cmd}");
}

#[test]
fn output_file_creation_failure_with_empty_body_is_an_error() {
    let srv = TestServer::start();
    let dir = tempfile::tempdir().unwrap();
    let out = dir.path().join("missing-dir").join("out.txt");
    let r = RequestConfig {
        method: Some("HEAD".into()),
        output: Some(out.to_string_lossy().into_owned()),
        ..req(format!("{}/echo", srv.base_url))
    };
    let err = go(&r).unwrap_err();
    assert_eq!(err.code, Some(23));
    assert!(
        err.message
            .starts_with(&format!("Failed to write output file {}", out.display())),
        "{}",
        err.message
    );
}

#[test]
fn output_file_creation_failure_with_body_is_an_error() {
    let srv = TestServer::start();
    let dir = tempfile::tempdir().unwrap();
    let out = dir.path().join("missing-dir").join("out.txt");
    let r = RequestConfig {
        output: Some(out.to_string_lossy().into_owned()),
        ..req(format!("{}/echo", srv.base_url))
    };
    let err = go(&r).unwrap_err();
    assert_eq!(err.code, Some(23));
    assert!(
        err.message.starts_with("Failed to write output file "),
        "{}",
        err.message
    );
}

/// TCP listener that accepts but never speaks: an `https://` request to it
/// stalls in the TLS handshake, i.e. while libcurl is still CONNECTING.
fn silent_tls_url() -> (std::net::TcpListener, String) {
    let l = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let url = format!("https://{}/", l.local_addr().unwrap());
    (l, url)
}

fn pool(connect_timeout: u64) -> GlobalConfig {
    GlobalConfig {
        connection_pool: Some(ConnectionPoolConfig {
            enabled: Some(true),
            connect_timeout: Some(connect_timeout),
            ..Default::default()
        }),
        ..Default::default()
    }
}

#[test]
fn request_timeout_during_connect_reports_request_timeout() {
    let (_l, url) = silent_tls_url();
    let r = RequestConfig {
        timeout: Some(200),
        ..req(url)
    };
    let err = send(&r, &pool(30), &root()).unwrap_err();
    assert_eq!(err.message, "Request timeout after 200ms");
}

#[test]
fn connect_timeout_smaller_than_request_timeout_reports_connection_timeout() {
    let (_l, url) = silent_tls_url();
    let r = RequestConfig {
        timeout: Some(5000),
        ..req(url)
    };
    let err = send(&r, &pool(1), &root()).unwrap_err();
    assert_eq!(err.message, "Connection timeout after 1000ms");
}

/// Live HTTPS against the OS trust store (needs network): `cargo test -p
/// curl-runner-core https_uses_native_ca -- --ignored`.
#[test]
#[ignore = "needs network"]
fn https_uses_native_ca() {
    let resp = go(&req("https://example.com/".into())).unwrap();
    assert_eq!(resp.status, 200);
    assert!(resp.metrics.tls_handshake.unwrap() > 0.0);
}

// ---------------------------------------------------------------------------
// Security guards (Task 7)
// ---------------------------------------------------------------------------

use crate::model::{SecurityConfig, SslConfig};

fn sec(protocols: Option<&[&str]>, allow_paths: Option<bool>) -> GlobalConfig {
    GlobalConfig {
        security: Some(SecurityConfig {
            allowed_protocols: protocols.map(|p| p.iter().map(|s| s.to_string()).collect()),
            allow_paths,
        }),
        ..Default::default()
    }
}

fn file_form(field: &str, file: &str) -> Option<IndexMap<String, FormFieldValue>> {
    Some(IndexMap::from([(
        field.to_string(),
        FormFieldValue::File(FileAttachment {
            file: file.into(),
            ..Default::default()
        }),
    )]))
}

fn guard_msg(r: &RequestConfig, g: &GlobalConfig, cwd: &Path) -> String {
    let err = build_easy(r, g, cwd).expect_err("guard should refuse");
    assert_eq!(err.code, None);
    err.message
}

#[test]
fn form_file_relative_to_cwd() {
    // TS resolved formData files against `process.cwd()` (Bun.file), never
    // the YAML file's directory.
    let srv = TestServer::start();
    let dir = tempfile::tempdir().unwrap();
    std::fs::create_dir(dir.path().join("fixtures")).unwrap();
    std::fs::write(dir.path().join("fixtures").join("a.txt"), b"hello").unwrap();
    let r = RequestConfig {
        method: Some("POST".into()),
        form_data: file_form("doc", "./fixtures/a.txt"),
        ..req(format!("{}/multipart", srv.base_url))
    };
    let resp = send(&r, &GlobalConfig::default(), dir.path()).unwrap();
    assert_eq!(resp.body[0]["filename"], "a.txt");
    assert_eq!(resp.body[0]["size"], 5);
    // Not found relative to any other directory.
    let m = guard_msg(&r, &GlobalConfig::default(), &dir.path().join("fixtures"));
    assert_eq!(m, "File(s) not found: doc: ./fixtures/a.txt");
}

#[test]
fn form_file_outside_cwd_rejected_without_allow_path() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::create_dir(dir.path().join("proj")).unwrap();
    std::fs::write(dir.path().join("secret.txt"), b"s").unwrap();
    let cwd = dir.path().join("proj");
    for file in ["../secret.txt", "/etc/passwd"] {
        let r = RequestConfig {
            method: Some("POST".into()),
            form_data: file_form("attachment", file),
            ..req("http://127.0.0.1:1/".into())
        };
        let m = guard_msg(&r, &GlobalConfig::default(), &cwd);
        assert_eq!(
            m,
            format!(
                "formData field \"attachment\": Path \"{file}\" escapes the working directory. Use --allow-path to permit it."
            )
        );
    }
}

#[test]
fn form_file_outside_cwd_allowed_with_allow_path() {
    let srv = TestServer::start();
    let dir = tempfile::tempdir().unwrap();
    std::fs::create_dir(dir.path().join("proj")).unwrap();
    std::fs::write(dir.path().join("secret.txt"), b"s3").unwrap();
    let cwd = dir.path().join("proj");
    let r = RequestConfig {
        method: Some("POST".into()),
        form_data: file_form("attachment", "../secret.txt"),
        ..req(format!("{}/multipart", srv.base_url))
    };
    let resp = send(&r, &sec(None, Some(true)), &cwd).unwrap();
    assert_eq!(resp.body[0]["size"], 2);
    // With allowPaths a missing file is the existence error, not the guard's.
    let r = RequestConfig {
        form_data: file_form("attachment", "../nonexistent-test-file.txt"),
        ..req("http://127.0.0.1:1/".into())
    };
    let m = guard_msg(&r, &sec(None, Some(true)), &cwd);
    assert!(m.starts_with("File(s) not found"), "{m}");
}

#[test]
fn output_outside_cwd_is_rejected_and_nothing_is_written() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::create_dir(dir.path().join("proj")).unwrap();
    let cwd = dir.path().join("proj");
    for out in ["../escape.json", "/tmp/steal.json", "sub/../../escape.json"] {
        let r = RequestConfig {
            output: Some(out.into()),
            ..req("http://127.0.0.1:1/".into())
        };
        let m = guard_msg(&r, &GlobalConfig::default(), &cwd);
        assert!(m.contains("escapes the working directory"), "{m}");
        assert!(m.contains("--allow-path"), "{m}");
    }
    assert!(!dir.path().join("escape.json").exists());
}

#[test]
fn output_is_relative_to_cwd() {
    let srv = TestServer::start();
    let dir = tempfile::tempdir().unwrap();
    std::fs::create_dir(dir.path().join("sub")).unwrap();
    let r = RequestConfig {
        output: Some("out.json".into()),
        ..req(format!("{}/json/user", srv.base_url))
    };
    send(&r, &GlobalConfig::default(), dir.path()).unwrap();
    assert!(dir.path().join("out.json").exists());
    assert!(!dir.path().join("sub").join("out.json").exists());
}

#[test]
fn output_outside_cwd_allowed_with_allow_path() {
    let srv = TestServer::start();
    let dir = tempfile::tempdir().unwrap();
    std::fs::create_dir(dir.path().join("proj")).unwrap();
    let cwd = dir.path().join("proj");
    let r = RequestConfig {
        output: Some("../escape.json".into()),
        ..req(format!("{}/json/user", srv.base_url))
    };
    send(&r, &sec(None, Some(true)), &cwd).unwrap();
    assert!(dir.path().join("escape.json").exists());
}

#[test]
fn ssl_paths_are_not_confined() {
    let r = RequestConfig {
        ssl: Some(SslConfig {
            ca: Some("/etc/ssl/certs/ca.pem".into()),
            cert: Some("/etc/ssl/certs/client.crt".into()),
            key: Some("../../private.key".into()),
            verify: None,
        }),
        ..req("https://example.invalid/".into())
    };
    assert!(build_easy(&r, &GlobalConfig::default(), Path::new("/work/proj")).is_ok());
}

#[test]
fn blocked_url_protocols_are_refused_before_any_transfer() {
    let g = GlobalConfig::default();
    for (url, proto) in [
        ("file:///etc/passwd", "file"),
        ("gopher://internal.example.com/", "gopher"),
        ("ftp://files.example.com/data.csv", "ftp"),
    ] {
        let m = guard_msg(&req(url.into()), &g, &root());
        assert_eq!(
            m,
            format!(
                "Blocked protocol \"{proto}\" (allowed: http, https). Use --allow-protocol to permit it."
            )
        );
    }
    let m = guard_msg(&req("not-a-url".into()), &g, &root());
    assert_eq!(m, "Invalid URL: \"not-a-url\"");
}

#[test]
fn ftp_passes_the_guard_when_allowed() {
    let g = sec(Some(&["http", "https", "ftp"]), None);
    assert!(build_easy(&req("ftp://files.example.com/data.csv".into()), &g, &root()).is_ok());
    // Empty list falls back to http/https.
    let g = sec(Some(&[]), None);
    assert!(build_easy(&req("http://127.0.0.1:1/".into()), &g, &root()).is_ok());
    assert!(build_easy(&req("ftp://x/".into()), &g, &root()).is_err());
}

#[test]
fn unsupported_or_odd_allow_list_entries_fail_closed() {
    // `gopher`/`ftp` may be missing from this libcurl build, and an empty
    // intersection must still block everything rather than widen.
    let g = sec(Some(&["ftp", "nonsense"]), None);
    let err = send(&req("http://127.0.0.1:1/".into()), &g, &root()).unwrap_err();
    assert!(
        err.message.contains("Blocked protocol \"http\""),
        "{}",
        err.message
    );
    let g = sec(Some(&["ftp", "http"]), None);
    let m = send(&req("http://127.0.0.1:1/".into()), &g, &root()).unwrap_err();
    assert_eq!(m.code, Some(7), "{}", m.message);
}

/// One-shot HTTP server answering every request with `302 Location: <loc>`.
fn redirect_server(location: &str) -> (String, std::thread::JoinHandle<()>) {
    use std::io::{Read, Write};
    let l = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let url = format!("http://{}/", l.local_addr().unwrap());
    let resp = format!(
        "HTTP/1.1 302 Found\r\nLocation: {location}\r\nContent-Length: 0\r\nConnection: close\r\n\r\n"
    );
    let h = std::thread::spawn(move || {
        if let Ok((mut c, _)) = l.accept() {
            let mut buf = [0u8; 4096];
            let _ = c.read(&mut buf);
            let _ = c.write_all(resp.as_bytes());
        }
    });
    (url, h)
}

#[test]
fn redirect_to_disallowed_protocol_fails() {
    for (loc, quoted) in [
        ("file:///etc/passwd", "file"),
        ("ftp://127.0.0.1:1/x", "ftp"),
        ("gopher://127.0.0.1:1/x", "gopher"),
    ] {
        let (url, h) = redirect_server(loc);
        let err = send(&req(url), &GlobalConfig::default(), &root()).unwrap_err();
        h.join().unwrap();
        // CURLE_UNSUPPORTED_PROTOCOL
        assert_eq!(err.code, Some(1), "{loc}: {}", err.message);
        assert!(err.message.starts_with("curl: (1) "), "{}", err.message);
        let _ = quoted;
    }
}

#[test]
fn redirect_protocol_follows_the_allow_list() {
    // https is disallowed here: redirect to it is refused (code 1), whereas
    // with the default list libcurl attempts it and fails to connect (code 7).
    let (url, h) = redirect_server("https://127.0.0.1:1/");
    let err = send(&req(url), &sec(Some(&["http"]), None), &root()).unwrap_err();
    h.join().unwrap();
    assert_eq!(err.code, Some(1), "{}", err.message);

    let (url, h) = redirect_server("https://127.0.0.1:1/");
    let err = send(&req(url), &GlobalConfig::default(), &root()).unwrap_err();
    h.join().unwrap();
    assert_eq!(err.code, Some(7), "{}", err.message);
}

/// Part Content-Type as the curl tool (`-F 'f=@path;filename=x;type=t'`, mime
/// API) picks it: explicit type, else guessed from the part filename (custom
/// or the path's basename), else from the path, else
/// application/octet-stream. Expected values checked against curl 8.7.1.
#[test]
fn form_file_content_type_follows_curl_tool() {
    let srv = TestServer::start();
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("a.txt"), b"hi").unwrap();
    std::fs::write(dir.path().join("a.bin"), b"hi").unwrap();
    let file = |path: &str, filename: Option<&str>, ct: Option<&str>| {
        FormFieldValue::File(FileAttachment {
            file: path.into(),
            filename: filename.map(str::to_string),
            content_type: ct.map(str::to_string),
        })
    };
    let cases = [
        ("f1", file("a.txt", Some("profile.png"), None), "image/png"),
        ("f2", file("a.txt", Some("data.bin"), None), "text/plain"),
        ("f3", file("a.bin", None, None), "application/octet-stream"),
        ("f4", file("a.bin", Some("x.JPG"), None), "image/jpeg"),
        (
            "f5",
            file("a.txt", Some("p.png"), Some("application/custom")),
            "application/custom",
        ),
        (
            "f6",
            file("a.bin", Some("noext"), None),
            "application/octet-stream",
        ),
        ("f7", file("a.txt", None, None), "text/plain"),
    ];
    let r = RequestConfig {
        method: Some("POST".into()),
        form_data: Some(
            cases
                .iter()
                .map(|(n, v, _)| ((*n).to_string(), v.clone()))
                .collect(),
        ),
        ..req(format!("{}/multipart", srv.base_url))
    };
    let resp = send(&r, &GlobalConfig::default(), dir.path()).unwrap();
    let got: Vec<(String, String)> = resp
        .body
        .as_array()
        .unwrap()
        .iter()
        .map(|p| {
            (
                p["name"].as_str().unwrap().into(),
                p["contentType"].as_str().unwrap().into(),
            )
        })
        .collect();
    let want: Vec<(String, String)> = cases
        .iter()
        .map(|(n, _, ct)| ((*n).to_string(), (*ct).to_string()))
        .collect();
    assert_eq!(got, want);
}

#[test]
fn ssl_paths_resolve_against_cwd() {
    // TS passed them to the curl process, which resolved them against its cwd.
    assert_eq!(
        super::resolve_local_path("./certs/ca.pem", Path::new("/work/proj")).unwrap(),
        Path::new("/work/proj/./certs/ca.pem")
    );
    assert_eq!(
        super::resolve_local_path("/etc/ssl/ca.pem", Path::new("/work/proj")).unwrap(),
        Path::new("/etc/ssl/ca.pem")
    );
}

#[test]
fn supported_protocols_are_computed_once() {
    let a = super::easy::supported_protocols();
    assert!(std::ptr::eq(a, super::easy::supported_protocols()));
    assert!(a.iter().any(|p| p == "http") && a.iter().any(|p| p == "https"));
    assert!(a.iter().all(|p| *p == p.to_ascii_lowercase()));
}
