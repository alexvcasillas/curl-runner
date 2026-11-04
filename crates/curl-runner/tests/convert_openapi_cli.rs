//! `curl-runner convert openapi`: a spec in, YAML files and a `.env` out.

use assert_cmd::Command;
use std::path::Path;

const BIN: &str = env!("CARGO_BIN_EXE_curl-runner");
const SPEC: &str = include_str!("../../curl-runner-core/tests/fixtures/openapi/small_v30.yaml");
const POSTMAN: &str =
    include_str!("../../curl-runner-core/tests/fixtures/postman/sample.postman_collection.json");

fn convert(work: &Path, args: &[&str]) -> std::process::Output {
    Command::new(BIN)
        .env_clear()
        .envs(testkit::system_env())
        .current_dir(work)
        .arg("convert")
        .arg("openapi")
        .args(args)
        .output()
        .unwrap()
}

fn stderr(out: &std::process::Output) -> String {
    String::from_utf8_lossy(&out.stderr).into_owned()
}

#[test]
fn converts_into_named_folder() {
    let work = tempfile::tempdir().unwrap();
    std::fs::write(work.path().join("spec.yaml"), SPEC).unwrap();
    let out = convert(work.path(), &["spec.yaml"]);
    let err = stderr(&out);
    assert_eq!(out.status.code(), Some(0), "{err}");
    assert!(
        err.starts_with("Converted 6 requests in 3 folders to pets"),
        "{err}"
    );
    assert!(work.path().join("pets/.env").is_file());
    assert!(
        err.contains("Run them from pets so curl-runner loads its .env."),
        "{err}"
    );
}

#[test]
fn lists_secrets_first() {
    let work = tempfile::tempdir().unwrap();
    std::fs::write(work.path().join("spec.yaml"), SPEC).unwrap();
    let err = stderr(&convert(work.path(), &["spec.yaml", "-o", "out"]));
    let attention = err.find("Needs your attention:").expect(&err);
    let secret = err.find("Set the secret PETS_API_KEY").expect(&err);
    assert!(attention < secret, "{err}");
    let pick = err.find("Pick a file").expect(&err);
    assert!(pick < secret, "{err}");
    assert!(
        err.contains("Set the secrets above in out/.env.local."),
        "{err}"
    );
}

#[test]
fn bad_input_exits_1() {
    let work = tempfile::tempdir().unwrap();
    std::fs::write(work.path().join("c.json"), POSTMAN).unwrap();
    let out = convert(work.path(), &["c.json"]);
    assert_eq!(out.status.code(), Some(1));
    assert!(
        stderr(&out).contains("Not an OpenAPI document: no \"openapi\" or \"swagger\" field"),
        "{}",
        stderr(&out)
    );
}

#[test]
fn help_lists_openapi() {
    let work = tempfile::tempdir().unwrap();
    let out = convert(work.path(), &["--help"]);
    assert_eq!(out.status.code(), Some(0));
    let text = String::from_utf8_lossy(&out.stdout).into_owned();
    assert!(
        text.contains("curl-runner convert openapi <spec.json|spec.yaml> [-o <dir>]"),
        "{text}"
    );
}

#[test]
fn access_token_is_not_also_a_set_secret() {
    let work = tempfile::tempdir().unwrap();
    let spec = r#"{"openapi":"3.0.0","info":{"title":"Pets","version":"1"},
        "servers":[{"url":"https://api.example.com"}],
        "components":{"securitySchemes":{"oauth":{"type":"oauth2","flows":{}}}},
        "security":[{"oauth":[]}],
        "paths":{"/a":{"get":{"responses":{"200":{"description":"ok"}}}}}}"#;
    std::fs::write(work.path().join("spec.json"), spec).unwrap();
    let err = stderr(&convert(work.path(), &["spec.json", "-o", "out"]));
    assert!(
        err.contains("Put an OAuth 2 access token in PETS_OAUTH"),
        "{err}"
    );
    assert!(!err.contains("Set the secret PETS_OAUTH"), "{err}");
    assert!(
        err.contains("Set the secrets above in out/.env.local."),
        "{err}"
    );
}
