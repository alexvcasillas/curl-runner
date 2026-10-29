//! `--fix`: apply the fixes to a copy of the document (TS `applyFix`) and
//! print it with TS's YAML emitter (`serializeYaml`): same layout (nested
//! mappings indented by 4, list items as TS writes them) and the same text
//! wherever TS wrote valid YAML. Fixed (logged): empty `{}`/`[]` stay inline
//! (`key: {}`), and strings and keys that would not read back as themselves
//! are double-quoted.

use super::issues::{FixValue, Issue, array_index};
use crate::jsval::{is_js_space, num_to_string};
use crate::model::Json;
use crate::snapshot::json::js_order;
use crate::yaml_text::{map_key_text, scalar_text};
use indexmap::IndexMap;

/// A JS value as `applyFix` can leave it: `undefined` properties and array
/// holes (from `delete arr[i]`) are possible.
#[derive(Debug, Clone, PartialEq)]
enum Node {
    Undefined,
    Hole,
    Leaf(Json),
    Arr(Vec<Node>),
    Obj(IndexMap<String, Node>),
}

impl From<&Json> for Node {
    fn from(v: &Json) -> Self {
        match v {
            Json::Array(a) => Node::Arr(a.iter().map(Node::from).collect()),
            Json::Object(o) => Node::Obj(o.iter().map(|(k, v)| (k.clone(), v.into())).collect()),
            other => Node::Leaf(other.clone()),
        }
    }
}

impl Node {
    fn get_mut(&mut self, key: &str) -> Option<&mut Node> {
        match self {
            Node::Obj(o) => o.get_mut(key),
            Node::Arr(a) => array_index(key).and_then(|i| a.get_mut(i)),
            _ => None,
        }
    }

    fn get(&self, key: &str) -> Node {
        match self {
            Node::Obj(o) => o.get(key).cloned(),
            Node::Arr(a) => array_index(key).and_then(|i| a.get(i)).cloned(),
            _ => None,
        }
        .filter(|n| *n != Node::Hole)
        .unwrap_or(Node::Undefined)
    }

    /// `obj[key] = v` (named properties of arrays are not serialized, so
    /// they are dropped).
    fn set(&mut self, key: &str, v: Node) {
        match self {
            Node::Obj(o) => {
                o.insert(key.to_string(), v);
            }
            Node::Arr(a) => {
                if let Some(i) = array_index(key) {
                    if i >= a.len() {
                        a.resize(i + 1, Node::Hole);
                    }
                    a[i] = v;
                }
            }
            _ => {}
        }
    }

    /// The data the written YAML stands for: `undefined` properties and
    /// array holes are not written.
    fn to_json(&self) -> Json {
        match self {
            Node::Undefined | Node::Hole => Json::Null,
            Node::Leaf(v) => v.clone(),
            Node::Arr(a) => Json::Array(
                a.iter()
                    .filter(|n| **n != Node::Hole)
                    .map(Node::to_json)
                    .collect(),
            ),
            Node::Obj(o) => Json::Object(
                o.iter()
                    .filter(|(_, v)| **v != Node::Undefined)
                    .map(|(k, v)| (k.clone(), v.to_json()))
                    .collect(),
            ),
        }
    }

    /// `delete obj[key]`.
    fn delete(&mut self, key: &str) {
        match self {
            Node::Obj(o) => {
                o.shift_remove(key);
            }
            Node::Arr(a) => {
                if let Some(slot) = array_index(key).and_then(|i| a.get_mut(i)) {
                    *slot = Node::Hole;
                }
            }
            _ => {}
        }
    }
}

/// The path parts of `applyFix`: `.`-separated, `name[3]` split in two.
fn path_parts(path: &str) -> Vec<String> {
    let mut out = Vec::new();
    for p in path.split('.') {
        let indexed = p
            .strip_suffix(']')
            .and_then(|s| s.split_once('['))
            .filter(|(n, i)| {
                !n.is_empty()
                    && n.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'_')
                    && !i.is_empty()
                    && i.bytes().all(|b| b.is_ascii_digit())
            });
        match indexed {
            Some((name, idx)) => {
                out.push(name.to_string());
                out.push(num_to_string(idx.parse::<f64>().unwrap_or(f64::NAN)));
            }
            None => out.push(p.to_string()),
        }
    }
    out
}

