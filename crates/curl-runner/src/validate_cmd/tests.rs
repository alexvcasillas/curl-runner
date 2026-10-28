//! Port of `commands/validate.test.ts` (96 cases), then exact-output,
//! crash-emulation and `--fix` serialization tests.

use super::*;
use crate::output::Console;

struct Out {
    lines: Vec<String>,
    stdout: String,
    stderr: String,
    code: i32,
    file: String,
    content: String,
}

fn validate_in(yaml: &str, flags: &[&str], color: bool) -> Out {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("test.yaml");
    std::fs::write(&path, yaml).unwrap();
    let file = path.to_string_lossy().into_owned();
    let mut argv = vec!["validate".to_string(), file.clone()];
    argv.extend(flags.iter().map(|s| s.to_string()));
    let (mut c, cap) = Console::capture();
    let code = run_in(&argv, dir.path(), color, &mut c);
    let stdout = cap.stdout();
    Out {
        lines: stdout.lines().map(String::from).collect(),
        stdout,
        stderr: cap.stderr(),
        code,
        content: std::fs::read_to_string(&path).unwrap(),
        file,
    }
}

fn validate(yaml: &str, flags: &[&str]) -> Out {
    validate_in(yaml, flags, true)
}

fn has(o: &Out, f: impl Fn(&str) -> bool) -> bool {
    o.lines.iter().any(|l| f(l))
}

/// One TS case: validate `yaml` and expect a line matching `pred`.
macro_rules! case {
    ($name:ident, $yaml:expr, $pred:expr) => {
        #[test]
        fn $name() {
            let o = validate($yaml, &[]);
            assert!(has(&o, $pred), "{}", o.stdout);
        }
    };
}

fn contains(s: &'static str) -> impl Fn(&str) -> bool {
    move |l: &str| l.contains(s)
}

fn both(a: &'static str, b: &'static str) -> impl Fn(&str) -> bool {
    move |l: &str| l.contains(a) && l.contains(b)
}

const VALID: &str = "All files valid";

// ---------------------------------------------------------------- valid files

case!(
    validates_simple_get,
    "\nrequest:\n  url: https://api.example.com/users\n  method: GET\n",
    contains(VALID)
);
case!(
    validates_post_with_body_and_content_type,
    r#"
request:
  url: https://api.example.com/users
  method: POST
  headers:
    Content-Type: application/json
  body:
    name: "John Doe"
"#,
    contains(VALID)
);
case!(
    validates_basic_auth_with_env,
    r#"
request:
  url: https://api.example.com/users
  auth:
    type: basic
    username: ${env.USER}
    password: ${env.PASS}
"#,
    contains(VALID)
);
case!(
    validates_bearer_auth_with_env,
    r#"
request:
  url: https://api.example.com/users
  auth:
    type: bearer
    token: ${env.TOKEN}
"#,
    contains(VALID)
);
case!(
    validates_retry_config,
    r#"
request:
  url: https://api.example.com/users
  method: GET
  retry:
    count: 3
    delay: 1000
    backoff: 2
"#,
    contains(VALID)
);
case!(
    validates_expect_status_array,
    r#"
request:
  url: https://api.example.com/users
  expect:
    status: [200, 201, 204]
    headers:
      Content-Type: application/json
"#,
    contains(VALID)
);
case!(
    validates_response_time_with_variable,
    r#"
request:
  url: https://api.example.com/users
  expect:
    responseTime: "< ${TIMEOUT}"
"#,
    contains(VALID)
);
case!(
    validates_collection_with_defaults,
    r#"
collection:
  name: User API
  defaults:
    headers:
      Authorization: Bearer token
  requests:
    - url: https://api.example.com/users
    - url: https://api.example.com/posts
"#,
    contains(VALID)
);
case!(
    validates_global_all_options,
    r#"
global:
  execution: parallel
  maxConcurrency: 5
  continueOnError: true
  http2: true
  output:
    format: pretty
    prettyLevel: detailed
    verbose: true

requests:
  - url: https://api.example.com/users
"#,
    contains(VALID)
);

#[test]
fn validates_all_http_methods() {
    for m in ["GET", "POST", "PUT", "DELETE", "PATCH", "HEAD", "OPTIONS"] {
        let o = validate(
            &format!("\nrequest:\n  url: https://api.example.com\n  method: {m}\n"),
            &[],
        );
        assert!(has(&o, contains(VALID)), "{m}: {}", o.stdout);
    }
}

