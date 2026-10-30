//! Public lint API, plus the rule-internal checks that moved with the rules.

use super::issues::find_closest_match;
use super::*;
use crate::model::{
    DiffConfig, EnvConfig, GlobalConfig, GlobalDiffConfig, GlobalSnapshotConfig, ProfileConfig,
    RetryConfig, SecurityConfig, SnapshotConfig,
};

#[test]
fn unknown_key_is_a_warning_with_rename_fix() {
    let LintOutcome::Linted { issues, .. } = lint_text(
        "request:\n  url: https://api.example.com\n  retry:\n    count: 3\n    backof: 2\n",
    )
    .unwrap() else {
        panic!()
    };
    let i = issues
        .iter()
        .find(|i| i.path == "request.retry.backof")
        .unwrap();
    assert_eq!(i.severity, Severity::Warning);
    assert!(
        matches!(&i.fix, Some(Fix { value: FixValue::Rename { to, .. }, .. }) if to == "backoff")
    );
    assert!(is_valid(&issues));
}

#[test]
fn invalid_method_is_an_error() {
    let LintOutcome::Linted { issues, .. } =
        lint_text("request:\n  url: https://api.example.com\n  method: FETCH\n").unwrap()
    else {
        panic!()
    };
    assert!(
        issues
            .iter()
            .any(|i| i.path == "request.method" && i.severity == Severity::Error)
    );
    assert!(!is_valid(&issues));
}

#[test]
fn parse_error_is_reported_not_crashed() {
    assert!(
        matches!(lint_text("request: [\n"), Ok(LintOutcome::ParseError(m)) if m.starts_with("Failed to parse YAML"))
    );
}

#[test]
fn scalar_root_crashes_with_message() {
    assert!(matches!(lint_text("hello"), Err(Crash(m)) if m.contains("is not an Object")));
}

