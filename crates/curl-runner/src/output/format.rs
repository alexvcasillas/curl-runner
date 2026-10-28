//! Formatting helpers (`core/format/formatters.ts`), with JS number semantics.

use curl_runner_core::jsval::num_to_string;
use curl_runner_core::model::Json;

/// JS `Number.prototype.toFixed(digits)`: exact ties round away from zero
/// (Rust's formatter rounds them to even).
pub fn to_fixed(x: f64, digits: usize) -> String {
    if !x.is_finite() {
        return num_to_string(x);
    }
    if x.abs() >= 1e21 {
        return num_to_string(x);
    }
    let neg = x < 0.0;
    let a = x.abs();
    let mut s = format!("{a:.digits$}");
    // Exact decimal expansion far enough to see an exact tie.
    let wide = format!("{a:.prec$}", prec = digits + 40);
    let tail = &wide[wide.len() - 40..];
    if tail.starts_with('5') && tail[1..].bytes().all(|b| b == b'0') {
        let step = 10f64.powi(-(digits as i32));
        let trunc = &wide[..wide.len() - 40].trim_end_matches('.');
        let base: f64 = trunc.parse().unwrap_or(a);
        s = format!("{:.digits$}", base + step);
    }
    // JS keeps the sign even when the digits round to zero ("-0.00").
    if neg { format!("-{s}") } else { s }
}

/// `formatDuration`.
pub fn format_duration(ms: f64) -> String {
    if ms < 1000.0 {
        format!("{}ms", to_fixed(ms, 0))
    } else {
        format!("{}s", to_fixed(ms / 1000.0, 2))
    }
}

/// `formatLatency`.
pub fn format_latency(ms: f64) -> String {
    if ms < 1.0 {
        format!("{}µs", to_fixed(ms * 1000.0, 0))
    } else if ms < 1000.0 {
        format!("{}ms", to_fixed(ms, 1))
    } else {
        format!("{}s", to_fixed(ms / 1000.0, 2))
    }
}

/// `formatSize` (unit index clamped to GB; TS printed `undefined` past it).
pub fn format_size(bytes: Option<f64>) -> String {
    let b = match bytes {
        Some(b) if b != 0.0 && !b.is_nan() => b,
        _ => return "0 B".into(),
    };
    const SIZES: [&str; 4] = ["B", "KB", "MB", "GB"];
    let i = (b.ln() / 1024f64.ln()).floor().clamp(0.0, 3.0) as i32;
    format!("{} {}", to_fixed(b / 1024f64.powi(i), 2), SIZES[i as usize])
}

/// `formatJson` for `pretty` (2-space), `json` (compact) and `raw` (strings
/// as-is, otherwise compact).
pub fn format_json(data: &Json, format: &str) -> String {
    match format {
        "raw" => match data {
            Json::String(s) => s.clone(),
            v => stringify(v, None),
        },
        "json" => stringify(data, None),
        _ => stringify(data, Some(2)),
    }
}

/// `JSON.stringify(v, null, indent)` with JS number formatting.
pub fn stringify(v: &Json, indent: Option<usize>) -> String {
    let mut out = String::new();
    write_json(&mut out, v, indent, 0);
    out
}

