use super::*;

fn conv(cmd: &str) -> ConvertResult {
    convert_curl_to_yaml(cmd, &ConvertOptions::default()).unwrap()
}

#[test]
fn full_post_conversion() {
    let r = conv(
        "curl -X POST https://api.example.com/users -H 'Authorization: Bearer TOKEN' -d '{\"name\":\"Alex\"}'",
    );
    for want in [
        "method: POST",
        "url: https://api.example.com/users",
        "auth:",
        "token: TOKEN",
        "json:",
        "name: Alex",
    ] {
        assert!(r.output.contains(want), "{want} in {}", r.output);
    }
}

#[test]
fn get_with_query_params() {
    let r = conv("curl \"https://api.example.com/search?q=hello&page=1\"");
    assert!(r.output.contains("url: https://api.example.com/search\n"));
    assert!(r.output.contains("params:"));
    assert!(r.output.contains("q: hello"));
}

#[test]
fn debug_mode_includes_tokens_ast_and_ir() {
    let r = convert_curl_to_yaml(
        "curl https://example.com",
        &ConvertOptions {
            debug: true,
            ..Default::default()
        },
    )
    .unwrap();
    let d = r.debug.unwrap();
    assert!(d.get("tokens").is_some() && d.get("ast").is_some() && d.get("ir").is_some());
}

#[test]
fn complex_real_world_curl() {
    let r = conv(
        "curl -X POST https://api.stripe.com/v1/charges         -u sk_test_key:         -d amount=2000         -d currency=usd         -d source=tok_visa",
    );
    assert!(r.output.contains("method: POST"));
    assert!(r.output.contains("auth:"));
    assert!(r.output.contains("type: basic"));
}

fn args(a: &[&str]) -> Vec<String> {
    a.iter().map(|s| s.to_string()).collect()
}

#[test]
fn parse_convert_args_subcommands() {
    let r = parse_convert_args(&args(&["convert", "curl", "curl https://example.com"])).unwrap();
    assert_eq!(
        (r.subcommand, r.input.as_str()),
        (Subcommand::Curl, "curl https://example.com")
    );
    let r = parse_convert_args(&args(&["convert", "file", "script.sh"])).unwrap();
    assert_eq!(
        (r.subcommand, r.input.as_str()),
        (Subcommand::File, "script.sh")
    );
    let r = parse_convert_args(&args(&["convert", "yaml", "test.yaml"])).unwrap();
    assert_eq!(
        (r.subcommand, r.input.as_str()),
        (Subcommand::Yaml, "test.yaml")
    );
}

#[test]
fn parse_convert_args_options() {
    let r = parse_convert_args(&args(&[
        "convert", "curl", "input", "--output", "out.yaml", "--debug", "--pretty",
    ]))
    .unwrap();
    assert_eq!(r.options.output.as_deref(), Some("out.yaml"));
    assert!(r.options.debug);
    assert_eq!(r.options.pretty, Some(true));
}

#[test]
fn parse_convert_args_rejects_bad_input() {
    assert!(parse_convert_args(&args(&["convert"])).is_none());
    assert!(parse_convert_args(&args(&["convert", "curl"])).is_none());
    assert!(parse_convert_args(&args(&["convert", "unknown", "input"])).is_none());
}

#[test]
fn script_conversion_single_and_batch() {
    let one = convert_script_to_yaml("curl https://a.com\n", &ConvertOptions::default()).unwrap();
    assert!(one.output.starts_with("request:\n  name: request_1\n"));
    let two = convert_script_to_yaml(
        "curl https://a.com\ncurl https://b.com\n",
        &ConvertOptions::default(),
    )
    .unwrap();
    assert!(two.output.starts_with("requests:\n  - name: request_1\n"));
    assert!(two.output.contains("\n\n  - name: request_2\n"));
    let none = convert_script_to_yaml("echo hi\n", &ConvertOptions::default()).unwrap();
    assert_eq!((none.output.as_str(), none.warnings.len()), ("", 1));
}

#[test]
fn yaml_to_curl_collects_all_request_shapes() {
    let y = "request:\n  url: https://a.com\nrequests:\n  - url: https://b.com\n    method: POST\n    body: {a: 1}\ncollection:\n  requests:\n    - url: https://c.com\n";
    let r = convert_yaml_to_curl(y).unwrap();
    let parts: Vec<&str> = r.output.split("\n\n").collect();
    assert_eq!(parts.len(), 3);
    assert_eq!(parts[0], "curl \\\n  https://a.com");
    assert!(parts[1].starts_with(
        "curl \\\n  -X POST \\\n  -d '{\"a\":1}' \\\n  -H 'Content-Type: application/json'"
    ));
}

#[test]
fn yaml_to_curl_errors_match_ts() {
    let e = |y: &str| convert_yaml_to_curl(y).unwrap_err().0;
    assert_eq!(e(""), "null is not an object (evaluating 'yaml.request')");
    assert_eq!(
        e("request:\n  method: GET\n"),
        "undefined is not an object (evaluating 's.includes')"
    );
    assert_eq!(
        e("requests: 5\n"),
        "Spread syntax requires ...iterable[Symbol.iterator] to be a function"
    );
    assert_eq!(
        e("requests:\n  - null\n"),
        "null is not an object (evaluating 'config.headers')"
    );
    assert!(e("a: [1\n  b: }\n").starts_with("YAML Parse error:"));
    let none = convert_yaml_to_curl("other: 1\n").unwrap();
    assert_eq!(none.warnings, ["No requests found in YAML file"]);
}

#[test]
fn malformed_percent_escape_fails_like_ts() {
    let e = convert_curl_to_yaml("curl https://a.com/?a=%E0%A4%A", &ConvertOptions::default())
        .unwrap_err();
    assert_eq!(e.0, "URI error");
}
