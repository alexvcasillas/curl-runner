//! YAML documents with `Bun.YAML.parse` scalar resolution.
//!
//! The syntax is parsed by libyaml (the parser behind `serde_yaml_ng`, so the
//! same files are accepted and rejected), but plain scalars are resolved the
//! way Bun 1.3's YAML scanner does (`src/interchange/yaml.zig`,
//! `scanPlainScalar`/`tryResolveNumber`), not by YAML 1.2 core rules:
//! - `yes/no/on/off` (lower, Capitalized, UPPER) are booleans;
//! - numbers are decimal even with leading zeros (`010` -> 10), `0x`/`0o`
//!   prefixes (lower case only) are hex/octal, there are no binary literals,
//!   `_` separators make a string, and a few lenient forms (`1e`, `1-`,
//!   `--1`) parse as numbers;
//! - quoted and block scalars are strings, whatever their tag; on plain
//!   scalars `!!bool`/`!!int`/`!!float`/`!!null` only accept a value of that
//!   kind, every other tag gives a string.
//!
//! Every number is an `f64`, as in JS. Duplicate keys keep the last value;
//! merge keys (`<<`) follow Bun's `MappingProps` (merged entries never
//! override, and are appended last-first).
//!
//! Known difference: Bun ends the document at a plain scalar starting with
//! `---` on an unindented line (`v: ---`); here it is the string `---`.

use serde_yaml_ng::{Mapping, Number, Value};
use std::collections::HashMap;
use std::ffi::CStr;
use std::marker::PhantomData;
use std::mem::MaybeUninit;
use unsafe_libyaml as unsafe_yaml;

/// The node-copy budget for aliases (guards against "billion laughs").
const ALIAS_BUDGET: usize = 1_000_000;

/// Deepest nesting of sequences and mappings (aliases included). Deeper
/// input is a parse error, so nothing that walks a value recursively
/// (conversion, drop, interpolation, the validators, `JSON.stringify`) can
/// overflow a stack, even a 2 MiB thread stack in a debug build (1000 levels
/// did). 128 is `serde_yaml_ng`'s limit, which applied before this loader;
/// Bun allows more (it errors somewhere between 1000 and 5000 levels).
pub(crate) const MAX_DEPTH: usize = 128;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum Tag {
    None,
    /// `!`
    NonSpecific,
    Bool,
    Int,
    Float,
    Null,
    /// `!!str`, unknown and custom tags: always a string.
    Str,
}

impl Tag {
    fn from_libyaml(tag: Option<&[u8]>) -> Tag {
        match tag {
            None => Tag::None,
            Some(b"!") => Tag::NonSpecific,
            Some(b"tag:yaml.org,2002:bool") => Tag::Bool,
            Some(b"tag:yaml.org,2002:int") => Tag::Int,
            Some(b"tag:yaml.org,2002:float") => Tag::Float,
            Some(b"tag:yaml.org,2002:null") => Tag::Null,
            Some(_) => Tag::Str,
        }
    }
}

fn keyword(t: &str) -> Option<Value> {
    Some(match t {
        "null" | "Null" | "NULL" | "~" => Value::Null,
        "true" | "True" | "TRUE" | "yes" | "Yes" | "YES" | "on" | "On" | "ON" => Value::Bool(true),
        "false" | "False" | "FALSE" | "no" | "No" | "NO" | "off" | "Off" | "OFF" => {
            Value::Bool(false)
        }
        _ => return None,
    })
}

