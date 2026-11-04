use std::fs;
use std::path::Path;

use curl_runner_cloud_proto::{Input, bundle_hash};

use super::*;

fn write(dir: &Path, rel: &str, text: &str) {
    let p = dir.join(rel);
    fs::create_dir_all(p.parent().unwrap()).unwrap();
    fs::write(p, text).unwrap();
}

fn load(config: &str) -> Result<Option<CloudConfig>, CloudConfigError> {
    let d = tempfile::tempdir().unwrap();
    write(d.path(), "curl-runner.yaml", config);
    load_cloud_config(d.path())
}

fn secret(name: &str) -> Input {
    Input {
        name: name.into(),
        description: None,
        secret: true,
        required: false,
        locked: false,
        default: None,
    }
}

fn paths(b: &Bundle) -> Vec<&str> {
    b.files.iter().map(|f| f.path.as_str()).collect()
}

#[test]
fn cloud_config_absent_is_none() {
    let d = tempfile::tempdir().unwrap();
    assert!(load_cloud_config(d.path()).unwrap().is_none());
    assert!(load("global:\n  variables: {A: b}\n").unwrap().is_none());
}

#[test]
fn cloud_config_parses_inputs_and_defaults() {
    let cfg = load(
        "global:\n  variables:\n    BASE_URL: https://api.example.com\n    API_KEY: x\n\
         cloud:\n  collection: payments-api\n  name: Payments\n  inputs:\n    \
         - name: API_KEY\n      secret: true\n      required: true\n    - name: BASE_URL\n",
    )
    .unwrap()
    .unwrap();
    assert_eq!(cfg.collection, "payments-api");
    assert_eq!(cfg.name.as_deref(), Some("Payments"));
    let key = &cfg.inputs[0];
    assert!(key.secret && key.required && !key.locked);
    assert_eq!(key.default, None);
    let url = &cfg.inputs[1];
    assert!(!url.secret && !url.required);
    assert_eq!(url.default.as_deref(), Some("https://api.example.com"));
}

#[test]
fn cloud_config_collection_variables_and_own_default() {
    let cfg = load(
        "collection:\n  variables: {REGION: eu}\ncloud:\n  collection: acme\n  inputs:\n    \
         - name: REGION\n    - name: MODE\n      default: fast\n",
    )
    .unwrap()
    .unwrap();
    assert_eq!(cfg.inputs[0].default.as_deref(), Some("eu"));
    assert_eq!(cfg.inputs[1].default.as_deref(), Some("fast"));
}

#[test]
fn cloud_config_rejects_bad_slug() {
    for bad in ["Payments API", "-a", "", &"a".repeat(64)] {
        let r = load(&format!("cloud:\n  collection: \"{bad}\"\n"));
        assert!(matches!(r, Err(CloudConfigError::BadSlug(_))), "{bad}");
    }
    assert!(load(&format!("cloud:\n  collection: {}\n", "a".repeat(63))).is_ok());
}

#[test]
fn secret_prefix_defaults_secret() {
    let cfg = load("cloud:\n  collection: acme\n  inputs:\n    - name: SECRET_TOKEN\n")
        .unwrap()
        .unwrap();
    assert!(cfg.inputs[0].secret);
}

#[test]
fn scan_skips_env_hidden_and_gitignored() {
    let d = tempfile::tempdir().unwrap();
    for f in [
        "a.yaml",
        ".env",
        ".hidden/b.yaml",
        "drafts/c.yaml",
        "curl-runner.yaml",
    ] {
        write(d.path(), f, "a: 1\n");
    }
    write(d.path(), ".gitignore", "drafts/\n");
    let b = scan_bundle(d.path()).unwrap();
    assert_eq!(paths(&b), ["a.yaml", "curl-runner.yaml"]);
}

