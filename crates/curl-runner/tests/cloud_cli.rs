//! `curl-runner login` and `logout` end to end against a fake API.

use assert_cmd::Command;
use std::sync::{Arc, Mutex};
use testkit::{Request, Response, TestServer};

type Seen = Arc<Mutex<Vec<(String, String, Option<String>, String)>>>;

fn fake_api(verify_ok: bool) -> (TestServer, Seen) {
    let seen: Seen = Arc::default();
    let s2 = seen.clone();
    let srv = TestServer::with_handler(move |r: Request| {
        s2.lock().unwrap().push((
            r.method.clone(),
            r.url.clone(),
            r.header("authorization").map(str::to_string),
            String::from_utf8_lossy(&r.body).into_owned(),
        ));
        match r.url.as_str() {
            "/v1/auth/email/start" => Response::from_data(b"{}".to_vec()).with_status_code(202),
            "/v1/auth/email/verify" if verify_ok => Response::from_data(
                br#"{"session":"crs_abc123","user":{"id":"u1","email":"dev@example.com","name":null}}"#.to_vec(),
            ),
            "/v1/auth/email/verify" => Response::from_data(
                br#"{"error":"code_invalid","message":"That code is wrong."}"#.to_vec(),
            )
            .with_status_code(400),
            "/v1/auth/sign-out" => Response::from_data(Vec::new()).with_status_code(204),
            _ => Response::from_data(b"{}".to_vec()).with_status_code(404),
        }
    });
    (srv, seen)
}

fn cmd(home: &std::path::Path, server: &str) -> Command {
    let mut c = Command::cargo_bin("curl-runner").unwrap();
    c.env_clear()
        .env("PATH", std::env::var("PATH").unwrap_or_default())
        .env("XDG_CONFIG_HOME", home)
        .env("APPDATA", home)
        .env("HOME", home)
        .env("CURL_RUNNER_TEST_SERVER", server);
    c
}

fn stderr(o: &std::process::Output) -> String {
    String::from_utf8_lossy(&o.stderr).into_owned()
}

fn creds_path(home: &std::path::Path) -> std::path::PathBuf {
    home.join("curl-runner").join("credentials.json")
}

fn login_ok(home: &std::path::Path, server: &str) {
    cmd(home, server)
        .args(["login", "--email", "dev@example.com"])
        .write_stdin("123456\n")
        .assert()
        .success();
}

#[test]
fn login_with_email_and_code_saves_credentials() {
    let (srv, seen) = fake_api(true);
    let home = tempfile::tempdir().unwrap();
    let out = cmd(home.path(), &srv.base_url)
        .args(["login", "--email", "dev@example.com"])
        .write_stdin("123456\n")
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let out = String::from_utf8(out).unwrap();
    assert!(out.contains(&format!(
        "Signed in to {} as dev@example.com.",
        srv.base_url
    )));
    assert!(!out.contains("crs_abc123"));
    let saved: serde_json::Value =
        serde_json::from_slice(&std::fs::read(creds_path(home.path())).unwrap()).unwrap();
    assert_eq!(saved["servers"][&srv.base_url]["session"], "crs_abc123");
    assert_eq!(saved["servers"][&srv.base_url]["email"], "dev@example.com");
    let seen = seen.lock().unwrap();
    let start: serde_json::Value = serde_json::from_str(&seen[0].3).unwrap();
    assert_eq!(start["client"], "device");
    assert_eq!(start["email"], "dev@example.com");
    assert!(start["label"].as_str().unwrap().starts_with("CLI on "));
    let verify: serde_json::Value = serde_json::from_str(&seen[1].3).unwrap();
    assert_eq!(verify["code"], "123456");
}

#[cfg(unix)]
#[test]
fn credentials_file_mode_0600() {
    use std::os::unix::fs::PermissionsExt;
    let (srv, _) = fake_api(true);
    let home = tempfile::tempdir().unwrap();
    login_ok(home.path(), &srv.base_url);
    let mode = std::fs::metadata(creds_path(home.path()))
        .unwrap()
        .permissions()
        .mode();
    assert_eq!(mode & 0o777, 0o600);
}

