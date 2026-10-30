//! JavaScript value semantics needed to match the TS validators, condition
//! evaluator and response store exactly (number formatting, `Number()`,
//! `String()`, loose equality, `JSON.stringify`, RegExp).

use crate::model::Json;
use std::collections::HashMap;
use std::sync::{Arc, LazyLock, Mutex};

/// Max string length (UTF-16 units) passed to a regex, as in TS (ReDoS guard).
pub const MAX_REGEX_INPUT_LENGTH: usize = 100_000;

/// JS `Number.prototype.toString()`.
pub fn num_to_string(x: f64) -> String {
    if x.is_nan() {
        return "NaN".into();
    }
    if x == 0.0 {
        return "0".into();
    }
    if x.is_infinite() {
        return if x > 0.0 { "Infinity" } else { "-Infinity" }.into();
    }
    let sign = if x < 0.0 { "-" } else { "" };
    let sci = format!("{:e}", x.abs());
    let (mant, exp) = sci.split_once('e').unwrap_or((&sci, "0"));
    let digits: String = mant.chars().filter(|c| *c != '.').collect();
    let e: i32 = exp.parse().unwrap_or(0);
    let k = digits.len() as i32;
    let n = e + 1;
    let body = if k <= n && n <= 21 {
        format!("{digits}{}", "0".repeat((n - k) as usize))
    } else if 0 < n && n <= 21 {
        format!("{}.{}", &digits[..n as usize], &digits[n as usize..])
    } else if -6 < n && n <= 0 {
        format!("0.{}{digits}", "0".repeat((-n) as usize))
    } else {
        let e1 = n - 1;
        let es = if e1 < 0 { "-" } else { "+" };
        if k == 1 {
            format!("{digits}e{es}{}", e1.abs())
        } else {
            format!("{}.{}e{es}{}", &digits[..1], &digits[1..], e1.abs())
        }
    };
    format!("{sign}{body}")
}

/// JS whitespace + line terminators (what `trim()` and `\s` cover).
pub fn is_js_space(c: char) -> bool {
    matches!(
        c,
        '\t' | '\n' | '\u{b}' | '\u{c}' | '\r' | ' ' | '\u{a0}' | '\u{1680}' | '\u{2000}'
            ..='\u{200a}'
                | '\u{2028}'
                | '\u{2029}'
                | '\u{202f}'
                | '\u{205f}'
                | '\u{3000}'
                | '\u{feff}'
    )
}

/// JS `String.prototype.trim`.
pub fn trim(s: &str) -> &str {
    s.trim_matches(is_js_space)
}

/// JS `parseInt(s, 10)` (NaN when no digits follow the optional sign).
pub fn parse_int(s: &str) -> f64 {
    let t = s.trim_start_matches(is_js_space);
    let (neg, rest) = match t.as_bytes().first() {
        Some(b'-') => (true, &t[1..]),
        Some(b'+') => (false, &t[1..]),
        _ => (false, t),
    };
    let end = rest
        .bytes()
        .position(|b| !b.is_ascii_digit())
        .unwrap_or(rest.len());
    if end == 0 {
        return f64::NAN;
    }
    let n: f64 = rest[..end].parse().unwrap_or(f64::NAN);
    if neg { -n } else { n }
}