#[test]
fn scan_gitignore_rules() {
    let d = tempfile::tempdir().unwrap();
    for f in [
        "keep.yaml",
        "tmp-1.yaml",
        "tmp-keep.yaml",
        "sub/tmp-2.yaml",
        "sub/only.yaml",
        "sub/x/only.yaml",
        "root.yaml",
        "sub/root.yaml",
        "sub/deep/skip.yaml",
        "sub/deep/n/skip.yaml",
    ] {
        write(d.path(), f, "a: 1\n");
    }
    write(
        d.path(),
        ".gitignore",
        "# c\ntmp-*.yaml\n!tmp-keep.yaml\n/root.yaml\n",
    );
    write(d.path(), "sub/.gitignore", "/only.yaml\ndeep/**\n");
    let b = scan_bundle(d.path()).unwrap();
    assert_eq!(
        paths(&b),
        [
            "keep.yaml",
            "sub/root.yaml",
            "sub/x/only.yaml",
            "tmp-keep.yaml"
        ]
    );
}

#[cfg(unix)]
#[test]
fn scan_skips_link_out() {
    let d = tempfile::tempdir().unwrap();
    let out = tempfile::tempdir().unwrap();
    write(out.path(), "o.yaml", "a: 1\n");
    write(d.path(), "a.yaml", "a: 1\n");
    std::os::unix::fs::symlink(out.path().join("o.yaml"), d.path().join("l.yaml")).unwrap();
    let b = scan_bundle(d.path()).unwrap();
    assert_eq!(paths(&b), ["a.yaml"]);
    assert_eq!(b.skipped[0].1, SkipReason::OutsideRoot);
}

#[test]
fn scan_too_big_is_skipped() {
    let d = tempfile::tempdir().unwrap();
    write(d.path(), "a.yaml", "a: 1\n");
    write(
        d.path(),
        "big.yaml",
        &format!("a: {}\n", "x".repeat(1 << 20)),
    );
    let b = scan_bundle(d.path()).unwrap();
    assert_eq!(paths(&b), ["a.yaml"]);
    assert_eq!(b.skipped[0].1, SkipReason::TooBig);
}

#[test]
fn scan_hash_matches_proto() {
    let d = tempfile::tempdir().unwrap();
    write(d.path(), "z.yaml", "a: 1\n");
    write(d.path(), "a/b.yaml", "a: 2\n");
    let b = scan_bundle(d.path()).unwrap();
    assert_eq!(paths(&b), ["a/b.yaml", "z.yaml"]);
    assert_eq!(b.hash, bundle_hash(&b.files));
}

fn bundle_of(text: &str) -> Bundle {
    let d = tempfile::tempdir().unwrap();
    write(d.path(), "curl-runner.yaml", text);
    scan_bundle(d.path()).unwrap()
}

#[test]
fn secret_literal_detected() {
    let b = bundle_of("global:\n  variables:\n    API_KEY: sk_live_123\n");
    let leaks = find_secret_literals(&b, &[secret("API_KEY")]);
    assert_eq!(
        leaks,
        [SecretLeak {
            name: "API_KEY".into(),
            file: "curl-runner.yaml".into()
        }]
    );
    let b = bundle_of("collection:\n  variables:\n    API_KEY: k\n");
    assert_eq!(find_secret_literals(&b, &[secret("API_KEY")]).len(), 1);
    let b = bundle_of("variables:\n  API_KEY: k\n");
    assert_eq!(find_secret_literals(&b, &[secret("API_KEY")]).len(), 1);
}

#[test]
fn secret_reference_is_fine() {
    let b = bundle_of("global:\n  variables:\n    API_KEY: ${API_KEY}\n    B: \"\"\n");
    assert!(find_secret_literals(&b, &[secret("API_KEY"), secret("B")]).is_empty());
}

#[test]
fn non_secret_value_is_fine() {
    let b =
        bundle_of("global:\n  variables:\n    BASE_URL: https://api.example.com\n    API_KEY: k\n");
    let mut plain = secret("API_KEY");
    plain.secret = false;
    assert!(find_secret_literals(&b, &[plain]).is_empty());
    assert!(find_secret_literals(&b, &[secret("OTHER")]).is_empty());
}

#[test]
fn secret_in_other_places_is_not_scanned() {
    let b = bundle_of("request:\n  url: https://x\n  headers:\n    API_KEY: k\n");
    assert!(find_secret_literals(&b, &[secret("API_KEY")]).is_empty());
}