case!(
    validates_store_config,
    r#"
request:
  url: https://api.example.com/users
  store:
    userId: body.id
    statusCode: status
    contentType: headers.content-type
"#,
    contains(VALID)
);
case!(
    validates_when_with_store,
    r#"
requests:
  - url: https://api.example.com/login
    store:
      token: body.token
  - url: https://api.example.com/profile
    when:
      left: store.token
      operator: exists
"#,
    contains(VALID)
);
case!(
    validates_ssl_config,
    r#"
request:
  url: https://api.example.com
  ssl:
    verify: true
    ca: ./certs/ca.pem
    cert: ./certs/client.pem
    key: ./certs/client-key.pem
"#,
    contains(VALID)
);

// ----------------------------------------------------------------------- URLs

case!(
    reports_missing_url,
    "\nrequest:\n  method: GET\n",
    contains("URL is required")
);
case!(
    reports_missing_protocol,
    "\nrequest:\n  url: api.example.com/users\n",
    contains("http://")
);
case!(
    detects_htpp,
    "\nrequest:\n  url: htpp://api.example.com\n",
    both("typo", "htpp")
);
case!(
    detects_htps,
    "\nrequest:\n  url: htps://api.example.com\n",
    both("typo", "htps")
);
case!(
    detects_localhost_typo,
    "\nrequest:\n  url: http://localhsot:3000\n",
    both("typo", "localhsot")
);
case!(
    detects_double_slashes,
    "\nrequest:\n  url: https://api.example.com//users\n",
    contains("double slashes")
);
case!(
    detects_spaces,
    "\nrequest:\n  url: \"https://api.example.com/users/john doe\"\n",
    contains("spaces")
);
case!(
    detects_invalid_port,
    "\nrequest:\n  url: https://api.example.com:99999\n",
    contains("Invalid port")
);
case!(
    shows_http_info,
    "\nrequest:\n  url: http://api.example.com/users\n",
    both("HTTP", "HTTPS")
);
case!(
    skips_url_validation_for_variables,
    "\nrequest:\n  url: ${BASE_URL}/users\n",
    contains(VALID)
);

// -------------------------------------------------------------------- methods

case!(
    reports_invalid_method,
    "\nrequest:\n  url: https://api.example.com\n  method: INVALID\n",
    contains("Invalid method")
);
case!(
    reports_lowercase_method,
    "\nrequest:\n  url: https://api.example.com\n  method: get\n",
    contains("uppercase")
);
case!(
    suggests_closest_method,
    "\nrequest:\n  url: https://api.example.com\n  method: DELET\n",
    contains("DELETE")
);

// -------------------------------------------------------------------- headers

case!(
    suggests_header_casing,
    r#"
request:
  url: https://api.example.com
  method: POST
  headers:
    content-type: application/json
  body: {}
"#,
    contains("Content-Type")
);
case!(
    detects_duplicate_headers,
    r#"
request:
  url: https://api.example.com
  headers:
    Content-Type: application/json
    content-type: text/plain
"#,
    contains("Duplicate")
);
case!(
    warns_body_without_content_type,
    r#"
request:
  url: https://api.example.com
  method: POST
  body:
    test: value
"#,
    contains("Content-Type")
);
case!(
    warns_auth_block_and_header,
    r#"
request:
  url: https://api.example.com
  headers:
    Authorization: Bearer token123
  auth:
    type: bearer
    token: different-token
"#,
    both("auth block", "Authorization")
);

// ----------------------------------------------------------------------- body

case!(
    warns_body_with_get,
    r#"
request:
  url: https://api.example.com
  method: GET
  body:
    test: value
"#,
    both("GET", "ignored")
);
case!(
    warns_invalid_json_body,
    r#"
request:
  url: https://api.example.com
  method: POST
  headers:
    Content-Type: application/json
  body: "{invalid json"
"#,
    both("JSON", "invalid")
);
case!(
    reports_body_and_form_data,
    r#"
request:
  url: https://api.example.com
  method: POST
  body:
    test: value
  formData:
    field: value
"#,
    contains("Cannot use both")
);

// ----------------------------------------------------------------------- auth

case!(
    reports_invalid_auth_type,
    "\nrequest:\n  url: https://api.example.com\n  auth:\n    type: oauth\n",
    contains("Invalid auth type")
);
case!(
    reports_missing_username,
    "\nrequest:\n  url: https://api.example.com\n  auth:\n    type: basic\n    password: secret\n",
    contains("Username required")
);
case!(
    reports_missing_token,
    "\nrequest:\n  url: https://api.example.com\n  auth:\n    type: bearer\n",
    contains("Token required")
);
case!(
    warns_hardcoded_password,
    r#"
request:
  url: https://api.example.com
  auth:
    type: basic
    username: admin
    password: supersecret123
"#,
    contains("Hardcoded password")
);
case!(
    warns_hardcoded_token,
    r#"
request:
  url: https://api.example.com
  auth:
    type: bearer
    token: eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.test
"#,
    contains("Hardcoded token")
);
case!(
    suggests_closest_auth_type,
    "\nrequest:\n  url: https://api.example.com\n  auth:\n    type: basci\n",
    contains("basic")
);

