//! JavaScript behaviors the converter depends on: property enumeration order,
//! `localeCompare`, `encodeURIComponent`/`decodeURIComponent` and `atob`.

use indexmap::IndexMap;
use std::cmp::Ordering;

/// A JS `Record<string, V>`: assignment keeps the first position of a key and
/// iteration follows JS enumeration order.
#[derive(Debug, Clone, PartialEq)]
pub struct JsMap<V>(IndexMap<String, V>);

impl<V> Default for JsMap<V> {
    fn default() -> Self {
        JsMap(IndexMap::new())
    }
}

impl<V> JsMap<V> {
    pub fn new() -> Self {
        Self::default()
    }

    /// `record[k] = v`. Assigning `__proto__` sets the prototype instead of an
    /// own property (and ignores a non-object), so the key never shows up.
    pub fn insert(&mut self, k: impl Into<String>, v: V) {
        let k = k.into();
        if k != "__proto__" {
            self.0.insert(k, v);
        }
    }

    /// `{...source}`: copies `__proto__` as an own property.
    pub fn insert_own(&mut self, k: impl Into<String>, v: V) {
        self.0.insert(k.into(), v);
    }

    pub fn get(&self, k: &str) -> Option<&V> {
        self.0.get(k)
    }

    pub fn contains_key(&self, k: &str) -> bool {
        self.0.contains_key(k)
    }

