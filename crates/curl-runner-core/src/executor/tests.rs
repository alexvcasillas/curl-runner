//! Executor tests (ports of request-executor / dry-run / max-concurrency /
//! path- and url-guard integration specs, plus live `TestServer` runs).

use super::*;
use crate::model::SnapshotCompareResult;
use serde_json::json;
use std::time::Duration;
use testkit::TestServer;

#[derive(Default)]
struct Rec {
    starts: Vec<usize>,
    commands: Vec<String>,
    retries: Vec<(usize, u32, u64)>,
    results: Vec<(usize, bool)>,
    stored: Vec<Store>,
    stops: usize,
}

impl EventSink for Rec {
    fn on_start(&mut self, index: usize, _req: &RequestConfig) {
        self.starts.push(index);
    }
    fn on_command(&mut self, _index: usize, _req: &RequestConfig, command: &str) {
        self.commands.push(command.to_string());
    }
    fn on_retry(&mut self, index: usize, _req: &RequestConfig, attempt: u32, delay_ms: u64) {
        self.retries.push((index, attempt, delay_ms));
    }
    fn on_result(&mut self, index: usize, r: &ExecutionResult) {
        self.results.push((index, r.success));
    }
    fn on_stored(&mut self, values: &Store) {
        self.stored.push(values.clone());
    }
    fn on_stop(&mut self) {
        self.stops += 1;
    }
}

fn prep(v: Json) -> PreparedRequest {
    PreparedRequest { raw: v }
}

fn exec(global: Json) -> Executor {
    Executor::new(serde_json::from_value(global).unwrap())
}

fn run(ex: &Executor, reqs: Vec<Json>) -> (ExecutionSummary, Rec) {
    let mut rec = Rec::default();
    let s = ex.run(reqs.into_iter().map(prep).collect(), &mut rec);
    (s, rec)
}

const SEQ: &str = "sequential";
const PAR: &str = "parallel";

fn both() -> [&'static str; 2] {
    [SEQ, PAR]
}

fn closed_port_url() -> String {
    // Bind then drop: nothing listens there afterwards.
    let l = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let port = l.local_addr().unwrap().port();
    drop(l);
    format!("http://127.0.0.1:{port}/")
}

// ---------- basic execution ----------

#[test]
fn runs_requests_and_fills_result() {
    let srv = TestServer::start();
    for mode in both() {
        let ex = exec(json!({ "execution": mode }));
        let (s, rec) = run(
            &ex,
            vec![json!({ "name": "u", "url": format!("{}/json/user", srv.base_url) })],
        );
        assert_eq!((s.total, s.successful, s.failed, s.skipped), (1, 1, 0, 0));
        let r = &s.results[0];
        assert_eq!(r.status, Some(200));
        assert_eq!(r.body.as_ref().unwrap()["name"], "Test User");
        assert!(r.headers.is_some() && r.header_history.is_some());
        assert!(r.metrics.as_ref().unwrap().duration > 0.0);
        assert_eq!(r.dry_run, None);
        assert_eq!(rec.starts, [1]);
        assert_eq!(rec.commands.len(), 1);
        assert_eq!(rec.results, [(1, true)]);
        assert!(s.duration > 0.0);
    }
}

#[test]
fn status_500_without_retry_or_expect_is_success() {
    // TS: only transport failures, exhausted retries and `expect` fail.
    let srv = TestServer::start();
    let ex = exec(json!({}));
    let (s, _) = run(
        &ex,
        vec![json!({ "url": format!("{}/status/500", srv.base_url) })],
    );
    assert!(s.results[0].success);
    assert_eq!(s.results[0].status, Some(500));
}

// ---------- retries ----------

#[test]
fn retries_flaky_then_passes() {
    for mode in both() {
        let srv = TestServer::start();
        let ex = exec(json!({ "execution": mode }));
        let (s, rec) = run(
            &ex,
            vec![json!({ "url": format!("{}/flaky/a/2", srv.base_url), "retry": { "count": 3 } })],
        );
        let r = &s.results[0];
        assert!(r.success, "{mode}: {:?}", r.error);
        assert_eq!(r.body.as_ref().unwrap()["attempt"], 3);
        assert_eq!(rec.retries, [(1, 1, 0), (1, 2, 0)], "{mode}");
    }
}

#[test]
fn retry_exhausted_reports_ts_error_and_last_status() {
    for mode in both() {
        let srv = TestServer::start();
        let ex = exec(json!({ "execution": mode }));
        let (s, rec) = run(
            &ex,
            vec![json!({ "url": format!("{}/status/503", srv.base_url), "retry": { "count": 1 } })],
        );
        let r = &s.results[0];
        assert!(!r.success);
        assert_eq!(r.status, Some(503));
        assert_eq!(
            r.error.as_deref(),
            Some("Operation returned retriable result")
        );
        assert!(r.body.is_none() && r.headers.is_none());
        assert_eq!(rec.retries.len(), 1);
        assert_eq!(s.failed, 1);
    }
}

