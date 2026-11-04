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