#[test]
fn login_without_email_and_no_tty_exits_2() {
    let (srv, _) = fake_api(true);
    let home = tempfile::tempdir().unwrap();
    let o = cmd(home.path(), &srv.base_url)
        .arg("login")
        .write_stdin("")
        .output()
        .unwrap();
    assert_eq!(o.status.code(), Some(2));
    assert!(stderr(&o).contains("Run curl-runner login in a terminal, or set CURL_RUNNER_TOKEN."));
}

#[test]
fn login_wrong_code_message() {
    let (srv, _) = fake_api(false);
    let home = tempfile::tempdir().unwrap();
    let o = cmd(home.path(), &srv.base_url)
        .args(["login", "--email", "dev@example.com"])
        .write_stdin("000000\n")
        .output()
        .unwrap();
    assert_eq!(o.status.code(), Some(1));
    assert!(stderr(&o).contains("That code is not right. Check it and try again."));
    assert!(!creds_path(home.path()).exists());
}

#[test]
fn logout_signs_out_and_removes_entry() {
    let (srv, seen) = fake_api(true);
    let home = tempfile::tempdir().unwrap();
    login_ok(home.path(), &srv.base_url);
    let o = cmd(home.path(), &srv.base_url)
        .arg("logout")
        .output()
        .unwrap();
    assert!(o.status.success());
    assert!(String::from_utf8_lossy(&o.stdout).contains("Signed out of"));
    let last = seen.lock().unwrap().last().cloned().unwrap();
    assert_eq!(last.1, "/v1/auth/sign-out");
    assert_eq!(last.2.as_deref(), Some("Bearer crs_abc123"));
    let saved: serde_json::Value =
        serde_json::from_slice(&std::fs::read(creds_path(home.path())).unwrap()).unwrap();
    assert!(saved["servers"].as_object().unwrap().is_empty());
}

#[test]
fn logout_removes_entry_even_offline() {
    let (srv, _) = fake_api(true);
    let home = tempfile::tempdir().unwrap();
    login_ok(home.path(), &srv.base_url);
    // Same server key, but the server is now gone.
    let url = srv.base_url.clone();
    drop(srv);
    cmd(home.path(), &url).arg("logout").assert().success();
    assert!(
        !std::fs::read_to_string(creds_path(home.path()))
            .unwrap()
            .contains("crs_abc123")
    );
}

#[test]
fn logout_when_signed_out_is_a_no_op() {
    let home = tempfile::tempdir().unwrap();
    let o = cmd(home.path(), "http://127.0.0.1:1")
        .arg("logout")
        .output()
        .unwrap();
    assert!(o.status.success());
    assert!(String::from_utf8_lossy(&o.stdout).contains("Not signed in to"));
}

#[test]
fn help_lists_cloud_commands() {
    let out = std::process::Command::new(env!("CARGO_BIN_EXE_curl-runner"))
        .arg("--help")
        .output()
        .unwrap();
    let text = String::from_utf8(out.stdout).unwrap();
    assert!(text.contains("curl-runner login"));
    assert!(text.contains("curl-runner logout"));
}

#[test]
fn flag_errors_and_equals_form() {
    let (srv, _) = fake_api(true);
    let home = tempfile::tempdir().unwrap();
    for args in [
        vec!["login", "--server"],
        vec!["login", "--email", "--server=x"],
        vec!["logout", "--server", "--email"],
    ] {
        let o = cmd(home.path(), &srv.base_url)
            .args(&args)
            .write_stdin("")
            .output()
            .unwrap();
        assert_eq!(o.status.code(), Some(2), "{args:?}");
        assert!(stderr(&o).contains("needs a value"), "{args:?}");
    }
    let o = cmd(home.path(), &srv.base_url)
        .args([
            "login",
            "--email=dev@example.com",
            &format!("--server={}", srv.base_url),
        ])
        .write_stdin("123456\n")
        .output()
        .unwrap();
    assert!(o.status.success());
}