#[test]
fn custom_retryable_statuses() {
    let srv = TestServer::start();
    let ex = exec(json!({}));
    let (s, rec) = run(
        &ex,
        vec![
            json!({ "url": format!("{}/status/404", srv.base_url),
                "retry": { "count": 2, "retryableStatuses": [404] } }),
            json!({ "url": format!("{}/status/503", srv.base_url),
                "retry": { "count": 2, "retryableStatuses": [404] } }),
        ],
    );
    // 404 retried and exhausted -> failure stops the run.
    assert_eq!(rec.retries.len(), 2);
    assert_eq!(s.total, 1);
    assert!(!s.results[0].success);
}

#[test]
fn retry_delay_with_backoff_is_waited() {
    for mode in both() {
        let srv = TestServer::start();
        let ex = exec(json!({ "execution": mode }));
        let t = Instant::now();
        let (s, rec) = run(
            &ex,
            vec![json!({ "url": format!("{}/status/503", srv.base_url),
                "retry": { "count": 2, "delay": 50, "backoff": 2 } })],
        );
        assert!(t.elapsed() >= Duration::from_millis(140), "{mode}");
        assert_eq!(rec.retries, [(1, 1, 50), (1, 2, 100)]);
        assert!(!s.results[0].success);
    }
}

#[test]
fn transport_failure_fails_and_is_retried() {
    for mode in both() {
        let ex = exec(json!({ "execution": mode }));
        let (s, rec) = run(
            &ex,
            vec![json!({ "url": closed_port_url(), "retry": { "count": 1 } })],
        );
        let r = &s.results[0];
        assert!(!r.success);
        assert!(
            r.error.as_deref().unwrap().starts_with("curl: (7)"),
            "{:?}",
            r.error
        );
        assert_eq!(r.status, None);
        assert_eq!(rec.retries.len(), 1, "{mode}");
    }
}

// ---------- concurrency ----------

#[test]
fn max_concurrency_respected() {
    let srv = TestServer::start();
    let ex = exec(json!({ "execution": PAR, "maxConcurrency": 2 }));
    let reqs = (0..6)
        .map(|_| json!({ "url": format!("{}/delay/200", srv.base_url) }))
        .collect();
    let t = Instant::now();
    let (s, _) = run(&ex, reqs);
    assert!(t.elapsed() >= Duration::from_millis(600));
    assert_eq!((s.total, s.successful), (6, 6));
}

#[test]
fn max_concurrency_one_is_serial() {
    let srv = TestServer::start();
    let ex = exec(json!({ "execution": PAR, "maxConcurrency": 1 }));
    let reqs = (0..3)
        .map(|_| json!({ "url": format!("{}/delay/100", srv.base_url) }))
        .collect();
    let t = Instant::now();
    let (s, _) = run(&ex, reqs);
    assert!(t.elapsed() >= Duration::from_millis(300));
    assert_eq!(s.successful, 3);
}

#[test]
fn parallel_results_keep_request_order() {
    let srv = TestServer::start();
    let ex = exec(json!({ "execution": PAR }));
    let (s, rec) = run(
        &ex,
        vec![
            json!({ "name": "slow", "url": format!("{}/delay/300", srv.base_url) }),
            json!({ "name": "fast", "url": format!("{}/delay/1", srv.base_url) }),
        ],
    );
    let names: Vec<_> = s
        .results
        .iter()
        .map(|r| r.request.name.clone().unwrap())
        .collect();
    assert_eq!(names, ["slow", "fast"]);
    // events arrive in completion order
    assert_eq!(rec.results, [(2, true), (1, true)]);
}

#[test]
fn parallel_retry_does_not_block_other_transfers() {
    let srv = TestServer::start();
    let ex = exec(json!({ "execution": PAR, "maxConcurrency": 2 }));
    let (s, rec) = run(
        &ex,
        vec![
            json!({ "url": format!("{}/flaky/p/1", srv.base_url), "retry": { "count": 1, "delay": 400 } }),
            json!({ "name": "other", "url": format!("{}/delay/1", srv.base_url) }),
        ],
    );
    assert_eq!(s.successful, 2);
    // the second request finished while the first waited for its retry
    assert_eq!(rec.results.first(), Some(&(2, true)));
}

#[test]
fn pool_settings_build_a_multi_handle() {
    // Live pooled runs force HTTP/2, whose cleartext upgrade the tiny_http
    // test server cannot answer (see task 6); check the setup only.
    for g in [
        json!({ "connectionPool": { "enabled": true, "maxStreamsPerHost": 1 } }),
        json!({ "connectionPool": { "enabled": true } }),
        json!({}),
    ] {
        let _ = parallel::new_multi(&exec(g));
    }
}

// ---------- store / when / expect / continueOnError ----------

