//! `curl-runner upgrade` end to end: the real binary, copied out of `target/`
//! (so it is detected as standalone), against a fake GitHub from `testkit`.

use sha2::{Digest, Sha256};
use std::io::Write;
use std::process::Command;
use testkit::{Response, TestServer};

const BIN: &str = env!("CARGO_BIN_EXE_curl-runner");
const NEW: &[u8] = b"#!/bin/sh\necho upgraded\n";

fn platform() -> (&'static str, &'static str) {
    let os = match std::env::consts::OS {
        "macos" => "darwin",
        o => o,
    };
    let arch = if std::env::consts::ARCH == "aarch64" {
        "arm64"
    } else {
        "x64"
    };
    match (os, arch) {
        ("darwin", "arm64") => ("darwin-arm64", "tar.gz"),
        ("darwin", _) => ("darwin-x64", "tar.gz"),
        ("linux", "arm64") => ("linux-arm64", "tar.gz"),
        ("linux", _) => ("linux-x64", "tar.gz"),
        _ => ("windows-x64", "zip"),
    }
}

fn archive(platform: &str) -> Vec<u8> {
    let mut tar_bytes = Vec::new();
    let mut b = tar::Builder::new(&mut tar_bytes);
    let mut h = tar::Header::new_gnu();
    h.set_size(NEW.len() as u64);
    h.set_mode(0o644);
    h.set_cksum();
    b.append_data(&mut h, format!("curl-runner-{platform}"), NEW)
        .unwrap();
    b.finish().unwrap();
    drop(b);
    let mut gz = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::fast());
    gz.write_all(&tar_bytes).unwrap();
    gz.finish().unwrap()
}

fn github(good_sum: bool) -> TestServer {
    let (plat, _) = platform();
    let data = archive(plat);
    let name = format!("curl-runner-cli-99.0.0-{plat}.tar.gz");
    let sum: String = if good_sum {
        Sha256::digest(&data)
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect()
    } else {
        "f".repeat(64)
    };
    TestServer::with_handler(move |r| {
        let base = "/alexvcasillas/curl-runner/releases/download/@curl-runner/cli@99.0.0/";
        if r.url == "/repos/alexvcasillas/curl-runner/releases/latest" {
            Response::from_data(br#"{"tag_name":"@curl-runner/cli@99.0.0"}"#.to_vec())
        } else if r.url == format!("{base}{name}") {
            Response::from_data(data.clone())
        } else if r.url == format!("{base}SHA256SUMS.txt") {
            Response::from_data(format!("{sum}  {name}\n").into_bytes())
        } else {
            Response::from_data(vec![]).with_status_code(404)
        }
    })
}

struct Installed {
    _dir: tempfile::TempDir,
    exe: std::path::PathBuf,
}

fn install_copy() -> Installed {
    let dir = tempfile::tempdir().unwrap();
    let exe = dir.path().join("curl-runner");
    std::fs::copy(BIN, &exe).unwrap();
    Installed { _dir: dir, exe }
}

fn run(i: &Installed, srv: &TestServer, args: &[&str]) -> (i32, String) {
    let out = Command::new(&i.exe)
        .env_clear()
        .envs(testkit::system_env())
        .env("NO_COLOR", "1")
        .env("CURL_RUNNER_TEST_GITHUB_BASE", &srv.base_url)
        .arg("upgrade")
        .args(args)
        .output()
        .unwrap();
    (
        out.status.code().unwrap_or(-1),
        String::from_utf8_lossy(&out.stdout).into_owned(),
    )
}

#[cfg(unix)]
#[test]
fn upgrade_replaces_the_standalone_binary() {
    let (srv, i) = (github(true), install_copy());
    let (code, out) = run(&i, &srv, &[]);
    assert_eq!(code, 0, "{out}");
    assert!(out.contains("Installation: standalone binary"), "{out}");
    assert!(out.contains("Latest version: 99.0.0"), "{out}");
    assert!(out.contains("Upgrade complete!"), "{out}");
    assert_eq!(std::fs::read(&i.exe).unwrap(), NEW);
}

#[test]
fn bad_checksum_leaves_the_binary_untouched() {
    let (srv, i) = (github(false), install_copy());
    let before = std::fs::read(&i.exe).unwrap();
    let (code, out) = run(&i, &srv, &[]);
    assert_eq!(code, 1, "{out}");
    assert!(out.contains("checksum mismatch"), "{out}");
    assert_eq!(std::fs::read(&i.exe).unwrap(), before);
}

#[test]
fn dry_run_downloads_nothing() {
    let (srv, i) = (github(true), install_copy());
    let before = std::fs::read(&i.exe).unwrap();
    let (code, out) = run(&i, &srv, &["--dry-run"]);
    assert_eq!(code, 0, "{out}");
    assert!(out.contains("Dry run - would execute:"), "{out}");
    assert_eq!(std::fs::read(&i.exe).unwrap(), before);
}

#[test]
fn help_and_unreachable_registry() {
    let (srv, i) = (github(true), install_copy());
    let (code, out) = run(&i, &srv, &["--help"]);
    assert_eq!(code, 0);
    assert!(out.contains("curl-runner upgrade [options]"), "{out}");
    // A dev build (this test binary lives in target/) is never replaced.
    let out = Command::new(BIN)
        .env_clear()
        .envs(testkit::system_env())
        .env("NO_COLOR", "1")
        .env("CURL_RUNNER_TEST_GITHUB_BASE", &srv.base_url)
        .arg("upgrade")
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(1));
    assert!(String::from_utf8_lossy(&out.stdout).contains("Cannot upgrade"));
    let dead = Command::new(&i.exe)
        .env_clear()
        .envs(testkit::system_env())
        .env("NO_COLOR", "1")
        .env("CURL_RUNNER_TEST_GITHUB_BASE", "http://127.0.0.1:1")
        .arg("upgrade")
        .output()
        .unwrap();
    assert_eq!(dead.status.code(), Some(1));
    assert!(String::from_utf8_lossy(&dead.stdout).contains("Failed to fetch latest version"));
}