/// WTF `parseDouble` (double-conversion with trailing junk allowed): the
/// longest numeric prefix; `None` when there is none.
fn parse_double_prefix(s: &str) -> Option<f64> {
    let b = s.as_bytes();
    let mut i = 0;
    let neg = match b.first() {
        Some(b'-') => {
            i = 1;
            true
        }
        Some(b'+') => {
            i = 1;
            false
        }
        _ => false,
    };
    let int_start = i;
    while b.get(i).is_some_and(u8::is_ascii_digit) {
        i += 1;
    }
    let int = &s[int_start..i];
    let mut frac = "";
    if b.get(i) == Some(&b'.') {
        let fs = i + 1;
        let mut j = fs;
        while b.get(j).is_some_and(u8::is_ascii_digit) {
            j += 1;
        }
        frac = &s[fs..j];
        i = j;
    }
    if int.is_empty() && frac.is_empty() {
        return None;
    }
    let mut exp = "0".to_string();
    if matches!(b.get(i), Some(b'e' | b'E')) {
        let mut j = i + 1;
        let es = j;
        if matches!(b.get(j), Some(b'+' | b'-')) {
            j += 1;
        }
        let ds = j;
        while b.get(j).is_some_and(u8::is_ascii_digit) {
            j += 1;
        }
        if j > ds {
            exp = s[es..j].to_string();
        }
    }
    let int = if int.is_empty() { "0" } else { int };
    let frac = if frac.is_empty() { "0" } else { frac };
    let v: f64 = format!("{int}.{frac}e{exp}").parse().ok()?;
    Some(if neg { -v } else { v })
}

/// Zig `std.fmt.parseUnsigned(u64, s, 0)` (no `_` reaches it here).
fn parse_unsigned_base0(s: &str) -> Option<u64> {
    let b = s.as_bytes();
    let (radix, digits) = if b.len() > 2 && b[0] == b'0' {
        match b[1].to_ascii_lowercase() {
            b'b' => (2, &s[2..]),
            b'o' => (8, &s[2..]),
            b'x' => (16, &s[2..]),
            _ => (10, s),
        }
    } else {
        (10, s)
    };
    if digits.is_empty() {
        return None;
    }
    u64::from_str_radix(digits, radix).ok()
}

#[derive(PartialEq, Clone, Copy)]
enum First {
    Positive,
    Negative,
    Dot,
    Other,
}

/// `tryResolveNumber` over a whole plain scalar; `None` keeps the string.
fn number(t: &str) -> Option<f64> {
    let b = t.as_bytes();
    let at = |i: usize| b.get(i).copied().unwrap_or(0);
    let (first_char, i) = match at(0) {
        b'-' => (First::Negative, 1),
        b'+' => (First::Positive, 1),
        b'0'..=b'9' => (First::Other, 0),
        b'.' if matches!(at(1), b'n' | b'N' | b'i' | b'I') => (First::Dot, 1),
        b'.' => (First::Other, 0),
        _ => return None,
    };
    let sign = if first_char == First::Negative {
        -1.0
    } else {
        1.0
    };

    match first_char {
        First::Dot => {
            return match &t[1..] {
                "nan" | "NaN" | "NAN" => Some(f64::NAN),
                "inf" | "Inf" | "INF" => Some(f64::INFINITY),
                _ => None,
            };
        }
        // Bun's condition is `next == '.' and peek(1) == 'i' or peek(1) == 'I'`.
        First::Negative | First::Positive
            if (at(i) == b'.' && at(i + 1) == b'i') || at(i + 1) == b'I' =>
        {
            let rest = t.get(i + 1..)?;
            let inf = if at(i + 1) == b'i' {
                rest == "inf"
            } else {
                rest == "Inf" || rest == "INF"
            };
            return inf.then_some(sign * f64::INFINITY);
        }
        _ => {}
    }

    let start = i;
    let mut decimal = at(i) == b'.';
    let (mut x, mut o, mut e, mut minus, mut hex) = (false, false, false, false, false);
    let mut j = if first_char == First::Other { i + 1 } else { i };
    let mut first = true;
    loop {
        let c = at(j);
        if j >= b.len() {
            if first && matches!(first_char, First::Positive | First::Negative) {
                return None;
            }
            break;
        }
        match c {
            b'0' => {
                if first && matches!(at(j + 1), b'b' | b'B') {
                    return None;
                }
            }
            b'1'..=b'9' => {}
            b'e' | b'E' => {
                if e {
                    hex = true;
                }
                e = true;
            }
            b'a'..=b'd' | b'f' | b'A'..=b'D' | b'F' => {
                hex = true;
                if first && matches!(c, b'b' | b'B') {
                    return None;
                }
            }
            b'x' if !x => x = true,
            b'o' if !o => o = true,
            b'.' if !decimal => decimal = true,
            b'+' if !x => {}
            b'-' if !minus => minus = true,
            // Anything else (or a repeated `x`/`o`/`.`/`-`) ends the number
            // before the end of the scalar: a string.
            _ => return None,
        }
        first = false;
        j += 1;
    }

    let slice = &t[start..];
    let v = if x || o || hex {
        parse_unsigned_base0(slice)? as f64
    } else {
        parse_double_prefix(slice)?
    };
    Some(sign * v)
}

