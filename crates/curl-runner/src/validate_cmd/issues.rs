//! Issue collection and the helpers every rule group shares (TS
//! `addIssue`, `checkUnknownKeys`, `findClosestMatch`, `validateBooleanField`,
//! `isSensitiveValue`), plus the bits of JS value semantics the rules need.
//!
//! TS validates the untyped `Bun.YAML.parse` result, so the rules here read
//! the raw JSON and reproduce JS property access, `typeof`, truthiness and
//! the `TypeError`s TS throws on unexpected shapes ([`Crash`]).

use curl_runner_core::jsval::JsVal;
use curl_runner_core::model::Json;
use curl_runner_core::snapshot::json::js_entries;

pub use crate::output::format::truthy;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Severity {
    Error,
    Warning,
    Info,
}

/// What a fix's `apply()` returns: a new value, `undefined` (delete the key)
/// or `{ rename, value? }`.
#[derive(Debug, Clone, PartialEq)]
pub enum FixValue {
    Set(Json),
    Delete,
    Rename { to: String, value: Option<Json> },
}

#[derive(Debug, Clone, PartialEq)]
pub struct Fix {
    pub description: String,
    pub value: FixValue,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Issue {
    pub path: String,
    pub severity: Severity,
    pub message: String,
    pub fix: Option<Fix>,
}

/// A `TypeError` the TS validator throws; the CLI prints its message and exits 1.
#[derive(Debug, Clone, PartialEq)]
pub struct Crash(pub String);

pub type Res = Result<(), Crash>;

/// Per-file state (TS `ValidateCommand` fields).
#[derive(Default)]
pub struct Ctx {
    pub issues: Vec<Issue>,
    /// `requestNames` (a JS `Set` of `path:name`).
    pub request_names: Vec<String>,
}

impl Ctx {
    pub fn add(&mut self, path: &str, severity: Severity, message: impl Into<String>) {
        self.add_fix(path, severity, message, None);
    }

    pub fn add_fix(
        &mut self,
        path: &str,
        severity: Severity,
        message: impl Into<String>,
        fix: Option<Fix>,
    ) {
        self.issues.push(Issue {
            path: if path.is_empty() {
                "(root)".into()
            } else {
                path.into()
            },
            severity,
            message: message.into(),
            fix,
        });
    }

    /// `checkUnknownKeys`: `Object.keys(obj)` against `valid`.
    pub fn check_unknown_keys(&mut self, obj: &Json, valid: &[&str], path: &str) -> Res {
        for key in object_keys(obj)? {
            if valid.contains(&key.as_str()) {
                continue;
            }
            match find_closest_match(&key, valid) {
                Some(s) => self.add_fix(
                    &format!("{path}.{key}"),
                    Severity::Warning,
                    format!("Unknown key \"{key}\" (did you mean \"{s}\"?)"),
                    Some(Fix {
                        description: format!("Rename to \"{s}\""),
                        value: FixValue::Rename {
                            to: s.into(),
                            value: None,
                        },
                    }),
                ),
                None => self.add(
                    &format!("{path}.{key}"),
                    Severity::Warning,
                    format!("Unknown key \"{key}\""),
                ),
            }
        }
        Ok(())
    }

    /// `validateBooleanField`.
    pub fn boolean_field(&mut self, obj: &Json, field: &str, path: &str) {
        let Some(v) = prop(obj, field) else { return };
        if v.is_boolean() {
            return;
        }
        let raw = js_string(v);
        let lower = raw.to_lowercase();
        if lower == "true" || lower == "false" {
            self.add_fix(
                &format!("{path}.{field}"),
                Severity::Warning,
                format!("Should be boolean, not string \"{raw}\""),
                Some(Fix {
                    description: format!("Convert to {lower}"),
                    value: FixValue::Set(Json::Bool(lower == "true")),
                }),
            );
        } else {
            self.add(
                &format!("{path}.{field}"),
                Severity::Error,
                "Must be a boolean",
            );
        }
    }

    /// `validateFilePath`.
    pub fn file_path(&mut self, file: &str, path: &str) {
        if !file.contains("${") && file.contains("..") {
            self.add(
                path,
                Severity::Info,
                "Path contains \"..\"; ensure this is intentional",
            );
        }
    }

    /// An enum value check with a closest-match fix (`execution`, `format`,
    /// `prettyLevel`, `auth.type`, `operator`): `Invalid {what} "{v}". Must be: ...`.
    /// Non-string values make TS's `findClosestMatch` throw.
    pub fn invalid_choice(&mut self, v: &Json, valid: &[&str], path: &str, what: &str) -> Res {
        let s = as_str(v).ok_or_else(to_lower_crash)?;
        let suggestion = find_closest_match(s, valid);
        self.add_fix(
            path,
            Severity::Error,
            format!("Invalid {what} \"{s}\". Must be: {}", valid.join(", ")),
            suggestion.map(change_to),
        );
        Ok(())
    }
}

/// `{ description: 'Change to "x"', apply: () => x }`.
pub fn change_to(s: &str) -> Fix {
    Fix {
        description: format!("Change to \"{s}\""),
        value: FixValue::Set(Json::String(s.into())),
    }
}

pub fn to_lower_crash() -> Crash {
    Crash(
        "input.toLowerCase is not a function. (In 'input.toLowerCase()', 'input.toLowerCase' is undefined)"
            .into(),
    )
}

// ------------------------------------------------------------ JS semantics

/// `obj[key]` for a parsed YAML value (own keys; array indices).
pub fn prop<'a>(v: &'a Json, key: &str) -> Option<&'a Json> {
    match v {
        Json::Object(o) => o.get(key),
        Json::Array(a) => array_index(key).and_then(|i| a.get(i)),
        _ => None,
    }
}

