//! Response assembly from a finished libcurl handle
//! (`core/curl/response-parser.ts` + `core/curl/body-parser.ts`).

use super::{Collector, Response, TransportError};

/// `CURLE_WRITE_ERROR`, what libcurl reports when the body write fails.
const CURLE_WRITE_ERROR: u32 = 23;
use crate::model::{HeaderBlock, HeaderValue, Headers, Json, Metrics};
use curl::easy::Easy2;
use std::time::Duration;

/// Raw libcurl timing values in seconds (the `-w %{json}` fields TS reads).
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct RawTimings {
    pub time_total: Option<f64>,
    pub time_namelookup: Option<f64>,
    pub time_connect: Option<f64>,
    pub time_appconnect: Option<f64>,
    pub time_starttransfer: Option<f64>,
    pub size_download: Option<f64>,
}

/// TS `parseMetrics`: seconds -> milliseconds; `download` is total time.
pub fn metrics_from_raw(raw: &RawTimings) -> Metrics {
    let ms = |v: Option<f64>| v.unwrap_or(0.0) * 1000.0;
    Metrics {
        duration: ms(raw.time_total),
        size: raw.size_download,
        dns_lookup: Some(ms(raw.time_namelookup)),
        tcp_connection: Some(ms(raw.time_connect)),
        tls_handshake: Some(ms(raw.time_appconnect)),
        first_byte: Some(ms(raw.time_starttransfer)),
        download: Some(ms(raw.time_total)),
    }
}

/// `^HTTP/[\d.]+\s+(\d{3})` from response-parser.ts.
fn status_line(s: &str) -> Option<u16> {
    let rest = s.strip_prefix("HTTP/")?;
    let ver_len = rest.find(|c: char| !(c.is_ascii_digit() || c == '.'))?;
    if ver_len == 0 {
        return None;
    }
    let rest = &rest[ver_len..];
    let digits = rest.trim_start();
    if digits.len() == rest.len() {
        return None;
    }
    let code = digits.get(..3)?;
    if code.bytes().all(|b| b.is_ascii_digit()) {
        code.parse().ok()
    } else {
        None
    }
}

/// TS `parseHeaderBlocks`: split `-D -` style header text into blocks.
/// Keys lowercased, repeated keys become `Many`. Returns the blocks and the
/// byte offset where the headers end.
pub fn parse_header_blocks(text: &str) -> (Vec<HeaderBlock>, usize) {
    let bytes = text.as_bytes();
    let mut blocks = Vec::new();
    let mut i = 0;
    while i < bytes.len() {
        while i < bytes.len() && matches!(bytes[i], b'\r' | b'\n' | b' ' | b'\t') {
            i += 1;
        }
        if i >= bytes.len() {
            break;
        }
        let Some(status) = status_line(&text[i..]) else {
            break;
        };
        let crlf = text[i..].find("\r\n\r\n").map(|p| p + i);
        let lf = text[i..].find("\n\n").map(|p| p + i);
        let (end, sep) = match (crlf, lf) {
            (Some(c), l) if l.is_none_or(|l| c <= l) => (c, 4),
            (_, Some(l)) => (l, 2),
            _ => break,
        };
        let mut headers = Headers::new();
        if let Some(nl) = text[i..].find('\n') {
            let section_start = i + nl + 1;
            if section_start < end {
                for line in text[section_start..end].split('\n') {
                    let line = line.strip_suffix('\r').unwrap_or(line);
                    let Some((k, v)) = line.split_once(':') else {
                        continue;
                    };
                    let key = k.trim().to_lowercase();
                    if key.is_empty() {
                        continue;
                    }
                    let value = v.trim().to_string();
                    match headers.get_mut(&key) {
                        None => {
                            headers.insert(key, HeaderValue::One(value));
                        }
                        Some(HeaderValue::Many(vs)) => vs.push(value),
                        Some(HeaderValue::One(first)) => {
                            let first = std::mem::take(first);
                            headers.insert(key, HeaderValue::Many(vec![first, value]));
                        }
                    }
                }
            }
        }
        blocks.push(HeaderBlock { status, headers });
        i = end + sep;
    }
    (blocks, i)
}

/// Last block with status >= 200 (skips 1xx informational).
pub fn final_block(blocks: &[HeaderBlock]) -> Option<&HeaderBlock> {
    blocks.iter().rev().find(|b| b.status >= 200)
}

