//! `curl-runner convert` against goldens produced by the TypeScript CLI.
//!
//! `golden/convert/cases.json` lists every case (args + input files); the
//! `<case>.stdout|stderr|code|out` files next to it are what
//! `golden/convert/generate.ts` recorded from `bun run packages/cli/src/cli.ts
//! convert ...`. The Rust binary must reproduce them byte for byte, ANSI colors
//! included (TS always colors).

use assert_cmd::Command;
use serde_json::Value;
use std::path::{Path, PathBuf};

const BIN: &str = env!("CARGO_BIN_EXE_curl-runner");

fn golden_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/golden/convert")
}

fn run_case(case: &Value, dir: &Path) -> Vec<String> {
    let name = case["name"].as_str().unwrap();
    let work = tempfile::tempdir().unwrap();
    if let Some(files) = case.get("files").and_then(Value::as_object) {
        for (f, content) in files {
            let p = work.path().join(f);
            std::fs::create_dir_all(p.parent().unwrap()).unwrap();
            std::fs::write(p, content.as_str().unwrap()).unwrap();
        }
    }
    let args: Vec<&str> = case["args"]
        .as_array()
        .unwrap()
        .iter()
        .map(|a| a.as_str().unwrap())
        .collect();
    let out = Command::new(BIN)
        .env_clear()
        .current_dir(work.path())
        .arg("convert")
        .args(&args)
        .output()
        .unwrap();

    let mut bad = Vec::new();
    let mut check = |what: &str, got: &[u8]| {
        let want = std::fs::read(dir.join(format!("{name}.{what}"))).unwrap_or_default();
        if want != got {
            bad.push(format!(
                "{name}.{what}\n  want: {:?}\n  got:  {:?}",
                String::from_utf8_lossy(&want),
                String::from_utf8_lossy(got)
            ));
        }
    };
    check("stdout", &out.stdout);
    check("stderr", &out.stderr);
    check(
        "code",
        format!("{}\n", out.status.code().unwrap_or(-1)).as_bytes(),
    );
    if let Some(o) = case.get("out").and_then(Value::as_str) {
        let written = std::fs::read(work.path().join(o)).unwrap_or_default();
        check("out", &written);
    }
    bad
}

#[test]
fn convert_matches_the_typescript_cli_byte_for_byte() {
    let dir = golden_dir();
    let cases: Value =
        serde_json::from_str(&std::fs::read_to_string(dir.join("cases.json")).unwrap()).unwrap();
    let cases = cases.as_array().unwrap();
    assert!(cases.len() > 300, "cases.json looks truncated");
    let failures: Vec<String> = cases.iter().flat_map(|c| run_case(c, &dir)).collect();
    assert!(
        failures.is_empty(),
        "{} golden mismatches:\n{}",
        failures.len(),
        failures.join("\n")
    );
}
