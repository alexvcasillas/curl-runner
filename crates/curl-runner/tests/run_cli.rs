//! End-to-end `curl-runner <files>` runs against `testkit::TestServer`.

use assert_cmd::Command;
use serde_json::Value;
use std::path::Path;
use testkit::TestServer;

const BIN: &str = env!("CARGO_BIN_EXE_curl-runner");

struct Env {
    server: TestServer,
    dir: tempfile::TempDir,
}

fn setup(files: &[(&str, &str)]) -> Env {
    let server = TestServer::start();
    let dir = tempfile::tempdir().unwrap();
    for (name, body) in files {
        let path = dir.path().join(name);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, body.replace("{BASE}", &server.base_url)).unwrap();
    }
    Env { server, dir }
}

impl Env {
    fn cmd(&self, args: &[&str]) -> Command {
        let mut c = Command::new(BIN);
        c.env_clear()
            .envs(testkit::system_env())
            .env("NO_COLOR", "1")
            .env("BASE_URL", &self.server.base_url)
            .current_dir(self.dir.path())
            .args(args);
        c
    }

    fn run(&self, args: &[&str]) -> (i32, String, String) {
        let out = self.cmd(args).output().unwrap();
        (
            out.status.code().unwrap_or(-1),
            String::from_utf8_lossy(&out.stdout).into_owned(),
            String::from_utf8_lossy(&out.stderr).into_owned(),
        )
    }

    fn path(&self, p: &str) -> std::path::PathBuf {
        self.dir.path().join(p)
    }
}

const PASS2: &str = r#"
requests:
  - name: One
    url: "{BASE}/json/user"
    expect: { status: 200 }
  - name: Two
    url: "{BASE}/json/users"
    expect: { status: 200 }
"#;

/// `n` requests of which `fail` expect the wrong status.
fn mixed(n: usize, fail: usize) -> String {
    let mut s = String::from("requests:\n");
    for i in 0..n {
        let status = if i < fail { 201 } else { 200 };
        s.push_str(&format!(
            "  - name: R{i}\n    url: \"{{BASE}}/json/user\"\n    expect: {{ status: {status} }}\n"
        ));
    }
    s
}

#[test]
fn exit_0_all_pass() {
    let e = setup(&[("t.yaml", PASS2)]);
    let (code, out, err) = e.run(&["t.yaml"]);
    assert_eq!(code, 0, "{out}{err}");
    assert!(out.contains("ℹ Found 1 YAML file(s)\nℹ Processing: t.yaml\n"));
    assert!(out.contains("✓ One [t]\n   ├─ GET: "), "{out}");
    assert!(out.contains("Summary: 2 requests completed successfully\n"));
    assert!(!out.contains('\x1b'), "NO_COLOR output has ANSI codes");
}

#[test]
fn exit_1_failure_default() {
    let e = setup(&[("t.yaml", &mixed(3, 1))]);
    let (code, out, err) = e.run(&["t.yaml"]);
    assert_eq!(code, 1);
    assert!(out.contains("x R0 [t]"), "{out}");
    assert!(out.contains("   ├─ x Status: 200\n"));
    assert!(out.contains("  ✗ Error: Expected status 201, got 200\n"));
    assert!(err.contains("✗ Stopping execution due to error"));
    // stops at the first failure
    assert!(
        out.contains("Summary: 0/1 request completed, 1 failed\n"),
        "{out}"
    );
}

#[test]
fn exit_0_continue_on_error() {
    let e = setup(&[("t.yaml", &mixed(3, 1))]);
    let (code, out, _) = e.run(&["t.yaml", "--continue-on-error"]);
    assert_eq!(code, 0);
    assert!(
        out.contains("Summary: 2/3 requests completed, 1 failed\n"),
        "{out}"
    );
}

#[test]
fn exit_1_strict_exit_with_continue() {
    let e = setup(&[("t.yaml", &mixed(3, 1))]);
    let (code, out, _) = e.run(&["t.yaml", "--continue-on-error", "--strict-exit"]);
    assert_eq!(code, 1);
    assert!(out.contains("Summary: 2/3 requests completed, 1 failed\n"));
}

#[test]
fn fail_on_threshold() {
    let e = setup(&[("t.yaml", &mixed(4, 3))]);
    let base = ["t.yaml", "--continue-on-error", "--fail-on"];
    assert_eq!(e.run(&[&base[..], &["3"]].concat()).0, 0);
    assert_eq!(e.run(&[&base[..], &["2"]].concat()).0, 1);
}

#[test]
fn fail_on_percentage() {
    let e = setup(&[("t.yaml", &mixed(4, 1))]);
    let base = ["t.yaml", "--continue-on-error", "--fail-on-percentage"];
    assert_eq!(e.run(&[&base[..], &["25"]].concat()).0, 0);
    assert_eq!(e.run(&[&base[..], &["20"]].concat()).0, 1);
}

/// Normalize what depends on time or the port: `duration`, `metrics.*`
/// values, `date` headers and the server origin.
fn normalize(v: &mut Value, base: &str) {
    match v {
        Value::Object(o) => {
            for (k, x) in o.iter_mut() {
                if k.eq_ignore_ascii_case("date") && x.is_string() {
                    *x = Value::from("DATE");
                } else if k == "metrics"
                    && let Value::Object(m) = x
                {
                    for mv in m.values_mut() {
                        *mv = Value::from(0);
                    }
                } else {
                    normalize(x, base);
                }
            }
        }
        Value::Array(a) => a.iter_mut().for_each(|x| normalize(x, base)),
        Value::String(s) => *s = s.replace(base, "http://SERVER"),
        _ => {}
    }
}

fn read_golden() -> Value {
    let p = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/golden/summary.json");
    serde_json::from_str(&std::fs::read_to_string(p).unwrap()).unwrap()
}