#[test]
fn plain_http_server_is_refused() {
    let home = tempfile::tempdir().unwrap();
    let o = cmd(home.path(), "http://127.0.0.1:1")
        .args([
            "login",
            "--email",
            "a@example.com",
            "--server",
            "http://api.example.com",
        ])
        .output()
        .unwrap();
    assert_eq!(o.status.code(), Some(2));
    assert!(
        stderr(&o).contains("Use an https:// server. Plain http is only allowed for localhost.")
    );
}

#[test]
fn redirecting_server_gets_no_follow_up_request() {
    let (target, seen) = fake_api(true);
    let to = target.base_url.clone();
    let redirector = TestServer::with_handler(move |r: Request| {
        Response::from_data(Vec::new())
            .with_status_code(302)
            .with_header("Location", &format!("{to}{}", r.url))
    });
    let home = tempfile::tempdir().unwrap();
    let o = cmd(home.path(), &redirector.base_url)
        .args(["login", "--email", "dev@example.com"])
        .write_stdin("123456\n")
        .output()
        .unwrap();
    assert_eq!(o.status.code(), Some(1));
    assert!(stderr(&o).contains("redirect"));
    assert!(seen.lock().unwrap().is_empty());
}

#[test]
fn login_to_a_second_server_keeps_the_first_entry() {
    let (a, _) = fake_api(true);
    let (b, _) = fake_api(true);
    let home = tempfile::tempdir().unwrap();
    login_ok(home.path(), &a.base_url);
    login_ok(home.path(), &b.base_url);
    let saved: serde_json::Value =
        serde_json::from_slice(&std::fs::read(creds_path(home.path())).unwrap()).unwrap();
    assert_eq!(saved["servers"].as_object().unwrap().len(), 2);
    assert_eq!(saved["servers"][&a.base_url]["email"], "dev@example.com");
    assert_eq!(saved["servers"][&b.base_url]["email"], "dev@example.com");
}

// ---- publish ----

const VALID: &str = "request:\n  url: https://api.example.com/users\n  method: GET\n";
const CONFIG: &str = "cloud:\n  collection: payments-api\n";

type Calls = Arc<Mutex<Vec<(String, String, Option<String>, Vec<u8>)>>>;

/// A fake API; `respond` gets each request and its zero-based index.
fn fake(
    respond: impl Fn(&Request, usize) -> Response + Send + Sync + 'static,
) -> (TestServer, Calls) {
    let calls: Calls = Arc::default();
    let c2 = calls.clone();
    let srv = TestServer::with_handler(move |r: Request| {
        let n = {
            let mut g = c2.lock().unwrap();
            g.push((
                r.method.clone(),
                r.url.clone(),
                r.header("authorization").map(str::to_string),
                r.body.clone(),
            ));
            g.len() - 1
        };
        respond(&r, n)
    });
    (srv, calls)
}

fn json(status: u16, body: &str) -> Response {
    Response::from_data(body.as_bytes().to_vec()).with_status_code(status)
}

fn project(config: &str, files: &[(&str, &str)]) -> tempfile::TempDir {
    let d = tempfile::tempdir().unwrap();
    std::fs::write(d.path().join("curl-runner.yaml"), config).unwrap();
    for (name, text) in files {
        std::fs::write(d.path().join(name), text).unwrap();
    }
    d
}

fn publish(server: &str, dir: &std::path::Path, extra: &[&str]) -> Command {
    let home = dir.join("no-home");
    let mut c = cmd(&home, server);
    c.env("CURL_RUNNER_TOKEN", "crt_secret")
        .env("CURL_RUNNER_TEAM", "acme")
        .env("CURL_RUNNER_TEST_NO_SLEEP", "1")
        .arg("publish")
        .arg(dir)
        .args(extra);
    c
}

fn created(v: u32) -> Response {
    json(
        201,
        &format!(
            r#"{{"collection":"payments-api","version":{v},"status":"created","changed":{{"added":["a.yaml","b.yaml"],"changed":["c.yaml"],"removed":[]}}}}"#
        ),
    )
}