/// JS `Number(string)`.
pub fn str_to_number(s: &str) -> f64 {
    let t = trim(s);
    if t.is_empty() {
        return 0.0;
    }
    match t {
        "Infinity" | "+Infinity" => return f64::INFINITY,
        "-Infinity" => return f64::NEG_INFINITY,
        _ => {}
    }
    for (prefix, radix) in [
        ("0x", 16),
        ("0X", 16),
        ("0o", 8),
        ("0O", 8),
        ("0b", 2),
        ("0B", 2),
    ] {
        if let Some(rest) = t.strip_prefix(prefix) {
            if rest.is_empty() || !rest.chars().all(|c| c.is_digit(radix)) {
                return f64::NAN;
            }
            return rest.chars().fold(0.0, |acc, c| {
                acc * radix as f64 + c.to_digit(radix).unwrap_or(0) as f64
            });
        }
    }
    // Decimal literal only: Rust's parser also accepts "inf"/"nan".
    let b = t.strip_prefix(['+', '-']).unwrap_or(t).as_bytes();
    let int_end = b
        .iter()
        .position(|c| !c.is_ascii_digit())
        .unwrap_or(b.len());
    let mut i = int_end;
    let mut mantissa = int_end;
    if b.get(i) == Some(&b'.') {
        i += 1;
        let frac_start = i;
        while i < b.len() && b[i].is_ascii_digit() {
            i += 1;
        }
        mantissa += i - frac_start;
    }
    if mantissa == 0 {
        return f64::NAN;
    }
    if matches!(b.get(i), Some(b'e' | b'E')) {
        i += 1;
        if matches!(b.get(i), Some(b'+' | b'-')) {
            i += 1;
        }
        let es = i;
        while i < b.len() && b[i].is_ascii_digit() {
            i += 1;
        }
        if i == es {
            return f64::NAN;
        }
    }
    if i != b.len() {
        return f64::NAN;
    }
    t.parse().unwrap_or(f64::NAN)
}

/// A JS value as the TS code sees it (`undefined` kept distinct from `null`).
#[derive(Debug, Clone, PartialEq)]
pub enum JsVal {
    Undefined,
    Null,
    Bool(bool),
    Num(f64),
    Str(String),
    /// Array or object.
    Obj(Json),
}

impl JsVal {
    pub fn from_json(v: &Json) -> JsVal {
        match v {
            Json::Null => JsVal::Null,
            Json::Bool(b) => JsVal::Bool(*b),
            Json::Number(n) => JsVal::Num(n.as_f64().unwrap_or(f64::NAN)),
            Json::String(s) => JsVal::Str(s.clone()),
            other => JsVal::Obj(other.clone()),
        }
    }

    pub fn from_opt(v: Option<&Json>) -> JsVal {
        v.map_or(JsVal::Undefined, JsVal::from_json)
    }

    /// JS `Number(x)`.
    pub fn to_number(&self) -> f64 {
        match self {
            JsVal::Undefined => f64::NAN,
            JsVal::Null => 0.0,
            JsVal::Bool(b) => f64::from(u8::from(*b)),
            JsVal::Num(n) => *n,
            JsVal::Str(s) => str_to_number(s),
            JsVal::Obj(_) => str_to_number(&self.to_js_string()),
        }
    }

    /// JS `String(x)`.
    pub fn to_js_string(&self) -> String {
        match self {
            JsVal::Undefined => "undefined".into(),
            JsVal::Null => "null".into(),
            JsVal::Bool(b) => b.to_string(),
            JsVal::Num(n) => num_to_string(*n),
            JsVal::Str(s) => s.clone(),
            JsVal::Obj(Json::Array(a)) => a
                .iter()
                .map(|v| match v {
                    Json::Null => String::new(),
                    v => JsVal::from_json(v).to_js_string(),
                })
                .collect::<Vec<_>>()
                .join(","),
            JsVal::Obj(_) => "[object Object]".into(),
        }
    }

    /// JS abstract equality `==` (objects are never identical here).
    pub fn loose_eq(&self, other: &JsVal) -> bool {
        use JsVal::*;
        match (self, other) {
            (Undefined | Null, Undefined | Null) => true,
            (Undefined | Null, _) | (_, Undefined | Null) => false,
            (Bool(b), o) => Num(f64::from(u8::from(*b))).loose_eq(o),
            (s, Bool(b)) => s.loose_eq(&Num(f64::from(u8::from(*b)))),
            (Obj(_), Obj(_)) => false,
            (Obj(_), o) => Str(self.to_js_string()).loose_eq(o),
            (s, Obj(_)) => s.loose_eq(&Str(other.to_js_string())),
            (Num(a), Num(b)) => a == b,
            (Str(a), Str(b)) => a == b,
            (Num(a), Str(b)) => *a == str_to_number(b),
            (Str(a), Num(b)) => str_to_number(a) == *b,
        }
    }
}

