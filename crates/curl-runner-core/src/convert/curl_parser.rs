//! Curl semantic parser: shell tokens to a [`CurlAst`] (`convert/curl-parser.ts`).
//!
//! The TS quirks are kept on purpose: a combined boolean token such as `-sSL`
//! advances past as many tokens as it has characters, and a combined value
//! flag such as `-XPOST` also swallows the next token.

use super::tokenizer::tokenize;
use super::types::{CurlAst, UnsupportedFlag};
use crate::jsval::{str_to_number, trim};

const VALUE_FLAGS: &[&str] = &[
    "-X",
    "--request",
    "-H",
    "--header",
    "-d",
    "--data",
    "--data-raw",
    "--data-binary",
    "--data-ascii",
    "--data-urlencode",
    "-F",
    "--form",
    "--form-string",
    "-u",
    "--user",
    "-o",
    "--output",
    "-x",
    "--proxy",
    "--max-time",
    "-m",
    "--max-redirs",
    "--cacert",
    "--cert",
    "--key",
    "-b",
    "--cookie",
    "-c",
    "--cookie-jar",
    "-A",
    "--user-agent",
    "-e",
    "--referer",
    "--connect-timeout",
    "--retry",
    "--retry-delay",
    "-w",
    "--write-out",
    "--resolve",
    "--interface",
    "--local-port",
    "--limit-rate",
    "--ciphers",
    "--proto",
    "--retry-max-time",
    "--tlsv1.2",
    "--tlsv1.3",
];

const BOOLEAN_FLAGS: &[&str] = &[
    "-G",
    "--get",
    "-I",
    "--head",
    "-L",
    "--location",
    "-k",
    "--insecure",
    "-s",
    "--silent",
    "-S",
    "--show-error",
    "-v",
    "--verbose",
    "--compressed",
    "--http2",
    "--http2-prior-knowledge",
    "--http1.1",
    "-N",
    "--no-buffer",
    "--raw",
    "-f",
    "--fail",
    "--fail-early",
    "--globoff",
    "-#",
    "--progress-bar",
    "--tr-encoding",
    "--tcp-nodelay",
    "--tcp-fastopen",
    "--retry-all-errors",
    "--retry-connrefused",
];

/// Parse a raw curl command string (`parseCurl`).
pub fn parse_curl(input: &str) -> CurlAst {
    parse_tokens(&tokenize(trim(input)))
}

impl CurlAst {
    fn touch(&mut self, key: &'static str) {
        if !self.order.contains(&key) {
            self.order.push(key);
        }
    }
}

/// Parse pre-tokenized input (`parseTokens`).
pub fn parse_tokens(tokens: &[String]) -> CurlAst {
    let mut ast = CurlAst::default();
    let mut i = usize::from(tokens.first().is_some_and(|t| t == "curl"));

    while i < tokens.len() {
        let token = tokens[i].as_str();

        if !token.starts_with('-') {
            if ast.url.is_empty() {
                ast.url = token.to_string();
            }
            i += 1;
            continue;
        }

        if !token.starts_with("--") && token.encode_utf16().count() > 2 {
            let mut chars = token.chars();
            chars.next();
            let second = chars.next().unwrap_or('-');
            let first_flag = format!("-{second}");
            if VALUE_FLAGS.contains(&first_flag.as_str()) {
                let value: String = chars.collect();
                i = process_flag(&first_flag, Some(&value), i, &mut ast);
                continue;
            }

            let flags: Vec<String> = token.chars().skip(1).map(|c| format!("-{c}")).collect();
            if flags.iter().all(|f| BOOLEAN_FLAGS.contains(&f.as_str())) {
                for f in &flags {
                    i = process_flag(f, None, i, &mut ast);
                }
                i += 1;
                continue;
            }
        }

        if VALUE_FLAGS.contains(&token) {
            let value = tokens.get(i + 1).map(String::as_str);
            i = process_flag(token, value, i, &mut ast);
            continue;
        }

        if BOOLEAN_FLAGS.contains(&token) {
            i = process_flag(token, None, i, &mut ast);
            continue;
        }

        match tokens.get(i + 1) {
            Some(next) if !next.is_empty() && !next.starts_with('-') => {
                ast.unsupported_flags.push(UnsupportedFlag {
                    flag: token.to_string(),
                    value: Some(next.clone()),
                });
                i += 2;
            }
            _ => {
                ast.unsupported_flags.push(UnsupportedFlag {
                    flag: token.to_string(),
                    value: None,
                });
                i += 1;
            }
        }
    }

    // The TS parser assigns these last, in this order, when non-empty.
    for (key, empty) in [
        ("data", ast.data.is_empty()),
        ("dataRaw", ast.data_raw.is_empty()),
        ("dataBinary", ast.data_binary.is_empty()),
        ("dataUrlencode", ast.data_urlencode.is_empty()),
        ("form", ast.form.is_empty()),
        ("formString", ast.form_string.is_empty()),
    ] {
        if !empty {
            ast.touch(key);
        }
    }
    ast
}