fn apply_fix(data: &mut Node, path: &str, value: &FixValue) {
    if path.is_empty() || path == "(root)" {
        return;
    }
    let parts = path_parts(path);
    let (last, walk) = parts.split_last().expect("split yields one part");
    let mut current = data;
    for key in walk {
        match current.get_mut(key) {
            Some(next @ (Node::Arr(_) | Node::Obj(_))) => current = next,
            _ => return,
        }
    }
    if !matches!(current, Node::Arr(_) | Node::Obj(_)) {
        return;
    }
    match value {
        FixValue::Rename { to, value } => {
            let old = match value {
                Some(v) if !v.is_null() => Node::from(v),
                _ => current.get(last),
            };
            current.delete(last);
            current.set(to, old);
        }
        FixValue::Delete => current.delete(last),
        FixValue::Set(v) => current.set(last, Node::from(v)),
    }
}

/// The fixed document (as written, and as data), or `None` when no issue
/// has a fix.
pub fn fix_document(content: &Json, issues: &[Issue]) -> Option<(String, Json)> {
    if !issues.iter().any(|i| i.fix.is_some()) {
        return None;
    }
    let mut data = Node::from(content);
    for issue in issues {
        if let Some(fix) = &issue.fix {
            apply_fix(&mut data, &issue.path, &fix.value);
        }
    }
    Some((serialize(&data, 0), data.to_json()))
}

/// The data [`fix_document`] gives for a single fix at `path`.
pub(super) fn fixed_data(content: &Json, path: &str, value: &FixValue) -> Json {
    let mut data = Node::from(content);
    apply_fix(&mut data, path, value);
    data.to_json()
}

/// The text [`fix_document`] writes for a scalar; `None` for a list or mapping.
pub(super) fn scalar_yaml(v: &Json) -> Option<String> {
    (!v.is_array() && !v.is_object()).then(|| serialize(&Node::Leaf(v.clone()), 0))
}

/// The strings TS quotes (with `JSON.stringify`).
fn ts_quotes(s: &str) -> bool {
    s.contains(['\n', ':', '#'])
        || s.starts_with('$')
        || s.starts_with(|c: char| c.is_ascii_digit())
        || matches!(s, "" | "true" | "false" | "null")
}

/// A string value: quoted where TS quotes it, and wherever the plain text
/// would not read back as the same string (see `scalar_text`).
fn serialize_str(s: &str) -> String {
    scalar_text(s, ts_quotes(s), false)
}

/// Empty once written: no items, or only holes (`delete arr[i]`).
fn only_holes(a: &[Node]) -> bool {
    a.iter().all(|n| *n == Node::Hole)
}

fn is_container(n: &Node) -> bool {
    matches!(n, Node::Arr(_) | Node::Obj(_))
}