/// A plain scalar as Bun resolves it.
pub(crate) fn resolve_plain(t: &str, tag: Tag) -> Value {
    if t.is_empty() {
        return match tag {
            Tag::None | Tag::Null => Value::Null,
            _ => Value::String(String::new()),
        };
    }
    if let Some(v) = keyword(t) {
        let ok = matches!(
            (&v, tag),
            (_, Tag::None) | (Value::Bool(_), Tag::Bool) | (Value::Null, Tag::Null)
        );
        return if ok { v } else { Value::String(t.into()) };
    }
    if matches!(tag, Tag::None | Tag::Int | Tag::Float)
        && let Some(n) = number(t)
    {
        return Value::Number(Number::from(n));
    }
    Value::String(t.into())
}

/// The libyaml parser (freed on drop). It reads `input` in place, so the
/// lifetime ties the parser to the borrowed bytes.
struct Parser<'a> {
    raw: Box<MaybeUninit<unsafe_yaml::yaml_parser_t>>,
    _input: PhantomData<&'a [u8]>,
}

impl<'a> Parser<'a> {
    fn new(input: &'a [u8]) -> Option<Parser<'a>> {
        let mut raw = Box::new(MaybeUninit::<unsafe_yaml::yaml_parser_t>::uninit());
        // SAFETY: `raw` is a valid allocation; libyaml initializes it, and
        // `input` outlives the parser (enforced by `'a`).
        unsafe {
            let p = raw.as_mut_ptr();
            if unsafe_yaml::yaml_parser_initialize(p).fail {
                return None;
            }
            unsafe_yaml::yaml_parser_set_encoding(p, unsafe_yaml::YAML_UTF8_ENCODING);
            unsafe_yaml::yaml_parser_set_input_string(p, input.as_ptr(), input.len() as u64);
        }
        Some(Parser {
            raw,
            _input: PhantomData,
        })
    }
}

impl Drop for Parser<'_> {
    fn drop(&mut self) {
        // SAFETY: initialized in `new`.
        unsafe { unsafe_yaml::yaml_parser_delete(self.raw.as_mut_ptr()) }
    }
}

/// # Safety
/// `p` is null or a NUL-terminated string owned by a live event.
unsafe fn c_bytes<'a>(p: *const u8) -> Option<&'a [u8]> {
    (!p.is_null()).then(|| unsafe { CStr::from_ptr(p.cast()) }.to_bytes())
}

/// An open sequence or mapping: its items, anchor and the depth of its
/// deepest child so far.
enum Frame {
    Seq(Vec<Value>, Option<Vec<u8>>, usize),
    Map(Mapping, Option<Value>, Option<Vec<u8>>, usize),
}

impl Frame {
    fn child_depth(&mut self) -> &mut usize {
        match self {
            Frame::Seq(_, _, d) | Frame::Map(_, _, _, d) => d,
        }
    }
}

/// Bun's `MappingProps.merge`: the merged mapping's entries, last first,
/// each appended unless its key is already present.
fn merge(m: &mut Mapping, from: &Mapping) {
    let entries: Vec<_> = from.iter().collect();
    for (k, v) in entries.into_iter().rev() {
        if !m.contains_key(k) {
            m.insert(k.clone(), v.clone());
        }
    }
}