#[test]
fn output_json_shape() {
    let fixture = std::fs::read_to_string(
        Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/summary.yaml"),
    )
    .unwrap();
    let e = setup(&[("summary.yaml", &fixture)]);
    let (code, out, err) = e.run(&[
        "summary.yaml",
        "--continue-on-error",
        "--output",
        "out.json",
        "-q",
    ]);
    assert_eq!(code, 0, "{out}{err}");
    let text = std::fs::read_to_string(e.path("out.json")).unwrap();
    assert!(
        text.starts_with("{\n  \"total\": 5,\n"),
        "2-space pretty JSON"
    );
    let mut v: Value = serde_json::from_str(&text).unwrap();
    normalize(&mut v, &e.server.base_url);
    v["duration"] = Value::from(0);
    // Order-insensitive on purpose: `serde_json::Value` object equality
    // ignores key order. Rust writes keys in a fixed struct order, TS in
    // YAML/insertion order; see "--output JSON key order" in
    // crates/CHANGES-FROM-TS.md.
    assert_eq!(
        v,
        read_golden(),
        "{}",
        serde_json::to_string_pretty(&v).unwrap()
    );
}

#[test]
fn ci_style_invocation() {
    let e = setup(&[("a.yaml", PASS2), ("b.yaml", &mixed(2, 1))]);
    let (code, out, err) = e.run(&[
        "a.yaml",
        "b.yaml",
        "--continue-on-error",
        "--strict-exit",
        "--retries",
        "3",
        "--retry-delay",
        "10",
        "--timeout",
        "30000",
        "--output-format",
        "pretty",
        "--pretty-level",
        "minimal",
        "--output",
        "curl-runner-results.json",
    ]);
    assert_eq!(code, 1, "{out}{err}");
    assert!(out.contains("\n▶ a.yaml (2 requests)\n"), "{out}");
    assert!(out.contains("\n▶ b.yaml (2 requests)\n"));
    assert!(out.contains("x R0 [b]"));
    assert!(out.contains("  ✗ Error: Expected status 201, got 200\n"));
    assert!(out.contains("ℹ Results saved to curl-runner-results.json\n"));
    assert!(
        out.contains("\n◆ Global Summary: 3/4 requests completed, 1 failed\n"),
        "{out}"
    );
    // One aggregated results file for all files (TS kept only the last).
    assert_eq!(out.matches("Results saved to").count(), 1, "{out}");
    let v: Value =
        serde_json::from_str(&std::fs::read_to_string(e.path("curl-runner-results.json")).unwrap())
            .unwrap();
    assert_eq!(
        (v["total"].as_u64(), v["failed"].as_u64()),
        (Some(4), Some(1))
    );
}

#[test]
fn multi_file_groups_stop_per_file() {
    let e = setup(&[("a.yaml", &mixed(2, 1)), ("b.yaml", PASS2)]);
    let (code, out, err) = e.run(&["a.yaml", "b.yaml"]);
    assert_eq!(code, 1);
    assert!(err.contains("Stopping execution due to error"));
    // a.yaml stopped after R0, b.yaml still ran.
    assert!(
        out.contains("Summary: 0/1 request completed, 1 failed\n\n\n▶ b.yaml (2 requests)\n"),
        "{out}"
    );
    assert!(out.contains("◆ Global Summary: 2/3 requests completed, 1 failed"));
}

#[test]
fn json_output_format_prints_results_and_summary() {
    let e = setup(&[("t.yaml", &mixed(2, 1))]);
    let (code, out, _) = e.run(&["t.yaml", "--continue-on-error", "--output-format", "json"]);
    assert_eq!(code, 0);
    let docs: Vec<Value> = serde_json::Deserializer::from_str(&out)
        .into_iter::<Value>()
        .collect::<Result<_, _>>()
        .unwrap_or_else(|e| panic!("{e}: {out}"));
    assert_eq!(docs.len(), 3, "{out}");
    assert_eq!(docs[0]["request"]["name"], "R0");
    assert_eq!(docs[0]["error"], "Expected status 201, got 200");
    assert!(docs[0].get("metrics").is_none());
    assert_eq!(docs[2]["summary"]["total"], 2);
    assert_eq!(docs[2]["results"].as_array().unwrap().len(), 2);
}

#[test]
fn raw_output_format_prints_bodies_only() {
    let e = setup(&[("t.yaml", PASS2)]);
    let (code, out, _) = e.run(&["t.yaml", "--output-format", "raw"]);
    assert_eq!(code, 0);
    let lines: Vec<&str> = out.lines().collect();
    assert_eq!(lines.len(), 2, "{out}");
    assert!(!lines[0].contains(": "), "compact JSON: {out}");
    let first: Value = serde_json::from_str(lines[0]).unwrap();
    let second: Value = serde_json::from_str(lines[1]).unwrap();
    assert_eq!(first["name"], "Test User");
    assert_eq!(
        second,
        serde_json::json!([{"id": 1, "name": "Test User"}, {"id": 2, "name": "Other User"}])
    );
}

#[test]
fn standard_and_detailed_levels() {
    let e = setup(&[("t.yaml", &mixed(1, 1))]);
    let (_, out, _) = e.run(&["t.yaml", "--pretty-level", "standard"]);
    assert!(
        out.contains("\nExecuting 1 request(s) in sequential mode\n\n"),
        "{out}"
    );
    assert!(out.contains("   ├─ Method: GET\n"));
    assert!(out.contains("   └─ Response Body:\n"));
    let (_, out, _) = e.run(&["t.yaml", "--pretty-level", "detailed"]);
    assert!(out.contains("  Command:\n    curl "), "{out}");
    assert!(out.contains("   ├─ Headers:\n"));
    assert!(out.contains("   └─ Metrics:\n      ├─ Request Duration: "));
    assert!(out.contains("  • R0: Expected status 201, got 200\n"));
}