fn write_json(out: &mut String, v: &Json, indent: Option<usize>, depth: usize) {
    let nl = |out: &mut String, d: usize| {
        if let Some(n) = indent {
            out.push('\n');
            out.push_str(&" ".repeat(n * d));
        }
    };
    match v {
        Json::Null => out.push_str("null"),
        Json::Bool(b) => out.push_str(if *b { "true" } else { "false" }),
        Json::Number(n) => match (n.as_i64(), n.as_u64()) {
            (Some(i), _) => out.push_str(&i.to_string()),
            (_, Some(u)) => out.push_str(&u.to_string()),
            _ => {
                let f = n.as_f64().unwrap_or(f64::NAN);
                // JSON.stringify(NaN/Infinity) === "null"
                if f.is_finite() {
                    out.push_str(&num_to_string(f));
                } else {
                    out.push_str("null");
                }
            }
        },
        Json::String(s) => out.push_str(&serde_json::to_string(s).unwrap_or_default()),
        Json::Array(a) => {
            if a.is_empty() {
                out.push_str("[]");
                return;
            }
            out.push('[');
            for (i, x) in a.iter().enumerate() {
                if i > 0 {
                    out.push(',');
                }
                nl(out, depth + 1);
                write_json(out, x, indent, depth + 1);
            }
            nl(out, depth);
            out.push(']');
        }
        Json::Object(o) => {
            if o.is_empty() {
                out.push_str("{}");
                return;
            }
            out.push('{');
            for (i, (k, x)) in o.iter().enumerate() {
                if i > 0 {
                    out.push(',');
                }
                nl(out, depth + 1);
                out.push_str(&serde_json::to_string(k).unwrap_or_default());
                out.push(':');
                if indent.is_some() {
                    out.push(' ');
                }
                write_json(out, x, indent, depth + 1);
            }
            nl(out, depth);
            out.push('}');
        }
    }
}

/// JS truthiness of an optional JSON value (`result.body ? ... : ...`).
pub fn truthy(v: Option<&Json>) -> bool {
    match v {
        None | Some(Json::Null) => false,
        Some(Json::Bool(b)) => *b,
        Some(Json::Number(n)) => n.as_f64().is_some_and(|f| f != 0.0 && !f.is_nan()),
        Some(Json::String(s)) => !s.is_empty(),
        Some(_) => true,
    }
}

/// `truncate` (by chars).
#[allow(dead_code)] // watch port
pub fn truncate(s: &str, max: usize) -> String {
    if s.chars().count() <= max {
        return s.to_string();
    }
    let keep: String = s.chars().take(max.saturating_sub(3)).collect();
    format!("{keep}...")
}

/// `getShortFilename`: drop the directory and the first `.yaml`.
pub fn short_filename(path: &str) -> String {
    let base = path.rsplit('/').next().unwrap_or(path);
    base.replacen(".yaml", "", 1)
}

/// `formatPercent`.
#[allow(dead_code)] // watch port
pub fn format_percent(value: f64, decimals: usize) -> String {
    format!("{}%", to_fixed(value * 100.0, decimals))
}