#[test]
fn store_flows_between_requests() {
    let srv = TestServer::start();
    let ex = exec(json!({}));
    let (s, rec) = run(
        &ex,
        vec![
            json!({ "url": format!("{}/json/user", srv.base_url), "store": { "uid": "body.id" } }),
            json!({ "url": format!("{}/echo", srv.base_url),
                "params": { "id": "${store.uid}" }, "timeout": "${store.uid}0000" }),
        ],
    );
    assert_eq!(s.successful, 2, "{:?}", s.results[1].error);
    assert_eq!(s.results[1].body.as_ref().unwrap()["query"]["id"], "1");
    assert_eq!(s.results[1].request.timeout, Some(10000));
    assert_eq!(rec.stored.len(), 1);
    assert_eq!(rec.stored[0].get("uid").map(String::as_str), Some("1"));
}

#[test]
fn store_value_in_numeric_field_that_does_not_fit_fails_request() {
    let srv = TestServer::start();
    let ex = exec(json!({}));
    let (s, rec) = run(
        &ex,
        vec![
            json!({ "url": format!("{}/json/user", srv.base_url), "store": { "n": "body.name" } }),
            json!({ "name": "bad", "url": format!("{}/echo", srv.base_url), "timeout": "${store.n}" }),
        ],
    );
    let r = &s.results[1];
    assert!(!r.success);
    assert!(
        r.error.as_deref().unwrap().contains("timeout"),
        "{:?}",
        r.error
    );
    assert_eq!(r.request.name.as_deref(), Some("bad"));
    assert_eq!(rec.starts, [1]);
}

#[test]
fn untyped_request_fails_not_panics() {
    for mode in both() {
        let ex = exec(json!({ "execution": mode, "continueOnError": true }));
        let (s, _) = run(&ex, vec![json!({ "url": "http://x", "timeout": "abc" })]);
        assert_eq!(s.failed, 1);
        assert!(s.results[0].error.as_deref().unwrap().contains("timeout"));
    }
}

#[test]
fn when_skips_request() {
    let srv = TestServer::start();
    let ex = exec(json!({}));
    let (s, rec) = run(
        &ex,
        vec![
            json!({ "url": format!("{}/json/user", srv.base_url), "store": { "id": "body.id" } }),
            json!({ "name": "skip", "url": format!("{}/echo", srv.base_url), "when": "store.id == 2" }),
            json!({ "name": "run", "url": format!("{}/echo", srv.base_url), "when": "store.id == 1" }),
        ],
    );
    assert_eq!((s.total, s.successful, s.failed, s.skipped), (3, 2, 0, 1));
    let sk = &s.results[1];
    assert!(sk.success && sk.skipped == Some(true));
    assert!(sk.skip_reason.is_some());
    assert!(sk.metrics.is_none());
    assert_eq!(rec.starts, [1, 3]);
    assert_eq!(rec.results.len(), 3);
}

#[test]
fn expect_failure_inverts() {
    let srv = TestServer::start();
    let ex = exec(json!({ "continueOnError": true }));
    let (s, _) = run(
        &ex,
        vec![
            json!({ "url": format!("{}/status/404", srv.base_url),
                "expect": { "failure": true, "status": 404 } }),
            json!({ "url": format!("{}/status/200", srv.base_url), "expect": { "failure": true } }),
        ],
    );
    assert!(s.results[0].success, "{:?}", s.results[0].error);
    assert!(!s.results[1].success);
    assert!(s.results[1].error.is_some());
}

#[test]
fn expect_mismatch_error_text() {
    for mode in both() {
        let srv = TestServer::start();
        let ex = exec(json!({ "execution": mode }));
        let (s, _) = run(
            &ex,
            vec![
                json!({ "url": format!("{}/status/404", srv.base_url), "expect": { "status": 200 } }),
            ],
        );
        assert_eq!(
            s.results[0].error.as_deref(),
            Some("Expected status 200, got 404")
        );
        assert_eq!(s.results[0].status, Some(404));
    }
}

#[test]
fn continue_on_error_runs_all() {
    let srv = TestServer::start();
    let reqs = || {
        vec![
            json!({ "url": format!("{}/status/404", srv.base_url), "expect": { "status": 200 } }),
            json!({ "url": format!("{}/json/user", srv.base_url) }),
        ]
    };
    let (s, rec) = run(&exec(json!({})), reqs());
    assert_eq!((s.total, s.failed), (1, 1));
    assert_eq!(rec.stops, 1);
    let (s, rec) = run(&exec(json!({ "continueOnError": true })), reqs());
    assert_eq!((s.total, s.successful, s.failed), (2, 1, 1));
    assert_eq!(rec.stops, 0);
}

#[test]
fn parallel_stop_on_error_stops_launching() {
    let srv = TestServer::start();
    let ex = exec(json!({ "execution": PAR, "maxConcurrency": 1 }));
    let (s, rec) = run(
        &ex,
        vec![
            json!({ "url": format!("{}/status/404", srv.base_url), "expect": { "status": 200 } }),
            json!({ "url": format!("{}/json/user", srv.base_url) }),
            json!({ "url": format!("{}/json/user", srv.base_url) }),
        ],
    );
    assert_eq!((s.total, s.failed), (1, 1));
    assert_eq!(rec.stops, 1);
}

