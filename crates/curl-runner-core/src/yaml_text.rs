//! YAML scalar text helpers shared by the emitters (`convert`'s
//! serializer and `validate --fix`): when a plain scalar needs quotes, and
//! how to quote it.

use crate::model::Json;

pub fn quote(s: &str) -> String {
    format!("\"{}\"", s.replace('\\', "\\\\").replace('"', "\\\""))
}

/// Quote a YAML string value if necessary.
pub fn yaml_str(s: &str) -> String {
    if s.is_empty() {
        return "\"\"".into();
    }
    if needs_quoting(s) {
        return quote_safe(s);
    }
    // Whatever the TS rule leaves bare must still read back as the same string
    // (`- a`, multi-line text, `#x`, ...).
    let plain_ok = reads_back(&format!("v: {s}"), &serde_json::json!({ "v": s }));
    if plain_ok {
        s.to_string()
    } else {
        quote_escaped(s)
    }
}

/// `quote`, except text with line breaks or control characters, which needs
/// escapes to survive.
fn quote_safe(s: &str) -> String {
    if s.chars()
        .any(|c| c.is_control() || matches!(c, '\u{2028}' | '\u{2029}' | '\u{feff}'))
    {
        quote_escaped(s)
    } else {
        quote(s)
    }
}

/// Quote a YAML key if necessary.
pub fn yaml_key(s: &str) -> String {
    let mut b = s.bytes();
    let ok = b
        .next()
        .is_some_and(|c| c.is_ascii_alphabetic() || c == b'_')
        && b.all(|c| c.is_ascii_alphanumeric() || c == b'_' || c == b'-');
    // `on: x` reads back as the key `true` (YAML 1.1 booleans, like Bun).
    let reads_as_scalar = matches!(
        s.to_lowercase().as_str(),
        "true" | "false" | "null" | "yes" | "no" | "on" | "off"
    );
    if ok && !reads_as_scalar {
        s.to_string()
    } else {
        quote_safe(s)
    }
}

pub fn needs_quoting(s: &str) -> bool {
    if s.contains([
        '{', '}', '[', ']', ',', '&', '*', '?', '|', '>', '!', '%', '#', '@', '`',
    ]) {
        return true;
    }
    if s.contains(": ") || s.ends_with(':') {
        return true;
    }
    if s.starts_with('\'') || s.starts_with('"') {
        return true;
    }
    if matches!(
        s.to_lowercase().as_str(),
        "true" | "false" | "null" | "yes" | "no" | "on" | "off"
    ) {
        return true;
    }
    if is_plain_number(s) {
        return true;
    }
    s.starts_with(' ') || s.ends_with(' ')
}

/// `/^-?\d+(\.\d+)?$/` (ASCII digits).
pub fn is_plain_number(s: &str) -> bool {
    let s = s.strip_prefix('-').unwrap_or(s);
    let (int, frac) = match s.split_once('.') {
        Some((i, f)) => (i, Some(f)),
        None => (s, None),
    };
    let digits = |x: &str| !x.is_empty() && x.bytes().all(|b| b.is_ascii_digit());
    digits(int) && frac.is_none_or(digits)
}

/// A double-quoted YAML scalar that reads back as `s`: `JSON.stringify`'s
/// escapes (so the text matches TS where TS quoted), plus `\uXXXX` for the
/// characters YAML does not allow raw or reads as line breaks (DEL, C1
/// controls, NEL, U+2028/U+2029, BOM, U+FFFE/U+FFFF).
pub fn quote_escaped(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 2);
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
            c if (c as u32) < 0x20
                || ('\u{7f}'..='\u{9f}').contains(&c)
                || matches!(
                    c,
                    '\u{2028}' | '\u{2029}' | '\u{feff}' | '\u{fffe}' | '\u{ffff}'
                ) =>
            {
                out.push_str(&format!("\\u{:04x}", c as u32));
            }
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

/// Whether `text` reads back (with the runtime's parser) as `want`.
pub fn reads_back(text: &str, want: &Json) -> bool {
    crate::parser::parse_raw_stream(text).is_ok_and(|v| v == *want)
}

/// A string scalar for an emitter with its own quoting rule: quoted when
/// `ts_quotes` (the emitter's rule, where TS quoted it), and also whenever the
/// plain text would not read back as the same string under the runtime
/// parser (`yes`, `-1`, `~`, `*/*`, `@x`, `a: b`, ...). `flow` is for an item
/// of an inline `[a, b]` list, where `,` `[` `]` `{` `}` also matter.
pub fn scalar_text(s: &str, ts_quotes: bool, flow: bool) -> String {
    let plain_ok = || {
        if flow {
            reads_back(&format!("v: [{s}]"), &serde_json::json!({ "v": [s] }))
        } else {
            reads_back(&format!("v: {s}"), &serde_json::json!({ "v": s }))
        }
    };
    if ts_quotes || !plain_ok() {
        quote_escaped(s)
    } else {
        s.to_string()
    }
}

/// A mapping key: bare unless it would not read back as itself (`yes`,
/// `010`, `a: b`, `#x`, empty, ...).
pub fn map_key_text(k: &str) -> String {
    let mut want = serde_json::Map::new();
    want.insert(k.to_string(), Json::from(1));
    if !k.is_empty() && reads_back(&format!("{k}: 1"), &Json::Object(want)) {
        k.to_string()
    } else {
        quote_escaped(k)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn quoting() {
        assert!(needs_quoting("a: b") && needs_quoting("*/*") && needs_quoting("Yes"));
        assert!(needs_quoting("-1") && needs_quoting(" x") && !needs_quoting("plain"));
        assert_eq!(yaml_key("Content-Type"), "Content-Type");
        assert_eq!(yaml_key("a b"), "\"a b\"");
        assert_eq!(
            quote_escaped("a\"b\\\n\u{85}\u{2028}é"),
            "\"a\\\"b\\\\\\n\\u0085\\u2028é\""
        );
    }

    #[test]
    fn scalar_text_quotes_what_would_not_read_back() {
        // plain when it reads back, in block and flow position
        assert_eq!(scalar_text("hello world", false, false), "hello world");
        assert_eq!(scalar_text("hello world", false, true), "hello world");
        // the emitter's own rule wins
        assert_eq!(scalar_text("a: b", true, false), "\"a: b\"");
        // not quoted by the rule, but would not read back
        for s in ["yes", "~", "-1", "*/*", "@x", "", "- x", "a: b", "010"] {
            assert_eq!(scalar_text(s, false, false), quote_escaped(s), "{s:?}");
        }
        // only a problem inside `[...]`
        assert_eq!(scalar_text("a,b", false, false), "a,b");
        assert_eq!(scalar_text("a,b", false, true), "\"a,b\"");
    }

    #[test]
    fn map_key_text_is_bare_when_it_reads_back() {
        assert_eq!(map_key_text("Content-Type"), "Content-Type");
        assert_eq!(map_key_text("my key"), "my key");
        for k in ["", "yes", "010", "a: b", "#x", "- x"] {
            assert_eq!(map_key_text(k), quote_escaped(k), "{k:?}");
        }
    }
}