const UNCHANGED: &str = r#"{"collection":"payments-api","version":14,"status":"unchanged","changed":{"added":[],"changed":[],"removed":[]}}"#;

#[test]
fn publish_validates_all_dirs_before_sending() {
    let (srv, calls) = fake(|_, _| created(1));
    let a = project(CONFIG, &[("a.yaml", "a: [unclosed")]);
    let b = project(
        "cloud:\n  collection: other\n",
        &[("b.yaml", "b: [unclosed")],
    );
    let o = publish(&srv.base_url, a.path(), &[])
        .arg(b.path())
        .output()
        .unwrap();
    assert_eq!(o.status.code(), Some(1));
    let all = format!(
        "{}{}",
        String::from_utf8_lossy(&o.stdout),
        String::from_utf8_lossy(&o.stderr)
    );
    assert!(all.contains("a.yaml"), "{all}");
    assert!(all.contains("b.yaml"), "{all}");
    assert!(calls.lock().unwrap().is_empty());
}

#[test]
fn publish_dir_without_cloud_block_is_an_error() {
    let (srv, calls) = fake(|_, _| created(1));
    let a = project("global:\n  variables: {A: b}\n", &[("a.yaml", VALID)]);
    let o = publish(&srv.base_url, a.path(), &[]).output().unwrap();
    assert_eq!(o.status.code(), Some(1));
    assert!(
        stderr(&o)
            .contains("no cloud.collection in curl-runner.yaml. Add one, or pass --collection.")
    );
    assert!(calls.lock().unwrap().is_empty());
    // --collection fills the gap.
    let o = publish(&srv.base_url, a.path(), &["--collection", "payments-api"])
        .output()
        .unwrap();
    assert_eq!(o.status.code(), Some(0), "{}", stderr(&o));
}

#[test]
fn publish_collection_flag_needs_one_dir() {
    let (srv, calls) = fake(|_, _| created(1));
    let a = project(CONFIG, &[("a.yaml", VALID)]);
    let b = project(CONFIG, &[("a.yaml", VALID)]);
    let o = publish(&srv.base_url, a.path(), &["--collection=x"])
        .arg(b.path())
        .output()
        .unwrap();
    assert_eq!(o.status.code(), Some(1));
    assert!(stderr(&o).contains("--collection"));
    assert!(calls.lock().unwrap().is_empty());
}

#[test]
fn publish_secret_literal_fails_exit_1() {
    let (srv, calls) = fake(|_, _| created(1));
    let a = project(
        "cloud:\n  collection: payments-api\n  inputs:\n    - name: SECRET_KEY\n",
        &[(
            "a.yaml",
            "global:\n  variables:\n    SECRET_KEY: hunter2\nrequest:\n  url: https://api.example.com/x\n  method: GET\n",
        )],
    );
    let o = publish(&srv.base_url, a.path(), &[]).output().unwrap();
    assert_eq!(o.status.code(), Some(1));
    let err = stderr(&o);
    assert!(
        err.contains("SECRET_KEY") && err.contains("a.yaml"),
        "{err}"
    );
    assert!(!err.contains("hunter2"));
    assert!(calls.lock().unwrap().is_empty());
}

#[test]
fn publish_created_prints_counts() {
    let (srv, calls) = fake(|_, _| created(14));
    let a = project(CONFIG, &[("a.yaml", VALID)]);
    let o = publish(&srv.base_url, a.path(), &[]).output().unwrap();
    assert_eq!(o.status.code(), Some(0), "{}", stderr(&o));
    assert!(
        String::from_utf8_lossy(&o.stdout)
            .contains("payments-api: published v14 (2 added, 1 changed, 0 removed)")
    );
    let calls = calls.lock().unwrap();
    assert_eq!(calls.len(), 1);
    assert_eq!(calls[0].0, "PUT");
    assert_eq!(
        calls[0].1,
        "/v1/teams/acme/collections/payments-api/versions"
    );
    assert_eq!(calls[0].2.as_deref(), Some("Bearer crt_secret"));
}