#[test]
fn dry_run_prints_commands_and_exits_0() {
    let e = setup(&[("t.yaml", &mixed(2, 2))]);
    let (code, out, _) = e.run(&["t.yaml", "--dry-run"]);
    assert_eq!(code, 0);
    assert!(out.contains("  Command:\n    curl "));
    assert!(out.contains("   ├─ ✓ Status: DRY-RUN\n"));
    assert!(out.contains("Summary: 2 requests completed successfully"));
}

#[test]
fn directory_and_quiet() {
    let e = setup(&[("tests/a.yaml", PASS2), ("tests/sub/b.yaml", PASS2)]);
    let (code, out, _) = e.run(&["tests/", "-q"]);
    assert_eq!((code, out.as_str()), (0, ""));
    let (_, out, _) = e.run(&["tests", "--all"]);
    assert!(out.contains("Found 2 YAML file(s)"), "{out}");
    assert!(out.contains("Processing: tests/sub/b.yaml"));
}

#[test]
fn no_files_and_bad_yaml_exit_1() {
    let e = setup(&[("bad.yaml", "requests: [ {")]);
    let (code, _, err) = e.run(&["missing.yaml"]);
    assert_eq!(code, 1);
    assert!(err.contains("✗ No YAML files found"));
    let (code, _, err) = e.run(&["bad.yaml"]);
    assert_eq!(code, 1);
    assert!(err.contains("bad.yaml"), "{err}");
    let e = setup(&[("empty.yaml", "global: {}\n")]);
    let (code, _, err) = e.run(&["empty.yaml"]);
    assert_eq!(code, 1);
    assert!(err.contains("No requests found in YAML files"));
}

#[test]
fn secrets_from_env_file_are_redacted_in_output() {
    let e = setup(&[
        (".env", "SECRET_TOKEN=tok-1234567890\n"),
        (
            "t.yaml",
            "requests:\n  - name: S\n    url: \"{BASE}/json/user?t=${SECRET_TOKEN}\"\n",
        ),
    ]);
    let (code, out, _) = e.run(&["t.yaml", "--pretty-level", "detailed", "-o", "r.json"]);
    assert_eq!(code, 0, "{out}");
    assert!(!out.contains("tok-1234567890"), "{out}");
    let saved = std::fs::read_to_string(e.path("r.json")).unwrap();
    assert!(!saved.contains("tok-1234567890"));
}

#[cfg(unix)]
#[test]
fn sigint_exits_130() {
    use std::time::{Duration, Instant};
    let e = setup(&[(
        "t.yaml",
        "requests:\n  - name: Fast\n    url: \"{BASE}/json/user\"\n  - name: Slow\n    url: \"{BASE}/delay/5000\"\n",
    )]);
    let child = std::process::Command::new(BIN)
        .env_clear()
        .envs(testkit::system_env())
        .env("NO_COLOR", "1")
        .current_dir(e.dir.path())
        .arg("t.yaml")
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .unwrap();
    std::thread::sleep(Duration::from_millis(700));
    let start = Instant::now();
    let ok = std::process::Command::new("kill")
        .args(["-INT", &child.id().to_string()])
        .status()
        .unwrap();
    assert!(ok.success());
    let out = child.wait_with_output().unwrap();
    assert!(
        start.elapsed() < Duration::from_secs(2),
        "{:?}",
        start.elapsed()
    );
    assert_eq!(out.status.code(), Some(130));
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("✓ Fast [t]"), "{stdout}");
    assert!(
        stdout.contains("Summary: 1 request completed successfully"),
        "{stdout}"
    );
    assert!(String::from_utf8_lossy(&out.stderr).contains("Interrupted"));
}

#[test]
fn stored_values_follow_their_result() {
    let e = setup(&[(
        "t.yaml",
        "requests:\n  - name: Get\n    url: \"{BASE}/json/user\"\n    store: { userId: body.id }\n",
    )]);
    let (code, out, _) = e.run(&["t.yaml"]);
    assert_eq!(code, 0);
    assert!(
        out.contains(" | 68.00 B\n\nℹ Stored: userId = \"1\"\nSummary: "),
        "{out}"
    );
}

#[test]
fn output_file_aggregates_all_files() {
    let e = setup(&[("a.yaml", &mixed(2, 1)), ("b.yaml", &mixed(3, 0))]);
    let (code, out, _) = e.run(&[
        "a.yaml",
        "b.yaml",
        "--continue-on-error",
        "--output",
        "r.json",
    ]);
    assert_eq!(code, 0, "{out}");
    let v: Value =
        serde_json::from_str(&std::fs::read_to_string(e.path("r.json")).unwrap()).unwrap();
    assert_eq!(
        (
            v["total"].as_u64(),
            v["successful"].as_u64(),
            v["failed"].as_u64(),
            v["skipped"].as_u64()
        ),
        (Some(5), Some(4), Some(1), Some(0))
    );
    let files: Vec<&str> = v["results"]
        .as_array()
        .unwrap()
        .iter()
        .map(|r| r["request"]["sourceFile"].as_str().unwrap())
        .collect();
    assert_eq!(files, ["a.yaml", "a.yaml", "b.yaml", "b.yaml", "b.yaml"]);
    assert!(v["duration"].as_f64().unwrap() > 0.0);
    // Exit code is computed on the aggregate too.
    let (code, _, _) = e.run(&["a.yaml", "b.yaml", "--continue-on-error", "--fail-on", "0"]);
    assert_eq!(code, 1);
}

#[test]
fn redaction_scope_matches_ts_plus_explicit_secrets() {
    let e = setup(&[
        (".env", "SECRET_TOKEN=tok-1234567890\n"),
        (
            "t.yaml",
            "requests:\n  - name: Echo\n    url: \"{BASE}/echo\"\n    method: POST\n    body:\n      aws: AKIAABCDEFGHIJKLMNOP\n      s: ${SECRET_TOKEN}\n",
        ),
    ]);
    let (code, out, _) = e.run(&["t.yaml", "--pretty-level", "detailed", "-o", "r.json"]);
    assert_eq!(code, 0, "{out}");
    // A token-like string in a body: TS does not redact console bodies.
    assert!(out.contains("AKIAABCDEFGHIJKLMNOP"), "{out}");
    // An explicit SECRET_ value is redacted everywhere.
    assert!(!out.contains("tok-1234567890"), "{out}");
    let saved = std::fs::read_to_string(e.path("r.json")).unwrap();
    assert!(!saved.contains("tok-1234567890"));
    // The results file is fully redacted, as in TS.
    assert!(!saved.contains("AKIAABCDEFGHIJKLMNOP"));
}

