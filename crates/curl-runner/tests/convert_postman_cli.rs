//! `curl-runner convert postman`: a collection in, YAML files and a `.env` out.

use assert_cmd::Command;
use std::path::Path;

const BIN: &str = env!("CARGO_BIN_EXE_curl-runner");
const SAMPLE: &str =
    include_str!("../../curl-runner-core/tests/fixtures/postman/sample.postman_collection.json");

fn convert(work: &Path, args: &[&str]) -> std::process::Output {
    Command::new(BIN)
        .env_clear()
        .envs(testkit::system_env())
        .current_dir(work)
        .arg("convert")
        .arg("postman")
        .args(args)
        .output()
        .unwrap()
}

fn stderr(out: &std::process::Output) -> String {
    String::from_utf8_lossy(&out.stderr).into_owned()
}

#[test]
fn writes_files_and_reports() {
    let work = tempfile::tempdir().unwrap();
    std::fs::write(work.path().join("c.json"), SAMPLE).unwrap();
    let out = convert(work.path(), &["c.json", "-o", "out"]);
    let err = stderr(&out);
    assert_eq!(out.status.code(), Some(0), "{err}");
    assert!(
        err.contains("Converted 5 requests in 2 folders to out"),
        "{err}"
    );
    assert!(err.contains("Needs your attention:"), "{err}");
    assert!(
        err.contains("Set the secret") || err.contains("Pick a file"),
        "{err}"
    );
    assert!(
        err.contains("Run them from out so curl-runner loads its .env."),
        "{err}"
    );
    assert!(err.contains("Set the secret apiKey"), "{err}");
    assert!(err.contains("Set the secret PASSWORD"), "{err}");
    assert!(
        err.contains("Set the secrets above in out/.env.local."),
        "{err}"
    );
    assert!(!err.contains("test-key-123"), "{err}");
    assert!(work.path().join("out/.env").is_file());
    assert!(!tree_text(&work.path().join("out")).contains("test-key-123"));
}

#[test]
fn rejects_a_v1_collection() {
    let work = tempfile::tempdir().unwrap();
    std::fs::write(
        work.path().join("v1.json"),
        r#"{"info":{"name":"Old","schema":"https://schema.getpostman.com/json/collection/v1.0.0/collection.json"},"item":[]}"#,
    )
    .unwrap();
    let out = convert(work.path(), &["v1.json"]);
    assert_eq!(out.status.code(), Some(1));
    assert!(
        stderr(&out).contains("Unsupported Postman collection schema"),
        "{}",
        stderr(&out)
    );
}

#[test]
fn default_output_is_named_after_the_collection() {
    let work = tempfile::tempdir().unwrap();
    std::fs::write(work.path().join("c.json"), SAMPLE).unwrap();
    let out = convert(work.path(), &["c.json"]);
    assert_eq!(out.status.code(), Some(0), "{}", stderr(&out));
    assert!(work.path().join("example-api/.env").is_file());
}

#[test]
fn a_missing_file_exits_1() {
    let work = tempfile::tempdir().unwrap();
    let out = convert(work.path(), &["nope.json"]);
    assert_eq!(out.status.code(), Some(1));
    assert!(stderr(&out).contains("ENOENT"), "{}", stderr(&out));
}

#[test]
fn help_describes_the_directory_output() {
    let work = tempfile::tempdir().unwrap();
    let out = convert(work.path(), &["--help"]);
    assert_eq!(out.status.code(), Some(0));
    assert!(String::from_utf8_lossy(&out.stdout).contains("-o, --output <dir>"));
}

fn tree_text(dir: &Path) -> String {
    let mut all = String::new();
    for e in std::fs::read_dir(dir).unwrap() {
        let p = e.unwrap().path();
        if p.is_dir() {
            all.push_str(&tree_text(&p));
        } else {
            all.push_str(&std::fs::read_to_string(&p).unwrap());
        }
    }
    all
}