/// `key in v` for an object (TS only uses it on objects and arrays).
pub fn has_key(v: &Json, key: &str) -> bool {
    prop(v, key).is_some()
}

pub fn array_index(k: &str) -> Option<usize> {
    let canonical =
        !k.is_empty() && k.bytes().all(|b| b.is_ascii_digit()) && (k == "0" || !k.starts_with('0'));
    canonical
        .then(|| k.parse::<u32>().ok())
        .flatten()
        .filter(|i| *i != u32::MAX)
        .map(|i| i as usize)
}

/// `Object.keys(v)`; `null` throws as in TS.
pub fn object_keys(v: &Json) -> Result<Vec<String>, Crash> {
    Ok(match v {
        Json::Null => {
            return Err(Crash(
                "null is not an object (evaluating 'Object.keys(obj)')".into(),
            ));
        }
        Json::Object(o) => js_entries(o).into_iter().map(|(k, _)| k.clone()).collect(),
        Json::Array(a) => (0..a.len()).map(|i| i.to_string()).collect(),
        Json::String(s) => (0..s.encode_utf16().count())
            .map(|i| i.to_string())
            .collect(),
        _ => Vec::new(),
    })
}

/// `Object.entries(v)` of an object (other shapes have no string-keyed
/// entries the rules could match).
pub fn entries(v: &Json) -> Vec<(&String, &Json)> {
    match v {
        Json::Object(o) => js_entries(o),
        _ => Vec::new(),
    }
}

/// Defined and truthy.
pub fn truthy_prop<'a>(v: &'a Json, key: &str) -> Option<&'a Json> {
    prop(v, key).filter(|x| truthy(Some(x)))
}

pub fn as_str(v: &Json) -> Option<&str> {
    v.as_str()
}

pub fn as_num(v: &Json) -> Option<f64> {
    v.as_f64()
}

/// JS `typeof v === 'object'` (null, arrays and objects).
pub fn is_object(v: &Json) -> bool {
    matches!(v, Json::Null | Json::Array(_) | Json::Object(_))
}

/// `typeof v !== 'object' || Array.isArray(v)` (the TS "Must be an object" test).
pub fn not_plain_object(v: &Json) -> bool {
    !is_object(v) || v.is_array()
}

/// `typeof v !== 'number' || v < min`.
pub fn not_num_at_least(v: &Json, min: f64) -> bool {
    as_num(v).is_none_or(|n| n < min)
}

/// JS `String(v)`.
pub fn js_string(v: &Json) -> String {
    JsVal::from_json(v).to_js_string()
}

/// JS `Number(v)` (for relational comparisons on untyped values).
pub fn js_number(v: &Json) -> f64 {
    JsVal::from_json(v).to_number()
}

// ----------------------------------------------------------- suggestions

/// `findClosestMatch`: case-insensitive exact or prefix match first, else the
/// option with the smallest edit distance below 3.
pub fn find_closest_match<'a>(input: &str, options: &[&'a str]) -> Option<&'a str> {
    let lower_input = input.to_lowercase();
    let mut best = None;
    let mut best_score = 3;
    for &option in options {
        let lower_option = option.to_lowercase();
        if lower_input == lower_option
            || lower_option.starts_with(&lower_input)
            || lower_input.starts_with(&lower_option)
        {
            return Some(option);
        }
        let d = edit_distance(&lower_input, &lower_option);
        if d < best_score {
            best_score = d;
            best = Some(option);
        }
    }
    best
}

/// Levenshtein distance over UTF-16 code units (JS `charAt`).
pub fn edit_distance(a: &str, b: &str) -> usize {
    let a: Vec<u16> = a.encode_utf16().collect();
    let b: Vec<u16> = b.encode_utf16().collect();
    if a.is_empty() {
        return b.len();
    }
    if b.is_empty() {
        return a.len();
    }
    let mut prev: Vec<usize> = (0..=a.len()).collect();
    for i in 1..=b.len() {
        let mut cur = vec![i; a.len() + 1];
        for j in 1..=a.len() {
            cur[j] = if b[i - 1] == a[j - 1] {
                prev[j - 1]
            } else {
                (prev[j - 1] + 1).min(cur[j - 1] + 1).min(prev[j] + 1)
            };
        }
        prev = cur;
    }
    prev[a.len()]
}

