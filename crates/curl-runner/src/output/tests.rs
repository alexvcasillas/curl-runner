//! Ports of `logger-quiet.test.ts` and `dry-run-logger.test.ts`, plus layout
//! checks against TS output captured from the Bun CLI.

use super::*;
use curl_runner_core::model::{HeaderValue, Metrics};
use indexmap::IndexMap;
use serde_json::json;

fn out(over: impl FnOnce(&mut OutputConfig)) -> OutputConfig {
    let mut o = OutputConfig::default();
    over(&mut o);
    o
}

fn logger(o: OutputConfig) -> Logger {
    Logger::new(OutCfg::from_config(Some(&o)), false, None)
}

fn quiet(format: &str) -> Logger {
    logger(out(|o| {
        o.quiet = Some(true);
        o.format = Some(format.into());
    }))
}

fn req(name: Option<&str>, url: &str) -> RequestConfig {
    RequestConfig {
        name: name.map(String::from),
        url: url.into(),
        method: Some("GET".into()),
        ..Default::default()
    }
}

fn make_result() -> ExecutionResult {
    let mut r = ExecutionResult::new(req(Some("Test"), "https://example.com"), true);
    r.status = Some(200);
    r.body = Some(json!("{\"ok\":true}"));
    let mut h = IndexMap::new();
    h.insert(
        "content-type".to_string(),
        HeaderValue::One("application/json".into()),
    );
    r.headers = Some(h);
    r.metrics = Some(Metrics {
        duration: 100.0,
        size: Some(50.0),
        ..Default::default()
    });
    r
}

fn dry_result(name: Option<&str>, url: &str) -> ExecutionResult {
    let mut r = ExecutionResult::new(req(name, url), true);
    r.dry_run = Some(true);
    r.metrics = Some(Metrics::default());
    r
}

fn summary(results: Vec<ExecutionResult>) -> ExecutionSummary {
    curl_runner_core::executor::create_summary(results, 100.0)
}

fn run(f: impl FnOnce(&mut Console)) -> Captured {
    let (mut c, cap) = Console::capture();
    f(&mut c);
    cap
}

// ---- logger-quiet.test.ts ----

#[test]
fn quiet_suppresses_execution_start_in_all_formats() {
    for f in ["pretty", "json", "raw"] {
        let cap = run(|c| quiet(f).log_execution_start(c, 1, "sequential"));
        assert_eq!(cap.stdout(), "", "{f}");
    }
}

#[test]
fn quiet_suppresses_methods() {
    let l = quiet("pretty");
    let cap = run(|c| {
        l.log_execution_start(c, 3, "parallel");
        l.log_request_complete(c, &make_result());
        l.log_summary(c, &summary(vec![make_result()]), false);
        l.log_info(c, "some info");
        l.log_success(c, "done");
        l.log_warning(c, "careful");
        l.log_retry(c, 1, 3);
        l.log_skipped(
            c,
            &req(Some("Test"), "https://example.com"),
            1,
            Some("condition not met"),
        );
        l.log_file_header(c, "test.yaml", 3);
    });
    assert_eq!(cap.stdout(), "");
    assert_eq!(cap.stderr(), "");
}

#[test]
fn log_error_shows_when_quiet() {
    let cap = run(|c| quiet("pretty").log_error(c, "something broke"));
    assert!(cap.stderr().contains("something broke"));
}

#[test]
fn quiet_suppresses_raw_and_json_results_and_summaries() {
    for f in ["raw", "json"] {
        let l = quiet(f);
        let mut r = make_result();
        r.body = Some(json!("{\"data\":\"value\"}"));
        let cap = run(|c| {
            l.log_request_complete(c, &r);
            l.log_summary(c, &summary(vec![make_result()]), false);
        });
        assert_eq!(cap.stdout(), "", "{f}");
    }
}

#[test]
fn dry_run_command_shows_when_quiet() {
    let l = logger(out(|o| {
        o.quiet = Some(true);
        o.format = Some("pretty".into());
        o.dry_run = Some(true);
    }));
    let cap = run(|c| l.log_command(c, "curl https://example.com"));
    assert!(cap.stdout().contains("curl"));
}