fn process_flag(flag: &str, value: Option<&str>, i: usize, ast: &mut CurlAst) -> usize {
    let own = || value.map(str::to_string);
    match flag {
        "-X" | "--request" => {
            ast.method = value.map(str::to_uppercase);
            ast.touch("method");
            i + 2
        }
        "-H" | "--header" => {
            if let Some(v) = value.filter(|v| !v.is_empty())
                && let Some(c) = v.find(':').filter(|c| *c > 0)
            {
                ast.headers
                    .push((trim(&v[..c]).to_string(), trim(&v[c + 1..]).to_string()));
            }
            i + 2
        }
        "-d" | "--data" | "--data-ascii" => {
            ast.data.extend(own());
            i + 2
        }
        "--data-raw" => {
            ast.data_raw.extend(own());
            i + 2
        }
        "--data-binary" => {
            ast.data_binary.extend(own());
            i + 2
        }
        "--data-urlencode" => {
            ast.data_urlencode.extend(own());
            i + 2
        }
        "-F" | "--form" => {
            ast.form.extend(own());
            i + 2
        }
        "--form-string" => {
            ast.form_string.extend(own());
            i + 2
        }
        "-u" | "--user" => {
            ast.user = own();
            ast.touch("user");
            i + 2
        }
        "-o" | "--output" => {
            ast.output = own();
            ast.touch("output");
            i + 2
        }
        "-x" | "--proxy" => {
            ast.proxy = own();
            ast.touch("proxy");
            i + 2
        }
        "--max-time" | "-m" => {
            if let Some(v) = value.filter(|v| !v.is_empty()) {
                ast.max_time = Some(str_to_number(v));
                ast.touch("maxTime");
            }
            i + 2
        }
        "--max-redirs" => {
            if let Some(v) = value.filter(|v| !v.is_empty()) {
                ast.max_redirs = Some(str_to_number(v));
                ast.touch("maxRedirs");
            }
            i + 2
        }
        "--cacert" => {
            ast.cacert = own();
            ast.touch("cacert");
            i + 2
        }
        "--cert" => {
            ast.cert = own();
            ast.touch("cert");
            i + 2
        }
        "--key" => {
            ast.key = own();
            ast.touch("key");
            i + 2
        }
        "-b" | "--cookie" => {
            ast.cookie = own();
            ast.touch("cookie");
            i + 2
        }
        "-c" | "--cookie-jar" => {
            ast.cookie_jar = own();
            ast.touch("cookieJar");
            i + 2
        }
        "-A" | "--user-agent" => {
            ast.user_agent = own();
            ast.touch("userAgent");
            i + 2
        }
        "-e" | "--referer" => {
            ast.referer = own();
            ast.touch("referer");
            i + 2
        }
        "--retry" => {
            ast.retry = own();
            i + 2
        }
        "--retry-delay" => {
            ast.retry_delay = own();
            i + 2
        }
        "--retry-max-time" => {
            ast.unsupported_flags.push(UnsupportedFlag {
                flag: flag.to_string(),
                value: own(),
            });
            i + 2
        }
        "--retry-all-errors" | "--retry-connrefused" => {
            ast.unsupported_flags.push(UnsupportedFlag {
                flag: flag.to_string(),
                value: None,
            });
            i + 1
        }
        "-G" | "--get" => {
            ast.get = true;
            ast.touch("get");
            i + 1
        }
        "-I" | "--head" => {
            ast.head = true;
            ast.touch("head");
            i + 1
        }
        "-L" | "--location" => {
            ast.location = true;
            ast.touch("location");
            i + 1
        }
        "-k" | "--insecure" => {
            ast.insecure = true;
            ast.touch("insecure");
            i + 1
        }
        "--compressed" => {
            ast.compressed = true;
            ast.touch("compressed");
            i + 1
        }
        "--http2" => {
            ast.http2 = true;
            ast.touch("http2");
            i + 1
        }
        "-s" | "--silent" => {
            ast.silent = true;
            ast.touch("silent");
            i + 1
        }
        "-v" | "--verbose" => {
            ast.verbose = true;
            ast.touch("verbose");
            i + 1
        }
        f if VALUE_FLAGS.contains(&f) => i + 2,
        _ => i + 1,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn p(s: &str) -> CurlAst {
        parse_curl(s)
    }

    fn h(k: &str, v: &str) -> (String, String) {
        (k.to_string(), v.to_string())
    }

    #[test]
    fn simple_get() {
        let a = p("curl https://example.com");
        assert_eq!(a.url, "https://example.com");
        assert_eq!(a.method, None);
        assert!(a.headers.is_empty());
    }

    #[test]
    fn explicit_method() {
        let a = p("curl -X POST https://api.example.com");
        assert_eq!(a.method.as_deref(), Some("POST"));
        assert_eq!(a.url, "https://api.example.com");
    }

    #[test]
    fn headers() {
        let a = p(
            "curl -H \"Content-Type: application/json\" -H \"Accept: text/plain\" https://example.com",
        );
        assert_eq!(
            a.headers,
            [
                h("Content-Type", "application/json"),
                h("Accept", "text/plain")
            ]
        );
    }

    #[test]
    fn data_flag() {
        assert_eq!(
            p("curl -d '{\"name\":\"Alex\"}' https://example.com").data,
            ["{\"name\":\"Alex\"}"]
        );
    }

    #[test]
    fn multiple_data_flags() {
        assert_eq!(
            p("curl -d \"a=1\" -d \"b=2\" https://example.com").data,
            ["a=1", "b=2"]
        );
    }

    #[test]
    fn form_data() {
        assert_eq!(
            p("curl -F \"file=@test.png\" -F \"name=test\" https://example.com").form,
            ["file=@test.png", "name=test"]
        );
    }

    #[test]
    fn basic_auth() {
        assert_eq!(
            p("curl -u admin:secret https://example.com")
                .user
                .as_deref(),
            Some("admin:secret")
        );
    }

    #[test]
    fn bearer_token_via_header() {
        assert_eq!(
            p("curl -H \"Authorization: Bearer mytoken123\" https://example.com").headers,
            [h("Authorization", "Bearer mytoken123")]
        );
    }

    #[test]
    fn insecure_and_follow_redirects() {
        assert!(p("curl -k https://self-signed.example.com").insecure);
        assert!(p("curl -L https://example.com").location);
    }

    #[test]
    fn combined_short_flags() {
        let a = p("curl -sSL https://example.com");
        assert!(a.silent && a.location);
        // TS quirk: the combined token advances by its length, skipping the URL.
        assert_eq!(a.url, "");
    }

    #[test]
    fn combined_value_flag() {
        let a = p("curl -XPOST https://example.com");
        assert_eq!(a.method.as_deref(), Some("POST"));
        assert_eq!(a.url, "");
    }

    #[test]
    fn timeout_proxy_http2() {
        assert_eq!(
            p("curl --max-time 30 https://example.com").max_time,
            Some(30.0)
        );
        assert_eq!(
            p("curl -x http://proxy:8080 https://example.com")
                .proxy
                .as_deref(),
            Some("http://proxy:8080")
        );
        assert!(p("curl --http2 https://example.com").http2);
    }

    #[test]
    fn ssl_certificates() {
        let a =
            p("curl --cacert ca.pem --cert client.pem --key client-key.pem https://example.com");
        assert_eq!(a.cacert.as_deref(), Some("ca.pem"));
        assert_eq!(a.cert.as_deref(), Some("client.pem"));
        assert_eq!(a.key.as_deref(), Some("client-key.pem"));
    }

    #[test]
    fn output_and_user_agent() {
        assert_eq!(
            p("curl -o output.json https://example.com")
                .output
                .as_deref(),
            Some("output.json")
        );
        assert_eq!(
            p("curl -A \"MyApp/1.0\" https://example.com")
                .user_agent
                .as_deref(),
            Some("MyApp/1.0")
        );
    }

    #[test]
    fn unknown_flags_stored() {
        let a = p("curl --unknown-flag value https://example.com");
        assert_eq!(
            a.unsupported_flags,
            [UnsupportedFlag {
                flag: "--unknown-flag".into(),
                value: Some("value".into())
            }]
        );
        assert_eq!(a.url, "https://example.com");
    }

    #[test]
    fn multiline_command() {
        let a = p(
            "curl -X POST \\\n  -H \"Content-Type: application/json\" \\\n  -d '{\"test\":true}' \\\n  https://api.example.com",
        );
        assert_eq!(a.method.as_deref(), Some("POST"));
        assert_eq!(a.url, "https://api.example.com");
        assert_eq!(a.data, ["{\"test\":true}"]);
    }

    #[test]
    fn head_and_get_flags() {
        assert!(p("curl -I https://example.com").head);
        let a = p("curl -G -d \"q=test\" https://example.com");
        assert!(a.get);
        assert_eq!(a.data, ["q=test"]);
    }

    #[test]
    fn data_urlencode_and_raw() {
        assert_eq!(
            p("curl --data-urlencode \"name=John Doe\" https://example.com").data_urlencode,
            ["name=John Doe"]
        );
        assert_eq!(
            p("curl --data-raw \"@not-a-file\" https://example.com").data_raw,
            ["@not-a-file"]
        );
    }

    #[test]
    fn compressed_and_cookies() {
        assert!(p("curl --compressed https://example.com").compressed);
        let a = p("curl -b \"session=abc123\" -c cookies.txt https://example.com");
        assert_eq!(a.cookie.as_deref(), Some("session=abc123"));
        assert_eq!(a.cookie_jar.as_deref(), Some("cookies.txt"));
    }

    #[test]
    fn max_redirs() {
        let a = p("curl -L --max-redirs 5 https://example.com");
        assert!(a.location);
        assert_eq!(a.max_redirs, Some(5.0));
    }
}