    pub fn len(&self) -> usize {
        self.0.len()
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// `Object.entries` order.
    pub fn entries(&self) -> Vec<(&String, &V)> {
        crate::snapshot::json::js_order(self.0.iter())
    }
}

const PUNCT_ORDER: &str = "\t\n\u{b}\u{c}\r _-,;:!?.'\"()[]{}@*/\\&#%`^+<=>|~$0123456789";

/// Latin letters with one accent: (letter, base, accent weight). The accent
/// only matters once the base letters tie (ICU secondary level): acute, grave,
/// breve, circumflex, caron, cedilla, ring, diaeresis, double acute, tilde,
/// dot above, ogonek, macron.
const ACCENTED: &[(char, char, u8)] = &[
    ('À', 'A', 4),
    ('Á', 'A', 2),
    ('Â', 'A', 8),
    ('Ã', 'A', 18),
    ('Ä', 'A', 14),
    ('Å', 'A', 12),
    ('Ç', 'C', 11),
    ('È', 'E', 4),
    ('É', 'E', 2),
    ('Ê', 'E', 8),
    ('Ë', 'E', 14),
    ('Ì', 'I', 4),
    ('Í', 'I', 2),
    ('Î', 'I', 8),
    ('Ï', 'I', 14),
    ('Ñ', 'N', 18),
    ('Ò', 'O', 4),
    ('Ó', 'O', 2),
    ('Ô', 'O', 8),
    ('Õ', 'O', 18),
    ('Ö', 'O', 14),
    ('Ù', 'U', 4),
    ('Ú', 'U', 2),
    ('Û', 'U', 8),
    ('Ü', 'U', 14),
    ('Ý', 'Y', 2),
    ('à', 'a', 4),
    ('á', 'a', 2),
    ('â', 'a', 8),
    ('ã', 'a', 18),
    ('ä', 'a', 14),
    ('å', 'a', 12),
    ('ç', 'c', 11),
    ('è', 'e', 4),
    ('é', 'e', 2),
    ('ê', 'e', 8),
    ('ë', 'e', 14),
    ('ì', 'i', 4),
    ('í', 'i', 2),
    ('î', 'i', 8),
    ('ï', 'i', 14),
    ('ñ', 'n', 18),
    ('ò', 'o', 4),
    ('ó', 'o', 2),
    ('ô', 'o', 8),
    ('õ', 'o', 18),
    ('ö', 'o', 14),
    ('ù', 'u', 4),
    ('ú', 'u', 2),
    ('û', 'u', 8),
    ('ü', 'u', 14),
    ('ý', 'y', 2),
    ('ÿ', 'y', 14),
    ('Ā', 'A', 24),
    ('ā', 'a', 24),
    ('Ă', 'A', 6),
    ('ă', 'a', 6),
    ('Ą', 'A', 22),
    ('ą', 'a', 22),
    ('Ć', 'C', 2),
    ('ć', 'c', 2),
    ('Ĉ', 'C', 8),
    ('ĉ', 'c', 8),
    ('Ċ', 'C', 20),
    ('ċ', 'c', 20),
    ('Č', 'C', 10),
    ('č', 'c', 10),
    ('Ď', 'D', 10),
    ('ď', 'd', 10),
    ('Ē', 'E', 24),
    ('ē', 'e', 24),
    ('Ĕ', 'E', 6),
    ('ĕ', 'e', 6),
    ('Ė', 'E', 20),
    ('ė', 'e', 20),
    ('Ę', 'E', 22),
    ('ę', 'e', 22),
    ('Ě', 'E', 10),
    ('ě', 'e', 10),
    ('Ĝ', 'G', 8),
    ('ĝ', 'g', 8),
    ('Ğ', 'G', 6),
    ('ğ', 'g', 6),
    ('Ġ', 'G', 20),
    ('ġ', 'g', 20),
    ('Ģ', 'G', 11),
    ('ģ', 'g', 11),
    ('Ĥ', 'H', 8),
    ('ĥ', 'h', 8),
    ('Ĩ', 'I', 18),
    ('ĩ', 'i', 18),
    ('Ī', 'I', 24),
    ('ī', 'i', 24),
    ('Ĭ', 'I', 6),
    ('ĭ', 'i', 6),
    ('Į', 'I', 22),
    ('į', 'i', 22),
    ('İ', 'I', 20),
    ('Ĵ', 'J', 8),
    ('ĵ', 'j', 8),
    ('Ķ', 'K', 11),
    ('ķ', 'k', 11),
    ('Ĺ', 'L', 2),
    ('ĺ', 'l', 2),
    ('Ļ', 'L', 11),
    ('ļ', 'l', 11),
    ('Ľ', 'L', 10),
    ('ľ', 'l', 10),
    ('Ń', 'N', 2),
    ('ń', 'n', 2),
    ('Ņ', 'N', 11),
    ('ņ', 'n', 11),
    ('Ň', 'N', 10),
    ('ň', 'n', 10),
    ('Ō', 'O', 24),
    ('ō', 'o', 24),
    ('Ŏ', 'O', 6),
    ('ŏ', 'o', 6),
    ('Ő', 'O', 16),
    ('ő', 'o', 16),
    ('Ŕ', 'R', 2),
    ('ŕ', 'r', 2),
    ('Ŗ', 'R', 11),
    ('ŗ', 'r', 11),
    ('Ř', 'R', 10),
    ('ř', 'r', 10),
    ('Ś', 'S', 2),
    ('ś', 's', 2),
    ('Ŝ', 'S', 8),
    ('ŝ', 's', 8),
    ('Ş', 'S', 11),
    ('ş', 's', 11),
    ('Š', 'S', 10),
    ('š', 's', 10),
    ('Ţ', 'T', 11),
    ('ţ', 't', 11),
    ('Ť', 'T', 10),
    ('ť', 't', 10),
    ('Ũ', 'U', 18),
    ('ũ', 'u', 18),
    ('Ū', 'U', 24),
    ('ū', 'u', 24),
    ('Ŭ', 'U', 6),
    ('ŭ', 'u', 6),
    ('Ů', 'U', 12),
    ('ů', 'u', 12),
    ('Ű', 'U', 16),
    ('ű', 'u', 16),
    ('Ų', 'U', 22),
    ('ų', 'u', 22),
    ('Ŵ', 'W', 8),
    ('ŵ', 'w', 8),
    ('Ŷ', 'Y', 8),
    ('ŷ', 'y', 8),
    ('Ÿ', 'Y', 14),
    ('Ź', 'Z', 2),
    ('ź', 'z', 2),
    ('Ż', 'Z', 20),
    ('ż', 'z', 20),
    ('Ž', 'Z', 10),
    ('ž', 'z', 10),
    ('Ǎ', 'A', 10),
    ('ǎ', 'a', 10),
    ('Ǐ', 'I', 10),
    ('ǐ', 'i', 10),
    ('Ǒ', 'O', 10),
    ('ǒ', 'o', 10),
    ('Ǔ', 'U', 10),
    ('ǔ', 'u', 10),
];

fn accent(c: char) -> Option<(char, u8)> {
    ACCENTED
        .binary_search_by_key(&c, |e| e.0)
        .ok()
        .map(|i| (ACCENTED[i].1, ACCENTED[i].2))
}

/// Primary collation weight (ICU root collation for ASCII); `None` for
/// completely ignorable control characters.
fn primary(c: char) -> Option<u32> {
    if c.is_ascii_alphabetic() {
        return Some(100 + u32::from(c.to_ascii_lowercase()) - u32::from(b'a'));
    }
    if let Some(p) = PUNCT_ORDER.find(c) {
        return Some(p as u32);
    }
    if c.is_control() {
        return None;
    }
    Some(1000 + u32::from(c))
}

/// `a.localeCompare(b)` in the default (en-US, ICU) locale: punctuation, then
/// digits, then letters compared case-insensitively (accents break ties, then
/// lowercase before uppercase). Exact for ASCII and single-accent Latin
/// letters; other characters sort by code point after the letters.
pub fn locale_compare(a: &str, b: &str) -> Ordering {
    let keys = |s: &str| -> (Vec<u32>, Vec<u8>, Vec<bool>) {
        let (mut p, mut a2, mut t) = (Vec::new(), Vec::new(), Vec::new());
        for c in s.chars() {
            let (base, acc) = accent(c).unwrap_or((c, 0));
            if let Some(w) = primary(base) {
                p.push(w);
                a2.push(acc);
                t.push(base.is_uppercase());
            }
        }
        (p, a2, t)
    };
    let (pa, sa, ta) = keys(a);
    let (pb, sb, tb) = keys(b);
    pa.cmp(&pb)
        .then_with(|| sa.cmp(&sb))
        .then_with(|| ta.cmp(&tb))
}

/// `encodeURIComponent`.
pub fn encode_uri_component(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for b in s.bytes() {
        if b.is_ascii_alphanumeric() || b"-_.!~*'()".contains(&b) {
            out.push(char::from(b));
        } else {
            out.push_str(&format!("%{b:02X}"));
        }
    }
    out
}

/// `decodeURIComponent` failed (JS `URIError`; Bun's message is `URI error`).
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("URI error")]
pub struct UriError;