// ---- dry-run-logger.test.ts ----

#[test]
fn dry_run_always_shows_command() {
    for verbose in [None, Some(false)] {
        let l = logger(out(|o| {
            o.dry_run = Some(true);
            o.verbose = verbose;
        }));
        let cap = run(|c| l.log_command(c, "curl -X GET https://api.example.com"));
        assert!(cap.stdout().contains("Command"));
        assert!(cap.stdout().contains("curl -X GET https://api.example.com"));
    }
}

#[test]
fn dry_run_status_in_all_pretty_levels() {
    for level in ["minimal", "standard", "detailed"] {
        let l = logger(out(|o| {
            o.format = Some("pretty".into());
            o.pretty_level = Some(level.into());
        }));
        let cap = run(|c| {
            l.log_request_complete(c, &dry_result(Some("Test"), "https://api.example.com"))
        });
        assert!(cap.stdout().contains("DRY-RUN"), "{level}");
    }
}

#[test]
fn dry_run_json_has_dry_run_and_no_status() {
    let l = logger(out(|o| o.format = Some("json".into())));
    let cap =
        run(|c| l.log_request_complete(c, &dry_result(Some("Test"), "https://api.example.com")));
    let v: serde_json::Value = serde_json::from_str(&cap.stdout()).unwrap();
    assert_eq!(v["dryRun"], true);
    assert!(v.get("status").is_none());
}

#[test]
fn dry_run_minimal_checkmark_and_no_status_code() {
    let l = logger(out(|o| o.pretty_level = Some("minimal".into())));
    let cap =
        run(|c| l.log_request_complete(c, &dry_result(Some("Test"), "https://api.example.com")));
    let s = cap.stdout();
    assert!(s.contains('✓') && s.contains("DRY-RUN"));
    assert!(!regex::Regex::new(r"Status.*\d{3}").unwrap().is_match(&s));
}

#[test]
fn dry_run_standard_has_no_body() {
    let l = logger(out(|o| {
        o.pretty_level = Some("standard".into());
        o.show_body = Some(true);
    }));
    let cap =
        run(|c| l.log_request_complete(c, &dry_result(Some("Test"), "https://api.example.com")));
    assert!(!cap.stdout().contains("Response Body"));
}

#[test]
fn dry_run_summaries() {
    let l = logger(out(|o| o.format = Some("json".into())));
    let s = summary(vec![
        dry_result(None, "https://api.example.com/1"),
        dry_result(None, "https://api.example.com/2"),
    ]);
    let cap = run(|c| l.log_summary(c, &s, false));
    let v: serde_json::Value = serde_json::from_str(&cap.stdout()).unwrap();
    assert_eq!(v["results"][0]["dryRun"], true);
    assert_eq!(v["results"][1]["dryRun"], true);

    let l = logger(out(|o| o.pretty_level = Some("minimal".into())));
    let s = summary(
        (1..=3)
            .map(|i| dry_result(None, &format!("https://x/{i}")))
            .collect(),
    );
    let cap = run(|c| l.log_summary(c, &s, false));
    assert!(cap.stdout().contains("3 requests completed successfully"));
}

#[test]
fn dry_run_raw_prints_nothing() {
    let l = logger(out(|o| o.format = Some("raw".into())));
    let cap =
        run(|c| l.log_request_complete(c, &dry_result(Some("Test"), "https://api.example.com")));
    assert_eq!(cap.stdout(), "");
}

// ---- layout (captured from the TS CLI) ----

fn failing_result() -> ExecutionResult {
    let mut r = ExecutionResult::new(req(Some("Wrong status"), "http://h/status/404"), false);
    r.request.source_file = Some("dir/summary.yaml".into());
    r.status = Some(404);
    r.body = Some(json!({"status": 404}));
    r.error = Some("Expected status 200, got 404".into());
    r.metrics = Some(Metrics {
        duration: 7.0,
        size: Some(14.0),
        ..Default::default()
    });
    r
}