#[test]
fn parallel_ignores_when_and_store_like_ts() {
    let srv = TestServer::start();
    let ex = exec(json!({ "execution": PAR }));
    let (s, rec) = run(
        &ex,
        vec![json!({ "url": format!("{}/echo", srv.base_url),
            "params": { "id": "${store.x}" }, "when": "store.x == 1", "store": { "a": "body.path" } })],
    );
    assert_eq!(s.skipped, 0);
    assert_eq!(
        s.results[0].body.as_ref().unwrap()["query"]["id"],
        "${store.x}"
    );
    assert!(rec.stored.is_empty());
}

// ---------- cancellation ----------

fn cancel_after(ex: &Executor, ms: u64) {
    let h = ex.cancel_handle();
    std::thread::spawn(move || {
        std::thread::sleep(Duration::from_millis(ms));
        h.store(true, Ordering::Relaxed);
    });
}

#[test]
fn cancel_stops_parallel_run() {
    let srv = TestServer::start();
    let ex = exec(json!({ "execution": PAR, "maxConcurrency": 2 }));
    let mut reqs = vec![json!({ "url": format!("{}/delay/1", srv.base_url) })];
    reqs.extend((0..4).map(|_| json!({ "url": format!("{}/delay/5000", srv.base_url) })));
    cancel_after(&ex, 300);
    let t = Instant::now();
    let (s, rec) = run(&ex, reqs);
    assert!(t.elapsed() < Duration::from_millis(4000));
    assert_eq!(s.total, 1, "only the completed request is reported");
    assert!(rec.starts.len() <= 3);
}

#[test]
fn cancel_stops_sequential_between_requests_and_in_retry_sleep() {
    let srv = TestServer::start();
    let ex = exec(json!({}));
    cancel_after(&ex, 300);
    let t = Instant::now();
    let (s, _) = run(
        &ex,
        vec![
            json!({ "url": format!("{}/delay/1", srv.base_url) }),
            json!({ "url": format!("{}/status/503", srv.base_url),
                "retry": { "count": 5, "delay": 2000 } }),
            json!({ "url": format!("{}/json/user", srv.base_url) }),
        ],
    );
    assert!(t.elapsed() < Duration::from_millis(1800));
    assert_eq!(s.total, 1);
}

#[test]
fn pre_cancelled_run_is_empty() {
    for mode in both() {
        let ex = exec(json!({ "execution": mode }));
        ex.cancel_handle().store(true, Ordering::Relaxed);
        let (s, rec) = run(&ex, vec![json!({ "url": "http://127.0.0.1:9/" })]);
        assert_eq!(s.total, 0);
        assert!(rec.starts.is_empty());
    }
}

// ---------- dry run ----------

#[test]
fn dry_run_does_not_send_and_emits_command() {
    for mode in both() {
        let url = closed_port_url();
        let ex = exec(json!({ "execution": mode, "dryRun": true, "maxConcurrency": 2 }));
        let reqs = vec![
            json!({ "url": format!("{url}users"), "method": "GET",
                "expect": { "status": 200, "body": { "id": 1 } }, "retry": { "count": 3 } }),
            json!({ "url": format!("{url}login"), "method": "POST", "body": { "u": "t" },
                "store": { "token": "body.token" } }),
            json!({ "url": format!("{url}p"), "headers": { "Authorization": "Bearer ${store.token}" } }),
        ];
        let (s, rec) = run(&ex, reqs);
        assert_eq!((s.total, s.successful, s.failed), (3, 3, 0), "{mode}");
        for r in &s.results {
            assert_eq!(r.dry_run, Some(true));
            assert!(r.success);
            assert_eq!(r.metrics.as_ref().unwrap().duration, 0.0);
            assert!(
                r.status.is_none() && r.body.is_none() && r.headers.is_none() && r.error.is_none()
            );
        }
        assert_eq!(rec.commands.len(), 3);
        assert!(rec.commands[1].contains("POST"), "{}", rec.commands[1]);
        assert!(rec.retries.is_empty());
        assert!(rec.stored.is_empty());
    }
}

#[test]
fn dry_run_preserves_request_and_methods() {
    let ex = exec(json!({ "dryRun": true }));
    let methods = ["GET", "POST", "PUT", "DELETE", "PATCH", "HEAD", "OPTIONS"];
    let reqs = methods
        .iter()
        .map(|m| {
            json!({ "name": m, "url": "https://api.example.com/r", "method": m,
            "insecure": true, "proxy": "http://p:1", "followRedirects": true,
            "auth": { "type": "basic", "username": "u", "password": "p" } })
        })
        .collect();
    let (s, _) = run(&ex, reqs);
    assert_eq!(s.successful, 7);
    assert_eq!(s.results[3].request.method.as_deref(), Some("DELETE"));
    assert_eq!(s.results[3].request.url, "https://api.example.com/r");
}