/// JS `JSON.stringify` of a defined value (whole floats print as integers).
pub fn stringify(v: &Json) -> String {
    match v {
        Json::Null => "null".into(),
        Json::Bool(b) => b.to_string(),
        Json::Number(n) => num_to_string(n.as_f64().unwrap_or(f64::NAN)),
        Json::String(s) => serde_json::to_string(s).unwrap_or_default(),
        Json::Array(a) => format!(
            "[{}]",
            a.iter().map(stringify).collect::<Vec<_>>().join(",")
        ),
        Json::Object(o) => format!(
            "{{{}}}",
            o.iter()
                .map(|(k, v)| format!(
                    "{}:{}",
                    serde_json::to_string(k).unwrap_or_default(),
                    stringify(v)
                ))
                .collect::<Vec<_>>()
                .join(",")
        ),
    }
}

/// Template-literal rendering of `JSON.stringify(x)`: `undefined` when absent.
pub fn stringify_opt(v: Option<&Json>) -> String {
    v.map_or_else(|| "undefined".to_string(), stringify)
}

/// `s.slice(0, MAX_REGEX_INPUT_LENGTH)` by UTF-16 units.
pub fn cap_regex_input(s: &str) -> &str {
    if s.len() <= MAX_REGEX_INPUT_LENGTH {
        return s;
    }
    let mut units = 0;
    for (i, c) in s.char_indices() {
        units += c.len_utf16();
        if units > MAX_REGEX_INPUT_LENGTH {
            return &s[..i];
        }
    }
    s
}

const WORD: &str = "A-Za-z0-9_";
const SPACE: &str = r"\t\n\x0B\x0C\r \x{a0}\x{1680}\x{2000}-\x{200a}\x{2028}\x{2029}\x{202f}\x{205f}\x{3000}\x{feff}";

/// Translate JS RegExp syntax into fancy-regex syntax: ASCII `\d \w \b`, JS
/// `\s`, JS `.`, `\0`, literal `[`/`&`/`~` and class-escape-adjacent `-`
/// inside classes.
fn translate(pattern: &str) -> String {
    let mut out = String::with_capacity(pattern.len() + 16);
    let mut in_class = false;
    // last emitted item in a class was an expanded class escape (\d \w \s ...)
    let mut prev_escape = false;
    let chars: Vec<char> = pattern.chars().collect();
    let mut i = 0;
    while i < chars.len() {
        let c = chars[i];
        i += 1;
        let mut escape = false;
        match c {
            '\\' => {
                let Some(&n) = chars.get(i) else {
                    out.push('\\');
                    break;
                };
                i += 1;
                let cls = |neg: bool, body: &str, out: &mut String| {
                    if in_class && !neg {
                        out.push_str(body);
                    } else {
                        out.push_str(if neg { "[^" } else { "[" });
                        out.push_str(body);
                        out.push(']');
                    }
                };
                match n {
                    'd' => cls(false, "0-9", &mut out),
                    'D' => cls(true, "0-9", &mut out),
                    'w' => cls(false, WORD, &mut out),
                    'W' => cls(true, WORD, &mut out),
                    's' => cls(false, SPACE, &mut out),
                    'S' => cls(true, SPACE, &mut out),
                    'b' if in_class => out.push_str(r"\x08"),
                    'b' => out.push_str(&format!(
                        "(?:(?<=[{WORD}])(?![{WORD}])|(?<![{WORD}])(?=[{WORD}]))"
                    )),
                    'B' if !in_class => out.push_str(&format!(
                        "(?:(?<=[{WORD}])(?=[{WORD}])|(?<![{WORD}])(?![{WORD}]))"
                    )),
                    '0' => out.push_str(r"\x00"),
                    other => {
                        out.push('\\');
                        out.push(other);
                    }
                }
                escape = matches!(n, 'd' | 'D' | 'w' | 'W' | 's' | 'S');
            }
            '-' if in_class => {
                let next_escape = chars.get(i) == Some(&'\\')
                    && matches!(chars.get(i + 1), Some('d' | 'D' | 'w' | 'W' | 's' | 'S'));
                if prev_escape || next_escape {
                    out.push_str(r"\-");
                } else {
                    out.push('-');
                }
            }
            '[' if in_class => out.push_str(r"\["),
            '[' => {
                in_class = true;
                out.push('[');
            }
            ']' if in_class => {
                in_class = false;
                out.push(']');
            }
            '&' | '~' if in_class => {
                out.push('\\');
                out.push(c);
            }
            '.' if !in_class => out.push_str(r"[^\n\r\x{2028}\x{2029}]"),
            other => out.push(other),
        }
        prev_escape = in_class && escape;
    }
    out
}

