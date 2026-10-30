//! Source ranges for lint [`Issue`]s: map an issue path
//! (`request.retry.backof`, `requests[2].url`) to byte ranges in the YAML
//! text, so an editor can underline it.
//!
//! The spans come from the libyaml events of the first document (the same
//! parser `yaml_load` uses). Aliases are not expanded: a path through an
//! alias gives the alias node. A repeated key resolves to its last
//! occurrence, as in the loaded value.

use super::{FixValue, Issue, ROOT_PATH};
use crate::yaml_load::{Event, MAX_DEPTH, Parser};
use std::ops::Range;

/// Where a node sits in the text, as byte ranges.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NodeSpans {
    /// The mapping key; `None` for sequence items and the root.
    pub key: Option<Range<usize>>,
    /// The value node (quotes included for quoted scalars).
    pub value: Range<usize>,
}

enum Node {
    Scalar(Range<usize>),
    Alias(Range<usize>),
    Seq(Range<usize>, Vec<Node>),
    Map(Range<usize>, Vec<Entry>),
}

struct Entry {
    /// The key text; `None` for a non-scalar key.
    key: Option<String>,
    key_span: Range<usize>,
    value: Node,
}

impl Node {
    fn span(&self) -> Range<usize> {
        match self {
            Node::Scalar(r) | Node::Alias(r) | Node::Seq(r, _) | Node::Map(r, _) => r.clone(),
        }
    }
}

enum Frame {
    Seq(usize, Vec<Node>),
    /// Start, entries, and the pending key (text, span).
    Map(usize, Vec<Entry>, Option<(Option<String>, Range<usize>)>),
}

/// The node tree of the first document; `None` on a syntax error, an empty
/// stream or input nested deeper than `MAX_DEPTH`.
fn tree(text: &str) -> Option<Node> {
    let mut parser = Parser::new(text.as_bytes())?;
    let mut stack: Vec<Frame> = Vec::new();
    loop {
        let (ev, span) = parser.next_marked()?;
        // (node, key text when used as a key)
        let (node, key_text) = match ev {
            Event::StreamEnd | Event::DocumentEnd => return None,
            Event::Alias(_) => (Node::Alias(span), None),
            Event::Scalar { value, .. } => (Node::Scalar(span), String::from_utf8(value).ok()),
            Event::SequenceStart(_) | Event::MappingStart(_) => {
                if stack.len() >= MAX_DEPTH {
                    return None;
                }
                stack.push(if matches!(ev, Event::SequenceStart(_)) {
                    Frame::Seq(span.start, Vec::new())
                } else {
                    Frame::Map(span.start, Vec::new(), None)
                });
                continue;
            }
            Event::End => match stack.pop()? {
                Frame::Seq(start, items) => (Node::Seq(start..span.end, items), None),
                Frame::Map(start, entries, _) => (Node::Map(start..span.end, entries), None),
            },
            Event::DocumentStart | Event::Other => continue,
        };
        match stack.last_mut() {
            None => return Some(node),
            Some(Frame::Seq(_, items)) => items.push(node),
            Some(Frame::Map(_, entries, pending)) => match pending.take() {
                None => *pending = Some((key_text, node.span())),
                Some((key, key_span)) => entries.push(Entry {
                    key,
                    key_span,
                    value: node,
                }),
            },
        }
    }
}

/// Resolves the rest of a path (`.key`, `[i]`, or a bare `key` at the
/// start) from `node`, giving the key span and the node. Keys may contain
/// dots: every key that matches is tried, longest first.
fn resolve<'a>(
    node: &'a Node,
    rest: &str,
    key: Option<Range<usize>>,
    top: bool,
) -> Option<(Option<Range<usize>>, &'a Node)> {
    if rest.is_empty() {
        return Some((key, node));
    }
    match node {
        Node::Scalar(_) => None,
        // Anchors are not expanded: a path through an alias gives the alias.
        Node::Alias(_) => Some((key, node)),
        Node::Seq(_, items) => {
            let inner = rest.strip_prefix('[')?;
            let close = inner.find(']')?;
            let item = items.get(inner[..close].parse::<usize>().ok()?)?;
            resolve(item, &inner[close + 1..], None, false)
        }
        Node::Map(_, entries) => {
            let rest = if top { rest } else { rest.strip_prefix('.')? };
            let mut found: Vec<(&str, &Entry)> = entries
                .iter()
                .rev()
                .filter_map(|e| {
                    let k = e.key.as_deref()?;
                    let after = rest.strip_prefix(k)?;
                    (after.is_empty() || after.starts_with(['.', '['])).then_some((k, e))
                })
                .collect();
            // Longest key first; a repeated key keeps only its last
            // occurrence (first here, as the scan is reversed; the sort is stable).
            found.sort_by_key(|(k, _)| std::cmp::Reverse(k.len()));
            found.dedup_by_key(|(k, _)| *k);
            found.into_iter().find_map(|(k, e)| {
                resolve(&e.value, &rest[k.len()..], Some(e.key_span.clone()), false)
            })
        }
    }
}