// ------------------------------------------------------------------- security

case!(
    warns_insecure,
    "\nrequest:\n  url: https://api.example.com\n  insecure: true\n",
    both("Insecure", "SSL")
);
case!(
    warns_sensitive_variables,
    r#"
global:
  variables:
    api_key: sk_live_abcdef123456
requests:
  - url: https://api.example.com
"#,
    contains("sensitive")
);
case!(
    warns_sensitive_headers,
    "\nrequest:\n  url: https://api.example.com\n  headers:\n    X-API-Key: sk_live_supersecret\n",
    contains("sensitive")
);

// --------------------------------------------------------------- unknown keys

case!(
    detects_unknown_top_level_key,
    "\nrequest:\n  url: https://api.example.com\nunknown_key: value\n",
    both("Unknown", "unknown_key")
);
case!(
    detects_unknown_request_key,
    "\nrequest:\n  url: https://api.example.com\n  metod: GET\n",
    both("metod", "method")
);
case!(
    detects_unknown_global_key,
    "\nglobal:\n  excecution: parallel\nrequests:\n  - url: https://api.example.com\n",
    contains("excecution")
);

// ------------------------------------------------------------ timeout / retry

case!(
    reports_negative_timeout,
    "\nrequest:\n  url: https://api.example.com\n  timeout: -1000\n",
    contains("positive")
);
case!(
    warns_short_timeout,
    "\nrequest:\n  url: https://api.example.com\n  timeout: 50\n",
    contains("short timeout")
);
case!(
    warns_long_timeout,
    "\nrequest:\n  url: https://api.example.com\n  timeout: 600000\n",
    contains("long timeout")
);
case!(
    reports_missing_retry_count,
    "\nrequest:\n  url: https://api.example.com\n  retry:\n    delay: 1000\n",
    both("count", "required")
);
case!(
    warns_high_retry_count,
    "\nrequest:\n  url: https://api.example.com\n  retry:\n    count: 15\n",
    contains("High retry count")
);
case!(
    warns_high_backoff,
    "\nrequest:\n  url: https://api.example.com\n  retry:\n    count: 3\n    backoff: 10\n",
    both("backoff", "delay")
);
case!(
    warns_post_retry,
    "\nrequest:\n  url: https://api.example.com\n  method: POST\n  retry:\n    count: 3\n",
    both("POST", "duplicate")
);
case!(
    reports_string_timeout,
    "\nrequest:\n  url: https://api.example.com\n  timeout: \"5000\"\n",
    both("number", "string")
);

// --------------------------------------------------------------------- expect

case!(
    reports_invalid_status,
    "\nrequest:\n  url: https://api.example.com\n  expect:\n    status: 1000\n",
    contains("100-599")
);
case!(
    reports_status_below_100,
    "\nrequest:\n  url: https://api.example.com\n  expect:\n    status: 50\n",
    contains("100-599")
);
case!(
    reports_string_status,
    "\nrequest:\n  url: https://api.example.com\n  expect:\n    status: \"200\"\n",
    both("number", "string")
);
case!(
    warns_empty_status_array,
    "\nrequest:\n  url: https://api.example.com\n  expect:\n    status: []\n",
    contains("Empty")
);
case!(
    warns_invalid_response_time,
    "\nrequest:\n  url: https://api.example.com\n  expect:\n    responseTime: \"fast\"\n",
    contains("responseTime")
);

// ----------------------------------------------------------------------- when

case!(
    reports_missing_left,
    "\nrequest:\n  url: https://api.example.com\n  when:\n    operator: \"==\"\n    right: 200\n",
    contains("Left operand")
);
case!(
    reports_missing_operator,
    "\nrequest:\n  url: https://api.example.com\n  when:\n    left: store.status\n    right: 200\n",
    both("Operator", "required")
);
case!(
    reports_invalid_operator,
    r#"
request:
  url: https://api.example.com
  when:
    left: store.status
    operator: "==="
    right: 200
"#,
    contains("Invalid operator")
);
case!(
    reports_missing_right,
    "\nrequest:\n  url: https://api.example.com\n  when:\n    left: store.status\n    operator: \"==\"\n",
    contains("Right operand")
);
case!(
    warns_when_string_without_store,
    "\nrequest:\n  url: https://api.example.com\n  when: \"someVar == 200\"\n",
    contains("store.")
);
case!(
    warns_empty_all,
    "\nrequest:\n  url: https://api.example.com\n  when:\n    all: []\n",
    both("Empty", "all")
);