#[cfg(unix)]
#[test]
fn sigint_still_writes_output_file() {
    use std::time::Duration;
    let e = setup(&[
        (
            "a.yaml",
            "requests:\n  - name: Fast\n    url: \"{BASE}/json/user\"\n",
        ),
        (
            "b.yaml",
            "requests:\n  - name: Slow\n    url: \"{BASE}/delay/5000\"\n",
        ),
    ]);
    let child = std::process::Command::new(BIN)
        .env_clear()
        .envs(testkit::system_env())
        .env("NO_COLOR", "1")
        .current_dir(e.dir.path())
        .args(["a.yaml", "b.yaml", "--output", "r.json"])
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .unwrap();
    std::thread::sleep(Duration::from_millis(700));
    std::process::Command::new("kill")
        .args(["-INT", &child.id().to_string()])
        .status()
        .unwrap();
    let out = child.wait_with_output().unwrap();
    assert_eq!(out.status.code(), Some(130));
    let v: Value =
        serde_json::from_str(&std::fs::read_to_string(e.path("r.json")).unwrap()).unwrap();
    assert_eq!(v["total"].as_u64(), Some(1));
    assert_eq!(v["results"][0]["request"]["name"], "Fast");
}

#[test]
fn later_request_secrets_are_masked_in_earlier_output() {
    let e = setup(&[(
        "t.yaml",
        "requests:\n  - name: Echo\n    url: \"{BASE}/echo\"\n    method: POST\n    body: { leaked: tok-LATER-abcdef }\n  - name: Authed\n    url: \"{BASE}/json/user\"\n    auth: { type: bearer, token: tok-LATER-abcdef }\n",
    )]);
    let (code, out, _) = e.run(&["t.yaml", "--pretty-level", "detailed"]);
    assert_eq!(code, 0, "{out}");
    assert!(!out.contains("tok-LATER-abcdef"), "{out}");
    assert!(out.contains("\"leaked\": \"[REDACTED]\""), "{out}");
}

// ---------- snapshots ----------

const SNAP: &str = r#"
requests:
  - name: User
    url: "{BASE}/json/user"
    snapshot: true
"#;

#[test]
fn snapshot_flag_creates_then_matches() {
    let e = setup(&[("t.yaml", SNAP)]);
    let (code, out, err) = e.run(&["t.yaml", "--snapshot"]);
    assert_eq!(code, 0, "{out}{err}");
    assert!(
        out.contains("  NEW Snapshot created for \"User\"\n"),
        "{out}"
    );
    let file = std::fs::read_to_string(e.path("__snapshots__/t.snap.json")).unwrap();
    assert!(file.starts_with("{\n  \"version\": 1,\n  \"snapshots\": {\n    \"User\": {\n"));
    let (code, out, _) = e.run(&["t.yaml", "--snapshot"]);
    assert_eq!(code, 0);
    assert!(
        out.contains("  PASS Snapshot matches for \"User\"\n"),
        "{out}"
    );
    assert_eq!(
        std::fs::read_to_string(e.path("__snapshots__/t.snap.json")).unwrap(),
        file,
        "a passing run does not rewrite the file"
    );
}

#[test]
fn snapshot_ci_fails_on_missing_snapshot_without_writing() {
    let e = setup(&[("t.yaml", SNAP)]);
    let (code, out, _) = e.run(&["t.yaml", "--snapshot", "--ci-snapshot"]);
    assert_eq!(code, 1, "{out}");
    assert!(out.contains("FAIL Snapshot mismatch for \"User\""), "{out}");
    assert!(out.contains("(root):\n    - \"snapshot\"\n"), "{out}");
    assert!(!e.path("__snapshots__").exists());
}

#[test]
fn snapshot_mismatch_fails_and_update_flags_rewrite() {
    let e = setup(&[("t.yaml", SNAP)]);
    assert_eq!(e.run(&["t.yaml", "--snapshot"]).0, 0);
    let path = e.path("__snapshots__/t.snap.json");
    let stale = std::fs::read_to_string(&path)
        .unwrap()
        .replace("\"id\": 1", "\"id\": 99");
    assert_ne!(
        stale,
        std::fs::read_to_string(&path).unwrap(),
        "fixture has id 1"
    );
    std::fs::write(&path, &stale).unwrap();

    let (code, out, _) = e.run(&["t.yaml", "--snapshot"]);
    assert_eq!(code, 1, "{out}");
    assert!(out.contains("FAIL Snapshot mismatch"), "{out}");
    assert!(
        out.contains("Run with --update-snapshots (-u) to update"),
        "{out}"
    );
    assert_eq!(std::fs::read_to_string(&path).unwrap(), stale);

    let (code, out, _) = e.run(&["t.yaml", "-u"]);
    assert_eq!(code, 0, "{out}");
    assert!(
        out.contains("UPDATED Snapshot updated for \"User\""),
        "{out}"
    );
    assert_ne!(std::fs::read_to_string(&path).unwrap(), stale);
    assert_eq!(e.run(&["t.yaml", "--snapshot"]).0, 0);
}