#[test]
fn dry_run_still_applies_guards_and_missing_files() {
    for mode in both() {
        let ex = exec(json!({ "execution": mode, "dryRun": true, "continueOnError": true }));
        let (s, _) = run(
            &ex,
            vec![
                json!({ "url": "file:///etc/passwd" }),
                json!({ "url": "https://x", "formData": { "f": { "file": "nope-missing.bin" } } }),
                json!({ "url": "https://x" }),
            ],
        );
        assert_eq!((s.total, s.successful, s.failed), (3, 1, 2), "{mode}");
        assert!(
            s.results[1]
                .error
                .as_deref()
                .unwrap()
                .starts_with("File(s) not found")
        );
        assert!(s.results[0].dry_run.is_none());
    }
}

// ---------- guards (url-guard / path-guard integration) ----------

#[test]
fn blocked_protocols_fail_with_metrics() {
    for url in ["file:///etc/passwd", "gopher://x/", "ftp://x/", "not a url"] {
        let ex = exec(json!({}));
        let (s, rec) = run(&ex, vec![json!({ "url": url })]);
        let r = &s.results[0];
        assert!(!r.success, "{url}");
        assert!(r.error.is_some());
        assert!(r.metrics.as_ref().unwrap().duration >= 0.0);
        assert!(rec.commands.is_empty(), "no command for blocked {url}");
    }
}

#[test]
fn ftp_allowed_by_security_config_passes_guard() {
    let ex = exec(
        json!({ "dryRun": true, "security": { "allowedProtocols": ["http", "https", "ftp"] } }),
    );
    let (s, _) = run(&ex, vec![json!({ "url": "ftp://example.com/f" })]);
    assert!(s.results[0].success);
}

#[test]
fn mixed_valid_and_blocked_sequential_and_parallel() {
    let srv = TestServer::start();
    for mode in both() {
        let ex = exec(json!({ "execution": mode, "continueOnError": true }));
        let (s, _) = run(
            &ex,
            vec![
                json!({ "url": format!("{}/json/user", srv.base_url) }),
                json!({ "url": "file:///etc/passwd" }),
            ],
        );
        assert_eq!((s.total, s.successful, s.failed), (2, 1, 1), "{mode}");
    }
}

#[test]
fn output_path_confinement() {
    let srv = TestServer::start();
    let dir = tempfile::tempdir().unwrap();
    let cwd = dir.path().canonicalize().unwrap();
    for mode in both() {
        let ex = exec(json!({ "execution": mode, "continueOnError": true })).with_cwd(&cwd);
        let (s, _) = run(
            &ex,
            vec![
                json!({ "url": format!("{}/json/user", srv.base_url), "output": "../escape.json" }),
                json!({ "url": format!("{}/json/user", srv.base_url), "output": "/etc/passwd" }),
                json!({ "url": format!("{}/json/user", srv.base_url), "output": "in.json" }),
            ],
        );
        assert_eq!((s.successful, s.failed), (1, 2), "{mode}");
        assert!(cwd.join("in.json").exists());
        assert!(!cwd.parent().unwrap().join("escape.json").exists());
    }
    let ex = exec(json!({ "security": { "allowPaths": true } })).with_cwd(cwd.join("sub"));
    std::fs::create_dir(cwd.join("sub")).unwrap();
    let (s, _) = run(
        &ex,
        vec![json!({ "url": format!("{}/json/user", srv.base_url), "output": "../escape.json" })],
    );
    assert!(s.results[0].success, "{:?}", s.results[0].error);
    assert!(cwd.join("escape.json").exists());
}

#[test]
fn form_file_outside_cwd_blocked() {
    let dir = tempfile::tempdir().unwrap();
    let ex = exec(json!({ "dryRun": true })).with_cwd(dir.path());
    let (s, _) = run(
        &ex,
        vec![json!({ "url": "https://x", "formData": { "f": { "file": "/etc/passwd" } } })],
    );
    assert!(
        s.results[0]
            .error
            .as_deref()
            .unwrap()
            .starts_with("formData field \"f\"")
    );
}

// ---------- secrets / snapshot hook ----------

#[test]
fn registers_request_secrets_before_events() {
    let red = Arc::new(Mutex::new(Redactor::default()));
    let ex = exec(json!({ "dryRun": true })).with_redactor(red.clone());
    run(
        &ex,
        vec![
            json!({ "url": "https://x", "auth": { "type": "bearer", "token": "tok-abcdef-123456" } }),
        ],
    );
    let r = red.lock().unwrap();
    assert!(
        !r.redact("x tok-abcdef-123456 y")
            .contains("tok-abcdef-123456")
    );
}