// ---------------------------------------------------------------------- store

case!(
    warns_invalid_store_path,
    "\nrequest:\n  url: https://api.example.com\n  store:\n    value: invalid.path\n",
    |l: &str| l.contains("body") || l.contains("status")
);
case!(
    warns_invalid_store_name,
    "\nrequest:\n  url: https://api.example.com\n  store:\n    \"invalid-name\": body.id\n",
    contains("alphanumeric")
);

// --------------------------------------------------------------------- global

case!(
    reports_invalid_execution,
    "\nglobal:\n  execution: invalid\nrequests:\n  - url: https://api.example.com\n",
    contains("Invalid execution mode")
);
case!(
    warns_max_concurrency_sequential,
    "\nglobal:\n  execution: sequential\n  maxConcurrency: 5\nrequests:\n  - url: https://api.example.com\n",
    both("maxConcurrency", "sequential")
);
case!(
    reports_missing_request,
    "\nglobal:\n  execution: parallel\n",
    contains("Missing request")
);
case!(
    reports_invalid_output_format,
    "\nglobal:\n  output:\n    format: xml\nrequests:\n  - url: https://api.example.com\n",
    contains("Invalid format")
);
case!(
    reports_boolean_string,
    "\nglobal:\n  continueOnError: \"true\"\nrequests:\n  - url: https://api.example.com\n",
    both("boolean", "string")
);

// ------------------------------------------------------------------------ ssl

case!(
    warns_cert_without_key,
    "\nrequest:\n  url: https://api.example.com\n  ssl:\n    cert: ./certs/client.pem\n",
    both("cert", "key")
);
case!(
    warns_key_without_cert,
    "\nrequest:\n  url: https://api.example.com\n  ssl:\n    key: ./certs/client-key.pem\n",
    both("key", "cert")
);

// -------------------------------------------------------------- request names

case!(
    warns_duplicate_names,
    r#"
requests:
  - name: get-users
    url: https://api.example.com/users
  - name: get-users
    url: https://api.example.com/users/2
"#,
    both("Duplicate", "get-users")
);

// ------------------------------------------------------------------------ fix

#[test]
fn fixes_lowercase_method() {
    let o = validate(
        "\nrequest:\n  url: https://api.example.com\n  method: get\n",
        &["--fix"],
    );
    assert!(o.content.contains("GET"), "{}", o.content);
}

#[test]
fn fixes_missing_protocol() {
    let o = validate(
        "\nrequest:\n  url: api.example.com\n  method: GET\n",
        &["--fix"],
    );
    assert!(
        o.content.contains("https://api.example.com"),
        "{}",
        o.content
    );
}

#[test]
fn fixes_htpp() {
    let o = validate(
        "\nrequest:\n  url: htpp://api.example.com\n  method: GET\n",
        &["--fix"],
    );
    assert!(
        o.content.contains("http://api.example.com"),
        "{}",
        o.content
    );
}

#[test]
fn fixes_spaces() {
    let o = validate(
        "\nrequest:\n  url: \"https://api.example.com/users/john doe\"\n  method: GET\n",
        &["--fix"],
    );
    assert!(o.content.contains("%20"), "{}", o.content);
}

#[test]
fn reports_fixed_count() {
    let o = validate(
        "\nrequest:\n  url: api.example.com\n  method: get\n",
        &["--fix"],
    );
    assert!(has(&o, contains("Fixed")), "{}", o.stdout);
}

// -------------------------------------------------------------------- options

const PLAIN_GET: &str = "\nrequest:\n  url: https://api.example.com\n  method: GET\n";
const LOWER_GET: &str = "\nrequest:\n  url: https://api.example.com\n  method: get\n";

#[test]
fn quiet_hides_valid_files() {
    let o = validate(PLAIN_GET, &["--quiet"]);
    assert!(!has(&o, |l: &str| l.contains(&o.file)), "{}", o.stdout);
}

#[test]
fn q_short_flag() {
    let o = validate(PLAIN_GET, &["-q"]);
    assert!(!has(&o, |l: &str| l.contains(&o.file)), "{}", o.stdout);
}

#[test]
fn f_short_flag() {
    let o = validate(LOWER_GET, &["-f"]);
    assert!(o.content.contains("GET"), "{}", o.content);
}

#[test]
fn strict_treats_warnings_as_errors() {
    let o = validate(LOWER_GET, &["--strict"]);
    assert!(has(&o, contains("file(s) with")), "{}", o.stdout);
}

#[test]
fn s_short_flag() {
    let o = validate(LOWER_GET, &["-s"]);
    assert!(has(&o, contains("file(s) with")), "{}", o.stdout);
}