/// JS has one number type: whole floats become integers (`2.0` -> `2`).
fn js_numbers(v: Json) -> Json {
    match v {
        Json::Number(n) if n.is_f64() => match n.as_f64() {
            Some(f) if f.fract() == 0.0 && f.abs() < 9.007_199_254_740_992e15 => {
                Json::from(f as i64)
            }
            _ => Json::Number(n),
        },
        Json::Array(a) => Json::Array(a.into_iter().map(js_numbers).collect()),
        Json::Object(o) => Json::Object(o.into_iter().map(|(k, v)| (k, js_numbers(v))).collect()),
        other => other,
    }
}

/// Body as TS sees it: UTF-8 (lossy), trimmed, then `parseResponseBody`
/// (JSON when the content type says so or it looks like JSON, else string).
pub fn parse_body(bytes: &[u8], content_type: Option<&str>) -> Json {
    let text = String::from_utf8_lossy(bytes);
    let body = crate::jsval::trim(&text);
    let json_ct = content_type.is_some_and(|ct| ct.contains("application/json"));
    if !body.is_empty()
        && (json_ct || body.starts_with('{') || body.starts_with('['))
        && let Ok(v) = serde_json::from_str::<Json>(body)
    {
        return js_numbers(v);
    }
    Json::String(body.to_string())
}

/// 2xx/3xx.
pub fn is_success_status(status: Option<u16>) -> bool {
    status.is_some_and(|s| (200..400).contains(&s))
}

/// libcurl second-precision double -> seconds rounded to the microsecond
/// (what curl's `-w` prints).
fn secs(d: Result<Duration, curl::Error>) -> Option<f64> {
    d.ok()
        .map(|d| (d.as_nanos() as f64 / 1000.0).round() / 1_000_000.0)
}