struct Snap {
    calls: Arc<Mutex<u32>>,
    matches: bool,
}
impl SnapshotHook for Snap {
    fn compare(
        &mut self,
        _req: &RequestConfig,
        _r: &ExecutionResult,
    ) -> Option<SnapshotCompareResult> {
        *self.calls.lock().unwrap() += 1;
        Some(SnapshotCompareResult {
            matches: self.matches,
            ..Default::default()
        })
    }
}

#[test]
fn snapshot_hook_runs_for_successful_results_with_source_file() {
    let srv = TestServer::start();
    let calls = Arc::new(Mutex::new(0));
    let ex = exec(json!({ "continueOnError": true })).with_snapshot_hook(Box::new(Snap {
        calls: calls.clone(),
        matches: false,
    }));
    let (s, _) = run(
        &ex,
        vec![
            json!({ "url": format!("{}/json/user", srv.base_url), "sourceFile": "a.yaml" }),
            json!({ "url": format!("{}/json/user", srv.base_url) }),
            json!({ "url": format!("{}/status/404", srv.base_url), "sourceFile": "a.yaml",
                "expect": { "status": 200 } }),
        ],
    );
    assert_eq!(*calls.lock().unwrap(), 1);
    assert_eq!(s.results[0].error.as_deref(), Some("Snapshot mismatch"));
    assert!(s.results[1].success);
}

// ---------- summary ----------

#[test]
fn summary_counts_like_create_summary() {
    let mut ok = ExecutionResult::new(RequestConfig::default(), true);
    ok.dry_run = Some(true);
    let bad = ExecutionResult::new(RequestConfig::default(), false);
    let mut skip = ExecutionResult::new(RequestConfig::default(), true);
    skip.skipped = Some(true);
    let s = create_summary(vec![ok, bad, skip], 5.0);
    assert_eq!((s.total, s.successful, s.failed, s.skipped), (3, 1, 1, 1));
    assert_eq!(s.duration, 5.0);
}

#[test]
fn save_summary_guards_and_redacts() {
    let dir = tempfile::tempdir().unwrap();
    let cwd = dir.path().canonicalize().unwrap();
    let mut r = ExecutionResult::new(RequestConfig::default(), true);
    r.error = Some("leak SECRETVALUE".into());
    let s = create_summary(vec![r], 1.0);
    let g = |f: &str| -> GlobalConfig {
        serde_json::from_value(json!({ "output": { "saveToFile": f } })).unwrap()
    };
    assert_eq!(
        save_summary(&s, &GlobalConfig::default(), &cwd, None),
        Ok(None)
    );
    let e = save_summary(&s, &g("../out.json"), &cwd, None).unwrap_err();
    assert!(e.starts_with("Cannot save results: "), "{e}");
    let red = Redactor::new(["SECRETVALUE".to_string()]);
    let p = save_summary(&s, &g("out.json"), &cwd, Some(&red))
        .unwrap()
        .unwrap();
    let text = std::fs::read_to_string(p).unwrap();
    assert!(text.contains("\n  \"total\": 1"));
    assert!(!text.contains("SECRETVALUE"));
}

#[test]
fn save_summary_prints_numbers_like_json_stringify() {
    let dir = tempfile::tempdir().unwrap();
    let cwd = dir.path().canonicalize().unwrap();
    let req = crate::parser::load_str(
        "request:\n  url: u\n  body: {id: 1234567890123456789, c: 100000000000000000000, n: -1234567890123456789, b: 1e21}\n",
        "f",
    )
    .unwrap()
    .requests
    .remove(0);
    let s = create_summary(vec![ExecutionResult::new(req, true)], 2.0);
    let g: GlobalConfig =
        serde_json::from_value(json!({ "output": { "saveToFile": "out.json" } })).unwrap();
    save_summary(&s, &g, &cwd, None).unwrap();
    let text = std::fs::read_to_string(cwd.join("out.json")).unwrap();
    for want in [
        "\"id\": 1234567890123456800",
        "\"c\": 100000000000000000000",
        "\"n\": -1234567890123456800",
        "\"b\": 1e+21",
        "\"duration\": 2",
    ] {
        assert!(text.contains(want), "{want}: {text}");
    }
}

#[test]
fn save_summary_replaces_file_atomically_without_leftovers() {
    let dir = tempfile::tempdir().unwrap();
    let cwd = dir.path().canonicalize().unwrap();
    std::fs::write(
        cwd.join("out.json"),
        "old content that is much longer than nothing",
    )
    .unwrap();
    let s = create_summary(vec![], 1.0);
    let g: GlobalConfig =
        serde_json::from_value(json!({ "output": { "saveToFile": "out.json" } })).unwrap();
    save_summary(&s, &g, &cwd, None).unwrap();
    let text = std::fs::read_to_string(cwd.join("out.json")).unwrap();
    assert!(
        text.starts_with("{\n  \"total\": 0") && text.ends_with('}'),
        "{text}"
    );
    let names: Vec<String> = std::fs::read_dir(&cwd)
        .unwrap()
        .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
        .collect();
    assert_eq!(names, ["out.json"], "temp file left behind");
}