// ------------------------------------------------------------------ discovery

#[test]
fn no_files_found() {
    let dir = tempfile::tempdir().unwrap();
    let (mut c, cap) = Console::capture();
    let argv: Vec<String> = ["validate", "/tmp/nonexistent-dir-12345/*.yaml"]
        .map(String::from)
        .to_vec();
    let code = run_in(&argv, dir.path(), true, &mut c);
    assert!(cap.stdout().contains("No YAML files found"));
    assert_eq!(code, 0);
}

// ---------------------------------------------------------------------- proxy

case!(
    reports_invalid_proxy,
    "\nrequest:\n  url: https://api.example.com\n  proxy: invalid-proxy\n",
    contains("Proxy")
);
case!(
    accepts_http_proxy,
    "\nrequest:\n  url: https://api.example.com\n  proxy: http://proxy.example.com:8080\n",
    contains(VALID)
);
case!(
    accepts_socks5_proxy,
    "\nrequest:\n  url: https://api.example.com\n  proxy: socks5://proxy.example.com:1080\n",
    contains(VALID)
);

// ----------------------------------------------------------------- collection

case!(
    warns_missing_collection_name,
    "\ncollection:\n  requests:\n    - url: https://api.example.com\n",
    both("name", "recommended")
);
case!(
    reports_missing_collection_requests,
    "\ncollection:\n  name: Test Collection\n",
    contains("requests")
);
case!(
    warns_empty_collection_requests,
    "\ncollection:\n  name: Test Collection\n  requests: []\n",
    contains("no requests")
);

// ------------------------------------------------------------------------- ci

case!(
    reports_fail_on_percentage_range,
    "\nglobal:\n  ci:\n    failOnPercentage: 150\nrequests:\n  - url: https://api.example.com\n",
    contains("0 and 100")
);
case!(
    reports_negative_fail_on,
    "\nglobal:\n  ci:\n    failOn: -1\nrequests:\n  - url: https://api.example.com\n",
    contains("non-negative")
);

// -------------------------------------------------------------------- profile

case!(
    reports_invalid_iterations,
    "\nglobal:\n  profile:\n    iterations: 0\nrequests:\n  - url: https://api.example.com\n",
    contains("positive")
);
case!(
    warns_high_iterations,
    "\nglobal:\n  profile:\n    iterations: 50000\nrequests:\n  - url: https://api.example.com\n",
    both("high", "iteration")
);

// -------------------------------------------------------------- connection pool

case!(
    warns_high_max_streams,
    r#"
global:
  connectionPool:
    enabled: true
    maxStreamsPerHost: 200
requests:
  - url: https://api.example.com
"#,
    both("stream", "issues")
);

// ===================================================== beyond the TS suite

/// Full plain-text output (`NO_COLOR`), with the temp path replaced by `F`.
fn plain(yaml: &str, flags: &[&str]) -> (String, i32, String) {
    let o = validate_in(yaml, flags, false);
    (o.stdout.replace(&o.file, "F"), o.code, o.stderr)
}

#[test]
fn exact_output_invalid_file() {
    let (out, code, _) = plain(LOWER_GET.replace("get", "INVALID").as_str(), &[]);
    assert_eq!(
        out,
        "\ncurl-runner validate\n\nFiles: 1 found\n\n✗ F\n  ● request.method: Invalid method \"INVALID\". Must be: GET, POST, PUT, DELETE, PATCH, HEAD, OPTIONS\n  → 1 error(s)\n\n\nSummary: 1 file(s) with 1 issue(s)\n"
    );
    assert_eq!(code, 1);
}

#[test]
fn exact_output_warnings_and_fix_hint() {
    let (out, code, _) = plain(LOWER_GET, &[]);
    assert_eq!(
        out,
        "\ncurl-runner validate\n\nFiles: 1 found\n\n✓ F\n  ● request.method: Method should be uppercase: \"GET\"\n    fix: Change to \"GET\"\n  → 1 warning(s)\n\n\nAll files valid!\n"
    );
    assert_eq!(code, 0);
}

#[test]
fn exact_output_strict_and_fix() {
    let (out, code, _) = plain(LOWER_GET, &["--strict", "--fix"]);
    assert_eq!(
        out,
        "\ncurl-runner validate\n\nFiles: 1 found\n\n✗ F\n  ● request.method: Method should be uppercase: \"GET\"\n  ✓ Fixed 1 issue(s)\n  → 1 warning(s)\n\n\nSummary: 1 file(s) with 1 issue(s)\nFixed: 1 issue(s)\n"
    );
    assert_eq!(code, 1);
}