/// TS `serializeYaml`.
fn serialize(node: &Node, indent: usize) -> String {
    let spaces = "  ".repeat(indent);
    match node {
        Node::Undefined | Node::Hole | Node::Leaf(Json::Null) => "null".into(),
        Node::Leaf(Json::String(s)) => serialize_str(s),
        Node::Leaf(Json::Number(n)) => num_to_string(n.as_f64().unwrap_or(f64::NAN)),
        Node::Leaf(Json::Bool(b)) => b.to_string(),
        Node::Leaf(other) => serialize(&Node::from(other), indent),
        // TS wrote one blank line per hole; an all-holes list read back as null.
        Node::Arr(a) if only_holes(a) => "[]".into(),
        Node::Arr(a) => a
            .iter()
            .map(|item| match item {
                // `map` skips holes and `join` prints them as ''.
                Node::Hole => String::new(),
                // Inline, so it stays on the item's line (TS emitted `[]`/`{}`
                // unindented on the next line, which no longer parsed).
                Node::Arr(a) if only_holes(a) => format!("{spaces}- []"),
                Node::Obj(o) if o.values().all(|v| *v == Node::Undefined) => {
                    format!("{spaces}- {{}}")
                }
                n if is_container(n) => {
                    let s = serialize(n, indent + 1);
                    let mut lines = s.split('\n');
                    let first = lines.next().unwrap_or_default();
                    let rest: Vec<String> = lines.map(|l| format!("{spaces}  {l}")).collect();
                    format!("{spaces}- {first}\n{}", rest.join("\n"))
                        .trim_end_matches(is_js_space)
                        .to_string()
                }
                n => format!("{spaces}- {}", serialize(n, indent)),
            })
            .collect::<Vec<_>>()
            .join("\n"),
        Node::Obj(o) => {
            let entries: Vec<_> = js_order(o.iter())
                .into_iter()
                .filter(|(_, v)| **v != Node::Undefined)
                .collect();
            if entries.is_empty() {
                return "{}".into();
            }
            entries
                .into_iter()
                .map(|(k, v)| {
                    let k = map_key_text(k);
                    match v {
                        Node::Arr(a) if only_holes(a) => format!("{spaces}{k}: []"),
                        Node::Obj(o) if o.values().all(|v| *v == Node::Undefined) => {
                            format!("{spaces}{k}: {{}}")
                        }
                        Node::Obj(_) => {
                            let s = serialize(v, indent + 1);
                            let body: Vec<String> =
                                s.split('\n').map(|l| format!("  {l}")).collect();
                            format!("{spaces}{k}:\n{}", body.join("\n"))
                        }
                        Node::Arr(_) => format!("{spaces}{k}:\n{}", serialize(v, indent + 1)),
                        _ => format!("{spaces}{k}: {}", serialize(v, indent)),
                    }
                })
                .collect::<Vec<_>>()
                .join("\n")
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn ser(v: Json) -> String {
        serialize(&Node::from(&v), 0)
    }

    #[test]
    fn serializer_quirks() {
        assert_eq!(ser(json!({})), "{}");
        assert_eq!(ser(json!([])), "[]");
        assert_eq!(
            ser(json!({"a": "x: y", "b": "1a", "c": "", "d": "true", "e": "$x", "f": "ok"})),
            "a: \"x: y\"\nb: \"1a\"\nc: \"\"\nd: \"true\"\ne: \"$x\"\nf: ok"
        );
        assert_eq!(ser(json!({"b": 1, "2": null})), "2: null\nb: 1");
        assert_eq!(ser(json!({"a": {"b": {}}})), "a:\n    b: {}");
        assert_eq!(ser(json!({"a": [1]})), "a:\n  - 1");
    }

    #[test]
    fn apply_fix_paths() {
        assert_eq!(path_parts("requests[0].url"), ["requests", "0", "url"]);
        assert_eq!(path_parts("a[007].b-c[1]"), ["a", "7", "b-c[1]"]);
        let mut d = Node::from(&json!({"r": [{"metod": "GET", "x": 1}]}));
        apply_fix(
            &mut d,
            "r[0].metod",
            &FixValue::Rename {
                to: "method".into(),
                value: None,
            },
        );
        assert_eq!(d, Node::from(&json!({"r": [{"x": 1, "method": "GET"}]})));
        let mut a = Node::from(&json!(["a", "b"]));
        apply_fix(&mut a, "0", &FixValue::Delete);
        assert_eq!(serialize(&a, 0), "\n- b");
        // Walking through a scalar is a no-op.
        let mut s = Node::from(&json!({"a": "x"}));
        apply_fix(&mut s, "a.b", &FixValue::Set(json!(1)));
        assert_eq!(s, Node::from(&json!({"a": "x"})));
    }
}