#[test]
fn minimal_layout_matches_ts() {
    let l = logger(OutputConfig::default());
    let cap = run(|c| l.log_request_complete(c, &failing_result()));
    assert_eq!(
        cap.stdout(),
        "x Wrong status [summary]\n   ├─ GET: http://h/status/404\n   ├─ x Status: 404\n   └─ Duration: 7ms | 14.00 B\n\n  ✗ Error: Expected status 200, got 404\n\n"
    );
}

#[test]
fn standard_layout_matches_ts() {
    let l = logger(out(|o| o.pretty_level = Some("standard".into())));
    let cap = run(|c| l.log_request_complete(c, &failing_result()));
    assert_eq!(
        cap.stdout(),
        "x Wrong status\n   ├─ URL: http://h/status/404\n   ├─ Method: GET\n   ├─ Status: 404\n   ├─ Duration: 7ms\n   └─ Response Body:\n      {\n        \"status\": 404\n      }\n\n  ✗ Error: Expected status 200, got 404\n\n"
    );
}

#[test]
fn detailed_layout_has_headers_and_metrics() {
    let l = logger(out(|o| o.pretty_level = Some("detailed".into())));
    let mut r = make_result();
    r.metrics.as_mut().unwrap().dns_lookup = Some(0.06);
    r.metrics.as_mut().unwrap().tls_handshake = Some(0.0);
    let cap = run(|c| l.log_request_complete(c, &r));
    let s = cap.stdout();
    assert!(s.contains("   ├─ Headers:\n   │  └─ content-type: application/json\n"));
    assert!(s.contains("   └─ Metrics:\n      ├─ Request Duration: 100ms\n      ├─ Response Size: 50.00 B\n      └─ DNS Lookup: 0ms\n"));
    assert!(!s.contains("TLS Handshake"));
}

#[test]
fn body_truncated_to_ten_lines_unless_verbose() {
    let mut r = make_result();
    r.body = Some(json!({"a": [1, 2, 3, 4, 5, 6, 7, 8, 9, 10]}));
    let std = logger(out(|o| o.pretty_level = Some("standard".into())));
    let cap = run(|c| std.log_request_complete(c, &r));
    assert!(cap.stdout().contains("... (4 more lines)"));
    let verbose = logger(out(|o| {
        o.pretty_level = Some("standard".into());
        o.verbose = Some(true);
    }));
    let cap = run(|c| verbose.log_request_complete(c, &r));
    assert!(!cap.stdout().contains("more lines"));
}

#[test]
fn multiple_validation_errors_listed() {
    let mut r = failing_result();
    r.error = Some("Expected status 200, got 404; Expected id to be 1, got 2; body missing".into());
    let cap = run(|c| logger(OutputConfig::default()).log_request_complete(c, &r));
    assert!(cap.stdout().contains(
        "  ✗ Validation Errors:\n    • status: expected 200, got 404\n    • id: expected 1, got 2\n    • body missing\n"
    ));
}

#[test]
fn summaries_match_ts() {
    let s = summary(vec![failing_result(), make_result()]);
    let cap = run(|c| logger(OutputConfig::default()).log_summary(c, &s, false));
    assert_eq!(cap.stdout(), "Summary: 1/2 requests completed, 1 failed\n");
    let cap = run(|c| logger(OutputConfig::default()).log_summary(c, &s, true));
    assert_eq!(
        cap.stdout(),
        "\n◆ Global Summary: 1/2 requests completed, 1 failed\n"
    );
    let detailed = logger(out(|o| o.pretty_level = Some("detailed".into())));
    let cap = run(|c| detailed.log_summary(c, &s, false));
    assert_eq!(
        cap.stdout(),
        "\nSummary: 1/2 requests completed, 1 failed (100ms)\n  • Wrong status: Expected status 200, got 404\n"
    );
}