#[test]
fn exact_output_valid_and_quiet() {
    let (out, code, _) = plain(PLAIN_GET, &[]);
    assert_eq!(
        out,
        "\ncurl-runner validate\n\nFiles: 1 found\n\n✓ F\n\nAll files valid!\n"
    );
    assert_eq!(code, 0);
    let (out, _, _) = plain(PLAIN_GET, &["-q"]);
    assert_eq!(
        out,
        "\ncurl-runner validate\n\nFiles: 1 found\n\n\nAll files valid!\n"
    );
}

#[test]
fn colored_icons() {
    let o = validate(LOWER_GET, &[]);
    assert!(o.stdout.contains("\x1b[1mcurl-runner validate\x1b[0m"));
    assert!(o.stdout.contains("\x1b[32m✓\x1b[0m "));
    assert!(o.stdout.contains("  \x1b[33m●\x1b[0m request.method"));
    assert!(o.stdout.contains("    \x1b[36mfix:\x1b[0m Change"));
    assert!(o.stdout.contains("  \x1b[2m→\x1b[0m 1 warning(s)"));
}

#[test]
fn info_only_file_and_counts() {
    let (out, _, _) = plain("request:\n  url: http://api.example.com\n", &[]);
    assert!(out.contains(
        "✓ F\n  ● request.url: Using HTTP instead of HTTPS; consider using HTTPS for security\n  → 1 info\n"
    ));
}

#[test]
fn parse_error_issue() {
    let (out, code, _) = plain("request: [a", &[]);
    assert!(out.contains("  ● (root): Failed to parse YAML: YAML Parse error: Unexpected"));
    assert_eq!(code, 1);
}

#[test]
fn multi_document_is_an_array() {
    let (out, _, _) = plain(
        "request:\n  url: https://a.com\n---\nrequest:\n  url: https://b.com\n",
        &[],
    );
    assert!(out.contains("  ● 0: Unknown top-level key \"0\"\n    fix: Remove unknown key \"0\"\n  ● 1: Unknown top-level key \"1\""), "{out}");
}

// TS throws a TypeError for these; the CLI prints it and exits 1.
#[test]
fn crashes_like_ts() {
    let cases = [
        ("", "null is not an object (evaluating 'content.global')"),
        (
            "hello",
            "content is not an Object. (evaluating '\"request\" in content')",
        ),
        (
            "requests:\n  a: 1",
            "Spread syntax requires ...iterable[Symbol.iterator] to be a function",
        ),
        (
            "requests:\n  - null",
            "null is not an object (evaluating 'Object.keys(obj)')",
        ),
        (
            "global:\n  execution: 5\nrequest:\n  url: https://a.com",
            "input.toLowerCase is not a function. (In 'input.toLowerCase()', 'input.toLowerCase' is undefined)",
        ),
        (
            "request:\n  url: https://a.com\n  method: 5",
            "method.toUpperCase is not a function. (In 'method.toUpperCase()', 'method.toUpperCase' is undefined)",
        ),
        (
            "request:\n  url: https://a.com\n  when: 5",
            "when is not an Object. (evaluating '\"all\" in when')",
        ),
        (
            "request:\n  url: https://a.com\n  when:\n    all: [null]",
            "null is not an object (evaluating 'cond.left')",
        ),
        (
            "request:\n  url: https://a.com\n  when:\n    left: 5\n    operator: \"==\"\n    right: 1",
            "cond.left.startsWith is not a function. (In 'cond.left.startsWith(\"store.\")', 'cond.left.startsWith' is undefined)",
        ),
        (
            "request:\n  url: https://a.com\n  snapshot: null",
            "null is not an object (evaluating 'Object.keys(obj)')",
        ),
    ];
    for (yaml, msg) in cases {
        let (out, code, err) = plain(yaml, &[]);
        assert_eq!(err, format!("✗ {msg}\n"), "{yaml}");
        assert_eq!(
            out, "\ncurl-runner validate\n\nFiles: 1 found\n\n",
            "{yaml}"
        );
        assert_eq!(code, 1);
    }
}

#[test]
fn help_flag() {
    let (mut c, cap) = Console::capture();
    let argv: Vec<String> = ["validate", "x.yaml", "-h"].map(String::from).to_vec();
    let code = run_in(&argv, Path::new("."), false, &mut c);
    assert_eq!(code, 0);
    assert!(
        cap.stdout()
            .starts_with("\ncurl-runner validate\n\nValidate YAML files")
    );
    assert!(
        cap.stdout()
            .ends_with("  • Unknown keys (with suggestions)\n\n")
    );
}