#[test]
fn snapshot_dir_flag_and_results_file_carry_snapshot_result() {
    let e = setup(&[("t.yaml", SNAP)]);
    let (code, out, _) = e.run(&[
        "t.yaml",
        "--snapshot",
        "--snapshot-dir",
        "snaps",
        "-o",
        "r.json",
    ]);
    assert_eq!(code, 0, "{out}");
    assert!(e.path("snaps/t.snap.json").exists());
    let saved: Value =
        serde_json::from_str(&std::fs::read_to_string(e.path("r.json")).unwrap()).unwrap();
    assert_eq!(
        saved["results"][0]["snapshotResult"],
        serde_json::json!({"match": true, "isNew": true, "updated": true, "differences": []})
    );
}

#[test]
fn snapshots_stay_off_without_the_flag() {
    let e = setup(&[(
        "t.yaml",
        "requests:\n  - name: U\n    url: \"{BASE}/json/user\"\n",
    )]);
    let (code, out, _) = e.run(&["t.yaml"]);
    assert_eq!(code, 0, "{out}");
    assert!(!out.contains("Snapshot"));
    assert!(!e.path("__snapshots__").exists());
}

const DIFF: &str = r#"
global:
  diff:
    exclude: ["*.date"]
requests:
  - name: User
    url: "{BASE}/json/user"
  - name: Users
    url: "{BASE}/json/users"
"#;

fn baseline(e: &Env, path: &str) -> Value {
    serde_json::from_str(&std::fs::read_to_string(e.path(path)).unwrap()).unwrap()
}

#[test]
fn diff_save_writes_a_ts_shaped_baseline_file() {
    let e = setup(&[("t.yaml", DIFF)]);
    let (code, out, err) = e.run(&["t.yaml", "--diff-save", "--diff-label", "staging"]);
    assert_eq!(code, 0, "{out}{err}");
    assert!(out.contains("ℹ Baseline saved as 'staging'\n"), "{out}");
    let text = std::fs::read_to_string(e.path("__baselines__/t.staging.baseline.json")).unwrap();
    assert!(
        text.starts_with("{\n  \"version\": 1,\n  \"label\": \"staging\",\n  \"capturedAt\": \"")
    );
    assert!(!text.ends_with('\n'));
    let v = baseline(&e, "__baselines__/t.staging.baseline.json");
    let keys: Vec<_> = v["baselines"].as_object().unwrap().keys().collect();
    assert_eq!(keys, ["User", "Users"]);
    let user = &v["baselines"]["User"];
    assert_eq!(user["status"], 200);
    assert_eq!(user["hash"].as_str().unwrap().len(), 8);
    assert!(user.get("timing").is_none());
    assert!(user["headers"]["content-type"].is_string());
}

#[test]
fn diff_compare_reports_unchanged_changed_and_new() {
    let e = setup(&[("t.yaml", DIFF)]);
    let (code, _, err) = e.run(&["t.yaml", "--diff-save", "--diff-label", "staging"]);
    assert_eq!(code, 0, "{err}");
    let (code, out, err) = e.run(&["t.yaml", "--diff-compare", "staging"]);
    assert_eq!(code, 0, "{out}{err}");
    assert!(out.contains("Response Diff: staging → current\n"), "{out}");
    assert!(out.contains("  ✓ User (no changes)\n"), "{out}");
    assert!(out.contains("Summary: 2 unchanged (2 total)"), "{out}");
    // edit the stored baseline: change a body field and drop a request
    let path = "__baselines__/t.staging.baseline.json";
    let mut v = baseline(&e, path);
    v["baselines"]["User"]["body"]["name"] = Value::from("someone else");
    v["baselines"].as_object_mut().unwrap().remove("Users");
    std::fs::write(e.path(path), serde_json::to_string_pretty(&v).unwrap()).unwrap();
    let (code, out, _) = e.run(&["t.yaml", "--diff-compare", "staging"]);
    assert_eq!(code, 0, "a changed diff does not fail the run");
    assert!(
        out.contains("  ✗ User\n    body.name:\n      - \"someone else\"\n"),
        "{out}"
    );
    assert!(out.contains("  NEW Users\n"), "{out}");
    assert!(out.contains("Summary: 1 changed, 1 new (2 total)"), "{out}");
}

#[test]
fn diff_compare_json_and_markdown_formats() {
    let e = setup(&[("t.yaml", DIFF)]);
    e.run(&["t.yaml", "--diff-save", "--diff-label", "s"]);
    let (_, out, _) = e.run(&["t.yaml", "--diff-compare", "s", "--diff-output", "json"]);
    let start = out.find("{\n  \"summary\"").expect(&out);
    let v: Value = serde_json::from_str(&out[start..]).unwrap();
    assert_eq!(v["summary"]["unchanged"], 2);
    assert_eq!(v["results"][0]["status"], "unchanged");
    let (_, out, _) = e.run(&["t.yaml", "--diff-compare", "s", "--diff-output", "markdown"]);
    assert!(
        out.contains("# Response Diff: s → current\n\n| Metric | Count |\n"),
        "{out}"
    );
}

#[test]
fn diff_compare_missing_label_saves_the_run_under_it() {
    let e = setup(&[("t.yaml", DIFF)]);
    let (code, out, err) = e.run(&["t.yaml", "--diff-compare", "prod"]);
    assert_eq!(code, 0, "{out}{err}");
    assert!(
        err.contains("⚠ No baseline 'prod' found. Saving current run as baseline.\n"),
        "{err}"
    );
    assert!(!out.contains("Response Diff"));
    assert!(e.path("__baselines__/t.prod.baseline.json").exists());
}

#[test]
fn diff_compare_never_overwrites_an_unusable_baseline() {
    let e = setup(&[
        ("t.yaml", DIFF),
        ("__baselines__/t.prod.baseline.json", "{not json"),
    ]);
    let (code, _, err) = e.run(&["t.yaml", "--diff-compare", "prod"]);
    assert_eq!(code, 0);
    assert!(err.contains("cannot be used"), "{err}");
    assert_eq!(
        std::fs::read_to_string(e.path("__baselines__/t.prod.baseline.json")).unwrap(),
        "{not json"
    );
}

