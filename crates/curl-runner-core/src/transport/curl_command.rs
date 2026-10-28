//! Equivalent `curl` command line for dry-run / verbose output
//! (`core/curl/args-builder.ts` + `utils/curl-builder.ts`).

use super::easy::{
    JSON_CONTENT_TYPE, basic_auth, bearer_line, body_payload, follows_redirects, form_value_string,
    insecure, method, needs_json_content_type, non_empty, request_url, user_header_lines,
};
use crate::config::DEFAULT_ALLOWED_PROTOCOLS;
use crate::model::{FormFieldValue, GlobalConfig, RequestConfig};

/// TS `DEFAULT_WRITE_OUT` (kept so the printed command matches TS output).
const WRITE_OUT: &str = "\n__CURL_METRICS_START__%{json}__CURL_METRICS_END__";

/// Milliseconds as curl `--max-time` seconds (5000 -> `5`, 1500 -> `1.5`).
fn ms_to_secs(ms: u64) -> String {
    if ms.is_multiple_of(1000) {
        (ms / 1000).to_string()
    } else {
        (ms as f64 / 1000.0).to_string()
    }
}

/// TS `buildCurlArgs` (single-request flags) -> `(args, url)`.
pub fn build_args(req: &RequestConfig, global: &GlobalConfig) -> (Vec<String>, String) {
    let mut a: Vec<String> = Vec::new();
    let mut push = |xs: &[&str]| a.extend(xs.iter().map(|x| x.to_string()));

    push(&["-X", method(req), "-w", WRITE_OUT, "-D", "-"]);

    for line in user_header_lines(req) {
        push(&["-H", &line]);
    }
    if let Some((u, p)) = basic_auth(req) {
        push(&["-u", &format!("{u}:{p}")]);
    }
    if let Some(line) = bearer_line(req) {
        push(&["-H", &line]);
    }

    if let Some(fields) = &req.form_data {
        for (name, value) in fields {
            if let FormFieldValue::File(f) = value {
                let mut spec = format!("{name}=@{}", f.file);
                if let Some(n) = non_empty(&f.filename) {
                    spec.push_str(&format!(";filename={n}"));
                }
                if let Some(ct) = non_empty(&f.content_type) {
                    spec.push_str(&format!(";type={ct}"));
                }
                push(&["-F", &spec]);
            } else {
                push(&[
                    "--form-string",
                    &format!("{name}={}", form_value_string(value)),
                ]);
            }
        }
    } else if let Some(body) = body_payload(req) {
        push(&["--data-raw", &body]);
        if needs_json_content_type(req) {
            push(&["-H", JSON_CONTENT_TYPE]);
        }
    }

    if let Some(ms) = req.timeout.filter(|t| *t > 0) {
        push(&["--max-time", &ms_to_secs(ms)]);
    }
    if follows_redirects(req) {
        push(&["-L"]);
        if let Some(m) = req.max_redirects.filter(|m| *m > 0) {
            push(&["--max-redirs", &m.to_string()]);
        }
    }
    if let Some(p) = non_empty(&req.proxy) {
        push(&["-x", p]);
    }
    if insecure(req) {
        push(&["-k"]);
    }
    if let Some(ssl) = &req.ssl {
        for (flag, v) in [
            ("--cacert", &ssl.ca),
            ("--cert", &ssl.cert),
            ("--key", &ssl.key),
        ] {
            if let Some(v) = non_empty(v) {
                push(&[flag, v]);
            }
        }
    }
    if let Some(o) = non_empty(&req.output) {
        push(&["-o", o]);
    }
    if req.http2 == Some(true) {
        push(&["--http2"]);
    }

    let protos = global
        .security
        .as_ref()
        .and_then(|s| s.allowed_protocols.as_ref())
        .filter(|p| !p.is_empty())
        .map_or_else(|| DEFAULT_ALLOWED_PROTOCOLS.join(","), |p| p.join(","));
    let protos = format!("-all,{protos}");
    push(&["--proto", &protos, "--proto-redir", &protos, "-s", "-S"]);

    (a, request_url(req))
}

