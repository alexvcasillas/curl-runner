//! `curl-runner validate` against goldens recorded from the TypeScript CLI
//! (`cd tests/golden/validate && bun packages/cli/src/cli.ts validate edge
//! [--strict -q] > edge[-strict].stdout; echo $? > edge[-strict].code`; TS sources removed in 2.0, see 7670118).
//! `fix-output.yaml` is TS's `--fix` output with the logged emitter fixes.
//! `edge/` holds odd-shaped inputs (wrong types, typos, fixable issues) that
//! TS validates without throwing. Stdout must match byte for byte, colors
//! included (TS always colors).

use assert_cmd::Command;
use std::path::{Path, PathBuf};

const BIN: &str = env!("CARGO_BIN_EXE_curl-runner");

fn golden_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/golden/validate")
}

fn check(name: &str, args: &[&str]) {
    let dir = golden_dir();
    let out = Command::new(BIN)
        .env_clear()
        .envs(testkit::system_env())
        .current_dir(&dir)
        .arg("validate")
        .args(args)
        .output()
        .unwrap();
    let want = std::fs::read_to_string(dir.join(format!("{name}.stdout"))).unwrap();
    let code: i32 = std::fs::read_to_string(dir.join(format!("{name}.code")))
        .unwrap()
        .trim()
        .parse()
        .unwrap();
    assert_eq!(String::from_utf8_lossy(&out.stdout), want, "{name}");
    assert_eq!(out.status.code(), Some(code), "{name}");
    assert!(out.stderr.is_empty(), "{name}");
}

#[test]
fn edge_cases_match_ts() {
    check("edge", &["edge"]);
}

#[test]
fn edge_cases_strict_quiet_match_ts() {
    check("edge-strict", &["edge", "--strict", "-q"]);
}

#[test]
fn fix_rewrites_like_ts() {
    let work = tempfile::tempdir().unwrap();
    let input = std::fs::read_to_string(golden_dir().join("fix-input.yaml")).unwrap();
    std::fs::write(work.path().join("f.yaml"), input).unwrap();
    let out = Command::new(BIN)
        .env_clear()
        .envs(testkit::system_env())
        .current_dir(work.path())
        .args(["validate", "f.yaml", "--fix"])
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(1));
    let want = std::fs::read(golden_dir().join("fix-output.yaml")).unwrap();
    assert_eq!(std::fs::read(work.path().join("f.yaml")).unwrap(), want);
}

#[test]
fn crash_prints_type_error() {
    let work = tempfile::tempdir().unwrap();
    std::fs::write(work.path().join("a.yaml"), "5\n").unwrap();
    let out = Command::new(BIN)
        .env_clear()
        .envs(testkit::system_env())
        .current_dir(work.path())
        .args(["validate", "a.yaml"])
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(1));
    assert_eq!(
        String::from_utf8_lossy(&out.stderr),
        "\x1b[31m✗ content is not an Object. (evaluating '\"request\" in content')\x1b[0m\n"
    );
}