// ------------------------------------------------------- sensitive values

const SENSITIVE_KEYS: &[&str] = &[
    "password",
    "passwd",
    "pwd",
    "secret",
    "api_key",
    "api-key",
    "apikey",
    "token",
    "auth",
    "credential",
    "private_key",
    "private-key",
    "privatekey",
];

fn is_token_char(c: u8) -> bool {
    c.is_ascii_alphanumeric() || c == b'-' || c == b'_'
}

/// `/Bearer\s+[A-Za-z0-9\-_]+\.[A-Za-z0-9\-_]+\.[A-Za-z0-9\-_]+/i` (searched).
/// The classes exclude what follows them, so greedy runs never backtrack.
fn has_bearer_jwt(s: &str) -> bool {
    let lower = s.to_ascii_lowercase();
    let mut from = 0;
    while let Some(off) = lower[from..].find("bearer") {
        let start = from + off;
        from = start + 1;
        let rest = &s[start + 6..];
        let after_space = rest.trim_start_matches(curl_runner_core::jsval::is_js_space);
        if after_space.len() == rest.len() {
            continue;
        }
        let b = after_space.as_bytes();
        let mut i = 0;
        let mut ok = true;
        for seg in 0..3 {
            let st = i;
            while i < b.len() && is_token_char(b[i]) {
                i += 1;
            }
            if i == st || (seg < 2 && b.get(i) != Some(&b'.')) {
                ok = false;
                break;
            }
            if seg < 2 {
                i += 1;
            }
        }
        if ok {
            return true;
        }
    }
    false
}

/// One of TS `SENSITIVE_PATTERNS` matches `s`.
fn matches_sensitive(s: &str) -> bool {
    let lower = s.to_ascii_lowercase();
    if SENSITIVE_KEYS.contains(&lower.as_str()) {
        return true;
    }
    if has_bearer_jwt(s) {
        return true;
    }
    if s.len() >= 32 && s.bytes().all(|b| b.is_ascii_hexdigit()) {
        return true;
    }
    let b = lower.as_bytes();
    if b.len() >= 8
        && lower.starts_with("sk")
        && matches!(b[2], b'-' | b'_')
        && &b[3..7] == b"live"
        && matches!(b[7], b'-' | b'_')
    {
        return true;
    }
    s.len() == 20 && lower.starts_with("akia") && s.bytes().all(|b| b.is_ascii_alphanumeric())
}

/// `isSensitiveValue`: the key, then the value, against the patterns.
pub fn is_sensitive_value(key: &str, value: &str) -> bool {
    !value.contains("${") && (matches_sensitive(key) || matches_sensitive(value))
}

/// `/^[a-zA-Z_][a-zA-Z0-9_]*$/`.
pub fn is_identifier(s: &str) -> bool {
    let mut b = s.bytes();
    b.next()
        .is_some_and(|c| c.is_ascii_alphabetic() || c == b'_')
        && b.all(|c| c.is_ascii_alphanumeric() || c == b'_')
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sensitive_patterns() {
        assert!(is_sensitive_value("API_KEY", "x"));
        assert!(is_sensitive_value("Private-Key", "x"));
        assert!(!is_sensitive_value("password", "${env.P}"));
        assert!(is_sensitive_value("h", "Bearer  a.b.c"));
        assert!(is_sensitive_value("h", "xx bearer\ta-1.b_2.c"));
        assert!(!is_sensitive_value("h", "Bearer a.b"));
        assert!(!is_sensitive_value("h", "Bearera.b.c"));
        assert!(is_sensitive_value("h", &"a".repeat(32)));
        assert!(!is_sensitive_value("h", &"g".repeat(32)));
        assert!(is_sensitive_value("h", "SK_live-x"));
        assert!(!is_sensitive_value("h", "sk_livex"));
        assert!(is_sensitive_value("h", "akia1234567890ABCDEF"));
        assert!(!is_sensitive_value("h", "AKIA1234567890ABCDE"));
    }

    #[test]
    fn keys_and_props() {
        let v: Json = serde_json::json!({"b": 1, "2": 2, "a": 3, "10": 4});
        assert_eq!(object_keys(&v).unwrap(), ["2", "10", "b", "a"]);
        assert_eq!(object_keys(&Json::from("ab")).unwrap(), ["0", "1"]);
        assert!(object_keys(&Json::from(5)).unwrap().is_empty());
        assert!(object_keys(&Json::Null).is_err());
        let a: Json = serde_json::json!([1, 2]);
        assert_eq!(prop(&a, "1"), Some(&Json::from(2)));
        assert_eq!(prop(&a, "01"), None);
        assert!(is_identifier("_a1") && !is_identifier("1a") && !is_identifier("a-b"));
    }

    #[test]
    fn edit_distance_utf16() {
        assert_eq!(edit_distance("kitten", "sitting"), 3);
        assert_eq!(edit_distance("", "abc"), 3);
        assert_eq!(edit_distance("😀", "a"), 2);
    }
}