/// TS `CurlBuilder.buildCommand`: args plus `-- <url>`.
pub fn command_args(req: &RequestConfig, global: &GlobalConfig) -> Vec<String> {
    let (mut args, url) = build_args(req, global);
    args.push("--".to_string());
    args.push(url);
    args
}

/// TS `formatArgsForDisplay`: single-quote args containing space or quotes.
pub fn format_args_for_display(args: &[String]) -> String {
    std::iter::once("curl")
        .chain(args.iter().map(String::as_str))
        .map(|arg| {
            if arg.contains([' ', '"', '\'']) {
                format!("'{}'", arg.replace('\'', r"'\''"))
            } else {
                arg.to_string()
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

/// Shell-safe `curl` command equivalent to the transfer (dry-run / verbose).
pub fn to_curl_command(req: &RequestConfig, global: &GlobalConfig) -> String {
    format_args_for_display(&command_args(req, global))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{AuthConfig, FileAttachment, FormFieldValue, SecurityConfig, SslConfig};
    use indexmap::IndexMap;
    use serde_json::json;

    fn req(url: &str) -> RequestConfig {
        RequestConfig {
            url: url.to_string(),
            ..Default::default()
        }
    }

    fn args(r: &RequestConfig) -> Vec<String> {
        build_args(r, &GlobalConfig::default()).0
    }

    fn has(a: &[String], s: &str) -> bool {
        a.iter().any(|x| x == s)
    }

    fn after<'a>(a: &'a [String], flag: &str) -> &'a str {
        let i = a.iter().position(|x| x == flag).expect(flag);
        &a[i + 1]
    }

    fn map(pairs: &[(&str, &str)]) -> IndexMap<String, String> {
        pairs
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect()
    }

    fn form(pairs: Vec<(&str, FormFieldValue)>) -> Option<IndexMap<String, FormFieldValue>> {
        Some(pairs.into_iter().map(|(k, v)| (k.to_string(), v)).collect())
    }

    fn file(path: &str, name: Option<&str>, ct: Option<&str>) -> FormFieldValue {
        FormFieldValue::File(FileAttachment {
            file: path.into(),
            filename: name.map(Into::into),
            content_type: ct.map(Into::into),
        })
    }

    fn post(url: &str) -> RequestConfig {
        RequestConfig {
            method: Some("POST".into()),
            ..req(url)
        }
    }

    // ---- args-builder.test.ts ----

    #[test]
    fn builds_basic_get_request() {
        let (a, url) = build_args(
            &req("https://api.example.com/users"),
            &GlobalConfig::default(),
        );
        assert_eq!(after(&a, "-X"), "GET");
        assert_eq!(url, "https://api.example.com/users");
    }

    #[test]
    fn builds_post_request() {
        assert_eq!(
            after(&args(&post("https://api.example.com/users")), "-X"),
            "POST"
        );
    }

    #[test]
    fn includes_write_out_marker_for_metrics() {
        let a = args(&req("https://api.example.com"));
        let w = after(&a, "-w");
        assert!(w.contains("__CURL_METRICS_START__") && w.contains("__CURL_METRICS_END__"));
    }

    #[test]
    fn dumps_response_headers_to_stdout() {
        assert_eq!(after(&args(&req("https://api.example.com")), "-D"), "-");
    }

    #[test]
    fn adds_headers() {
        let r = RequestConfig {
            headers: Some(map(&[
                ("Content-Type", "application/json"),
                ("Accept", "application/json"),
            ])),
            ..req("https://api.example.com")
        };
        let a = args(&r);
        assert!(has(&a, "-H"));
        assert!(has(&a, "Content-Type: application/json"));
        assert!(has(&a, "Accept: application/json"));
    }

    #[test]
    fn adds_basic_auth() {
        let r = RequestConfig {
            auth: Some(AuthConfig {
                auth_type: "basic".into(),
                username: Some("user".into()),
                password: Some("pass".into()),
                token: None,
            }),
            ..req("https://api.example.com")
        };
        assert_eq!(after(&args(&r), "-u"), "user:pass");
    }

    #[test]
    fn adds_bearer_token_auth() {
        let r = RequestConfig {
            auth: Some(AuthConfig {
                auth_type: "bearer".into(),
                token: Some("abc123".into()),
                ..Default::default()
            }),
            ..req("https://api.example.com")
        };
        assert!(has(&args(&r), "Authorization: Bearer abc123"));
    }

    #[test]
    fn adds_json_body() {
        let r = RequestConfig {
            body: Some(json!({"name": "test"})),
            ..post("https://api.example.com")
        };
        let a = args(&r);
        assert_eq!(after(&a, "--data-raw"), r#"{"name":"test"}"#);
        assert!(!has(&a, "-d"));
        assert!(has(&a, "Content-Type: application/json"));
    }

    #[test]
    fn adds_string_body() {
        let r = RequestConfig {
            body: Some(json!("raw data")),
            ..post("https://api.example.com")
        };
        let a = args(&r);
        assert_eq!(after(&a, "--data-raw"), "raw data");
        assert!(!has(&a, "-d"));
    }

    #[test]
    fn passes_body_starting_with_at_verbatim_via_data_raw() {
        let r = RequestConfig {
            body: Some(json!("@/etc/passwd")),
            ..post("https://api.example.com")
        };
        let a = args(&r);
        assert_eq!(after(&a, "--data-raw"), "@/etc/passwd");
        assert!(!has(&a, "-d"));
    }

    #[test]
    fn does_not_add_content_type_if_already_present() {
        let r = RequestConfig {
            headers: Some(map(&[("Content-Type", "text/plain")])),
            body: Some(json!("test")),
            ..post("https://api.example.com")
        };
        let a = args(&r);
        assert_eq!(a.iter().filter(|x| x.contains("Content-Type")).count(), 1);
        assert!(has(&a, "Content-Type: text/plain"));
    }

    #[test]
    fn falsy_bodies_send_nothing() {
        for b in [json!(null), json!(false), json!(0), json!("")] {
            let r = RequestConfig {
                body: Some(b.clone()),
                ..post("https://api.example.com")
            };
            assert!(!has(&args(&r), "--data-raw"), "{b}");
        }
        let r = RequestConfig {
            body: Some(json!(true)),
            ..post("https://api.example.com")
        };
        assert_eq!(after(&args(&r), "--data-raw"), "true");
    }

    #[test]
    fn adds_form_data_fields() {
        let r = RequestConfig {
            form_data: form(vec![
                ("name", FormFieldValue::Text("test".into())),
                ("value", FormFieldValue::Number(123.into())),
            ]),
            ..post("https://api.example.com/upload")
        };
        let a = args(&r);
        assert!(has(&a, "--form-string"));
        assert!(has(&a, "name=test"));
        assert!(has(&a, "value=123"));
    }

    #[test]
    fn adds_file_attachment() {
        let r = RequestConfig {
            form_data: form(vec![("file", file("/path/to/file.txt", None, None))]),
            ..post("https://api.example.com/upload")
        };
        assert_eq!(after(&args(&r), "-F"), "file=@/path/to/file.txt");
    }

    #[test]
    fn adds_file_attachment_with_filename_and_content_type() {
        let r = RequestConfig {
            form_data: form(vec![(
                "file",
                file("/path/to/file.txt", Some("custom.txt"), Some("text/plain")),
            )]),
            ..post("https://api.example.com/upload")
        };
        assert_eq!(
            after(&args(&r), "-F"),
            "file=@/path/to/file.txt;filename=custom.txt;type=text/plain"
        );
    }

    #[test]
    fn adds_timeout_in_fractional_seconds() {
        for (ms, s) in [(30_000, "30"), (5000, "5"), (1500, "1.5"), (250, "0.25")] {
            let r = RequestConfig {
                timeout: Some(ms),
                ..req("https://api.example.com")
            };
            assert_eq!(after(&args(&r), "--max-time"), s);
        }
        let r = RequestConfig {
            timeout: Some(0),
            ..req("https://api.example.com")
        };
        assert!(!has(&args(&r), "--max-time"));
    }

    #[test]
    fn adds_follow_redirects_by_default() {
        assert!(has(&args(&req("https://api.example.com")), "-L"));
    }

    #[test]
    fn does_not_add_follow_redirects_when_disabled() {
        let r = RequestConfig {
            follow_redirects: Some(false),
            max_redirects: Some(5),
            ..req("https://api.example.com")
        };
        let a = args(&r);
        assert!(!has(&a, "-L"));
        assert!(!has(&a, "--max-redirs"));
    }

    #[test]
    fn adds_max_redirects() {
        let r = RequestConfig {
            max_redirects: Some(5),
            ..req("https://api.example.com")
        };
        assert_eq!(after(&args(&r), "--max-redirs"), "5");
    }

    #[test]
    fn adds_proxy() {
        let r = RequestConfig {
            proxy: Some("http://proxy:8080".into()),
            ..req("https://api.example.com")
        };
        assert_eq!(after(&args(&r), "-x"), "http://proxy:8080");
    }

    #[test]
    fn adds_insecure_flag() {
        let r = RequestConfig {
            insecure: Some(true),
            ..req("https://api.example.com")
        };
        assert!(has(&args(&r), "-k"));
    }

    #[test]
    fn adds_ssl_certificate_options() {
        let r = RequestConfig {
            ssl: Some(SslConfig {
                ca: Some("/path/to/ca.pem".into()),
                cert: Some("/path/to/cert.pem".into()),
                key: Some("/path/to/key.pem".into()),
                verify: None,
            }),
            ..req("https://api.example.com")
        };
        let a = args(&r);
        assert_eq!(after(&a, "--cacert"), "/path/to/ca.pem");
        assert_eq!(after(&a, "--cert"), "/path/to/cert.pem");
        assert_eq!(after(&a, "--key"), "/path/to/key.pem");
        assert!(!has(&a, "-k"));
    }

    #[test]
    fn adds_insecure_from_ssl_verify_false() {
        let r = RequestConfig {
            ssl: Some(SslConfig {
                verify: Some(false),
                ..Default::default()
            }),
            ..req("https://api.example.com")
        };
        assert!(has(&args(&r), "-k"));
    }

    #[test]
    fn adds_output_file() {
        let r = RequestConfig {
            output: Some("/path/to/output".into()),
            ..req("https://api.example.com")
        };
        assert_eq!(after(&args(&r), "-o"), "/path/to/output");
    }

    #[test]
    fn adds_http2_flag_when_enabled() {
        let r = RequestConfig {
            http2: Some(true),
            ..req("https://api.example.com")
        };
        assert!(has(&args(&r), "--http2"));
    }

    #[test]
    fn adds_silent_flags() {
        let a = args(&req("https://api.example.com"));
        assert!(has(&a, "-s") && has(&a, "-S"));
    }

    #[test]
    fn emits_proto_flags_once_with_default_allowed_protocols() {
        let a = args(&req("https://api.example.com"));
        assert_eq!(after(&a, "--proto"), "-all,http,https");
        assert_eq!(after(&a, "--proto-redir"), "-all,http,https");
        assert_eq!(a.iter().filter(|x| *x == "--proto").count(), 1);
        assert_eq!(a.iter().filter(|x| *x == "--proto-redir").count(), 1);
    }

    #[test]
    fn emits_proto_redir_with_custom_allowed_protocols() {
        let g = GlobalConfig {
            security: Some(SecurityConfig {
                allowed_protocols: Some(vec!["http".into(), "https".into(), "ftp".into()]),
                allow_paths: None,
            }),
            ..Default::default()
        };
        let (a, _) = build_args(&req("ftp://files.example.com"), &g);
        assert_eq!(after(&a, "--proto-redir"), "-all,http,https,ftp");
    }

    #[test]
    fn appends_query_params_to_url() {
        let r = RequestConfig {
            params: Some(map(&[("q", "test"), ("limit", "10")])),
            ..req("https://api.example.com/search")
        };
        let (_, url) = build_args(&r, &GlobalConfig::default());
        assert_eq!(url, "https://api.example.com/search?q=test&limit=10");
    }

    #[test]
    fn appends_query_params_to_url_with_existing_params() {
        let r = RequestConfig {
            params: Some(map(&[("q", "test")])),
            ..req("https://api.example.com/search?existing=true")
        };
        let (_, url) = build_args(&r, &GlobalConfig::default());
        assert_eq!(url, "https://api.example.com/search?existing=true&q=test");
    }

    #[test]
    fn query_params_use_urlsearchparams_encoding() {
        let r = RequestConfig {
            params: Some(map(&[("q", "a b&c=d/é*~")])),
            ..req("https://x.test/")
        };
        let (_, url) = build_args(&r, &GlobalConfig::default());
        assert_eq!(url, "https://x.test/?q=a+b%26c%3Dd%2F%C3%A9*%7E");
    }

    // ---- formatArgsForDisplay ----

    fn s(v: &[&str]) -> Vec<String> {
        v.iter().map(|x| x.to_string()).collect()
    }

    #[test]
    fn formats_simple_args() {
        assert_eq!(
            format_args_for_display(&s(&["-X", "GET", "https://api.example.com"])),
            "curl -X GET https://api.example.com"
        );
    }

    #[test]
    fn quotes_args_with_spaces() {
        assert_eq!(
            format_args_for_display(&s(&["-H", "Content-Type: application/json"])),
            "curl -H 'Content-Type: application/json'"
        );
    }

    #[test]
    fn escapes_single_quotes_in_args() {
        assert_eq!(
            format_args_for_display(&s(&["-d", "it's data"])),
            r"curl -d 'it'\''s data'"
        );
        assert_eq!(
            format_args_for_display(&s(&["--form-string", "message=It's a test"])),
            r"curl --form-string 'message=It'\''s a test'"
        );
    }

    #[test]
    fn quotes_args_with_double_quotes() {
        assert_eq!(
            format_args_for_display(&s(&["-d", r#"{"key":"value"}"#])),
            r#"curl -d '{"key":"value"}'"#
        );
    }

    // ---- curl-builder.test.ts (buildCommand) ----

    fn cmd_args(r: &RequestConfig) -> Vec<String> {
        command_args(r, &GlobalConfig::default())
    }

    #[test]
    fn build_command_basic_get() {
        let r = RequestConfig {
            method: Some("GET".into()),
            ..req("https://example.com/api")
        };
        let a = cmd_args(&r);
        assert_eq!(after(&a, "-X"), "GET");
        assert!(has(&a, "https://example.com/api"));
    }

    #[test]
    fn build_command_post_with_json_body() {
        let r = RequestConfig {
            body: Some(json!({"name": "test"})),
            ..post("https://example.com/api")
        };
        let a = cmd_args(&r);
        assert_eq!(after(&a, "-X"), "POST");
        assert_eq!(after(&a, "--data-raw"), r#"{"name":"test"}"#);
        assert!(!has(&a, "-d"));
        assert!(has(&a, "Content-Type: application/json"));
    }

    #[test]
    fn build_command_form_data() {
        let r = RequestConfig {
            form_data: form(vec![
                ("username", FormFieldValue::Text("john".into())),
                ("age", FormFieldValue::Number(30.into())),
            ]),
            ..post("https://example.com/upload")
        };
        let a = cmd_args(&r);
        assert!(has(&a, "username=john") && has(&a, "age=30"));
        assert!(!has(&a, "-d"));
    }

    #[test]
    fn build_command_file_attachment_variants() {
        let cases = [
            (file("./test.pdf", None, None), "document=@./test.pdf"),
            (
                file("./test.pdf", Some("report.pdf"), None),
                "document=@./test.pdf;filename=report.pdf",
            ),
            (
                file("./data.json", None, Some("application/json")),
                "document=@./data.json;type=application/json",
            ),
            (
                file(
                    "./report.pdf",
                    Some("quarterly-report.pdf"),
                    Some("application/pdf"),
                ),
                "document=@./report.pdf;filename=quarterly-report.pdf;type=application/pdf",
            ),
        ];
        for (f, want) in cases {
            let r = RequestConfig {
                form_data: form(vec![("document", f)]),
                ..post("https://example.com/upload")
            };
            assert_eq!(after(&cmd_args(&r), "-F"), want);
        }
    }

    #[test]
    fn build_command_mixed_form_data_and_files() {
        let r = RequestConfig {
            form_data: form(vec![
                ("title", FormFieldValue::Text("My Document".into())),
                ("description", FormFieldValue::Text("Test upload".into())),
                ("file", file("./document.pdf", None, None)),
            ]),
            ..post("https://example.com/upload")
        };
        let a = cmd_args(&r);
        assert!(has(&a, "title=My Document") && has(&a, "description=Test upload"));
        assert_eq!(after(&a, "-F"), "file=@./document.pdf");
    }

    #[test]
    fn build_command_single_quotes_in_form_values_pass_through() {
        let r = RequestConfig {
            form_data: form(vec![(
                "message",
                FormFieldValue::Text("It's a test".into()),
            )]),
            ..post("https://example.com/upload")
        };
        assert!(has(&cmd_args(&r), "message=It's a test"));
    }

    #[test]
    fn build_command_prefers_form_data_over_body() {
        let r = RequestConfig {
            form_data: form(vec![("field", FormFieldValue::Text("value".into()))]),
            body: Some(json!({"name": "test"})),
            ..post("https://example.com/api")
        };
        let a = cmd_args(&r);
        assert!(has(&a, "field=value"));
        assert!(!has(&a, "-d") && !has(&a, "--data-raw"));
    }

    #[test]
    fn build_command_boolean_form_values() {
        let r = RequestConfig {
            form_data: form(vec![
                ("active", FormFieldValue::Bool(true)),
                ("disabled", FormFieldValue::Bool(false)),
            ]),
            ..post("https://example.com/api")
        };
        let a = cmd_args(&r);
        assert!(has(&a, "active=true") && has(&a, "disabled=false"));
    }

    #[test]
    fn build_command_passes_unresolved_var_literally() {
        assert!(has(
            &cmd_args(&req("https://example.com/api/${VAR}")),
            "https://example.com/api/${VAR}"
        ));
    }

    #[test]
    fn build_command_nested_json_body() {
        let r = RequestConfig {
            body: Some(json!({"nested": {"key": "value"}})),
            ..post("https://example.com/api")
        };
        assert_eq!(
            after(&cmd_args(&r), "--data-raw"),
            r#"{"nested":{"key":"value"}}"#
        );
    }

    #[test]
    fn build_command_http2_flag() {
        for (v, want) in [(Some(true), true), (Some(false), false), (None, false)] {
            let r = RequestConfig {
                http2: v,
                ..req("https://example.com/api")
            };
            assert_eq!(has(&cmd_args(&r), "--http2"), want);
        }
    }

    #[test]
    fn build_command_places_double_dash_before_url() {
        for url in ["https://example.com/api", "-K/tmp/evil"] {
            let a = cmd_args(&req(url));
            assert_eq!(a.last().unwrap(), url);
            assert_eq!(a[a.len() - 2], "--");
        }
    }

    #[test]
    fn to_curl_command_renders_display_string() {
        let r = RequestConfig {
            timeout: Some(1500),
            headers: Some(map(&[("X-A", "b c")])),
            ..req("https://example.com/api")
        };
        let out = to_curl_command(&r, &GlobalConfig::default());
        assert!(out.starts_with("curl -X GET -w "), "{out}");
        assert!(out.contains("-H 'X-A: b c'"), "{out}");
        assert!(out.contains("--max-time 1.5"), "{out}");
        assert!(out.ends_with("-s -S -- https://example.com/api"), "{out}");
    }
}