#[test]
fn publish_unchanged_line() {
    let (srv, _) = fake(|_, _| json(200, UNCHANGED));
    let a = project(CONFIG, &[("a.yaml", VALID)]);
    let o = publish(&srv.base_url, a.path(), &[]).output().unwrap();
    assert_eq!(o.status.code(), Some(0));
    assert!(String::from_utf8_lossy(&o.stdout).contains("payments-api: unchanged (v14)"));
}

#[test]
fn publish_retries_503_then_succeeds() {
    let (srv, calls) = fake(|_, n| {
        if n < 2 {
            json(503, r#"{"error":"unavailable","message":"Try again."}"#)
        } else {
            json(200, UNCHANGED)
        }
    });
    let a = project(CONFIG, &[("a.yaml", VALID)]);
    let o = publish(&srv.base_url, a.path(), &[]).output().unwrap();
    assert_eq!(o.status.code(), Some(0), "{}", stderr(&o));
    assert_eq!(calls.lock().unwrap().len(), 3);
}

#[test]
fn publish_retries_429_then_succeeds() {
    let (srv, calls) = fake(|_, n| {
        if n == 0 {
            json(429, r#"{"error":"rate_limited","message":"Slow down."}"#)
                .with_header("Retry-After", "0")
        } else {
            json(200, UNCHANGED)
        }
    });
    let a = project(CONFIG, &[("a.yaml", VALID)]);
    let o = publish(&srv.base_url, a.path(), &[]).output().unwrap();
    assert_eq!(o.status.code(), Some(0), "{}", stderr(&o));
    assert_eq!(calls.lock().unwrap().len(), 2);
}

#[test]
fn publish_gives_up_exit_5() {
    let (srv, calls) =
        fake(|_, _| json(503, r#"{"error":"unavailable","message":"Down for now."}"#));
    let a = project(CONFIG, &[("a.yaml", VALID)]);
    let o = publish(&srv.base_url, a.path(), &[]).output().unwrap();
    assert_eq!(o.status.code(), Some(5));
    assert!(stderr(&o).contains("Down for now."));
    assert_eq!(calls.lock().unwrap().len(), 5);
}

#[test]
fn publish_unreachable_exit_5() {
    let a = project(CONFIG, &[("a.yaml", VALID)]);
    let o = publish("http://127.0.0.1:1", a.path(), &[])
        .output()
        .unwrap();
    assert_eq!(o.status.code(), Some(5));
    assert!(stderr(&o).contains("Cannot reach http://127.0.0.1:1"));
}

#[test]
fn publish_wrong_source_exit_4_prints_server_message() {
    let (srv, calls) = fake(|_, _| {
        json(
            409,
            r#"{"error":"wrong_source","message":"This collection is edited in the cloud."}"#,
        )
    });
    let a = project(CONFIG, &[("a.yaml", VALID)]);
    let o = publish(&srv.base_url, a.path(), &[]).output().unwrap();
    assert_eq!(o.status.code(), Some(4));
    assert!(stderr(&o).contains("This collection is edited in the cloud."));
    assert_eq!(calls.lock().unwrap().len(), 1);
}

#[test]
fn publish_plan_limit_exit_3() {
    let (srv, _) = fake(|_, _| {
        json(
            402,
            r#"{"error":"plan_limit","message":"Upgrade to publish more."}"#,
        )
    });
    let a = project(CONFIG, &[("a.yaml", VALID)]);
    let o = publish(&srv.base_url, a.path(), &[]).output().unwrap();
    assert_eq!(o.status.code(), Some(3));
    assert!(stderr(&o).contains("Upgrade to publish more."));
}

#[test]
fn publish_unauthorized_exit_2() {
    let (srv, _) = fake(|_, _| json(401, r#"{"error":"unauthorized","message":"Bad token."}"#));
    let a = project(CONFIG, &[("a.yaml", VALID)]);
    let o = publish(&srv.base_url, a.path(), &[]).output().unwrap();
    assert_eq!(o.status.code(), Some(2));
    assert!(stderr(&o).contains("Bad token."));
}

#[test]
fn publish_404_means_cannot_publish_to_team() {
    let (srv, _) = fake(|_, _| json(404, r#"{"error":"not_found","message":"Not found."}"#));
    let a = project(CONFIG, &[("a.yaml", VALID)]);
    let o = publish(&srv.base_url, a.path(), &[]).output().unwrap();
    assert_eq!(o.status.code(), Some(2));
    assert!(
        stderr(&o).contains(
            "This token or account can't publish to team acme. Check --team and the token."
        )
    );
}

#[test]
fn publish_invalid_bundle_from_server_exit_1() {
    let (srv, _) = fake(|_, _| {
        json(
            400,
            r#"{"error":"invalid_bundle","message":"Bundle is bad."}"#,
        )
    });
    let a = project(CONFIG, &[("a.yaml", VALID)]);
    let o = publish(&srv.base_url, a.path(), &[]).output().unwrap();
    assert_eq!(o.status.code(), Some(1));
    assert!(stderr(&o).contains("Bundle is bad."));
}

#[test]
fn publish_no_credentials_exit_2() {
    let (srv, calls) = fake(|_, _| created(1));
    let a = project(CONFIG, &[("a.yaml", VALID)]);
    let mut c = cmd(&a.path().join("no-home"), &srv.base_url);
    let o = c.arg("publish").arg(a.path()).output().unwrap();
    assert_eq!(o.status.code(), Some(2));
    assert!(stderr(&o).contains("curl-runner login"));
    assert!(calls.lock().unwrap().is_empty());
}

#[test]
fn publish_corrupt_credentials_exit_2() {
    let (srv, _) = fake(|_, _| created(1));
    let a = project(CONFIG, &[("a.yaml", VALID)]);
    let home = a.path().join("no-home");
    std::fs::create_dir_all(home.join("curl-runner")).unwrap();
    std::fs::write(creds_path(&home), "{nope").unwrap();
    let o = cmd(&home, &srv.base_url)
        .arg("publish")
        .arg(a.path())
        .output()
        .unwrap();
    assert_eq!(o.status.code(), Some(2));
    assert!(stderr(&o).contains("Fix or delete it"));
}

#[test]
fn publish_token_without_team_exit_2() {
    let (srv, calls) = fake(|_, _| created(1));
    let a = project(CONFIG, &[("a.yaml", VALID)]);
    let o = publish(&srv.base_url, a.path(), &[])
        .env_remove("CURL_RUNNER_TEAM")
        .output()
        .unwrap();
    assert_eq!(o.status.code(), Some(2));
    assert!(stderr(&o).contains("--team"));
    assert!(calls.lock().unwrap().is_empty());
}

fn me(teams: &[&str]) -> String {
    let t: Vec<String> = teams
        .iter()
        .map(|s| format!(r#"{{"slug":"{s}","name":"{s}","role":"owner","plan":"team"}}"#))
        .collect();
    format!(
        r#"{{"user":{{"id":"u1","email":"dev@example.com","name":null}},"teams":[{}]}}"#,
        t.join(",")
    )
}

#[test]
fn publish_session_uses_only_team_from_me() {
    let (srv, calls) = fake(|r, _| {
        if r.url == "/v1/me" {
            json(200, &me(&["solo"]))
        } else {
            created(3)
        }
    });
    let a = project(CONFIG, &[("a.yaml", VALID)]);
    let o = publish(&srv.base_url, a.path(), &[])
        .env("CURL_RUNNER_TOKEN", "crs_sess")
        .env_remove("CURL_RUNNER_TEAM")
        .output()
        .unwrap();
    assert_eq!(o.status.code(), Some(0), "{}", stderr(&o));
    let calls = calls.lock().unwrap();
    assert_eq!(
        calls[1].1,
        "/v1/teams/solo/collections/payments-api/versions"
    );
}

#[test]
fn publish_session_with_many_teams_exit_2() {
    let (srv, calls) = fake(|_, _| json(200, &me(&["a", "b"])));
    let a = project(CONFIG, &[("a.yaml", VALID)]);
    let o = publish(&srv.base_url, a.path(), &[])
        .env("CURL_RUNNER_TOKEN", "crs_sess")
        .env_remove("CURL_RUNNER_TEAM")
        .output()
        .unwrap();
    assert_eq!(o.status.code(), Some(2));
    assert!(stderr(&o).contains("Pass --team: you're in 2 teams."));
    assert_eq!(calls.lock().unwrap().len(), 1);
}

#[test]
fn publish_dry_run_sends_nothing() {
    let (srv, calls) = fake(|r, _| match r.url.as_str() {
        "/v1/teams/acme/collections" => json(
            200,
            r#"[{"slug":"payments-api","name":"P","source":"git","access":"off","current_version":14,"updated_at":"2026-01-01T00:00:00Z"}]"#,
        ),
        "/v1/teams/acme/collections/payments-api/versions/14/files" => json(
            200,
            r#"[{"path":"a.yaml","text":"request:\n  url: https://api.example.com/users\n  method: GET\n"},{"path":"gone.yaml","text":"x: 1\n"}]"#,
        ),
        _ => json(500, r#"{"error":"unavailable","message":"unexpected"}"#),
    });
    let a = project(CONFIG, &[("a.yaml", VALID), ("b.yaml", VALID)]);
    let o = publish(&srv.base_url, a.path(), &["--dry-run"])
        .output()
        .unwrap();
    assert_eq!(o.status.code(), Some(0), "{}", stderr(&o));
    // curl-runner.yaml is part of the bundle too, so it counts as added.
    assert!(
        String::from_utf8_lossy(&o.stdout)
            .contains("payments-api: would publish (2 added, 0 changed, 1 removed)"),
        "{}",
        String::from_utf8_lossy(&o.stdout)
    );
    assert!(calls.lock().unwrap().iter().all(|c| c.0 == "GET"));
}

#[test]
fn publish_dry_run_new_collection_is_all_added() {
    let (srv, calls) = fake(|_, _| json(200, "[]"));
    let a = project(CONFIG, &[("a.yaml", VALID)]);
    let o = publish(&srv.base_url, a.path(), &["--dry-run"])
        .output()
        .unwrap();
    assert_eq!(o.status.code(), Some(0), "{}", stderr(&o));
    assert!(
        String::from_utf8_lossy(&o.stdout)
            .contains("payments-api: would publish (2 added, 0 changed, 0 removed)")
    );
    assert_eq!(calls.lock().unwrap().len(), 1);
}

#[test]
fn publish_sends_gzip_and_git_meta() {
    use std::io::Read;
    type Sent = Vec<(Option<String>, Vec<u8>)>;
    let seen: Arc<Mutex<Sent>> = Arc::default();
    let s2 = seen.clone();
    let (srv, _) = fake(move |r, _| {
        s2.lock().unwrap().push((
            r.header("content-encoding").map(str::to_string),
            r.body.clone(),
        ));
        json(200, UNCHANGED)
    });
    let a = project(
        "cloud:\n  collection: payments-api\n  name: Payments\n",
        &[("a.yaml", VALID)],
    );
    let o = publish(&srv.base_url, a.path(), &[])
        .env("GITHUB_SHA", "abc123")
        .env("GITHUB_REF", "refs/heads/main")
        .env("GITHUB_SERVER_URL", "https://github.com")
        .env("GITHUB_REPOSITORY", "acme/payments")
        .output()
        .unwrap();
    assert_eq!(o.status.code(), Some(0), "{}", stderr(&o));
    let seen = seen.lock().unwrap();
    assert_eq!(seen[0].0.as_deref(), Some("gzip"));
    let mut text = String::new();
    flate2::read::GzDecoder::new(&seen[0].1[..])
        .read_to_string(&mut text)
        .unwrap();
    let v: serde_json::Value = serde_json::from_str(&text).unwrap();
    assert_eq!(v["git"]["commit"], "abc123");
    assert_eq!(v["git"]["ref"], "refs/heads/main");
    assert_eq!(v["git"]["repo_url"], "https://github.com/acme/payments");
    assert_eq!(v["name"], "Payments");
    assert!(
        v["files"]
            .as_array()
            .unwrap()
            .iter()
            .any(|f| f["path"] == "a.yaml")
    );
    assert!(v["hash"].as_str().unwrap().len() >= 32);
}

#[test]
fn publish_without_ci_env_sends_no_git() {
    use std::io::Read;
    let seen: Arc<Mutex<Vec<Vec<u8>>>> = Arc::default();
    let s2 = seen.clone();
    let (srv, _) = fake(move |r, _| {
        s2.lock().unwrap().push(r.body.clone());
        json(200, UNCHANGED)
    });
    let a = project(CONFIG, &[("a.yaml", VALID)]);
    publish(&srv.base_url, a.path(), &[]).assert().success();
    let mut text = String::new();
    flate2::read::GzDecoder::new(&seen.lock().unwrap()[0][..])
        .read_to_string(&mut text)
        .unwrap();
    let v: serde_json::Value = serde_json::from_str(&text).unwrap();
    assert!(v["git"].is_null());
}

#[test]
fn publish_401_is_not_retried() {
    let (srv, calls) = fake(|_, _| json(401, r#"{"error":"unauthorized","message":"Bad token."}"#));
    let a = project(CONFIG, &[("a.yaml", VALID)]);
    let o = publish(&srv.base_url, a.path(), &[]).output().unwrap();
    assert_eq!(o.status.code(), Some(2));
    assert_eq!(calls.lock().unwrap().len(), 1);
    assert!(!stderr(&o).contains("Retrying"));
}

#[test]
fn publish_announces_retries_on_stderr() {
    let (srv, _) = fake(|_, n| {
        if n == 0 {
            json(503, r#"{"error":"unavailable","message":"Try again."}"#)
        } else {
            json(200, UNCHANGED)
        }
    });
    let a = project(CONFIG, &[("a.yaml", VALID)]);
    let o = publish(&srv.base_url, a.path(), &[]).output().unwrap();
    assert_eq!(o.status.code(), Some(0));
    assert!(stderr(&o).contains("Retrying in "), "{}", stderr(&o));
}

#[test]
fn publish_bad_team_is_a_usage_error() {
    let (srv, calls) = fake(|_, _| created(1));
    let a = project(CONFIG, &[("a.yaml", VALID)]);
    let o = publish(&srv.base_url, a.path(), &["--team", "Bad Team"])
        .output()
        .unwrap();
    assert_eq!(o.status.code(), Some(1));
    assert!(calls.lock().unwrap().is_empty());
}

#[test]
fn publish_long_name_fails_before_sending() {
    let (srv, calls) = fake(|_, _| created(1));
    let ok = project(CONFIG, &[("a.yaml", VALID)]);
    let long = project(
        &format!("cloud:\n  collection: other\n  name: {}\n", "n".repeat(101)),
        &[("a.yaml", VALID)],
    );
    let o = publish(&srv.base_url, ok.path(), &[])
        .arg(long.path())
        .output()
        .unwrap();
    assert_eq!(o.status.code(), Some(1));
    assert!(calls.lock().unwrap().is_empty());
}

#[test]
fn publish_dry_run_lists_collections_once() {
    let (srv, calls) = fake(|_, _| json(200, "[]"));
    let a = project(CONFIG, &[("a.yaml", VALID)]);
    let b = project("cloud:\n  collection: other\n", &[("a.yaml", VALID)]);
    let o = publish(&srv.base_url, a.path(), &["--dry-run"])
        .arg(b.path())
        .output()
        .unwrap();
    assert_eq!(o.status.code(), Some(0), "{}", stderr(&o));
    assert_eq!(calls.lock().unwrap().len(), 1);
}