// ---------- loader ----------

fn write(dir: &Path, name: &str, body: &str) -> PathBuf {
    let p = dir.join(name);
    std::fs::write(&p, body).unwrap();
    p
}

#[test]
fn load_run_interpolates_merges_and_keeps_store_refs() {
    let dir = tempfile::tempdir().unwrap();
    let cwd = dir.path().canonicalize().unwrap();
    write(&cwd, ".env", "TOKEN=fromdotenv\n");
    write(&cwd, ".env.staging", "STAGE=stg\n");
    let a = write(
        &cwd,
        "a.yaml",
        "global:\n  env: { environment: staging }\n  defaults: { timeout: 1000 }\n  variables: { BASE: http://h }\nrequests:\n  - url: \"${BASE}/a\"\n    headers: { A: \"${TOKEN}\", S: \"${STAGE}\", P: \"${PROC}\" }\n    timeout: \"${store.t}\"\n",
    );
    let sub = cwd.join("sub");
    std::fs::create_dir(&sub).unwrap();
    let b = write(&sub, "b.yaml", "request:\n  url: http://b\n");
    let env = |k: &str| (k == "PROC").then(|| "proc".to_string());
    let cli = crate::config::CliOptions {
        retries: Some(2),
        ..Default::default()
    };
    let run = load_run(&[a.clone(), b.clone()], None, &cli, &env, &cwd).unwrap();
    assert_eq!(run.requests.len(), 2);
    let r0 = &run.requests[0].raw;
    assert_eq!(r0["url"], "http://h/a");
    assert_eq!(
        r0["headers"],
        json!({ "A": "fromdotenv", "S": "stg", "P": "proc" })
    );
    assert_eq!(r0["timeout"], "${store.t}");
    assert_eq!(r0["retry"]["count"], 2);
    assert_eq!(r0["sourceFile"], a.to_string_lossy().as_ref());
    // file a's global.defaults fold into later files (TS mergedConfig.defaults)
    assert_eq!(run.requests[1].raw["timeout"], 1000);
    assert_eq!(
        run.global.env.as_ref().unwrap().environment.as_deref(),
        Some("staging")
    );
    assert_eq!(
        run.global.variables.as_ref().unwrap()["TOKEN"],
        "fromdotenv"
    );
    assert_eq!(run.global.variables.as_ref().unwrap()["BASE"], "http://h");
    assert_eq!(run.dotenv["STAGE"], "stg");
}

#[test]
fn load_run_reports_file_errors() {
    let dir = tempfile::tempdir().unwrap();
    let bad = write(dir.path(), "bad.yaml", "requests: [\n");
    let env = |_: &str| None;
    let e = load_run(
        std::slice::from_ref(&bad),
        None,
        &Default::default(),
        &env,
        dir.path(),
    )
    .unwrap_err();
    assert_eq!(e.file, bad);
    let missing = dir.path().join("missing.yaml");
    assert!(load_run(&[missing], None, &Default::default(), &env, dir.path()).is_err());
}

#[test]
fn load_run_then_executor_end_to_end() {
    let srv = TestServer::start();
    let dir = tempfile::tempdir().unwrap();
    let cwd = dir.path().canonicalize().unwrap();
    let f = write(
        &cwd,
        "t.yaml",
        &format!(
            "global:\n  variables: {{ U: \"{}\" }}\nrequests:\n  - url: \"${{U}}/json/user\"\n    store: {{ id: body.id }}\n  - url: \"${{U}}/echo\"\n    params: {{ id: \"${{store.id}}\" }}\n    expect: {{ status: 200 }}\n",
            srv.base_url
        ),
    );
    let env = |_: &str| None;
    let mut rec = Rec::default();
    let run = load_run(&[f], None, &Default::default(), &env, &cwd).unwrap();
    let ex = Executor::new(run.global).with_cwd(&cwd);
    let s = ex.run(run.requests, &mut rec);
    let global = ex.global;
    assert_eq!(
        s.successful,
        2,
        "{:?}",
        s.results.iter().map(|r| &r.error).collect::<Vec<_>>()
    );
    assert_eq!(s.results[1].body.as_ref().unwrap()["query"]["id"], "1");
    assert!(global.security.is_some());
}

// ---------- fix round 1 ----------

#[test]
fn parallel_unbounded_failure_still_runs_and_reports_all() {
    // TS `Promise.all`: without a concurrency limit nothing stops early.
    let srv = TestServer::start();
    for limit in [json!(null), json!(3), json!(10)] {
        let ex = exec(json!({ "execution": PAR, "maxConcurrency": limit }));
        let (s, rec) = run(
            &ex,
            vec![
                json!({ "url": "file:///etc/passwd" }),
                json!({ "url": format!("{}/json/user", srv.base_url) }),
                json!({ "url": "http://x", "timeout": "abc" }),
            ],
        );
        assert_eq!((s.total, s.successful, s.failed), (3, 1, 2), "{limit}");
        assert_eq!(rec.stops, 0);
    }
}