/// The spans of the node at `path` (validator syntax: `request.retry`,
/// `requests[2].url`; `""` and [`ROOT_PATH`] are the root, even when the
/// document has a literal `(root)` key), or `None` when it is not in the
/// first document.
pub fn locate(text: &str, path: &str) -> Option<NodeSpans> {
    locate_node(text, path).map(|(spans, _)| spans)
}

fn is_root(path: &str) -> bool {
    path.is_empty() || path == ROOT_PATH
}

/// What kind of node [`locate_node`] stopped at.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum NodeKind {
    Scalar,
    /// The path ends at, or runs through, an alias.
    Alias,
    /// A mapping or sequence.
    Container,
}

/// [`locate`], plus the kind of node found.
pub(crate) fn locate_node(text: &str, path: &str) -> Option<(NodeSpans, NodeKind)> {
    let root = tree(text)?;
    let path = if is_root(path) { "" } else { path };
    let (key, node) = resolve(&root, path, None, true)?;
    let kind = match node {
        Node::Scalar(_) => NodeKind::Scalar,
        Node::Alias(_) => NodeKind::Alias,
        Node::Seq(..) | Node::Map(..) => NodeKind::Container,
    };
    Some((
        NodeSpans {
            key,
            value: node.span(),
        },
        kind,
    ))
}

/// The range to underline for `issue`:
/// - a `Rename` fix or an `Unknown key` message: the key;
/// - otherwise a scalar value (an empty one falls back to its key);
/// - otherwise the key (a sequence item: the first line of the value);
/// - a path missing from the text: its nearest existing ancestor's key (a
///   sequence item: its first line);
/// - the root or an unresolvable path: the first non-blank line, trimmed;
/// - an empty text: `0..0`.
pub fn issue_range(text: &str, issue: &Issue) -> Range<usize> {
    if text.is_empty() {
        return 0..0;
    }
    let found = (!is_root(&issue.path))
        .then(|| locate_node(text, &issue.path))
        .flatten();
    let Some((NodeSpans { key, value }, kind)) = found else {
        return ancestor_range(text, &issue.path).unwrap_or_else(|| first_line(text));
    };
    let leaf = kind != NodeKind::Container;
    let want_key = issue.message.starts_with("Unknown key")
        || matches!(&issue.fix, Some(f) if matches!(f.value, FixValue::Rename { .. }));
    match key {
        Some(k) if want_key || !leaf || value.is_empty() => k,
        _ if leaf => value,
        _ => line_at(text, value.start),
    }
}

/// `path` without its last segment (`.key` or `[i]`); `None` at the top.
fn parent_path(path: &str) -> Option<&str> {
    let cut = path.rfind(['.', '['])?;
    (cut > 0).then(|| &path[..cut])
}

/// The key (a sequence item: the first line) of the nearest ancestor of
/// `path` found in `text`, below the root.
fn ancestor_range(text: &str, path: &str) -> Option<Range<usize>> {
    let mut path = path;
    while let Some(parent) = parent_path(path) {
        if let Some(NodeSpans { key, value }) = locate(text, parent) {
            return Some(key.unwrap_or_else(|| line_at(text, value.start)));
        }
        path = parent;
    }
    None
}

/// The line libyaml reports for the syntax error in `text`, trimmed (a
/// blank line, e.g. at end of input, falls back to the previous non-blank
/// one), or the first line when there is no error.
pub fn parse_error_range(text: &str) -> Range<usize> {
    let Some(mut parser) = Parser::new(text.as_bytes()) else {
        return first_line(text);
    };
    loop {
        match parser.next_marked() {
            Some((Event::StreamEnd, _)) => return first_line(text),
            Some(_) => {}
            None => break,
        }
    }
    let target = parser.problem_line();
    let mut best = None;
    for (i, r) in lines(text).enumerate() {
        if i > target {
            break;
        }
        if !r.is_empty() {
            best = Some(r);
        }
    }
    best.unwrap_or_else(|| first_line(text))
}