#[test]
fn diff_auto_mode_saves_then_compares_then_lists() {
    let e = setup(&[("t.yaml", DIFF)]);
    let (_, out, _) = e.run(&["t.yaml", "--diff"]);
    assert!(
        out.contains("ℹ No baselines found. Saved current run as 'baseline'.\n"),
        "{out}"
    );
    assert!(e.path("__baselines__/t.baseline.baseline.json").exists());
    let (_, out, _) = e.run(&["t.yaml", "-d"]);
    assert!(out.contains("Response Diff: baseline → current\n"), "{out}");
    let (_, _, _) = e.run(&["t.yaml", "--diff-save", "--diff-label", "other"]);
    let (_, out, _) = e.run(&["t.yaml", "--diff"]);
    assert!(
        out.contains("ℹ Available baselines: baseline, other\n"),
        "{out}"
    );
    assert!(out.contains("ℹ Use --diff-compare <label> to compare against a specific baseline.\n"));
}

#[test]
fn diff_dir_flag_moves_the_files() {
    let e = setup(&[("t.yaml", DIFF)]);
    e.run(&[
        "t.yaml",
        "--diff-save",
        "--diff-label",
        "x",
        "--diff-dir",
        ".bl",
    ]);
    assert!(e.path(".bl/t.x.baseline.json").exists());
    assert!(!e.path("__baselines__").exists());
}

#[test]
fn diff_subcommand_compares_two_stored_baselines() {
    let e = setup(&[("t.yaml", DIFF)]);
    e.run(&["t.yaml", "--diff-save", "--diff-label", "a"]);
    e.run(&["t.yaml", "--diff-save", "--diff-label", "b"]);
    // same data (dates are not excluded in the offline compare): only dates may differ
    let mut v = baseline(&e, "__baselines__/t.b.baseline.json");
    v["baselines"]["User"]["body"]["name"] = Value::from("changed");
    std::fs::write(
        e.path("__baselines__/t.b.baseline.json"),
        serde_json::to_string(&v).unwrap(),
    )
    .unwrap();
    let (code, out, err) = e.run(&["diff", "a", "b", "t.yaml"]);
    assert_eq!(code, 1, "{out}{err}");
    assert!(out.contains("Response Diff: a → b\n"), "{out}");
    assert!(out.contains("body.name:"), "{out}");
    // JSON output via the flag after the file
    let (code, out, _) = e.run(&["diff", "a", "b", "t.yaml", "--diff-output", "json"]);
    assert_eq!(code, 1);
    let r: Value = serde_json::from_str(&out).unwrap();
    assert_eq!(r["summary"]["changed"], 1);
    // identical labels: exit 0
    let (code, out, _) = e.run(&["diff", "a", "a", "t.yaml"]);
    assert_eq!(code, 0, "{out}");
    // missing label
    let (code, _, err) = e.run(&["diff", "a", "zzz", "t.yaml"]);
    assert_eq!(code, 1);
    assert!(err.contains("✗ Baseline 'zzz' not found"), "{err}");
    // file discovered when omitted
    let (code, out, _) = e.run(&["diff", "a", "a"]);
    assert_eq!(code, 0, "{out}");
}

#[test]
fn diff_subcommand_file_discovery_errors() {
    let e = setup(&[("x.txt", "")]);
    let (code, _, err) = e.run(&["diff", "a", "b"]);
    assert_eq!(code, 1);
    assert!(err.contains("No YAML files found. Specify a file"), "{err}");
    let e = setup(&[("a.yaml", DIFF), ("b.yaml", DIFF)]);
    let (code, _, err) = e.run(&["diff", "a", "b"]);
    assert_eq!(code, 1);
    assert!(
        err.contains("Multiple YAML files found. Specify which file to use."),
        "{err}"
    );
}

#[test]
fn diff_compare_never_replaces_an_existing_baseline_with_renamed_requests() {
    let e = setup(&[("t.yaml", DIFF)]);
    e.run(&["t.yaml", "--diff-save", "--diff-label", "old"]);
    let path = "__baselines__/t.old.baseline.json";
    let before = std::fs::read_to_string(e.path(path)).unwrap();
    let mut v = baseline(&e, path);
    let b = v["baselines"].as_object_mut().unwrap();
    let (user, users) = (b.remove("User").unwrap(), b.remove("Users").unwrap());
    b.insert("Old Name A".into(), user);
    b.insert("Old Name B".into(), users);
    std::fs::write(e.path(path), serde_json::to_string_pretty(&v).unwrap()).unwrap();
    let stored = std::fs::read_to_string(e.path(path)).unwrap();
    assert_ne!(stored, before);
    let (code, out, err) = e.run(&["t.yaml", "--diff-compare", "old"]);
    assert_eq!(code, 0, "{out}{err}");
    assert!(!err.contains("No baseline"), "{err}");
    assert!(
        out.contains("  NEW User\n") && out.contains("  NEW Users\n"),
        "{out}"
    );
    assert!(out.contains("Summary: 2 new (2 total)"), "{out}");
    assert_eq!(
        std::fs::read_to_string(e.path(path)).unwrap(),
        stored,
        "stored data kept"
    );
}

#[test]
fn diff_compare_never_replaces_an_existing_baseline_when_every_request_failed() {
    let e = setup(&[("t.yaml", DIFF)]);
    e.run(&["t.yaml", "--diff-save", "--diff-label", "old"]);
    let path = "__baselines__/t.old.baseline.json";
    let stored = std::fs::read_to_string(e.path(path)).unwrap();
    // same names, but every request now fails its expectation
    let failing = DIFF.replace("    url:", "    expect: { status: 404 }\n    url:");
    std::fs::write(
        e.path("t.yaml"),
        failing.replace("{BASE}", &e.server.base_url),
    )
    .unwrap();
    let (code, out, err) = e.run(&["t.yaml", "--diff-compare", "old"]);
    assert_eq!(code, 1, "{out}{err}");
    assert!(!err.contains("No baseline"), "{err}");
    assert!(out.contains("Summary:  (0 total)"), "{out}");
    assert_eq!(
        std::fs::read_to_string(e.path(path)).unwrap(),
        stored,
        "stored data kept"
    );
}

