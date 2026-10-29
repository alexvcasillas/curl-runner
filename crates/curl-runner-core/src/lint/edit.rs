//! Lint fixes as text edits: the change [`fix_document`](super::fix_document)
//! makes for one issue, applied to the source text instead of re-emitting
//! the whole document, so formatting and comments elsewhere survive.

use super::fix::{fixed_data, scalar_yaml};
use super::spans::{NodeKind, NodeSpans, locate_node};
use super::{FixValue, Issue};
use crate::parser;
use crate::yaml_text::map_key_text;
use std::ops::Range;

/// Replace `range` (bytes) of the text with `new_text`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TextEdit {
    pub range: Range<usize>,
    pub new_text: String,
}

impl TextEdit {
    /// `text` with the edit applied, or `None` when the range is out of
    /// bounds or not on char boundaries.
    pub fn apply(&self, text: &str) -> Option<String> {
        let (head, tail) = (text.get(..self.range.start)?, text.get(self.range.end..)?);
        if self.range.start > self.range.end {
            return None;
        }
        Some([head, &self.new_text, tail].concat())
    }
}

/// The text edit for `issue`'s fix, or `None` when there is no fix or it
/// can't be expressed as one edit:
/// - `Rename` replaces the key (and the value, when the fix carries one);
/// - `Set` replaces a scalar value with the text `fix_document` writes;
/// - `Delete` removes the entry's whole lines, newline included.
///
/// The edited text must read back as the data `fix_document` gives, so a
/// path that ends at or runs through an alias, sits behind a merge key, or
/// does not resolve exactly to its node gives `None`.
///
/// On demand: this parses the text about three times (spans, before and
/// after the edit), so call it when the user applies a quick fix, not on
/// every lint.
pub fn fix_edit(text: &str, issue: &Issue) -> Option<TextEdit> {
    let fix = issue.fix.as_ref()?;
    let (NodeSpans { key, value }, kind) = locate_node(text, &issue.path)?;
    let edit = match (&fix.value, kind) {
        (_, NodeKind::Alias) => return None,
        (FixValue::Set(v), NodeKind::Scalar) => TextEdit {
            range: value,
            new_text: scalar_yaml(v)?,
        },
        (FixValue::Set(_), NodeKind::Container) => return None,
        (FixValue::Rename { to, value: v }, _) => {
            let key = key?;
            let mut new_text = map_key_text(to);
            match v.as_ref().filter(|v| !v.is_null()) {
                None => TextEdit {
                    range: key,
                    new_text,
                },
                Some(v) if kind == NodeKind::Scalar => {
                    new_text.push_str(&text[key.end..value.start]);
                    new_text.push_str(&scalar_yaml(v)?);
                    TextEdit {
                        range: key.start..value.end,
                        new_text,
                    }
                }
                Some(_) => return None,
            }
        }
        (FixValue::Delete, _) => TextEdit {
            range: entry_lines(text, key?, value)?,
            new_text: String::new(),
        },
    };
    let before = parser::parse_raw_stream(text).ok()?;
    let after = parser::parse_raw_stream(&edit.apply(text)?).ok()?;
    (after == fixed_data(&before, &issue.path, &fix.value)).then_some(edit)
}