/// Each line's trimmed byte range.
fn lines(text: &str) -> impl Iterator<Item = Range<usize>> + '_ {
    text.split_inclusive('\n').scan(0, |start, line| {
        let s = *start;
        *start += line.len();
        Some(trimmed(text, s..s + line.len()))
    })
}

fn trimmed(text: &str, r: Range<usize>) -> Range<usize> {
    let line = &text[r.clone()];
    let lead = line.len() - line.trim_start().len();
    let trail = line.len() - line.trim_end().len();
    if lead == line.len() {
        return r.start..r.start;
    }
    r.start + lead..r.end - trail
}

/// The first non-blank line, trimmed (`0..0` when there is none).
fn first_line(text: &str) -> Range<usize> {
    lines(text).find(|r| !r.is_empty()).unwrap_or(0..0)
}

/// The trimmed line containing byte `at`.
fn line_at(text: &str, at: usize) -> Range<usize> {
    let at = at.min(text.len());
    let start = text[..at].rfind('\n').map_or(0, |i| i + 1);
    let end = text[at..].find('\n').map_or(text.len(), |i| at + i);
    trimmed(text, start..end)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lint::Severity;

    const DOC: &str =
        "# Créer 🚀\nrequest:\n  method: FETCH\n  retry:\n    backof: 2\nrequests:\n  - url: x\n";
    fn s(r: Range<usize>) -> &'static str {
        &DOC[r]
    }
    fn issue(path: &str, msg: &str) -> Issue {
        Issue {
            path: path.into(),
            severity: Severity::Error,
            message: msg.into(),
            fix: None,
        }
    }

    #[test]
    fn key_span_for_unknown_key() {
        assert_eq!(
            s(issue_range(
                DOC,
                &issue("request.retry.backof", "Unknown key \"backof\"")
            )),
            "backof"
        );
    }
    #[test]
    fn value_span_for_bad_value() {
        assert_eq!(
            s(issue_range(DOC, &issue("request.method", "Invalid method"))),
            "FETCH"
        );
    }
    #[test]
    fn sequence_index_paths() {
        assert_eq!(s(locate(DOC, "requests[0].url").unwrap().value), "x");
    }
    #[test]
    fn multibyte_prefix_does_not_shift() {
        assert!(DOC.is_char_boundary(issue_range(DOC, &issue("request.method", "x")).start));
    }
    #[test]
    fn root_and_missing_paths_fall_back_to_first_line() {
        assert_eq!(
            s(issue_range(DOC, &issue("", "Missing request(s)"))),
            "# Créer 🚀"
        );
        assert_eq!(s(issue_range(DOC, &issue("nope.x", "m"))), "# Créer 🚀");
    }
    #[test]
    fn missing_paths_fall_back_to_the_nearest_ancestor() {
        let t = "# top\nrequests:\n  - url: a\n  - url: b\n    method: GET\n";
        let r = |p: &str| at(t, issue_range(t, &issue(p, "m")));
        assert_eq!(r("requests[1].headers"), "- url: b");
        assert_eq!(r("requests[1].headers.Accept"), "- url: b");
        assert_eq!(r("requests[7].url"), "requests");
        assert_eq!(r("requests.x"), "requests");
        assert_eq!(r("nope.x"), "# top");
        let t = "request:\n  retry: 3\n";
        assert_eq!(
            at(t, issue_range(t, &issue("request.retry.count", "m"))),
            "retry"
        );
    }
    #[test]
    fn parent_paths() {
        assert_eq!(parent_path("requests[2].headers"), Some("requests[2]"));
        assert_eq!(parent_path("requests[2]"), Some("requests"));
        assert_eq!(parent_path("requests"), None);
        assert_eq!(parent_path("[1].b"), Some("[1]"));
        assert_eq!(parent_path("[1]"), None);
    }
    #[test]
    fn root_path_wins_over_a_literal_root_key() {
        let t = "# top\nrequest:\n  url: x\n\"(root)\": 1\n";
        let root = issue(crate::lint::ROOT_PATH, "Missing request(s)");
        assert_eq!(at(t, issue_range(t, &root)), "# top");
        let root = locate(t, crate::lint::ROOT_PATH).unwrap();
        assert_eq!(root, locate(t, "").unwrap());
        assert!(at(t, root.value).starts_with("request:"));
    }
    #[test]
    fn parse_error_points_at_bad_line() {
        let t = "request:\n  url: [\n";
        assert!(t[parse_error_range(t)].contains("url"));
    }

    fn at(t: &str, r: Range<usize>) -> &str {
        &t[r]
    }

    #[test]
    fn multibyte_values_are_byte_exact() {
        let t = "é: \"ünï 🚀\"\nnext: ok\n";
        let n = locate(t, "é").unwrap();
        assert_eq!(at(t, n.key.unwrap()), "é");
        assert_eq!(at(t, n.value), "\"ünï 🚀\"");
        assert_eq!(at(t, locate(t, "next").unwrap().value), "ok");
    }

    #[test]
    fn byte_order_mark_does_not_shift() {
        let t = "\u{feff}a: 1\n";
        assert_eq!(at(t, locate(t, "a").unwrap().value), "1");
    }

    #[test]
    fn quoted_scalars_include_quotes() {
        let t = "a: 'x'\nb: \"y\"\n";
        assert_eq!(at(t, locate(t, "a").unwrap().value), "'x'");
        assert_eq!(at(t, locate(t, "b").unwrap().value), "\"y\"");
    }

    #[test]
    fn duplicate_keys_resolve_to_the_last() {
        let t = "a: 1\na: 2\n";
        assert_eq!(at(t, locate(t, "a").unwrap().value), "2");
    }

    #[test]
    fn dotted_keys_and_aliases() {
        let t = "headers:\n  x.y: 1\nbase: &b {u: 1}\nreq: *b\n";
        assert_eq!(at(t, locate(t, "headers.x.y").unwrap().value), "1");
        assert_eq!(at(t, locate(t, "req.u").unwrap().value), "*b");
        assert_eq!(locate(t, "headers.x.z"), None);
        assert_eq!(locate(t, "headers.x.y.z"), None);
    }

    #[test]
    fn root_sequence_items_have_no_key() {
        let t = "- a: 1\n- b: 2\n";
        let n = locate(t, "[1].b").unwrap();
        assert_eq!(at(t, n.value), "2");
        assert_eq!(locate(t, "[1]").unwrap().key, None);
        assert_eq!(locate(t, "[2]"), None);
        assert_eq!(locate(t, "").unwrap().key, None);
    }

    #[test]
    fn first_document_only() {
        let t = "a: 1\n---\nb: 2\n";
        assert!(locate(t, "a").is_some());
        assert_eq!(locate(t, "b"), None);
    }

    #[test]
    fn range_rules() {
        let t =
            "request:\n  retry:\n    count: 1\n  url:\nrequests:\n  - url: x\n    method: GET\n";
        let r = |p: &str, m: &str| at(t, issue_range(t, &issue(p, m)));
        // Non-scalar value: the key.
        assert_eq!(r("request.retry", "Invalid retry"), "retry");
        // Empty scalar: the key.
        assert_eq!(r("request.url", "Missing url"), "url");
        // Sequence item mapping: its first line.
        assert_eq!(r("requests[0]", "Bad"), "- url: x");
        // Rename fix: the key.
        let mut i = issue("request.retry.count", "Typo");
        i.fix = Some(crate::lint::Fix {
            description: "rename".into(),
            value: FixValue::Rename {
                to: "x".into(),
                value: None,
            },
        });
        assert_eq!(at(t, issue_range(t, &i)), "count");
        assert_eq!(r("request.retry.count", "Bad"), "1");
        assert_eq!(issue_range("", &issue("a", "m")), 0..0);
    }

    #[test]
    fn unparsable_text_falls_back_to_first_line() {
        let t = "\n  first: [\n";
        assert_eq!(at(t, issue_range(t, &issue("first", "m"))), "first: [");
    }

    #[test]
    fn parse_error_lines() {
        let t = "a: 1\nb: 'x\nc: 2\n";
        // Unterminated quote: libyaml reports end of input; the last line before it.
        assert_eq!(at(t, parse_error_range(t)), "c: 2");
        let t = "a: 1\n  b: 2\nc: 3\n";
        assert_eq!(at(t, parse_error_range(t)), "b: 2");
        let t = "ok: 1\n";
        assert_eq!(at(t, parse_error_range(t)), "ok: 1");
        assert_eq!(parse_error_range(""), 0..0);
    }

    #[test]
    fn too_deep_input_does_not_resolve() {
        let n = MAX_DEPTH + 1;
        let t = format!("{}{}", "[".repeat(n), "]".repeat(n));
        assert_eq!(locate(&t, ""), None);
    }
}
