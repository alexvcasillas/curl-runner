//! `JSON.stringify` as V8 prints it (compact, and the `(v, null, n)` pretty
//! form), so hashes and snapshot files match the TS CLI byte for byte.
//!
//! Differences from `serde_json` that matter here: whole floats print as
//! integers (`1.0` -> `1`), exponents use JS number formatting (`1e+21`),
//! `-0` prints `0`, and object keys that are array indices (`"0"`, `"12"`) come
//! first in ascending order, like JS property enumeration. Rust strings cannot
//! hold lone surrogates, so only the escapes for `"`, `\`, and control
//! characters apply (U+2028/U+2029 and DEL are written raw, as in JS).

use crate::jsval::num_to_string;
use crate::model::Json;
use serde_json::Map;

/// `JSON.stringify(v)`.
pub fn js_stringify(v: &Json) -> String {
    let mut out = String::new();
    write(&mut out, v, None, 0);
    out
}

/// `JSON.stringify(v, null, indent)` (`indent` spaces, JS caps it at 10).
pub fn js_stringify_pretty(v: &Json, indent: usize) -> String {
    let mut out = String::new();
    write(&mut out, v, Some(indent.min(10)), 0);
    out
}

/// Object entries in JS enumeration order: canonical array-index keys
/// ascending, then the rest in insertion order.
pub fn js_entries(m: &Map<String, Json>) -> Vec<(&String, &Json)> {
    js_order(m.iter())
}

/// [`js_entries`] for any string-keyed collection.
pub fn js_order<'a, V>(
    entries: impl Iterator<Item = (&'a String, &'a V)>,
) -> Vec<(&'a String, &'a V)> {
    let mut indexed: Vec<(u32, &String, &V)> = Vec::new();
    let mut rest = Vec::new();
    for (k, v) in entries {
        match array_index(k) {
            Some(i) => indexed.push((i, k, v)),
            None => rest.push((k, v)),
        }
    }
    if indexed.is_empty() {
        return rest;
    }
    indexed.sort_by_key(|(i, _, _)| *i);
    indexed
        .into_iter()
        .map(|(_, k, v)| (k, v))
        .chain(rest)
        .collect()
}

/// Canonical array index (`0`..=`2^32-2`, no leading zeros).
fn array_index(k: &str) -> Option<u32> {
    if k.is_empty() || k.len() > 10 || !k.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    if k.len() > 1 && k.starts_with('0') {
        return None;
    }
    k.parse::<u32>().ok().filter(|i| *i != u32::MAX)
}

fn write_str(out: &mut String, s: &str) {
    out.push('"');
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\u{8}' => out.push_str("\\b"),
            '\u{c}' => out.push_str("\\f"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out.push('"');
}

fn newline(out: &mut String, indent: Option<usize>, depth: usize) {
    if let Some(n) = indent {
        out.push('\n');
        out.push_str(&" ".repeat(n * depth));
    }
}

fn write(out: &mut String, v: &Json, indent: Option<usize>, depth: usize) {
    match v {
        Json::Null => out.push_str("null"),
        Json::Bool(b) => out.push_str(if *b { "true" } else { "false" }),
        Json::Number(n) => {
            let f = n.as_f64().unwrap_or(f64::NAN);
            // JSON.stringify prints non-finite numbers as null.
            if f.is_finite() {
                out.push_str(&num_to_string(f));
            } else {
                out.push_str("null");
            }
        }
        Json::String(s) => write_str(out, s),
        Json::Array(a) => {
            if a.is_empty() {
                out.push_str("[]");
                return;
            }
            out.push('[');
            for (i, item) in a.iter().enumerate() {
                if i > 0 {
                    out.push(',');
                }
                newline(out, indent, depth + 1);
                write(out, item, indent, depth + 1);
            }
            newline(out, indent, depth);
            out.push(']');
        }
        Json::Object(m) => {
            if m.is_empty() {
                out.push_str("{}");
                return;
            }
            out.push('{');
            for (i, (k, val)) in js_entries(m).into_iter().enumerate() {
                if i > 0 {
                    out.push(',');
                }
                newline(out, indent, depth + 1);
                write_str(out, k);
                out.push(':');
                if indent.is_some() {
                    out.push(' ');
                }
                write(out, val, indent, depth + 1);
            }
            newline(out, indent, depth);
            out.push('}');
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn parse(s: &str) -> Json {
        serde_json::from_str(s).unwrap()
    }

    #[test]
    fn numbers_print_like_js() {
        assert_eq!(js_stringify(&parse("1.0")), "1");
        assert_eq!(
            js_stringify(&parse("[1e21, 1e-7, -0, -0.0, 0.1]")),
            "[1e+21,1e-7,0,0,0.1]"
        );
        assert_eq!(
            js_stringify(&parse("123456789012345680000")),
            "123456789012345680000"
        );
        // beyond 2^53: the nearest double, as JSON.parse would give
        assert_eq!(js_stringify(&parse("9007199254740993")), "9007199254740992");
        assert_eq!(
            js_stringify(&parse("18446744073709551615")),
            "18446744073709552000"
        );
        assert_eq!(js_stringify(&parse("1.5e300")), "1.5e+300");
        assert_eq!(js_stringify(&parse("100")), "100");
    }

    #[test]
    fn strings_escape_like_js() {
        assert_eq!(js_stringify(&json!("a\"b\\c")), r#""a\"b\\c""#);
        assert_eq!(js_stringify(&json!("\n\r\t\u{8}\u{c}")), r#""\n\r\t\b\f""#);
        assert_eq!(js_stringify(&json!("\u{1}\u{1f}")), r#""\u0001\u001f""#);
        // JSON.stringify does not escape these, nor non-ASCII, nor emoji, nor "/"
        assert_eq!(
            js_stringify(&json!("\u{2028}\u{2029}\u{7f}é😀/")),
            "\"\u{2028}\u{2029}\u{7f}é😀/\""
        );
    }

    #[test]
    fn key_order_is_insertion_with_index_keys_first() {
        assert_eq!(js_stringify(&parse(r#"{"b":1,"a":2}"#)), r#"{"b":1,"a":2}"#);
        assert_eq!(
            js_stringify(&parse(r#"{"b":1,"10":2,"2":3,"01":4,"a":5}"#)),
            r#"{"2":3,"10":2,"b":1,"01":4,"a":5}"#
        );
    }

    #[test]
    fn duplicate_keys_keep_first_position_last_value() {
        assert_eq!(
            js_stringify(&parse(r#"{"a":1,"b":2,"a":3}"#)),
            r#"{"a":3,"b":2}"#
        );
    }

    #[test]
    fn compact_has_no_spaces_and_pretty_uses_two_spaces() {
        let v = parse(r#"{"a":[1,{"b":null}],"c":{},"d":[],"e":"x"}"#);
        assert_eq!(
            js_stringify(&v),
            r#"{"a":[1,{"b":null}],"c":{},"d":[],"e":"x"}"#
        );
        assert_eq!(
            js_stringify_pretty(&v, 2),
            "{\n  \"a\": [\n    1,\n    {\n      \"b\": null\n    }\n  ],\n  \"c\": {},\n  \"d\": [],\n  \"e\": \"x\"\n}"
        );
    }
}