/// The whole lines of a mapping entry, from the start of the key's line
/// through the newline after the value. `None` when something other than
/// indentation precedes the key on its line (`- key:`, `{key: ...}`).
fn entry_lines(text: &str, key: Range<usize>, value: Range<usize>) -> Option<Range<usize>> {
    let start = text[..key.start].rfind('\n').map_or(0, |i| i + 1);
    if !text[start..key.start]
        .bytes()
        .all(|b| b == b' ' || b == b'\t')
    {
        return None;
    }
    // A block value ends where the next token starts: back up over blank
    // and comment-only lines (they belong to what follows) to its last
    // content line.
    let mut last = value.end;
    loop {
        let line_start = text[..last].rfind('\n').map_or(0, |i| i + 1);
        let line = text[line_start..last].trim();
        if line_start <= key.end || !(line.is_empty() || line.starts_with('#')) {
            break;
        }
        last = line_start - 1;
    }
    let last = last.max(key.end);
    let end = text[last..].find('\n').map_or(text.len(), |i| last + i + 1);
    Some(start..end)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lint::{LintOutcome, Severity, fix_document, lint_text};

    fn lint(text: &str) -> Vec<Issue> {
        match lint_text(text).unwrap() {
            LintOutcome::Linted { issues, .. } => issues,
            LintOutcome::ParseError(m) => panic!("{m}: {text}"),
        }
    }

    /// The fixable issue at `path`.
    fn issue_at(text: &str, path: &str) -> Issue {
        lint(text)
            .into_iter()
            .find(|i| i.path == path && i.fix.is_some())
            .unwrap_or_else(|| panic!("no fixable issue at {path}"))
    }

    /// Apply the fix for the issue at `path`; it must be gone after.
    fn fixed(text: &str, path: &str) -> String {
        let issue = issue_at(text, path);
        let out = fix_edit(text, &issue)
            .expect("an edit")
            .apply(text)
            .unwrap();
        assert!(
            !lint(&out).contains(&issue),
            "still reported after the edit:\n{out}"
        );
        out
    }

    #[test]
    fn rename_replaces_the_key() {
        let t =
            "request:\n  url: https://api.example.com\n  retry:\n    count: 3\n    backof: 2 # x\n";
        assert_eq!(
            fixed(t, "request.retry.backof"),
            "request:\n  url: https://api.example.com\n  retry:\n    count: 3\n    backoff: 2 # x\n"
        );
    }

    #[test]
    fn rename_with_value_replaces_key_and_value() {
        let t = "request:\n  url: https://api.example.com\n  headers:\n    content-type: application/json\n";
        assert_eq!(
            fixed(t, "request.headers.content-type"),
            "request:\n  url: https://api.example.com\n  headers:\n    Content-Type: application/json\n"
        );
    }

    #[test]
    fn set_replaces_the_value() {
        let t = "request:\n  url: https://api.example.com\n  method: get # keep\n";
        assert_eq!(
            fixed(t, "request.method"),
            "request:\n  url: https://api.example.com\n  method: GET # keep\n"
        );
    }

    #[test]
    fn delete_removes_the_entry_lines() {
        let t = "# head\nfoo:\n  a: 1\n  b: [2]\nrequest:\n  url: https://api.example.com\n";
        assert_eq!(
            fixed(t, "foo"),
            "# head\nrequest:\n  url: https://api.example.com\n"
        );
        // Blank and comment lines before the next key stay.
        let t = "foo:\n  a: 1\n\n# Requests below\nrequest:\n  url: https://api.example.com\n";
        assert_eq!(
            fixed(t, "foo"),
            "\n# Requests below\nrequest:\n  url: https://api.example.com\n"
        );
        let t = "request:\n  url: https://api.example.com\nfoo: 1";
        assert_eq!(
            fixed(t, "foo"),
            "request:\n  url: https://api.example.com\n"
        );
    }

    #[test]
    fn apply_rejects_ranges_that_do_not_fit() {
        let e = |range, new_text: &str| TextEdit {
            range,
            new_text: new_text.into(),
        };
        assert_eq!(e(1..2, "X").apply("abc").as_deref(), Some("aXc"));
        assert_eq!(e(2..4, "X").apply("abc"), None);
        #[allow(clippy::reversed_empty_ranges)]
        let reversed = 2..1;
        assert_eq!(e(reversed, "X").apply("abc"), None);
        assert_eq!(e(1..2, "X").apply("é"), None);
    }

    #[test]
    fn no_fix_or_unknown_path_gives_none() {
        let t = "request:\n  url: https://api.example.com\n  method: get\n";
        let mut i = issue_at(t, "request.method");
        i.path = "request.nope".into();
        assert_eq!(fix_edit(t, &i), None);
        i.fix = None;
        i.path = "request.method".into();
        assert_eq!(fix_edit(t, &i), None);
    }

    #[test]
    fn aliases_and_merge_keys_give_none() {
        // The target is an alias: editing `*m` would not touch the anchor.
        let t = "m: &m get\nrequest:\n  url: https://api.example.com\n  method: *m\n";
        assert_eq!(fix_edit(t, &issue_at(t, "request.method")), None);
        // The target sits behind an alias.
        let t = "base: &b\n  url: https://api.example.com\n  method: get\nrequest: *b\n";
        assert_eq!(fix_edit(t, &issue_at(t, "request.method")), None);
        // The target is anchored: the edit would change its aliases too.
        let t = "base: &b\n  url: https://api.example.com\n  method: get\nrequest: *b\nx: *b\n";
        assert_eq!(fix_edit(t, &issue_at(t, "base")), None);
        // Behind a merge key.
        let t = "base: &b\n  method: get\nrequest:\n  <<: *b\n  url: https://api.example.com\n";
        assert_eq!(fix_edit(t, &issue_at(t, "request.method")), None);
        // Deleting a key a merge would then supply.
        let t = "base: &b\n  foo: 1\nrequest:\n  url: https://api.example.com\n<<: *b\nfoo: 2\n";
        assert_eq!(fix_edit(t, &issue_at(t, "foo")), None);
    }

    /// One fixture per fixing rule: every `Set` rule, the `Delete` rule and
    /// both `Rename` rules.
    const FIXTURES: &[&str] = &[
        // Set: method case, closest method, boolean string, timeout string.
        "request:\n  url: https://api.example.com\n  method: get\n",
        "request:\n  url: https://api.example.com\n  method: GTE\n",
        "request:\n  url: https://api.example.com\n  insecure: \"TRUE\"\n  timeout: '5000'\n",
        // Set: enum choices.
        "global:\n  execution: paralel\n  output:\n    format: jsn\nrequest:\n  url: https://api.example.com\n  auth:\n    type: basci\n",
        // Set: status string, condition operator.
        "requests:\n  - url: https://api.example.com\n    expect:\n      status: \"200\"\n      failure: 'false'\n  - url: https://api.example.com/b\n    when:\n      left: x\n      operator: contain\n      right: y\n",
        // Set: URL typo, missing scheme, double slash, spaces.
        "requests:\n  - url: htpp://api.example.com\n  - url: api.example.com/a\n  - url: https://api.example.com//a\n  - url: https://api.example.com/a b\n",
        // Delete: unknown top-level key (block, flow and last line).
        "foo:\n  a: 1\nrequest:\n  url: https://api.example.com\nbar: [1, 2]\nbaz: x",
        // Rename: unknown key, header casing.
        "request:\n  url: https://api.example.com\n  retry:\n    backof: 2\n  headers:\n    content-type: application/json\n",
    ];

    fn summary(issues: &[Issue]) -> Vec<(String, Severity)> {
        let mut v: Vec<_> = issues
            .iter()
            .map(|i| (i.path.clone(), i.severity))
            .collect();
        v.sort_by_key(|(p, s)| (p.clone(), *s as u8));
        v
    }

    #[test]
    fn round_trips_with_fix_document() {
        let (mut set, mut delete, mut rename) = (0, 0, 0);
        for t in FIXTURES {
            let LintOutcome::Linted { content, issues } = lint_text(t).unwrap() else {
                panic!("{t}")
            };
            for issue in issues.iter().filter(|i| i.fix.is_some()) {
                let one = std::slice::from_ref(issue);
                let (doc, _) = fix_document(&content, one).unwrap();
                let edit = fix_edit(t, issue)
                    .unwrap_or_else(|| panic!("no edit for {:?} in\n{t}", issue.path));
                let out = edit.apply(t).unwrap();
                assert_eq!(
                    summary(&lint(&out)),
                    summary(&lint(&doc)),
                    "{:?}:\n{out}\nvs\n{doc}",
                    issue.path
                );
                match issue.fix.as_ref().unwrap().value {
                    FixValue::Set(_) => set += 1,
                    FixValue::Delete => delete += 1,
                    FixValue::Rename { .. } => rename += 1,
                }
            }
        }
        assert_eq!((set, delete, rename), (14, 3, 2));
    }
}