// --- profile mode ---

const PROFILE_FILE: &str = "requests:\n  - name: User\n    url: \"{BASE}/json/user\"\n  - url: \"{BASE}/json/users\"\n    expect: { status: 500 }\n";

#[test]
fn profile_prints_stats_and_exits_0() {
    let e = setup(&[("t.yaml", PROFILE_FILE)]);
    let (code, out, err) = e.run(&["t.yaml", "--profile", "6", "--profile-warmup", "2"]);
    assert_eq!(code, 0, "{out}{err}");
    assert!(out.contains("⚡ PROFILING User\n   6 iterations, 2 warmup, concurrency: 1\n"));
    // the second request has no name: its title is the URL; `expect` is ignored
    assert!(out.contains("⚡ PROFILING Request 2\n"), "{out}");
    assert!(
        out.contains(&format!("✓ {}/json/users\n", e.server.base_url)),
        "{out}"
    );
    assert!(out.contains("   ┌─────────────────────────────────────┐\n"));
    assert!(out.contains("   │ p50 "), "{out}");
    assert!(out.contains(" | 4 samples | 0 failures (0%)\n"), "{out}");
    assert!(
        out.contains("⚡ Profile Summary: 2 requests, 8 total iterations\n"),
        "{out}"
    );
    assert!(!out.contains("Distribution"));
}

#[test]
fn profile_histogram_and_concurrency() {
    let e = setup(&[("t.yaml", PROFILE_FILE)]);
    let (code, out, _) = e.run(&[
        "t.yaml",
        "-P",
        "12",
        "--profile-concurrency",
        "4",
        "--profile-histogram",
    ]);
    assert_eq!(code, 0, "{out}");
    assert!(out.contains("   12 iterations, 1 warmup, concurrency: 4\n"));
    assert!(out.contains("   Distribution:\n"), "{out}");
    assert!(out.contains("ms │"), "{out}");
    assert!(out.contains(" | 11 samples | "), "{out}");
}

#[test]
fn profile_export_json_and_csv() {
    let e = setup(&[("t.yaml", PROFILE_FILE)]);
    let (code, out, _) = e.run(&[
        "t.yaml",
        "-P",
        "4",
        "--profile-warmup",
        "1",
        "--profile-export",
        "p.json",
    ]);
    assert_eq!(code, 0, "{out}");
    assert!(out.contains("ℹ Profile results exported to p.json\n"));
    let v: Value =
        serde_json::from_str(&std::fs::read_to_string(e.path("p.json")).unwrap()).unwrap();
    let a = v.as_array().unwrap();
    assert_eq!(a.len(), 2);
    assert_eq!(a[0]["request"], "User");
    assert_eq!(a[1]["request"], format!("{}/json/users", e.server.base_url));
    assert_eq!(a[0]["summary"]["iterations"], 3);
    assert_eq!(a[0]["summary"]["warmup"], 1);
    assert_eq!(a[0]["timings"].as_array().unwrap().len(), 3);
    let raw = std::fs::read_to_string(e.path("p.json")).unwrap();
    assert!(
        raw.starts_with("[{\n  \"request\": \"User\",\n  \"summary\": {\n    \"iterations\": 3,"),
        "{raw}"
    );
    assert!(raw.contains("\n  ]\n},\n{\n"), "{raw}");

    let (code, _, _) = e.run(&[
        "t.yaml",
        "-P",
        "3",
        "--profile-warmup",
        "0",
        "--profile-export",
        "p.csv",
    ]);
    assert_eq!(code, 0);
    let csv = std::fs::read_to_string(e.path("p.csv")).unwrap();
    let blocks: Vec<&str> = csv.split("\n\n").collect();
    assert_eq!(blocks.len(), 2, "{csv}");
    for b in blocks {
        let lines: Vec<&str> = b.lines().collect();
        assert_eq!(lines[0], "iteration,latency_ms");
        assert_eq!(lines.len(), 4);
        assert!(lines[1].starts_with("1,") && lines[3].starts_with("3,"));
    }
}

#[test]
fn profile_config_from_the_file_needs_the_flag() {
    // As in TS the mode is picked before the files are read: `global.profile`
    // alone does not switch to profile mode, but supplies the settings.
    let e = setup(&[(
        "t.yaml",
        "global:\n  profile: { iterations: 3, warmup: 0 }\nrequests:\n  - url: \"{BASE}/json/user\"\n",
    )]);
    let (code, out, _) = e.run(&["t.yaml"]);
    assert_eq!(code, 0);
    assert!(!out.contains("PROFILING"), "{out}");
    let (code, out, _) = e.run(&["t.yaml", "--profile", "3"]);
    assert_eq!(code, 0);
    assert!(out.contains("3 iterations, 0 warmup"), "{out}");
}

#[test]
fn profile_json_and_raw_formats() {
    let e = setup(&[("t.yaml", PROFILE_FILE)]);
    let (code, out, _) = e.run(&["t.yaml", "-P", "3", "--output-format", "json"]);
    assert_eq!(code, 0, "{out}");
    let lines: Vec<&str> = out.lines().collect();
    assert_eq!(lines.len(), 2, "{out}");
    let v: Value = serde_json::from_str(lines[0]).unwrap();
    assert_eq!(v["request"]["name"], "User");
    assert_eq!(v["request"]["method"], "GET");
    assert_eq!(v["stats"]["iterations"], 2);
    let (_, out, _) = e.run(&["t.yaml", "-P", "3", "--output-format", "raw"]);
    let lines: Vec<&str> = out.lines().collect();
    assert_eq!(lines.len(), 2, "{out}");
    assert_eq!(lines[0].split('\t').count(), 4);
}

