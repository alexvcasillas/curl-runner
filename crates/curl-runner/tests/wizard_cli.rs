//! `curl-runner init` / `edit` through the real binary. Without a terminal
//! the wizard reads one answer per stdin line, so the whole flow is driven
//! from here; end of input cancels (TS: exit 0), it never waits.

use std::io::Write;
use std::path::Path;
use std::process::{Command, Output, Stdio};
use std::time::{Duration, Instant};

const BIN: &str = env!("CARGO_BIN_EXE_curl-runner");

/// Run with `stdin` (None: closed), killing it if it takes more than 15 s.
fn run(args: &[&str], dir: &Path, stdin: Option<&str>) -> (Output, Duration) {
    let mut cmd = Command::new(BIN);
    cmd.args(args)
        .current_dir(dir)
        .env_clear()
        .env("NO_COLOR", "1")
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    match stdin {
        Some(_) => cmd.stdin(Stdio::piped()),
        None => cmd.stdin(Stdio::null()),
    };
    let start = Instant::now();
    let mut child = cmd.spawn().unwrap();
    if let Some(input) = stdin {
        let mut sin = child.stdin.take().unwrap();
        let _ = sin.write_all(input.as_bytes());
    }
    loop {
        if child.try_wait().unwrap().is_some() {
            break;
        }
        if start.elapsed() > Duration::from_secs(15) {
            let _ = child.kill();
            let out = child.wait_with_output().unwrap();
            panic!("hung: {args:?}\n{}", String::from_utf8_lossy(&out.stderr));
        }
        std::thread::sleep(Duration::from_millis(10));
    }
    let took = start.elapsed();
    (child.wait_with_output().unwrap(), took)
}

fn s(b: &[u8]) -> String {
    String::from_utf8_lossy(b).into_owned()
}

#[test]
fn init_with_empty_stdin_cancels_promptly_with_exit_0() {
    // TS (`bun run cli.ts init < /dev/null`): prints the intro and the first
    // prompt, then the process ends with exit code 0.
    let d = tempfile::tempdir().unwrap();
    for args in [&["init"][..], &["init", "--wizard"], &["edit", "x.yaml"]] {
        let (o, took) = run(args, d.path(), None);
        assert!(took < Duration::from_secs(10), "{args:?} took {took:?}");
        let code = o.status.code();
        if args[0] == "edit" {
            assert_eq!(
                code,
                Some(1),
                "missing file is a failed cancel: {}",
                s(&o.stderr)
            );
            assert!(s(&o.stderr).contains("File not found: x.yaml"));
        } else {
            assert_eq!(code, Some(0), "{args:?}: {}", s(&o.stderr));
            let err = s(&o.stderr);
            assert!(err.contains("curl-runner"), "{err}");
        }
        assert_eq!(
            std::fs::read_dir(d.path()).unwrap().count(),
            0,
            "nothing written"
        );
    }
}

#[cfg(unix)]
#[test]
fn init_with_a_closed_stdin_descriptor_exits_promptly_with_exit_0() {
    let d = tempfile::tempdir().unwrap();
    let start = Instant::now();
    let o = Command::new("sh")
        .args(["-c", "exec \"$0\" init <&-", BIN])
        .current_dir(d.path())
        .env_clear()
        .output()
        .unwrap();
    assert!(start.elapsed() < Duration::from_secs(10));
    assert_eq!(o.status.code(), Some(0), "{}", s(&o.stderr));
}

#[test]
fn init_wizard_piped_writes_the_ts_yaml() {
    let d = tempfile::tempdir().unwrap();
    let input = [
        "basic-post",
        "https://api.example.com/users",
        "", // method POST (template)
        "Create user",
        "n", // headers? (template has Content-Type: say no, keep it)
        "json",
        "{\"name\": \"ann\", \"tags\": [\"a\", \"b\"]}",
        "bearer",
        "tok_123",
        "n",
        "n",
        "n", // advanced, retry, validation
        "out/users.yaml",
        "n", // run after
        "y", // save
    ]
    .join("\n")
        + "\n";
    let (o, _) = run(&["init", "--wizard"], d.path(), Some(&input));
    assert_eq!(o.status.code(), Some(0), "{}", s(&o.stderr));
    let want = "request:\n  url: \"https://api.example.com/users\"\n  name: Create user\n  method: POST\n  headers:\n    Content-Type: application/json\n  body:\n    name: ann\n    tags: [a, b]\n  auth:\n    type: bearer\n    token: tok_123";
    assert_eq!(
        std::fs::read_to_string(d.path().join("out/users.yaml")).unwrap(),
        want
    );
    // the preview goes to stdout, like console.log
    assert!(s(&o.stdout).contains(want));
    assert!(s(&o.stderr).contains("Saved to out/users.yaml"));
}

#[test]
fn init_wizard_output_option_is_the_default_path() {
    let d = tempfile::tempdir().unwrap();
    let input = "blank\nhttps://x.io\n\n\nn\nnone\nn\nn\nn\n\nn\ny\n";
    let (o, _) = run(&["init", "-w", "-o", "mine.yml"], d.path(), Some(input));
    assert_eq!(o.status.code(), Some(0), "{}", s(&o.stderr));
    assert_eq!(
        std::fs::read_to_string(d.path().join("mine.yml")).unwrap(),
        "request:\n  url: \"https://x.io\""
    );
}