#[test]
fn sequential_cancel_interrupts_in_flight_request() {
    let srv = TestServer::start();
    let ex = exec(json!({}));
    cancel_after(&ex, 300);
    let t = Instant::now();
    let (s, _) = run(
        &ex,
        vec![
            json!({ "url": format!("{}/delay/1", srv.base_url) }),
            json!({ "url": format!("{}/delay/5000", srv.base_url) }),
            json!({ "url": format!("{}/json/user", srv.base_url) }),
        ],
    );
    assert!(
        t.elapsed() < Duration::from_millis(1000),
        "{:?}",
        t.elapsed()
    );
    assert_eq!(s.total, 1);
    assert!(s.results[0].success);
}

#[test]
fn missing_form_file_fails_fast_without_retry() {
    for mode in both() {
        let ex = exec(json!({ "execution": mode }));
        let t = Instant::now();
        let (s, rec) = run(
            &ex,
            vec![
                json!({ "url": "http://127.0.0.1:9/", "formData": { "f": { "file": "nope-missing.bin" } },
                "retry": { "count": 3, "delay": 1000 } }),
            ],
        );
        assert!(rec.retries.is_empty(), "{mode}");
        assert!(t.elapsed() < Duration::from_millis(900));
        assert!(
            s.results[0]
                .error
                .as_deref()
                .unwrap()
                .starts_with("File(s) not found")
        );
    }
}

#[test]
fn huge_retry_delays_do_not_panic_and_stay_cancellable() {
    let srv = TestServer::start();
    for mode in both() {
        let ex = exec(json!({ "execution": mode }));
        cancel_after(&ex, 200);
        let t = Instant::now();
        let (s, rec) = run(
            &ex,
            vec![json!({ "url": format!("{}/status/503", srv.base_url),
                "retry": { "count": 2, "delay": 18446744073709551615u64, "backoff": 1e300 } })],
        );
        assert!(t.elapsed() < Duration::from_millis(1500));
        assert_eq!(s.total, 0, "{mode}");
        assert_eq!(rec.retries.len(), 1);
        assert_eq!(rec.retries[0].2, retry::MAX_DELAY_MS as u64);
    }
}

// ---------- fix A: parallel hang ----------

/// Regression for the parity "parallel hang": several parallel requests (some
/// failing with 404, no continueOnError) against a fresh test server, whose
/// keep-alive connections stay open in libcurl's cache. Each round runs under
/// a watchdog so a regression fails instead of hanging the suite.
#[test]
fn parallel_burst_with_failures_terminates() {
    for round in 0..20 {
        let srv = TestServer::start();
        let base = srv.base_url.clone();
        let (tx, rx) = std::sync::mpsc::channel();
        std::thread::spawn(move || {
            let ex = exec(json!({ "execution": PAR }));
            let mut reqs =
                vec![json!({ "url": format!("{base}/jp/posts/1"), "expect": { "status": 200 } })];
            for i in 0..7 {
                reqs.push(
                    json!({ "url": format!("{base}/jp/users/x{i}"), "expect": { "status": 200 } }),
                );
            }
            let (s, _) = run(&ex, reqs);
            let _ = tx.send((s.total, s.successful, s.failed));
        });
        let got = rx
            .recv_timeout(Duration::from_secs(10))
            .unwrap_or_else(|_| panic!("round {round}: parallel run did not terminate"));
        assert_eq!(got, (8, 1, 7), "round {round}");
        drop(srv);
    }
}

#[test]
fn complete_moves_the_response_and_keeps_it_only_for_store() {
    let resp = || Response {
        status: 200,
        headers: Default::default(),
        header_history: vec![],
        body: json!({ "id": 7 }),
        metrics: Default::default(),
    };
    let ex = exec(json!({}));
    let plain = RequestConfig {
        url: "http://x".into(),
        ..Default::default()
    };
    let policy = retry::RetryPolicy::for_request(&plain);
    let (r, kept) = ex.complete(&plain, &policy, Ok(resp()), Instant::now());
    assert!(r.success);
    assert_eq!(r.body, Some(json!({ "id": 7 })));
    assert!(kept.is_none());

    let storing = RequestConfig {
        store: Some(
            [("id".to_string(), "body.id".to_string())]
                .into_iter()
                .collect(),
        ),
        ..plain.clone()
    };
    let (r, kept) = ex.complete(&storing, &policy, Ok(resp()), Instant::now());
    assert_eq!(r.body, Some(json!({ "id": 7 })));
    assert_eq!(kept.unwrap().body, json!({ "id": 7 }));

    // A failed result never hands the response to the store.
    let failing = RequestConfig {
        expect: Some(serde_json::from_value(json!({ "status": 201 })).unwrap()),
        ..storing
    };
    let (r, kept) = ex.complete(&failing, &policy, Ok(resp()), Instant::now());
    assert!(!r.success);
    assert!(kept.is_none());
}