#[test]
fn closest_match_and_edit_distance() {
    fn m(i: &str, o: &[&'static str]) -> Option<&'static str> {
        find_closest_match(i, o)
    }
    assert_eq!(m("DELET", super::request::VALID_METHODS), Some("DELETE"));
    assert_eq!(
        m("parallell", &["sequential", "parallel"]),
        Some("parallel")
    );
    assert_eq!(m("xyz", &["sequential", "parallel"]), None);
    assert_eq!(m("", &["a", "b"]), Some("a"));
    assert_eq!(
        m("metod", super::request::VALID_REQUEST_KEYS),
        Some("method")
    );
}

// ---- #136: validator key lists track the config model -----------------

fn sorted(a: &[&str]) -> Vec<String> {
    let mut v: Vec<String> = a.iter().map(|s| s.to_string()).collect();
    v.sort();
    v
}

/// Serialized keys of a value built with every field set.
fn keys_of<T: serde::Serialize>(v: &T) -> Vec<String> {
    let mut k: Vec<String> = serde_json::to_value(v)
        .unwrap()
        .as_object()
        .unwrap()
        .keys()
        .cloned()
        .collect();
    k.sort();
    k
}

// Each literal sets EVERY field (no `..Default::default()`): a new model
// field fails to compile here, and its serialized key must be in the
// validator's list.
fn model_retry() -> Vec<String> {
    keys_of(&RetryConfig {
        count: Some(1),
        delay: Some(1),
        backoff: Some(1.0),
        retryable_statuses: Some(vec![503]),
    })
}

fn model_profile() -> Vec<String> {
    keys_of(&ProfileConfig {
        iterations: 1,
        warmup: Some(1),
        concurrency: Some(1),
        histogram: Some(true),
        export_file: Some("x".into()),
    })
}

fn model_env() -> Vec<String> {
    keys_of(&EnvConfig {
        environment: Some("x".into()),
        redact_secrets: Some(true),
    })
}

fn model_security() -> Vec<String> {
    keys_of(&SecurityConfig {
        allowed_protocols: Some(vec![]),
        allow_paths: Some(true),
    })
}

fn model_request_snapshot() -> Vec<String> {
    keys_of(&SnapshotConfig {
        enabled: Some(true),
        name: Some("x".into()),
        include: Some(vec![]),
        exclude: Some(vec![]),
        match_rules: Some(Default::default()),
    })
}

fn model_global_snapshot() -> Vec<String> {
    keys_of(&GlobalSnapshotConfig {
        enabled: Some(true),
        name: Some("x".into()),
        include: Some(vec![]),
        exclude: Some(vec![]),
        match_rules: Some(Default::default()),
        dir: Some("x".into()),
        update_mode: Some("none".into()),
        ci: Some(true),
    })
}

fn model_request_diff() -> Vec<String> {
    keys_of(&DiffConfig {
        enabled: Some(true),
        exclude: Some(vec![]),
        match_rules: Some(Default::default()),
        include_timings: Some(true),
    })
}

fn model_global_diff() -> Vec<String> {
    keys_of(&GlobalDiffConfig {
        enabled: Some(true),
        exclude: Some(vec![]),
        match_rules: Some(Default::default()),
        include_timings: Some(true),
        dir: Some("x".into()),
        label: Some("x".into()),
        compare_with: Some("x".into()),
        save: Some(true),
        output_format: Some("json".into()),
    })
}

fn model_global() -> Vec<String> {
    keys_of(&GlobalConfig {
        env: Some(Default::default()),
        execution: Some("parallel".into()),
        max_concurrency: Some(1),
        continue_on_error: Some(true),
        dry_run: Some(true),
        http2: Some(true),
        connection_pool: Some(Default::default()),
        ci: Some(Default::default()),
        ssl: Some(Default::default()),
        watch: Some(Default::default()),
        profile: Some(Default::default()),
        snapshot: Some(Default::default()),
        diff: Some(Default::default()),
        security: Some(Default::default()),
        variables: Some(Default::default()),
        output: Some(Default::default()),
        defaults: Some(Default::default()),
    })
}

#[test]
fn validator_key_lists_match_model() {
    use super::global::*;
    let cases: [(&str, &[&str], Vec<String>); 9] = [
        ("retry", super::request::VALID_RETRY_KEYS, model_retry()),
        ("profile", VALID_PROFILE_KEYS, model_profile()),
        ("env", VALID_ENV_KEYS, model_env()),
        ("security", VALID_SECURITY_KEYS, model_security()),
        (
            "request snapshot",
            VALID_REQUEST_SNAPSHOT_KEYS,
            model_request_snapshot(),
        ),
        (
            "global snapshot",
            VALID_GLOBAL_SNAPSHOT_KEYS,
            model_global_snapshot(),
        ),
        (
            "request diff",
            VALID_REQUEST_DIFF_KEYS,
            model_request_diff(),
        ),
        ("global diff", VALID_GLOBAL_DIFF_KEYS, model_global_diff()),
        ("global", VALID_GLOBAL_KEYS, model_global()),
    ];
    for (name, valid, model) in cases {
        assert_eq!(sorted(valid), model, "{name}");
    }
}

fn lint(yaml: &str) -> Vec<Issue> {
    match lint_text(yaml).unwrap() {
        LintOutcome::Linted { issues, .. } => issues,
        LintOutcome::ParseError(m) => panic!("{m}"),
    }
}

fn unknown_warnings(issues: &[Issue]) -> usize {
    issues
        .iter()
        .filter(|i| i.message.contains("Unknown key"))
        .count()
}

/// Minimal valid value for each key, as inline YAML.
fn sample(key: &str) -> &'static str {
    match key {
        "iterations" | "warmup" | "concurrency" | "count" | "delay" => "1",
        "backoff" => "2",
        "histogram" | "redactSecrets" | "allowPaths" | "enabled" | "includeTimings" | "save"
        | "ci" => "true",
        "retryableStatuses" => "[503]",
        "allowedProtocols" | "include" | "exclude" => "[http]",
        "match" => "{a: \"*\"}",
        "updateMode" => "none",
        "outputFormat" => "json",
        _ => "x",
    }
}

fn section_yaml(prefix: &str, keys: &[&str], extra: Option<&str>) -> String {
    let mut body: Vec<String> = keys.iter().map(|k| format!("{k}: {}", sample(k))).collect();
    if let Some(e) = extra {
        body.push(format!("{e}: 1"));
    }
    let obj = format!("{{ {} }}", body.join(", "));
    match prefix {
        "retry" | "request.snapshot" | "request.diff" => {
            let f = prefix.trim_start_matches("request.");
            format!("request:\n  url: https://example.com\n  {f}: {obj}\n")
        }
        p => format!("global:\n  {p}: {obj}\nrequest:\n  url: https://example.com\n"),
    }
}

#[test]
fn every_model_key_is_accepted_and_bogus_key_warns_once() {
    use super::global::*;
    let cases: [(&str, &[&str]); 8] = [
        ("retry", super::request::VALID_RETRY_KEYS),
        ("profile", VALID_PROFILE_KEYS),
        ("env", VALID_ENV_KEYS),
        ("security", VALID_SECURITY_KEYS),
        ("snapshot", VALID_GLOBAL_SNAPSHOT_KEYS),
        ("diff", VALID_GLOBAL_DIFF_KEYS),
        ("request.snapshot", VALID_REQUEST_SNAPSHOT_KEYS),
        ("request.diff", VALID_REQUEST_DIFF_KEYS),
    ];
    for (section, keys) in cases {
        // `--strict` passes only when there are no errors or warnings.
        let ok = lint(&section_yaml(section, keys, None));
        assert_eq!(unknown_warnings(&ok), 0, "{section}: {ok:?}");
        assert!(
            ok.iter().all(|i| i.severity == Severity::Info),
            "{section}: {ok:?}"
        );
        let bad = lint(&section_yaml(section, keys, Some("bogusKey")));
        assert_eq!(unknown_warnings(&bad), 1, "{section}: {bad:?}");
    }
}
