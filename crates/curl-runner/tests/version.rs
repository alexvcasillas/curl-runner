use std::process::Command;

#[test]
fn prints_version() {
    let out = Command::new(env!("CARGO_BIN_EXE_curl-runner"))
        .arg("--version")
        .output()
        .unwrap();
    assert!(out.status.success());
    assert_eq!(
        String::from_utf8(out.stdout).unwrap().trim(),
        curl_runner_core::VERSION
    );
}

#[test]
fn libcurl_supports_https() {
    let v = curl::Version::get();
    assert!(v.ssl_version().is_some(), "libcurl built without TLS");
    assert!(v.protocols().any(|p| p == "https"), "https unsupported");
}