#[test]
fn skipped_and_stored_and_stop_events() {
    let redactor = Arc::new(Mutex::new(Redactor::default()));
    let (c, cap) = Console::capture();
    let mut rep = Reporter::new(c, None, false, redactor);
    let mut r = ExecutionResult::new(req(None, "http://x"), true);
    r.skipped = Some(true);
    r.skip_reason = Some("condition not met: a == 1".into());
    rep.on_result(3, &r);
    let mut store = IndexMap::new();
    store.insert("id".to_string(), "x".repeat(60));
    rep.on_stored(&store);
    rep.on_stop();
    assert_eq!(
        cap.stdout(),
        format!(
            "⊘ Request 3 [SKIP]\n   └─ Reason: condition not met: a == 1\n\nℹ Stored: id = \"{}...\"\n",
            "x".repeat(50)
        )
    );
    assert_eq!(cap.stderr(), "✗ Stopping execution due to error\n");
}

#[test]
fn request_output_config_overrides_global() {
    let redactor = Arc::new(Mutex::new(Redactor::default()));
    let (c, cap) = Console::capture();
    let global = out(|o| o.format = Some("json".into()));
    let mut rep = Reporter::new(c, Some(global), false, redactor);
    let mut r = make_result();
    r.request.source_output_config = Some(out(|o| o.format = Some("raw".into())));
    rep.on_result(1, &r);
    assert_eq!(cap.stdout(), "{\"ok\":true}\n");
}

#[test]
fn printed_secrets_are_redacted() {
    let redactor = Arc::new(Mutex::new(Redactor::new(["s3cr3t-value".to_string()])));
    let (c, cap) = Console::capture();
    let mut rep = Reporter::new(
        c,
        Some(out(|o| o.pretty_level = Some("detailed".into()))),
        false,
        redactor,
    );
    let mut r = make_result();
    r.request.url = "http://h/?k=s3cr3t-value".into();
    r.body = Some(json!({"echo": "s3cr3t-value"}));
    rep.on_command(1, &r.request, "curl http://h/?k=s3cr3t-value");
    rep.on_result(1, &r);
    let s = cap.stdout();
    assert!(!s.contains("s3cr3t-value"), "{s}");
}

#[test]
fn json_result_shape_and_flags() {
    let l = logger(out(|o| {
        o.format = Some("json".into());
        o.show_headers = Some(true);
        o.show_metrics = Some(true);
    }));
    let v = l.result_json(&failing_result());
    let keys: Vec<&str> = v.as_object().unwrap().keys().map(String::as_str).collect();
    assert_eq!(
        keys,
        ["request", "success", "status", "body", "error", "metrics"]
    );
    let l = logger(out(|o| o.format = Some("json".into())));
    let v = l.result_json(&make_result());
    assert!(v.get("headers").is_none() && v.get("metrics").is_none());
    assert_eq!(
        v["request"],
        json!({"name": "Test", "url": "https://example.com", "method": "GET"})
    );
}

#[test]
fn colors_applied_when_enabled() {
    let l = Logger::new(OutCfg::from_config(None), true, None);
    let cap = run(|c| l.log_request_complete(c, &make_result()));
    assert!(
        cap.stdout()
            .starts_with("\x1b[32m✓\x1b[0m \x1b[1mTest\x1b[0m [inline]")
    );
}

#[test]
fn token_patterns_only_redacted_in_urls_and_commands() {
    let redactor = Arc::new(Mutex::new(Redactor::default()));
    let (c, cap) = Console::capture();
    let mut rep = Reporter::new(
        c,
        Some(out(|o| o.pretty_level = Some("detailed".into()))),
        false,
        redactor,
    );
    let mut r = make_result();
    r.request.url = "http://h/?token=abc123".into();
    r.body = Some(json!({"echo": "token=abc123"}));
    r.error = Some("got token=abc123".into());
    rep.on_command(1, &r.request, "curl http://h/?token=abc123");
    rep.on_result(1, &r);
    let s = cap.stdout();
    assert!(s.contains("    curl http://h/?token=[REDACTED]\n"), "{s}");
    assert!(s.contains("URL: http://h/?token=[REDACTED]\n"), "{s}");
    assert!(s.contains("\"echo\": \"token=abc123\""), "{s}");
    assert!(s.contains("got token=abc123"), "{s}");
}
