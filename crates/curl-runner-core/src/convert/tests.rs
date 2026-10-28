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
        "body:\n    name: Alex",
    ] {
        assert!(r.output.contains(want), "{want} in {}", r.output);
    }
    assert!(!r.output.contains("json:"), "{}", r.output);
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

const JSON_CT: &str = "-H 'Content-Type: application/json'";

/// The body bytes the runner would send for the YAML that `convert` produced.
fn sent_body(cmd: &str, options: &ConvertOptions) -> Option<String> {
    let yaml = convert_curl_to_yaml(cmd, options).unwrap().output;
    let parsed = crate::parser::load_str(&yaml, "convert.yaml").unwrap();
    crate::transport::easy::body_payload(&parsed.requests[0])
}

fn as_json(s: &str) -> serde_json::Value {
    serde_json::from_str(s).unwrap_or_else(|e| panic!("{s:?}: {e}"))
}

#[test]
fn json_data_becomes_the_body_without_a_json_key() {
    let r = conv(&format!(
        "curl -X POST https://api.example.com/u {JSON_CT} -d '{{\"name\":\"Ada\"}}'"
    ));
    assert!(
        r.output.contains("  body:\n    name: Ada\n"),
        "{}",
        r.output
    );
    assert!(!r.output.contains("json:"), "{}", r.output);
}

#[test]
fn json_arrays_and_nested_objects_convert_directly() {
    let r = conv(&format!(
        "curl -X POST https://a.test {JSON_CT} -d '[{{\"id\":1}},{{\"id\":2}}]'"
    ));
    assert!(
        r.output.contains("  body: [{\"id\":1},{\"id\":2}]\n"),
        "{}",
        r.output
    );
    let r = conv(&format!(
        "curl -X POST https://a.test {JSON_CT} -d '{{\"u\":{{\"n\":\"A\"}},\"ok\":true}}'"
    ));
    assert!(r.output.contains("    u: {\"n\":\"A\"}\n"), "{}", r.output);
    assert!(!r.output.contains("json:"), "{}", r.output);
}

#[test]
fn json_bodies_round_trip_to_the_same_bytes() {
    let bodies = [
        r#"{"name":"Ada"}"#,
        r#"{"a":{"b":[1,2,{"c":null}]},"s":"x: y","t":true,"n":1.5,"e":{}}"#,
        r#"[1,"two",null]"#,
        r#"[{"id":1,"tags":["x"]},{"id":2}]"#,
        r#"{}"#,
        r#"[]"#,
        r#"{"json":1}"#,
        r#"{"raw":"x","form":"y"}"#,
        r##"{"on":"yes","1":"a","k y":"#tag","empty":""}"##,
    ];
    for pretty in [None, Some(false)] {
        let opts = ConvertOptions {
            pretty,
            ..Default::default()
        };
        for b in bodies {
            let sent = sent_body(
                &format!("curl -X POST https://a.test {JSON_CT} -d '{b}'"),
                &opts,
            )
            .unwrap_or_else(|| panic!("no body for {b}"));
            assert_eq!(as_json(&sent), as_json(b), "pretty={pretty:?} body {b}");
        }
    }
}

#[test]
fn non_json_data_stays_a_string_body() {
    for d in ["hello world", "a=1&b=2", "{not json", "key: value"] {
        let r = conv(&format!("curl -X POST https://a.test {JSON_CT} -d '{d}'"));
        assert!(!r.output.contains("json:"), "{}", r.output);
        let sent = sent_body(
            &format!("curl -X POST https://a.test {JSON_CT} -d '{d}'"),
            &ConvertOptions::default(),
        );
        assert_eq!(sent.as_deref(), Some(d), "{d}");
    }
}

#[test]
fn json_scalars_keep_their_text() {
    for d in ["0", "null", "false", "true", "42", "\"s\""] {
        let sent = sent_body(
            &format!("curl -X POST https://a.test {JSON_CT} -d '{d}'"),
            &ConvertOptions::default(),
        );
        assert_eq!(sent.as_deref(), Some(d), "{d}");
    }
}

#[test]
fn yaml_body_converts_back_to_curl_d_with_the_json() {
    let yaml = "request:\n  method: POST\n  url: https://a.test\n  body:\n    name: Ada\n    tags: [\"a\", \"b\"]\n";
    let out = convert_yaml_to_curl(yaml).unwrap().output;
    assert!(
        out.contains(r#"-d '{"name":"Ada","tags":["a","b"]}'"#),
        "{out}"
    );
    let rt = convert_curl_to_yaml(&out, &ConvertOptions::default())
        .unwrap()
        .output;
    assert!(rt.contains("  body:\n    name: Ada\n"), "{rt}");
    let arr = "request:\n  method: POST\n  url: https://a.test\n  body: [1, 2]\n";
    assert!(
        convert_yaml_to_curl(arr)
            .unwrap()
            .output
            .contains("-d '[1,2]'")
    );
    let s = "request:\n  method: POST\n  url: https://a.test\n  body: plain text\n";
    assert!(
        convert_yaml_to_curl(s)
            .unwrap()
            .output
            .contains("-d 'plain text'")
    );
}