#[test]
fn closest_match_and_edit_distance() {
    use super::issues::find_closest_match;
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

#[test]
fn fix_golden_serialization() {
    // TS's `--fix` output (`bun packages/cli/src/cli.ts validate --fix`;
    // TS sources removed in 2.0, see 7670118), with
    // the logged emitter fixes (`key: {}` inline, `"*/*"` quoted).
    let yaml = include_str!("../../tests/golden/validate/fix-input.yaml");
    let want = include_str!("../../tests/golden/validate/fix-output.yaml");
    let o = validate(yaml, &["--fix"]);
    assert_eq!(o.content, want);
}

#[test]
fn parse_error_messages() {
    let (out, _, _) = plain("a: \"unterminated\n", &[]);
    assert!(
        out.contains("Failed to parse YAML: YAML Parse error: Unexpected EOF\n"),
        "{out}"
    );
    let (out, _, _) = plain("a: b: c\n", &[]);
    assert!(
        out.contains("Failed to parse YAML: YAML Parse error: Unexpected token\n"),
        "{out}"
    );
    // Duplicate keys: the last value wins, as in Bun.
    let (out, code, _) = plain("request:\n  url: https://a.com\n  url: b.com\n", &[]);
    assert!(
        out.contains("request.url: URL should start with http:// or https://"),
        "{out}"
    );
    assert_eq!(code, 0);
}

/// Every `--fix` output must read back (with the runtime parser) as exactly
/// the fixed data. Corpus: the repo's examples, two mutated copies of each
/// (lowercase methods, header casing, URL typos and spaces, string numbers
/// and booleans, misspelled keys, empty `{}`/`[]`, strings that need quotes),
/// the edge-case goldens and the fix golden.
#[test]
fn fix_output_reads_back_as_the_fixed_data() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let mut inputs: Vec<(String, String)> = Vec::new();
    let mut add_dir = |dir: &Path| {
        let mut files: Vec<_> = std::fs::read_dir(dir)
            .unwrap()
            .map(|e| e.unwrap().path())
            .filter(|p| p.extension().is_some_and(|e| e == "yaml"))
            .collect();
        files.sort();
        for f in files {
            inputs.push((
                f.display().to_string(),
                std::fs::read_to_string(&f).unwrap(),
            ));
        }
    };
    add_dir(&root.join("../../packages/cli/examples"));
    add_dir(&root.join("tests/golden/validate/edge"));
    let examples = inputs.len();
    for i in 0..examples {
        let (name, text) = inputs[i].clone();
        let m1 = text
            .replacen("method: GET", "method: get", 1)
            .replacen("method: POST", "method: post", 1)
            .replacen("Content-Type", "content-type", 1)
            .replacen("https://", "htpp://", 1)
            .replacen("execution: parallel", "execution: paralel", 1)
            .replacen("  url:", "  ulr:", 1)
            .replacen("headers:\n", "headers:\n      accept: \"*/*\"\n", 1);
        let m2 = text
            .replacen("method: GET", "method: Get", 1)
            .replacen("continueOnError: true", "continueOnError: \"true\"", 1)
            .replacen("  name:", "  nmae:", 1)
            .replacen("body:\n", "body:\n    e1: {}\n    e2: []\n    s1: \"yes\"\n    s2: \"-1\"\n    s3: \"~\"\n    s4: \"@x\"\n    s5: \"on\"\n    s6: \"010\"\n    s7: \"&a\"\n    \"yes\": 1\n    \"010\": 2\n    l: [{}, [], \"*\", \"| x\"]\n", 1);
        inputs.push((format!("{name} (m1)"), m1));
        inputs.push((format!("{name} (m2)"), m2));
    }
    inputs.push((
        "fix-input.yaml".into(),
        include_str!("../../tests/golden/validate/fix-input.yaml").into(),
    ));

    let mut fixed_files = 0;
    for (name, text) in &inputs {
        let Ok(content) = parser::parse_raw_stream(text) else {
            continue;
        };
        if !content.is_object() {
            continue;
        }
        let mut cx = Ctx::default();
        if PASSES
            .iter()
            .any(|(_, pass)| pass(&content, &mut cx).is_err())
        {
            continue;
        }
        let Some((out, data)) = fix::fix_document(&content, &cx.issues) else {
            continue;
        };
        fixed_files += 1;
        let back = parser::parse_raw_stream(&out)
            .unwrap_or_else(|e| panic!("{name}: --fix output does not parse: {e}\n{out}"));
        assert_eq!(
            back, data,
            "{name}: --fix output reads back differently:\n{out}"
        );
    }
    assert!(fixed_files > 100, "only {fixed_files} files had fixes");
}