fn compile_uncached(pattern: &str, case_insensitive: bool) -> Option<fancy_regex::Regex> {
    let mut src = String::new();
    if case_insensitive {
        src.push_str("(?i)");
    }
    src.push_str(&translate(pattern));
    fancy_regex::Regex::new(&src).ok()
}

type RegexCache = HashMap<(String, bool), Option<Arc<fancy_regex::Regex>>>;

/// Bounded compiled-regex cache (V8 caches too); `None` entries cache invalid patterns.
const REGEX_CACHE_MAX: usize = 256;
static REGEX_CACHE: LazyLock<Mutex<RegexCache>> = LazyLock::new(|| Mutex::new(HashMap::new()));

/// Compile a JS RegExp pattern (`i` flag optional), cached. `None` when invalid.
pub fn compile_regex(pattern: &str, case_insensitive: bool) -> Option<Arc<fancy_regex::Regex>> {
    let key = (pattern.to_string(), case_insensitive);
    if let Ok(cache) = REGEX_CACHE.lock()
        && let Some(hit) = cache.get(&key)
    {
        return hit.clone();
    }
    let compiled = compile_uncached(pattern, case_insensitive).map(Arc::new);
    if let Ok(mut cache) = REGEX_CACHE.lock() {
        if cache.len() >= REGEX_CACHE_MAX {
            cache.clear();
        }
        cache.insert(key, compiled.clone());
    }
    compiled
}