/// JS `String.prototype.substring(0, n)` by UTF-16 units (char-safe).
pub fn js_prefix(s: &str, n: usize) -> (String, bool) {
    let mut units = 0;
    for (i, c) in s.char_indices() {
        units += c.len_utf16();
        if units > n {
            return (s[..i].to_string(), true);
        }
    }
    (s.to_string(), false)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn to_fixed_matches_js() {
        assert_eq!(to_fixed(0.5, 0), "1");
        assert_eq!(to_fixed(2.5, 0), "3");
        assert_eq!(to_fixed(1.005, 2), "1.00"); // binary value below the tie
        assert_eq!(to_fixed(1.25, 1), "1.3");
        assert_eq!(to_fixed(50.7, 0), "51");
        assert_eq!(to_fixed(0.0, 2), "0.00");
        assert_eq!(to_fixed(-1.5, 0), "-2");
        assert_eq!(to_fixed(999.5, 0), "1000");
    }

    // formatters.test.ts
    #[test]
    fn format_duration_cases() {
        assert_eq!(format_duration(100.0), "100ms");
        assert_eq!(format_duration(500.0), "500ms");
        assert_eq!(format_duration(999.0), "999ms");
        assert_eq!(format_duration(1000.0), "1.00s");
        assert_eq!(format_duration(2500.0), "2.50s");
        assert_eq!(format_duration(10000.0), "10.00s");
        assert_eq!(format_duration(0.0), "0ms");
        assert_eq!(format_duration(50.7), "51ms");
    }

    #[test]
    fn format_latency_cases() {
        assert_eq!(format_latency(0.5), "500µs");
        assert_eq!(format_latency(0.1), "100µs");
        assert_eq!(format_latency(1.0), "1.0ms");
        assert_eq!(format_latency(50.5), "50.5ms");
        assert_eq!(format_latency(999.0), "999.0ms");
        assert_eq!(format_latency(1000.0), "1.00s");
        assert_eq!(format_latency(2500.0), "2.50s");
    }

    #[test]
    fn format_size_cases() {
        assert_eq!(format_size(Some(100.0)), "100.00 B");
        assert_eq!(format_size(Some(500.0)), "500.00 B");
        assert_eq!(format_size(Some(1024.0)), "1.00 KB");
        assert_eq!(format_size(Some(2048.0)), "2.00 KB");
        assert_eq!(format_size(Some(1048576.0)), "1.00 MB");
        assert_eq!(format_size(Some(5242880.0)), "5.00 MB");
        assert_eq!(format_size(None), "0 B");
        assert_eq!(format_size(Some(0.0)), "0 B");
    }

    #[test]
    fn format_json_cases() {
        assert_eq!(format_json(&json!({"a": 1}), "pretty"), "{\n  \"a\": 1\n}");
        assert_eq!(format_json(&json!({"a": 1}), "raw"), "{\"a\":1}");
        assert_eq!(format_json(&json!({"a": 1}), "json"), "{\"a\":1}");
        assert_eq!(format_json(&json!("hello"), "raw"), "hello");
        assert_eq!(format_json(&json!([1, 2, 3]), "json"), "[1,2,3]");
        let nested = format_json(&json!({"a": {"b": 1}}), "pretty");
        assert!(nested.contains("\"a\"") && nested.contains("\"b\""));
    }

    #[test]
    fn stringify_matches_js_layout() {
        let v = json!({"a": [], "b": {}, "c": [1, {"d": 1.0}], "e": 0.1, "f": "x\"y"});
        assert_eq!(
            stringify(&v, Some(2)),
            "{\n  \"a\": [],\n  \"b\": {},\n  \"c\": [\n    1,\n    {\n      \"d\": 1\n    }\n  ],\n  \"e\": 0.1,\n  \"f\": \"x\\\"y\"\n}"
        );
        assert_eq!(stringify(&json!("s"), Some(2)), "\"s\"");
    }

    #[test]
    fn truncate_cases() {
        assert_eq!(truncate("hello", 10), "hello");
        assert_eq!(truncate("hello world", 8), "hello...");
        assert_eq!(truncate("hello", 5), "hello");
        assert_eq!(truncate("hello world", 4), "h...");
    }

    #[test]
    fn short_filename_cases() {
        assert_eq!(short_filename("/path/to/file.yaml"), "file");
        assert_eq!(short_filename("api.yaml"), "api");
        assert_eq!(
            short_filename("/home/user/projects/api/tests/users.yaml"),
            "users"
        );
        assert_eq!(short_filename("/path/to/file.json"), "file.json");
    }

    #[test]
    fn format_percent_cases() {
        assert_eq!(format_percent(0.5, 1), "50.0%");
        assert_eq!(format_percent(1.0, 1), "100.0%");
        assert_eq!(format_percent(0.123, 1), "12.3%");
        assert_eq!(format_percent(0.5, 0), "50%");
        assert_eq!(format_percent(0.123, 2), "12.30%");
        assert_eq!(format_percent(0.0, 1), "0.0%");
    }

    #[test]
    fn truthiness() {
        assert!(!truthy(None));
        assert!(!truthy(Some(&json!(null))));
        assert!(!truthy(Some(&json!(""))));
        assert!(!truthy(Some(&json!(0))));
        assert!(!truthy(Some(&json!(false))));
        assert!(truthy(Some(&json!({}))));
        assert!(truthy(Some(&json!("x"))));
    }

    #[test]
    fn js_prefix_units() {
        assert_eq!(js_prefix("abcdef", 3), ("abc".into(), true));
        assert_eq!(js_prefix("abc", 3), ("abc".into(), false));
    }
}