fn hex_byte(b: &[u8], at: usize) -> Result<u8, UriError> {
    if b.get(at) != Some(&b'%') {
        return Err(UriError);
    }
    let h = b.get(at + 1..at + 3).ok_or(UriError)?;
    let h = std::str::from_utf8(h).map_err(|_| UriError)?;
    if !h.bytes().all(|c| c.is_ascii_hexdigit()) {
        return Err(UriError);
    }
    u8::from_str_radix(h, 16).map_err(|_| UriError)
}

/// `decodeURIComponent`.
pub fn decode_uri_component(s: &str) -> Result<String, UriError> {
    let b = s.as_bytes();
    let mut out: Vec<u8> = Vec::with_capacity(b.len());
    let mut i = 0;
    while i < b.len() {
        if b[i] != b'%' {
            out.push(b[i]);
            i += 1;
            continue;
        }
        let first = hex_byte(b, i)?;
        i += 3;
        if first < 0x80 {
            out.push(first);
            continue;
        }
        let n = first.leading_ones() as usize;
        if n == 1 || n > 4 {
            return Err(UriError);
        }
        let mut seq = vec![first];
        for _ in 1..n {
            let c = hex_byte(b, i)?;
            if c & 0xC0 != 0x80 {
                return Err(UriError);
            }
            seq.push(c);
            i += 3;
        }
        std::str::from_utf8(&seq).map_err(|_| UriError)?;
        out.extend_from_slice(&seq);
    }
    String::from_utf8(out).map_err(|_| UriError)
}

/// `atob` (forgiving base64): the decoded bytes as a Latin-1 string.
pub fn atob(input: &str) -> Option<String> {
    let mut s: Vec<u8> = Vec::new();
    for c in input.chars() {
        match c {
            '\t' | '\n' | '\u{c}' | '\r' | ' ' => {}
            c if c.is_ascii() => s.push(c as u8),
            _ => return None,
        }
    }
    if s.len().is_multiple_of(4) {
        for _ in 0..2 {
            if s.last() == Some(&b'=') {
                s.pop();
            }
        }
    }
    if s.len() % 4 == 1 {
        return None;
    }
    let mut out = String::new();
    let (mut acc, mut bits) = (0u32, 0u32);
    for &c in &s {
        let v = match c {
            b'A'..=b'Z' => c - b'A',
            b'a'..=b'z' => c - b'a' + 26,
            b'0'..=b'9' => c - b'0' + 52,
            b'+' => 62,
            b'/' => 63,
            _ => return None,
        };
        acc = (acc << 6) | u32::from(v);
        bits += 6;
        if bits >= 8 {
            bits -= 8;
            out.push(char::from(((acc >> bits) & 0xFF) as u8));
        }
    }
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn collation_matches_icu_for_ascii() {
        let mut v = vec![
            "z", "Z", "e", "E", "ab-c", "a-b", "A-b", "a b", "a_b", "a1", "ab",
        ];
        v.sort_by(|a, b| locale_compare(a, b));
        assert_eq!(
            v,
            [
                "a b", "a_b", "a-b", "A-b", "a1", "ab", "ab-c", "e", "E", "z", "Z"
            ]
        );
    }

    #[test]
    fn index_keys_come_first() {
        let m: JsMap<u8> = {
            let mut m = JsMap::new();
            m.insert("b", 1);
            m.insert("10", 2);
            m.insert("2", 3);
            m
        };
        let k: Vec<_> = m.entries().into_iter().map(|(k, _)| k.as_str()).collect();
        assert_eq!(k, ["2", "10", "b"]);
    }

    #[test]
    fn uri_round_trip_and_errors() {
        assert_eq!(encode_uri_component("a b&é"), "a%20b%26%C3%A9");
        assert_eq!(decode_uri_component("a%20b%C3%A9+").as_deref(), Ok("a bé+"));
        assert!(decode_uri_component("%E0%A4%A").is_err());
        assert!(decode_uri_component("%C0%80").is_err());
        assert!(decode_uri_component("%zz").is_err());
    }

    #[test]
    fn atob_is_forgiving_and_latin1() {
        assert_eq!(atob("dXNlcjpwYXNz").as_deref(), Some("user:pass"));
        assert_eq!(atob("dXNlcjpwYXNz=").as_deref(), None);
        assert_eq!(atob("YTpi").as_deref(), Some("a:b"));
        assert_eq!(atob("YQ").as_deref(), Some("a"));
        assert_eq!(atob("w6k=").as_deref(), Some("\u{c3}\u{a9}"));
        assert_eq!(atob("!!!"), None);
        assert_eq!(atob("A"), None);
    }
}