/// Assemble the `Response` from a handle whose transfer completed. Fails
/// only when the `output` file could not be created (empty-body case; a
/// failed write of a non-empty body already fails the transfer).
pub fn finish(mut handle: Easy2<Collector>) -> Result<Response, TransportError> {
    handle.get_mut().ensure_output_file();
    if let Some(msg) = handle.get_ref().output_error() {
        return Err(TransportError {
            code: Some(CURLE_WRITE_ERROR),
            message: msg.to_string(),
        });
    }
    let raw = RawTimings {
        time_total: secs(handle.total_time()),
        time_namelookup: secs(handle.namelookup_time()),
        time_connect: secs(handle.connect_time()),
        time_appconnect: secs(handle.appconnect_time()),
        time_starttransfer: secs(handle.starttransfer_time()),
        size_download: handle.download_size().ok(),
    };
    let code = handle.response_code().unwrap_or(0);
    let collector = handle.get_ref();
    let (header_history, _) =
        parse_header_blocks(&String::from_utf8_lossy(collector.header_bytes()));
    let fin = final_block(&header_history);
    let status = match u16::try_from(code) {
        Ok(c) if c != 0 => c,
        _ => fin.map_or(0, |b| b.status),
    };
    let headers = fin.map(|b| b.headers.clone()).unwrap_or_default();
    let content_type = headers.get("content-type").and_then(|v| match v {
        HeaderValue::One(s) => Some(s.as_str()),
        HeaderValue::Many(vs) => vs.first().map(String::as_str),
    });
    let body = parse_body(collector.body_bytes(), content_type);
    Ok(Response {
        status,
        headers,
        header_history,
        body,
        metrics: metrics_from_raw(&raw),
    })
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use serde_json::json;

    fn one(s: &str) -> HeaderValue {
        HeaderValue::One(s.to_string())
    }

    #[test]
    fn body_trim_is_js_trim() {
        // JS `trim()` keeps U+0085 (NEL, not JS whitespace) and strips BOM,
        // U+2028/U+2029 and the Zs spaces.
        assert_eq!(
            parse_body("\u{85}hi\u{85}".as_bytes(), Some("text/plain")),
            json!("\u{85}hi\u{85}")
        );
        assert_eq!(
            parse_body(
                "\u{feff}\u{2028}\u{3000} hi\t\u{2029}\u{a0}".as_bytes(),
                Some("text/plain")
            ),
            json!("hi")
        );
    }

    // ---- parseMetrics ----

    #[test]
    fn converts_seconds_to_milliseconds() {
        let m = metrics_from_raw(&RawTimings {
            time_total: Some(1.5),
            time_namelookup: Some(0.1),
            time_connect: Some(0.2),
            time_appconnect: Some(0.3),
            time_starttransfer: Some(0.5),
            size_download: None,
        });
        assert_eq!(m.duration, 1500.0);
        assert_eq!(m.dns_lookup, Some(100.0));
        assert_eq!(m.tcp_connection, Some(200.0));
        assert_eq!(m.tls_handshake, Some(300.0));
        assert_eq!(m.first_byte, Some(500.0));
        assert_eq!(m.download, Some(1500.0));
        assert_eq!(m.size, None);
    }

    #[test]
    fn includes_size_download() {
        let m = metrics_from_raw(&RawTimings {
            time_total: Some(0.5),
            size_download: Some(1024.0),
            ..Default::default()
        });
        assert_eq!(m.size, Some(1024.0));
    }

    #[test]
    fn handles_missing_fields() {
        let m = metrics_from_raw(&RawTimings::default());
        assert_eq!(m.duration, 0.0);
        for v in [
            m.dns_lookup,
            m.tcp_connection,
            m.tls_handshake,
            m.first_byte,
            m.download,
        ] {
            assert_eq!(v, Some(0.0));
        }
    }

    #[test]
    fn handles_zero_values() {
        let m = metrics_from_raw(&RawTimings {
            time_total: Some(0.0),
            size_download: Some(0.0),
            ..Default::default()
        });
        assert_eq!(m.duration, 0.0);
        assert_eq!(m.size, Some(0.0));
    }

    #[test]
    fn microsecond_values_match_js_parse_then_multiply() {
        // curl -w prints 0.001234; JS computes 0.001234 * 1000.
        let m = metrics_from_raw(&RawTimings {
            time_total: Some(0.001234),
            ..Default::default()
        });
        assert_eq!(m.duration, 0.001234 * 1000.0);
    }

    // ---- parseHeaderBlocks ----

    #[test]
    fn parses_http11_block_with_crlf() {
        let text =
            "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nX-Custom: value\r\n\r\nbody";
        let (blocks, off) = parse_header_blocks(text);
        assert_eq!(blocks.len(), 1);
        assert_eq!(blocks[0].status, 200);
        assert_eq!(blocks[0].headers["content-type"], one("application/json"));
        assert_eq!(blocks[0].headers["x-custom"], one("value"));
        assert_eq!(&text[off..], "body");
    }

    #[test]
    fn parses_http2_status_line() {
        let (blocks, _) =
            parse_header_blocks("HTTP/2 200\r\nContent-Type: application/json\r\n\r\n{}");
        assert_eq!(blocks.len(), 1);
        assert_eq!(blocks[0].status, 200);
        assert_eq!(blocks[0].headers["content-type"], one("application/json"));
    }

    #[test]
    fn handles_lf_only_line_endings() {
        let text = "HTTP/1.1 200 OK\nContent-Type: application/json\n\nbody";
        let (blocks, off) = parse_header_blocks(text);
        assert_eq!(blocks[0].headers["content-type"], one("application/json"));
        assert_eq!(&text[off..], "body");
    }

    #[test]
    fn parses_redirect_chain_returning_all_blocks() {
        let text = "HTTP/1.1 301 Moved Permanently\r\nLocation: /next\r\n\r\nHTTP/1.1 200 OK\r\nContent-Type: text/plain\r\n\r\nfinal";
        let (blocks, off) = parse_header_blocks(text);
        assert_eq!(
            blocks.iter().map(|b| b.status).collect::<Vec<_>>(),
            [301, 200]
        );
        assert_eq!(&text[off..], "final");
    }

    #[test]
    fn handles_1xx_informational_preamble() {
        let text = "HTTP/1.1 100 Continue\r\n\r\nHTTP/1.1 200 OK\r\nContent-Type: application/json\r\n\r\nbody";
        let (blocks, _) = parse_header_blocks(text);
        assert_eq!(
            blocks.iter().map(|b| b.status).collect::<Vec<_>>(),
            [100, 200]
        );
    }

    #[test]
    fn handles_head_style_empty_body() {
        let text = "HTTP/1.1 200 OK\r\nContent-Length: 0\r\n\r\n";
        let (blocks, off) = parse_header_blocks(text);
        assert_eq!(blocks[0].status, 200);
        assert_eq!(&text[off..], "");
    }

    #[test]
    fn does_not_consume_body_containing_blank_line() {
        let text = "HTTP/1.1 200 OK\r\nContent-Type: text/plain\r\n\r\nfirst\r\n\r\nsecond";
        let (blocks, off) = parse_header_blocks(text);
        assert_eq!(blocks.len(), 1);
        assert_eq!(&text[off..], "first\r\n\r\nsecond");
    }

    #[test]
    fn returns_no_blocks_when_no_status_line() {
        let (blocks, off) = parse_header_blocks("just a body");
        assert!(blocks.is_empty());
        assert_eq!(off, 0);
    }

    #[test]
    fn consumes_connect_proxy_preamble() {
        let text = "HTTP/1.1 200 Connection established\r\n\r\nHTTP/1.1 200 OK\r\nContent-Type: application/json\r\n\r\nbody";
        assert_eq!(parse_header_blocks(text).0.len(), 2);
    }

    #[test]
    fn repeated_headers_become_array() {
        let (blocks, _) = parse_header_blocks(
            "HTTP/1.1 200 OK\r\nSet-Cookie: a=1\r\nSet-Cookie: b=2\r\n\r\nbody",
        );
        assert_eq!(
            blocks[0].headers["set-cookie"],
            HeaderValue::Many(vec!["a=1".into(), "b=2".into()])
        );
    }

    #[test]
    fn lowercases_header_keys() {
        let (blocks, _) = parse_header_blocks("HTTP/1.1 200 OK\r\nX-CUSTOM: value\r\n\r\n");
        assert_eq!(blocks[0].headers["x-custom"], one("value"));
    }

    // ---- extractResponseFromOutput (final block selection) ----

    #[test]
    fn picks_last_non_informational_block_on_redirect_chain() {
        let (blocks, _) = parse_header_blocks(
            "HTTP/1.1 301 Moved\r\nLocation: /next\r\n\r\nHTTP/1.1 200 OK\r\nContent-Type: text/html\r\n\r\n",
        );
        let f = final_block(&blocks).unwrap();
        assert_eq!(f.status, 200);
        assert_eq!(f.headers["content-type"], one("text/html"));
    }

    #[test]
    fn skips_1xx_informational_when_picking_final_headers() {
        let (blocks, _) = parse_header_blocks(
            "HTTP/1.1 100 Continue\r\n\r\nHTTP/1.1 204 No Content\r\nX-Marker: end\r\n\r\n",
        );
        let f = final_block(&blocks).unwrap();
        assert_eq!(f.status, 204);
        assert_eq!(f.headers["x-marker"], one("end"));
    }

    #[test]
    fn returns_no_final_block_when_none() {
        assert!(final_block(&[]).is_none());
    }

    // ---- body-parser.ts (+ TS trims the body) ----

    #[test]
    fn body_json_when_looks_like_json() {
        assert_eq!(parse_body(b"  {\"x\":1}\n", None), json!({"x": 1}));
        assert_eq!(parse_body(b"[1,2.0]", None), json!([1, 2]));
    }

    #[test]
    fn body_json_when_content_type_json() {
        assert_eq!(
            parse_body(b"123", Some("application/json; charset=utf-8")),
            json!(123)
        );
        assert_eq!(parse_body(b"123", Some("text/plain")), json!("123"));
    }

    #[test]
    fn body_string_when_invalid_json_or_plain() {
        assert_eq!(parse_body(b"{oops", None), json!("{oops"));
        assert_eq!(
            parse_body(b"line1\nline2\nline3\n", None),
            json!("line1\nline2\nline3")
        );
        assert_eq!(parse_body(b"", None), json!(""));
    }

    /// Bytes 0..=255 decoded like `TextDecoder`: ASCII kept, each of the 128
    /// invalid high bytes becomes one U+FFFD.
    pub(crate) fn lossy_0_to_255() -> String {
        let mut s: String = (0u8..128).map(char::from).collect();
        s.push_str(&"\u{FFFD}".repeat(128));
        s
    }

    #[test]
    fn body_binary_is_lossy_string() {
        let bytes: Vec<u8> = (0..=255u8).collect();
        assert_eq!(parse_body(&bytes, None), Json::String(lossy_0_to_255()));
    }

    // ---- isSuccessStatus ----

    #[test]
    fn success_status_ranges() {
        for s in [200, 201, 204, 299, 300, 301, 302, 399] {
            assert!(is_success_status(Some(s)), "{s}");
        }
        for s in [100, 101, 400, 401, 404, 499, 500, 502, 503, 599] {
            assert!(!is_success_status(Some(s)), "{s}");
        }
        assert!(!is_success_status(None));
    }
}