/// `new RegExp(pattern, flags).test(input)` with the ReDoS input cap.
/// `None` when the pattern is invalid.
pub fn regex_test(pattern: &str, input: &str, case_insensitive: bool) -> Option<bool> {
    let re = compile_regex(pattern, case_insensitive)?;
    // Runtime errors (backtrack limit) count as no match.
    Some(re.is_match(cap_regex_input(input)).unwrap_or(false))
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_int_like_js() {
        assert_eq!(parse_int("5000"), 5000.0);
        assert_eq!(parse_int("  42ms"), 42.0);
        assert_eq!(parse_int("-7.9"), -7.0);
        assert_eq!(parse_int("+3"), 3.0);
        assert!(parse_int("abc").is_nan());
        assert!(parse_int("-").is_nan());
        assert!(parse_int("").is_nan());
        assert_eq!(
            num_to_string(parse_int("1".repeat(22).as_str())),
            "1.1111111111111111e+21"
        );
    }

    #[test]
    fn number_formatting() {
        assert_eq!(num_to_string(1.0), "1");
        assert_eq!(num_to_string(45.67), "45.67");
        assert_eq!(num_to_string(-0.0), "0");
        assert_eq!(num_to_string(1e21), "1e+21");
        assert_eq!(num_to_string(1.5e-7), "1.5e-7");
        assert_eq!(num_to_string(0.000001), "0.000001");
        assert_eq!(
            num_to_string(123456789012345680000.0),
            "123456789012345680000"
        );
        assert_eq!(num_to_string(f64::INFINITY), "Infinity");
        assert_eq!(num_to_string(0.1 + 0.2), "0.30000000000000004");
    }

    #[test]
    fn string_to_number() {
        assert_eq!(str_to_number(""), 0.0);
        assert_eq!(str_to_number("  "), 0.0);
        assert_eq!(str_to_number(" 42 "), 42.0);
        assert_eq!(str_to_number("0x10"), 16.0);
        assert_eq!(str_to_number("1e3"), 1000.0);
        assert_eq!(str_to_number(".5"), 0.5);
        assert_eq!(str_to_number("5."), 5.0);
        assert!(str_to_number("inf").is_nan());
        assert!(str_to_number("nan").is_nan());
        assert!(str_to_number("1.2.3").is_nan());
        assert!(str_to_number("-").is_nan());
        assert!(str_to_number("abc").is_nan());
        assert_eq!(str_to_number("-Infinity"), f64::NEG_INFINITY);
    }

    #[test]
    fn loose_equality() {
        use JsVal::*;
        assert!(Num(200.0).loose_eq(&Str("200".into())));
        assert!(Bool(true).loose_eq(&Num(1.0)));
        assert!(Bool(true).loose_eq(&Str("1".into())));
        assert!(!Bool(true).loose_eq(&Str("true".into())));
        assert!(Null.loose_eq(&Undefined));
        assert!(!Null.loose_eq(&Num(0.0)));
        assert!(Str("".into()).loose_eq(&Num(0.0)));
        assert!(Obj(serde_json::json!([5])).loose_eq(&Num(5.0)));
    }

    #[test]
    fn regex_translation() {
        assert_eq!(regex_test(r"^\d+$", "123", false), Some(true));
        // JS \d is ASCII only
        assert_eq!(regex_test(r"^\d+$", "\u{0663}", false), Some(false));
        assert_eq!(regex_test(r"^(?!admin).*$", "user", false), Some(true));
        assert_eq!(regex_test(r"^(?!admin).*$", "admin", false), Some(false));
        assert_eq!(regex_test(r"(a)\1", "aa", false), Some(true));
        assert_eq!(regex_test(r"\bfoo\b", "a foo b", false), Some(true));
        assert_eq!(regex_test(r"\bfoo\b", "afoo", false), Some(false));
        assert_eq!(regex_test(r"^[\d.]+$", "1.5", false), Some(true));
        assert_eq!(regex_test(r"^[^\D]+$", "15", false), Some(true));
        assert_eq!(regex_test(r"^[\w-]+$", "ab-c", false), Some(true));
        assert_eq!(regex_test(r"^[[]x$", "[x", false), Some(true));
        assert_eq!(regex_test("[invalid", "x", false), None);
        assert_eq!(regex_test("^JOHN$", "john", true), Some(true));
    }

    #[test]
    fn regex_class_escape_hyphen_is_literal() {
        assert_eq!(regex_test(r"^[\w-.]+$", "a-b.c", false), Some(true));
        assert_eq!(regex_test(r"^[\w-\.]+$", "a-b.c", false), Some(true));
        assert_eq!(regex_test(r"^[\s-x]+$", " -x", false), Some(true));
        assert_eq!(regex_test(r"^[a-\d]+$", "a-5", false), Some(true));
        assert_eq!(regex_test(r"^[.-\w]+$", "a-.", false), Some(true));
        assert_eq!(
            regex_test(r"^[\w-\.]+@([\w-]+\.)+[\w-]{2,4}$", "a.b@x.com", false),
            Some(true)
        );
        // real ranges still work
        assert_eq!(regex_test(r"^[a-c]+$", "abc", false), Some(true));
        assert_eq!(regex_test(r"^[a-c]+$", "d", false), Some(false));
    }

    #[test]
    fn regex_dot_and_nul() {
        assert_eq!(regex_test("^a.b$", "a\rb", false), Some(false));
        assert_eq!(regex_test("^a.b$", "a\nb", false), Some(false));
        assert_eq!(regex_test("^a.b$", "a\u{2028}b", false), Some(false));
        assert_eq!(regex_test("^a.b$", "axb", false), Some(true));
        assert_eq!(regex_test("^[.]$", ".", false), Some(true));
        assert_eq!(regex_test(r"^a\0b$", "a\0b", false), Some(true));
    }

    #[test]
    fn regex_cache_is_consistent() {
        for _ in 0..3 {
            assert_eq!(regex_test("^x+$", "xx", false), Some(true));
            assert_eq!(regex_test("[bad", "x", false), None);
            assert_eq!(regex_test("^X$", "x", true), Some(true));
            assert_eq!(regex_test("^X$", "x", false), Some(false));
        }
    }

    #[test]
    fn truthiness() {
        assert!(!truthy(None));
        assert!(!truthy(Some(&serde_json::json!(null))));
        assert!(!truthy(Some(&serde_json::json!(""))));
        assert!(!truthy(Some(&serde_json::json!(0))));
        assert!(!truthy(Some(&serde_json::json!(false))));
        assert!(truthy(Some(&serde_json::json!({}))));
        assert!(truthy(Some(&serde_json::json!("x"))));
    }
}
