//! Curl generator: [`CurlRunnerIr`] to a canonical, shell-safe curl command
//! (`generateCurlFromIR`). This is the `convert yaml` output and differs from
//! the dry-run command in `transport::curl_command` (other flags, line
//! continuations, no `-w`/`--proto` plumbing), so the two stay separate.

use super::types::{Body, CurlRunnerIr, FormField, FormFile};
use crate::jsval::num_to_string;
use crate::snapshot::js_stringify;

/// Shell-safe quoting for a value.
pub fn shell_quote(s: &str) -> String {
    if s.contains('\'') {
        let mut out = String::from("\"");
        for c in s.chars() {
            if matches!(c, '\\' | '"' | '$' | '`') {
                out.push('\\');
            }
            out.push(c);
        }
        out.push('"');
        return out;
    }
    if s.chars()
        .any(|c| !(c.is_ascii_alphanumeric() || "_./:@=,+%-".contains(c)))
    {
        return format!("'{s}'");
    }
    s.to_string()
}

fn file_spec(f: &FormFile) -> String {
    let mut spec = format!("@{}", f.file);
    if let Some(n) = f.filename.as_deref().filter(|n| !n.is_empty()) {
        spec.push_str(&format!(";filename={n}"));
    }
    if let Some(t) = f.content_type.as_deref().filter(|t| !t.is_empty()) {
        spec.push_str(&format!(";type={t}"));
    }
    spec
}

/// `generateCurlFromIR`.
pub fn generate_curl_from_ir(ir: &CurlRunnerIr) -> String {
    let mut seg: Vec<String> = vec!["curl".into()];

    if ir.method != "GET" || ir.body.is_some() || ir.form_data.is_some() {
        seg.push(format!("-X {}", ir.method));
    }

    for (k, v) in ir.headers.entries() {
        seg.push(format!("-H {}", shell_quote(&format!("{k}: {v}"))));
    }

    if let Some(a) = &ir.auth {
        let or_empty = |s: &Option<String>| s.clone().unwrap_or_default();
        match a.kind.as_str() {
            "basic" => seg.push(format!(
                "-u {}",
                shell_quote(&format!(
                    "{}:{}",
                    or_empty(&a.username),
                    or_empty(&a.password)
                ))
            )),
            "bearer" => seg.push(format!(
                "-H {}",
                shell_quote(&format!("Authorization: Bearer {}", or_empty(&a.token)))
            )),
            _ => {}
        }
    }

    if let Some(form) = &ir.form_data {
        for (name, value) in form.entries() {
            match value {
                FormField::Text(v) => {
                    seg.push(format!(
                        "--form-string {}",
                        shell_quote(&format!("{name}={v}"))
                    ));
                }
                FormField::File(f) => {
                    seg.push(format!(
                        "-F {}",
                        shell_quote(&format!("{name}={}", file_spec(f)))
                    ));
                }
            }
        }
    } else if let Some(body) = &ir.body {
        match body {
            Body::Json(c) => {
                seg.push(format!("-d {}", shell_quote(&js_stringify(c))));
                if ir.falsy_content_type
                    || ir.headers.get("Content-Type").is_none_or(String::is_empty)
                {
                    seg.push(format!(
                        "-H {}",
                        shell_quote("Content-Type: application/json")
                    ));
                }
            }
            Body::Urlencoded(c) => seg.push(format!("--data-urlencode {}", shell_quote(c))),
            Body::Binary(c) => seg.push(format!("--data-binary {}", shell_quote(c))),
            Body::Raw(c) => seg.push(format!("-d {}", shell_quote(c))),
        }
    }

    if let Some(t) = ir.timeout {
        seg.push(format!("--max-time {}", num_to_string((t / 1000.0).ceil())));
    }
    if ir.follow_redirects == Some(true) {
        seg.push("-L".into());
    }
    if let Some(m) = &ir.max_redirects {
        seg.push(format!("--max-redirs {}", m.to_js_string()));
    }
    if let Some(p) = ir.proxy.as_deref().filter(|p| !p.is_empty()) {
        seg.push(format!("-x {}", shell_quote(p)));
    }
    if ir.insecure {
        seg.push("-k".into());
    }
    if ir.http2 {
        seg.push("--http2".into());
    }
    if let Some(o) = ir.output.as_deref().filter(|o| !o.is_empty()) {
        seg.push(format!("-o {}", shell_quote(o)));
    }
    if let Some(ssl) = &ir.ssl {
        for (flag, v) in [
            ("--cacert", &ssl.ca),
            ("--cert", &ssl.cert),
            ("--key", &ssl.key),
        ] {
            if let Some(v) = v.as_deref().filter(|v| !v.is_empty()) {
                seg.push(format!("{flag} {}", shell_quote(v)));
            }
        }
    }

    let mut url = ir.url.clone();
    if let Some(p) = ir.params.as_ref().filter(|p| !p.is_empty()) {
        let mut qs = form_urlencoded::Serializer::new(String::new());
        for (k, v) in p.entries() {
            qs.append_pair(k, v);
        }
        url.push(if url.contains('?') { '&' } else { '?' });
        url.push_str(&qs.finish());
    }
    seg.push(shell_quote(&url));

    seg.join(" \\\n  ")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::convert::curl_parser::parse_curl;
    use crate::convert::normalizer::normalize;

    fn rt(s: &str) -> String {
        generate_curl_from_ir(&normalize(&parse_curl(s)).unwrap())
    }

    #[test]
    fn simple_get_round_trip() {
        let c = rt("curl https://api.example.com/users");
        assert!(c.contains("https://api.example.com/users"));
        assert!(!c.contains("-X"));
    }

    #[test]
    fn post_with_json_body_round_trip() {
        let c = rt(
            r#"curl -X POST -H "Content-Type: application/json" -d '{"name":"Alex"}' https://api.example.com/users"#,
        );
        assert!(c.contains("-X POST"));
        assert!(c.contains("{\"name\":\"Alex\"}"));
        assert!(c.contains("https://api.example.com/users"));
    }

    #[test]
    fn auth_round_trips() {
        assert!(
            rt("curl -H \"Authorization: Bearer TOKEN123\" https://api.example.com")
                .contains("Bearer TOKEN123")
        );
        assert!(rt("curl -u admin:secret https://api.example.com").contains("admin:secret"));
    }

    #[test]
    fn shell_quote_rules() {
        assert_eq!(shell_quote("a-b/c"), "a-b/c");
        assert_eq!(shell_quote(""), "");
        assert_eq!(shell_quote("a b"), "'a b'");
        assert_eq!(shell_quote("it's $x"), "\"it's \\$x\"");
    }
}