/// `appendMaybeMerge`: a `<<` key with a mapping (or a list, whose mappings
/// are merged in order) merges; anything else is a plain entry (the last
/// value of a repeated key wins, in the first key's position).
fn insert_maybe_merge(m: &mut Mapping, k: Value, v: Value) {
    if k.as_str() == Some("<<") {
        match &v {
            Value::Mapping(from) => return merge(m, from),
            Value::Sequence(items) => {
                for item in items {
                    if let Value::Mapping(from) = item {
                        merge(m, from);
                    }
                }
                return;
            }
            _ => {}
        }
    }
    m.insert(k, v);
}

fn node_count(v: &Value) -> usize {
    match v {
        Value::Sequence(s) => 1 + s.iter().map(node_count).sum::<usize>(),
        Value::Mapping(m) => {
            1 + m
                .iter()
                .map(|(k, v)| node_count(k) + node_count(v))
                .sum::<usize>()
        }
        _ => 1,
    }
}

/// A libyaml event, copied out so the C event can be freed before any of it
/// is used (so no early return can leak it).
enum Event {
    StreamEnd,
    DocumentStart,
    DocumentEnd,
    Alias(Vec<u8>),
    Scalar {
        value: Vec<u8>,
        plain: bool,
        tag: Option<Vec<u8>>,
        anchor: Option<Vec<u8>>,
    },
    SequenceStart(Option<Vec<u8>>),
    MappingStart(Option<Vec<u8>>),
    End,
    Other,
}

impl Parser<'_> {
    /// The next event, or `None` on a libyaml error.
    fn next(&mut self) -> Option<Event> {
        let mut ev = MaybeUninit::<unsafe_yaml::yaml_event_t>::uninit();
        // SAFETY: the parser is initialized. On success the event is
        // initialized; its data is copied into owned values and the event is
        // deleted before returning. On failure libyaml leaves no event.
        unsafe {
            if unsafe_yaml::yaml_parser_parse(self.raw.as_mut_ptr(), ev.as_mut_ptr()).fail {
                return None;
            }
            let ev = ev.assume_init_mut();
            let owned = |p: *const u8| c_bytes(p).map(<[u8]>::to_vec);
            let out = match ev.type_ {
                unsafe_yaml::YAML_STREAM_END_EVENT => Event::StreamEnd,
                unsafe_yaml::YAML_DOCUMENT_START_EVENT => Event::DocumentStart,
                unsafe_yaml::YAML_DOCUMENT_END_EVENT => Event::DocumentEnd,
                unsafe_yaml::YAML_ALIAS_EVENT => {
                    Event::Alias(owned(ev.data.alias.anchor).unwrap_or_default())
                }
                unsafe_yaml::YAML_SCALAR_EVENT => {
                    let s = ev.data.scalar;
                    let value = if s.value.is_null() || s.length == 0 {
                        Vec::new()
                    } else {
                        std::slice::from_raw_parts(s.value, s.length as usize).to_vec()
                    };
                    Event::Scalar {
                        value,
                        plain: s.style == unsafe_yaml::YAML_PLAIN_SCALAR_STYLE,
                        tag: owned(s.tag),
                        anchor: owned(s.anchor),
                    }
                }
                unsafe_yaml::YAML_SEQUENCE_START_EVENT => {
                    Event::SequenceStart(owned(ev.data.sequence_start.anchor))
                }
                unsafe_yaml::YAML_MAPPING_START_EVENT => {
                    Event::MappingStart(owned(ev.data.mapping_start.anchor))
                }
                unsafe_yaml::YAML_SEQUENCE_END_EVENT | unsafe_yaml::YAML_MAPPING_END_EVENT => {
                    Event::End
                }
                _ => Event::Other,
            };
            unsafe_yaml::yaml_event_delete(ev);
            Some(out)
        }
    }
}

