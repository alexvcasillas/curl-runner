use std::process::Command;

fn run(args: &[&str]) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_curl-runner"))
        .args(args)
        .output()
        .unwrap()
}

#[test]
fn help_prints_usage() {
    for flag in ["--help", "-h"] {
        let out = run(&[flag]);
        assert!(out.status.success());
        let text = String::from_utf8(out.stdout).unwrap();
        assert!(text.contains("USAGE:"));
        assert!(text.contains("--allow-protocol <proto>"));
        assert!(text.contains("--timeout <ms>"));
    }
}

#[test]
fn version_flag_wins_over_unknown_flags() {
    let out = run(&["--bogus", "--version"]);
    assert!(out.status.success());
    assert_eq!(
        String::from_utf8(out.stdout).unwrap().trim(),
        curl_runner_core::VERSION
    );
}