#[test]
fn quick_init_with_url_argument() {
    let d = tempfile::tempdir().unwrap();
    let (o, _) = run(
        &["init", "https://api.example.com/users"],
        d.path(),
        Some("POST\nusers.yaml\nn\n"),
    );
    assert_eq!(o.status.code(), Some(0), "{}", s(&o.stderr));
    assert_eq!(
        std::fs::read_to_string(d.path().join("users.yaml")).unwrap(),
        "request:\n  url: \"https://api.example.com/users\"\n  method: POST"
    );
    assert!(s(&o.stderr).contains("Created users.yaml"));
}

#[test]
fn run_after_creating_runs_the_file() {
    let d = tempfile::tempdir().unwrap();
    // an unreachable port: the run fails, which the wizard ignores (TS too)
    let input = "blank\nhttp://127.0.0.1:1/\n\n\nn\nnone\nn\nn\nn\nr.yaml\ny\ny\n";
    let (o, _) = run(&["init", "--wizard"], d.path(), Some(input));
    assert_eq!(o.status.code(), Some(0), "{}", s(&o.stderr));
    assert!(s(&o.stderr).contains("Running request..."));
    let all = format!("{}{}", s(&o.stdout), s(&o.stderr));
    assert!(all.contains("127.0.0.1"), "the request ran: {all}");
}

#[test]
fn cancel_midway_exits_0_and_writes_nothing() {
    let d = tempfile::tempdir().unwrap();
    let (o, _) = run(
        &["init", "--wizard"],
        d.path(),
        Some("blank\nhttps://x.io\nPOST\n"),
    );
    assert_eq!(o.status.code(), Some(0));
    assert!(s(&o.stderr).contains("Wizard cancelled"));
    assert_eq!(std::fs::read_dir(d.path()).unwrap().count(), 0);
}

const MULTI: &str = "# keep me\nglobal:\n  timeout: 9\nrequests:\n  - url: https://a.test\n    method: GET\n    expect:\n      status: 200\n      body:\n        ok: true\n  - url: https://b.test\n    method: POST\n";

#[test]
fn edit_keeps_everything_else_and_writes_atomically() {
    let d = tempfile::tempdir().unwrap();
    let f = d.path().join("t.yaml");
    std::fs::write(&f, MULTI).unwrap();
    // url, method -> HEAD, name, headers n, auth none, advanced n, retry n,
    // validation n, save to (default = the file), run n, save y
    let (o, _) = run(
        &["edit", "t.yaml"],
        d.path(),
        Some("\nHEAD\n\nn\n\nn\nn\nn\n\nn\ny\n"),
    );
    assert_eq!(o.status.code(), Some(0), "{}", s(&o.stderr));
    let text = std::fs::read_to_string(&f).unwrap();
    assert!(text.contains("method: HEAD"), "{text}");
    assert!(
        text.contains("https://b.test") && text.contains("timeout: 9") && text.contains("ok: true"),
        "{text}"
    );
    let names: Vec<_> = std::fs::read_dir(d.path())
        .unwrap()
        .map(|e| e.unwrap().file_name())
        .collect();
    assert_eq!(names, ["t.yaml"], "no temp file left");
}

#[test]
fn edit_refuses_files_it_could_corrupt() {
    let d = tempfile::tempdir().unwrap();
    for (name, src) in [
        ("list.yaml", "- url: a\n- url: b\n"),
        (
            "multi.yaml",
            "request:\n  url: a\n---\nrequest:\n  url: b\n",
        ),
        ("none.yaml", "global:\n  timeout: 1\n"),
        ("bad.yaml", "request: [\n"),
    ] {
        let f = d.path().join(name);
        std::fs::write(&f, src).unwrap();
        let (o, _) = run(&["edit", name], d.path(), Some("\n\n\n\n\n\n\n\n\n\n\n\n"));
        assert_eq!(o.status.code(), Some(1), "{name}: {}", s(&o.stderr));
        assert!(
            s(&o.stderr).contains('\u{2717}'),
            "{name}: {}",
            s(&o.stderr)
        );
        assert_eq!(
            std::fs::read_to_string(&f).unwrap(),
            src,
            "{name} untouched"
        );
    }
}

#[test]
fn edit_without_a_file_prints_usage() {
    let d = tempfile::tempdir().unwrap();
    let (o, _) = run(&["edit"], d.path(), None);
    assert_eq!(o.status.code(), Some(1));
    assert!(s(&o.stderr).contains("Usage: curl-runner edit <file.yaml>"));
}

#[test]
fn help_for_init_and_edit() {
    let d = tempfile::tempdir().unwrap();
    let (o, _) = run(&["init", "--help"], d.path(), None);
    assert_eq!(o.status.code(), Some(0));
    let out = s(&o.stdout);
    assert!(
        out.starts_with(
            "\n\u{1F680} CURL RUNNER - INIT\n\nUSAGE:\n  curl-runner init [url] [options]\n"
        ),
        "{out}"
    );
    assert!(
        out.ends_with("  curl-runner init -w -o api-test.yaml\n\n"),
        "{out:?}"
    );
    let (o, _) = run(&["edit", "-h"], d.path(), None);
    let out = s(&o.stdout);
    assert!(
        out.contains("CURL RUNNER - EDIT")
            && out.contains("Save to different file (default: overwrite original)")
    );
}