/// Why [`load`] failed.
#[derive(Debug, PartialEq)]
pub(crate) enum LoadError {
    /// A libyaml error: the caller gets the message from `serde_yaml_ng`,
    /// which uses the same parser.
    Syntax,
    /// Too deep, or too many alias copies (reported directly: re-parsing
    /// such input with `serde_yaml_ng` is slow).
    Limit(&'static str),
}

const TOO_DEEP: LoadError =
    LoadError::Limit("recursion limit exceeded: more than 128 nested levels");
const TOO_MANY_ALIASES: LoadError = LoadError::Limit("repetition limit exceeded");

/// Every document of `content`.
pub(crate) fn load(content: &str) -> Result<Vec<Value>, LoadError> {
    use LoadError::Syntax;
    let input = content.as_bytes();
    let mut parser = Parser::new(input).ok_or(Syntax)?;
    let mut docs = Vec::new();
    let mut stack: Vec<Frame> = Vec::new();
    // Anchored values with their nesting depth.
    let mut anchors: HashMap<Vec<u8>, (Value, usize)> = HashMap::new();
    let mut root: Option<Value> = None;
    let mut budget = ALIAS_BUDGET;

    loop {
        // (value, anchor, nesting depth: 0 for a scalar)
        let node: (Value, Option<Vec<u8>>, usize) = match parser.next().ok_or(Syntax)? {
            Event::StreamEnd => return Ok(docs),
            Event::DocumentStart => {
                anchors.clear();
                root = None;
                continue;
            }
            Event::DocumentEnd => {
                docs.push(root.take().unwrap_or(Value::Null));
                continue;
            }
            Event::Alias(name) => {
                let (v, depth) = anchors.get(&name).cloned().ok_or(Syntax)?;
                budget = budget.checked_sub(node_count(&v)).ok_or(TOO_MANY_ALIASES)?;
                (v, None, depth)
            }
            Event::Scalar {
                value,
                plain,
                tag,
                anchor,
            } => {
                let text = String::from_utf8(value).map_err(|_| Syntax)?;
                let v = if plain {
                    resolve_plain(&text, Tag::from_libyaml(tag.as_deref()))
                } else {
                    Value::String(text)
                };
                (v, anchor, 0)
            }
            Event::SequenceStart(anchor) => {
                if stack.len() >= MAX_DEPTH {
                    return Err(TOO_DEEP);
                }
                stack.push(Frame::Seq(Vec::new(), anchor, 0));
                continue;
            }
            Event::MappingStart(anchor) => {
                if stack.len() >= MAX_DEPTH {
                    return Err(TOO_DEEP);
                }
                stack.push(Frame::Map(Mapping::new(), None, anchor, 0));
                continue;
            }
            Event::End => match stack.pop().ok_or(Syntax)? {
                Frame::Seq(items, a, d) => (Value::Sequence(items), a, d + 1),
                Frame::Map(m, _, a, d) => (Value::Mapping(m), a, d + 1),
            },
            Event::Other => continue,
        };
        let (v, anchor, depth) = node;
        // Aliases can nest values deeper than the syntax does.
        if depth + stack.len() > MAX_DEPTH {
            return Err(TOO_DEEP);
        }
        if let Some(a) = anchor {
            anchors.insert(a, (v.clone(), depth));
        }
        match stack.last_mut() {
            None => root = Some(v),
            Some(frame) => {
                let d = frame.child_depth();
                *d = (*d).max(depth);
                match frame {
                    Frame::Seq(items, _, _) => items.push(v),
                    Frame::Map(m, key, _, _) => match key.take() {
                        None => *key = Some(v),
                        Some(k) => insert_maybe_merge(m, k, v),
                    },
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn num(t: &str) -> Option<f64> {
        number(t)
    }

    #[test]
    fn bun_number_quirks() {
        assert_eq!(num("010"), Some(10.0));
        assert_eq!(num("0x1F"), Some(31.0));
        assert_eq!(num("-0x10"), Some(-16.0));
        assert_eq!(num("0o17"), Some(15.0));
        assert_eq!(num("0b1"), None);
        assert_eq!(num("1_000"), None);
        assert_eq!(num("1e"), Some(1.0));
        assert_eq!(num("1-"), Some(1.0));
        assert_eq!(num("--1"), Some(1.0));
        assert_eq!(num("-+.1"), Some(-0.1));
        assert_eq!(num("+.5"), None);
        assert_eq!(num("1e1.5"), Some(10.0));
        assert_eq!(num("0xFFFFFFFFFFFFFFFF"), Some(18446744073709551615.0));
        assert_eq!(num("0x10000000000000000"), None);
        assert_eq!(num("-1Inf"), Some(f64::NEG_INFINITY));
        assert!(num(".NaN").unwrap().is_nan());
        assert_eq!(num("."), None);
        assert_eq!(num("+"), None);
    }

    #[test]
    fn empty_scalars_load() {
        let docs = load("a: \"\"\nb: ''\nc:\nd: |\ne: >-\n").unwrap();
        let m = docs[0].as_mapping().unwrap();
        assert_eq!(m["a"], Value::String(String::new()));
        assert_eq!(m["b"], Value::String(String::new()));
        assert_eq!(m["c"], Value::Null);
        assert_eq!(m["d"], Value::String(String::new()));
        assert_eq!(m["e"], Value::String(String::new()));
        assert!(load("").unwrap().is_empty());
    }

    #[test]
    fn tags_and_keywords() {
        assert_eq!(resolve_plain("yes", Tag::None), Value::Bool(true));
        assert_eq!(resolve_plain("yEs", Tag::None), Value::String("yEs".into()));
        assert_eq!(resolve_plain("yes", Tag::Int), Value::String("yes".into()));
        assert_eq!(resolve_plain("1", Tag::Bool), Value::String("1".into()));
        assert_eq!(resolve_plain("1.5", Tag::Int), Value::Number(1.5.into()));
        assert_eq!(
            resolve_plain("5", Tag::NonSpecific),
            Value::String("5".into())
        );
        assert_eq!(resolve_plain("", Tag::None), Value::Null);
    }

    /// The early-return paths (unknown alias, alias budget) run after the
    /// event was copied out and freed.
    #[test]
    fn alias_errors_return_cleanly() {
        let mut y = String::from("a0: &a0 [x, x, x, x, x, x, x, x, x, x]\n");
        for i in 1..8 {
            let p = i - 1;
            y.push_str(&format!(
                "a{i}: &a{i} [*a{p}, *a{p}, *a{p}, *a{p}, *a{p}, *a{p}, *a{p}, *a{p}, *a{p}, *a{p}]\n"
            ));
        }
        assert_eq!(load(&y), Err(TOO_MANY_ALIASES));
        assert_eq!(load("a: [1, *missing]\n"), Err(LoadError::Syntax));
        assert_eq!(load("a: &x [*x]\n"), Err(LoadError::Syntax));
    }

    /// 100000 levels: a clean error, no stack overflow (run in debug and
    /// release; test threads have small stacks).
    #[test]
    fn deep_nesting_is_an_error() {
        let n = 100_000;
        let deep = format!("{}{}", "[".repeat(n), "]".repeat(n));
        assert_eq!(load(&deep), Err(TOO_DEEP));
        let block: String = (0..n)
            .map(|i| format!("{}- \n", "  ".repeat(i.min(3000))))
            .collect();
        let _ = load(&block);
        assert!(
            load(&format!(
                "{}{}",
                "[".repeat(MAX_DEPTH),
                "]".repeat(MAX_DEPTH)
            ))
            .is_ok()
        );
        assert_eq!(
            load(&format!(
                "{}{}",
                "[".repeat(MAX_DEPTH + 1),
                "]".repeat(MAX_DEPTH + 1)
            )),
            Err(TOO_DEEP)
        );
        // Aliases nesting past the cap.
        let mut y = String::from("a0: &a0 [1]\n");
        for i in 1..=MAX_DEPTH {
            y.push_str(&format!("a{i}: &a{i} [*a{}]\n", i - 1));
        }
        assert_eq!(load(&y), Err(TOO_DEEP));
    }

    #[test]
    fn aliases_and_errors() {
        let d = load("a: &x [1, yes]\nb: *x\n").unwrap();
        let want: Value = serde_yaml_ng::from_str("a: [1.0, true]\nb: [1.0, true]").unwrap();
        assert_eq!(d, vec![want]);
        assert_eq!(load("a: [1"), Err(LoadError::Syntax));
        assert_eq!(load("a: *nope"), Err(LoadError::Syntax));
        assert_eq!(load("").unwrap(), Vec::<Value>::new());
    }
}