/// TS rewrote a multi-document file from its array of documents, erasing it
/// (`"\n"`). `--fix` now leaves any file whose root is not one mapping alone.
#[test]
fn fix_skips_files_that_are_not_one_mapping() {
    for yaml in [
        "request:\n  url: https://a.com\n  method: get\n---\nrequest:\n  url: https://b.com\n",
        "- a\n- b\n",
    ] {
        let (out, code, _) = plain(yaml, &["--fix"]);
        let o = validate_in(yaml, &["--fix"], false);
        assert_eq!(o.content, yaml, "file must be left unchanged");
        assert!(
            out.contains(
                "  ⚠ Not fixed: --fix only rewrites a file with a single mapping document\n"
            ),
            "{out}"
        );
        assert!(!out.contains("✓ Fixed"), "{out}");
        assert!(out.contains("Fixed: 0 issue(s)"), "{out}");
        assert_eq!(code, 1);
    }
}

// ---- #136: validator key lists track the config model -----------------

/// serde (camelCase) field names of each model struct, kept by hand so a
/// model change without a validator change fails here.
const MODEL_RETRY: &[&str] = &["count", "delay", "backoff", "retryableStatuses"];
const MODEL_PROFILE: &[&str] = &[
    "iterations",
    "warmup",
    "concurrency",
    "histogram",
    "exportFile",
];
const MODEL_ENV: &[&str] = &["environment", "redactSecrets"];
const MODEL_SECURITY: &[&str] = &["allowedProtocols", "allowPaths"];
// SnapshotConfig + GlobalSnapshotConfig (`match` is renamed from match_rules).
const MODEL_SNAPSHOT: &[&str] = &[
    "enabled",
    "name",
    "include",
    "exclude",
    "match",
    "dir",
    "updateMode",
    "ci",
];
// DiffConfig + GlobalDiffConfig.
const MODEL_DIFF: &[&str] = &[
    "enabled",
    "exclude",
    "match",
    "includeTimings",
    "dir",
    "label",
    "compareWith",
    "save",
    "outputFormat",
];

fn sorted(a: &[&str]) -> Vec<String> {
    let mut v: Vec<String> = a.iter().map(|s| s.to_string()).collect();
    v.sort();
    v
}

#[test]
fn validator_key_lists_match_model() {
    use super::global::*;
    assert_eq!(
        sorted(super::request::VALID_RETRY_KEYS),
        sorted(MODEL_RETRY)
    );
    assert_eq!(sorted(VALID_PROFILE_KEYS), sorted(MODEL_PROFILE));
    assert_eq!(sorted(VALID_ENV_KEYS), sorted(MODEL_ENV));
    assert_eq!(sorted(VALID_SECURITY_KEYS), sorted(MODEL_SECURITY));
    assert_eq!(sorted(VALID_SNAPSHOT_KEYS), sorted(MODEL_SNAPSHOT));
    assert_eq!(sorted(VALID_DIFF_KEYS), sorted(MODEL_DIFF));
}

fn unknown_warnings(o: &Out) -> usize {
    o.lines.iter().filter(|l| l.contains("Unknown key")).count()
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
        "retry" => format!("request:\n  url: https://example.com\n  retry: {obj}\n"),
        p => format!("global:\n  {p}: {obj}\nrequest:\n  url: https://example.com\n"),
    }
}

#[test]
fn every_model_key_is_accepted_and_bogus_key_warns_once() {
    let cases: [(&str, &[&str]); 6] = [
        ("retry", MODEL_RETRY),
        ("profile", MODEL_PROFILE),
        ("env", MODEL_ENV),
        ("security", MODEL_SECURITY),
        ("snapshot", MODEL_SNAPSHOT),
        ("diff", MODEL_DIFF),
    ];
    for (section, keys) in cases {
        let ok = validate(&section_yaml(section, keys, None), &["--strict"]);
        assert_eq!(unknown_warnings(&ok), 0, "{section}: {}", ok.stdout);
        assert_eq!(ok.code, 0, "{section}: {}", ok.stdout);
        let bad = validate(&section_yaml(section, keys, Some("bogusKey")), &[]);
        assert_eq!(unknown_warnings(&bad), 1, "{section}: {}", bad.stdout);
    }
}

#[test]
fn issue_136_repro_is_clean_under_strict() {
    let yaml = "global:
  env: { redactSecrets: true }
  security: { allowedProtocols: [http, https] }
  profile: { iterations: 5, histogram: true, exportFile: out.json }
  snapshot: { enabled: true, include: [body], exclude: [headers] }
  diff: { enabled: true }
request:
  name: r
  url: https://example.com
  retry: { count: 2, retryableStatuses: [503] }
";
    let o = validate(yaml, &["--strict"]);
    assert_eq!(o.code, 0, "{}", o.stdout);
    assert_eq!(unknown_warnings(&o), 0, "{}", o.stdout);
}