#[test]
fn profile_all_failures_exit_1() {
    let e = setup(&[(
        "t.yaml",
        "requests:\n  - name: Down\n    url: \"http://127.0.0.1:1/x\"\n    timeout: 2000\n",
    )]);
    let (code, out, _) = e.run(&["t.yaml", "-P", "3", "--profile-warmup", "0"]);
    assert_eq!(code, 1, "{out}");
    assert!(out.contains("⚠ Down\n"), "{out}");
    assert!(out.contains("| 0 samples | 3 failures (100%)\n"), "{out}");
    assert!(out.contains("   3 total failures\n"), "{out}");
}

#[test]
fn profile_ignores_dry_run_and_checks_no_files() {
    let e = setup(&[("t.yaml", PROFILE_FILE)]);
    let (code, out, _) = e.run(&["t.yaml", "-P", "2", "--dry-run", "--profile-warmup", "0"]);
    assert_eq!(code, 0);
    assert!(out.contains(" | 2 samples | 0 failures"), "{out}");
    let (code, _, err) = e.run(&["nothing.yaml", "-P", "2"]);
    assert_eq!(code, 1);
    assert!(err.contains("No YAML files found"), "{err}");
}

#[cfg(unix)]
#[test]
fn profile_sigint_prints_partial_stats_exports_and_exits_130() {
    use std::time::{Duration, Instant};
    let e = setup(&[(
        "t.yaml",
        "requests:\n  - name: Fast\n    url: \"{BASE}/json/user\"\n  - name: Slow\n    url: \"{BASE}/delay/200\"\n",
    )]);
    let child = std::process::Command::new(BIN)
        .env_clear()
        .envs(testkit::system_env())
        .env("NO_COLOR", "1")
        .current_dir(e.dir.path())
        .args([
            "t.yaml",
            "-P",
            "100000",
            "--profile-warmup",
            "0",
            "--profile-export",
            "p.json",
        ])
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .unwrap();
    std::thread::sleep(Duration::from_millis(1500));
    let start = Instant::now();
    let ok = std::process::Command::new("kill")
        .args(["-INT", &child.id().to_string()])
        .status()
        .unwrap();
    assert!(ok.success());
    let out = child.wait_with_output().unwrap();
    assert!(
        start.elapsed() < Duration::from_secs(3),
        "{:?}",
        start.elapsed()
    );
    assert_eq!(out.status.code(), Some(130));
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("⚡ PROFILING Fast"), "{stdout}");
    assert!(
        stdout.contains("⚡ Profile Summary: 1 request,"),
        "{stdout}"
    );
    assert!(
        stdout.contains("Profile results exported to p.json"),
        "{stdout}"
    );
    assert!(String::from_utf8_lossy(&out.stderr).contains("Interrupted"));
    let v: Value =
        serde_json::from_str(&std::fs::read_to_string(e.path("p.json")).unwrap()).unwrap();
    assert!(v[0]["summary"]["iterations"].as_u64().unwrap() > 0);
}

#[test]
fn yaml_integer_beyond_u64_is_read_as_a_float_like_js() {
    let e = setup(&[(
        "t.yaml",
        "request:\n  name: Big\n  url: \"{BASE}/x\"\n  method: POST\n  body: {id: 123456789012345678901234, neg: -123456789012345678901234}\n",
    )]);
    let (code, out, err) = e.run(&["t.yaml", "--dry-run"]);
    assert_eq!(code, 0, "{err}");
    assert!(
        out.contains(r#"{"id":1.2345678901234569e+23,"neg":-1.2345678901234569e+23}"#),
        "{out}"
    );
}

/// TS resolved formData files and `ssl` paths against the process cwd, not the
/// YAML file's directory (a suite in `tests/<area>/*.yaml` can reference
/// `./fixtures/...` from the repo root).
#[test]
fn form_file_in_a_subdir_yaml_resolves_against_cwd() {
    let yaml = r#"
requests:
  - name: Upload
    url: "{BASE}/multipart"
    method: POST
    formData:
      doc:
        file: ./fixtures/a.txt
    expect: { status: 200 }
"#;
    let e = setup(&[
        ("curl-runner/area/upload.yaml", yaml),
        ("fixtures/a.txt", "hello"),
    ]);
    let (code, out, err) = e.run(&["curl-runner/area/upload.yaml", "--output", "res.json"]);
    assert_eq!(code, 0, "{out}{err}");
    assert!(!out.contains("File(s) not found"), "{out}");
    let v: Value =
        serde_json::from_str(&std::fs::read_to_string(e.path("res.json")).unwrap()).unwrap();
    assert_eq!(v["results"][0]["body"][0]["filename"], "a.txt", "{v}");
    assert_eq!(v["results"][0]["body"][0]["size"], 5, "{v}");
}

/// TS validated `expect.responseTime` against `result.metrics.duration`: wall
/// time since the request started, retries and retry delays included.
#[test]
fn response_time_counts_retries_and_retry_delays() {
    let yaml = r#"
request:
  name: Flaky
  url: "{BASE}/flaky/rt/1"
  expect:
    status: 200
    responseTime: "< 2000"
"#;
    let e = setup(&[("t.yaml", yaml)]);
    let (code, out, err) = e.run(&[
        "t.yaml",
        "--retries",
        "3",
        "--retry-delay",
        "2500",
        "--output",
        "res.json",
    ]);
    assert_eq!(code, 1, "{out}{err}");
    let v: Value =
        serde_json::from_str(&std::fs::read_to_string(e.path("res.json")).unwrap()).unwrap();
    let r = &v["results"][0];
    assert_eq!(r["status"], 200, "{v}");
    assert!(
        r["error"]
            .as_str()
            .unwrap()
            .starts_with("Expected response time to match < 2000ms, got 25"),
        "{v}"
    );
    assert!(r["metrics"]["duration"].as_f64().unwrap() >= 2500.0, "{v}");
}
