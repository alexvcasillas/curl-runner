//! A tiny, safe Markdown subset for admin-written email: paragraphs, line
//! breaks, `**bold**`, `*italic*`, `[text](https://…)` and `mailto:` links,
//! and `- ` lists. Everything else is text, HTML-escaped. A marker without
//! its closing half is shown as typed.
//!
//! `{{first_name}}`, `{{team_name}}` and `{{plan}}` are replaced while the
//! body is parsed, so a value is always text: it is escaped and its own
//! Markdown is never read.

/// Per-recipient values for `{{first_name}}`, `{{team_name}}` and `{{plan}}`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Vars {
    pub first_name: String,
    pub team_name: String,
    pub plan: String,
}

impl Vars {
    fn get(&self, name: &str) -> Option<&str> {
        match name {
            "first_name" => Some(&self.first_name),
            "team_name" => Some(&self.team_name),
            "plan" => Some(&self.plan),
            _ => None,
        }
    }
}

/// One rendered block. The strings are HTML, already escaped.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MdBlock {
    Para(String),
    List(Vec<String>),
}

/// The HTML blocks and the plain-text version of one body.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Rendered {
    pub blocks: Vec<MdBlock>,
    /// Blocks separated by a blank line, no trailing newline.
    pub text: String,
}

const LINK_STYLE: &str = "color:#171717;text-decoration:underline;";

/// Render `md`. Without `vars`, `{{…}}` stays as typed.
pub fn render(md: &str, vars: Option<&Vars>) -> Rendered {
    let md = md.replace("\r\n", "\n").replace('\r', "\n");
    let mut blocks = Vec::new();
    let mut texts = Vec::new();
    // Consecutive non-blank lines, then split into paragraph and list runs.
    let mut run: Vec<&str> = Vec::new();
    let flush = |run: &mut Vec<&str>, blocks: &mut Vec<MdBlock>, texts: &mut Vec<String>| {
        let mut i = 0;
        // Within one run, a list follows its paragraph on the next line.
        let mut run_text = Vec::new();
        while i < run.len() {
            let is_item = |l: &str| l.trim_start().starts_with("- ");
            if is_item(run[i]) {
                let mut html = Vec::new();
                let mut text = Vec::new();
                while i < run.len() && is_item(run[i]) {
                    let item = run[i].trim_start()[2..].trim();
                    let (h, t) = inline_str(item, vars, true);
                    html.push(h);
                    text.push(format!("- {t}"));
                    i += 1;
                }
                blocks.push(MdBlock::List(html));
                run_text.push(text.join("\n"));
            } else {
                let mut html = Vec::new();
                let mut text = Vec::new();
                while i < run.len() && !is_item(run[i]) {
                    let (h, t) = inline_str(run[i].trim(), vars, true);
                    html.push(h);
                    text.push(t);
                    i += 1;
                }
                blocks.push(MdBlock::Para(html.join("<br>")));
                run_text.push(text.join("\n"));
            }
        }
        if !run_text.is_empty() {
            texts.push(run_text.join("\n"));
        }
        run.clear();
    };
    for line in md.split('\n') {
        if line.trim().is_empty() {
            flush(&mut run, &mut blocks, &mut texts);
        } else {
            run.push(line);
        }
    }
    flush(&mut run, &mut blocks, &mut texts);
    Rendered {
        blocks,
        text: texts.join("\n\n"),
    }
}

/// `s` with the variables replaced, as plain text (for a subject).
pub fn substitute(s: &str, vars: Option<&Vars>) -> String {
    let line = Line::new(s);
    let mut out = String::new();
    let mut i = 0;
    while i < line.chars.len() {
        if let Some((value, next)) = line.variable(i, line.chars.len(), vars) {
            out.push_str(value);
            i = next;
        } else {
            out.push(line.chars[i]);
            i += 1;
        }
    }
    out
}

fn inline_str(s: &str, vars: Option<&Vars>, links: bool) -> (String, String) {
    let line = Line::new(s);
    let mut html = String::new();
    let mut text = String::new();
    line.inline(0, line.chars.len(), vars, links, &mut html, &mut text);
    (html, text)
}

/// One line of inline Markdown, with where each marker next occurs, built
/// once. Every lookup is O(1) and each character is consumed by exactly one
/// span, so rendering is linear in the line's length however the markers
/// are (un)balanced.
struct Line {
    chars: Vec<char>,
    /// The first index at or after `i` where `**` starts.
    next_double: Vec<usize>,
    /// The first lone `*` at or after `i`: not part of `**`, not after a space.
    next_single: Vec<usize>,
    /// The first index at or after `i` where `}}` starts.
    next_vclose: Vec<usize>,
    /// The first `)` at or after `i`.
    next_paren: Vec<usize>,
    /// For a `[`, the `]` that closes it, counting nested brackets.
    bracket_close: Vec<Option<usize>>,
}

impl Line {
    fn new(s: &str) -> Self {
        let chars: Vec<char> = s.chars().collect();
        let n = chars.len();
        let at = |j: usize| chars.get(j).copied();
        let mut next_double = vec![n; n + 2];
        let mut next_single = vec![n; n + 2];
        let mut next_vclose = vec![n; n + 2];
        let mut next_paren = vec![n; n + 2];
        for i in (0..n).rev() {
            next_double[i] = if chars[i] == '*' && at(i + 1) == Some('*') {
                i
            } else {
                next_double[i + 1]
            };
            let lone = chars[i] == '*'
                && at(i + 1) != Some('*')
                && (i == 0 || (chars[i - 1] != '*' && !chars[i - 1].is_whitespace()));
            next_single[i] = if lone { i } else { next_single[i + 1] };
            next_vclose[i] = if chars[i] == '}' && at(i + 1) == Some('}') {
                i
            } else {
                next_vclose[i + 1]
            };
            next_paren[i] = if chars[i] == ')' {
                i
            } else {
                next_paren[i + 1]
            };
        }
        let mut bracket_close = vec![None; n];
        let mut open = Vec::new();
        for (i, &c) in chars.iter().enumerate() {
            match c {
                '[' => open.push(i),
                ']' => {
                    if let Some(o) = open.pop() {
                        bracket_close[o] = Some(i);
                    }
                }
                _ => {}
            }
        }
        Self {
            chars,
            next_double,
            next_single,
            next_vclose,
            next_paren,
            bracket_close,
        }
    }

    /// `table[from]` if it is before `end`.
    fn within(table: &[usize], from: usize, end: usize) -> Option<usize> {
        table.get(from).copied().filter(|&j| j < end)
    }

    fn starts_with(&self, i: usize, pat: &str) -> bool {
        (i..)
            .zip(pat.chars())
            .all(|(j, p)| self.chars.get(j) == Some(&p))
    }

    /// The value of a `{{name}}` starting at `i` (closing before `end`), and
    /// the index after it.
    fn variable<'a>(
        &self,
        i: usize,
        end: usize,
        vars: Option<&'a Vars>,
    ) -> Option<(&'a str, usize)> {
        let vars = vars?;
        if !self.starts_with(i, "{{") {
            return None;
        }
        let close = Self::within(&self.next_vclose, i + 2, end)?;
        let name: String = self.chars[i + 2..close].iter().collect();
        Some((vars.get(name.trim())?, close + 2))
    }

    fn inline(
        &self,
        start: usize,
        end: usize,
        vars: Option<&Vars>,
        links: bool,
        html: &mut String,
        text: &mut String,
    ) {
        let chars = &self.chars;
        let mut i = start;
        while i < end {
            if let Some((value, next)) = self.variable(i, end, vars) {
                html.push_str(&esc(value));
                text.push_str(value);
                i = next;
                continue;
            }
            if self.starts_with(i, "**")
                && i + 2 < end
                && !chars[i + 2].is_whitespace()
                && let Some(close) = Self::within(&self.next_double, i + 2, end)
                && close > i + 2
            {
                html.push_str("<strong>");
                self.inline(i + 2, close, vars, links, html, text);
                html.push_str("</strong>");
                i = close + 2;
                continue;
            }
            if chars[i] == '*'
                && i + 1 < end
                && chars[i + 1] != '*'
                && !chars[i + 1].is_whitespace()
                && let Some(close) = Self::within(&self.next_single, i + 1, end)
            {
                html.push_str("<em>");
                self.inline(i + 1, close, vars, links, html, text);
                html.push_str("</em>");
                i = close + 1;
                continue;
            }
            if chars[i] == '['
                && links
                && let Some(close) = self.bracket_close[i].filter(|&c| c < end)
                && close + 1 < end
                && chars[close + 1] == '('
                && let Some(paren) = Self::within(&self.next_paren, close + 2, end)
            {
                let url: String = chars[close + 2..paren].iter().collect();
                if safe_url(&url) {
                    let mut label_html = String::new();
                    let mut label_text = String::new();
                    self.inline(i + 1, close, vars, false, &mut label_html, &mut label_text);
                    html.push_str(&format!(
                        "<a href=\"{}\" style=\"{LINK_STYLE}\">{label_html}</a>",
                        esc(&url)
                    ));
                    if label_text == url {
                        text.push_str(&url);
                    } else {
                        text.push_str(&format!("{label_text} ({url})"));
                    }
                    i = paren + 1;
                    continue;
                }
            }
            html.push_str(&esc_char(chars[i]));
            text.push(chars[i]);
            i += 1;
        }
    }
}

/// Only `https://host…` and `mailto:…`, with nothing that could leave the
/// attribute or hide a scheme.
fn safe_url(url: &str) -> bool {
    let rest = if let Some(rest) = url.strip_prefix("https://") {
        rest
    } else if let Some(rest) = url.strip_prefix("mailto:") {
        rest
    } else {
        return false;
    };
    !rest.is_empty()
        && !rest.starts_with('/')
        && !url.chars().any(|c| {
            c.is_whitespace() || c.is_control() || matches!(c, '"' | '\'' | '<' | '>' | '`' | '\\')
        })
}

fn esc_char(c: char) -> String {
    match c {
        '&' => "&amp;".into(),
        '<' => "&lt;".into(),
        '>' => "&gt;".into(),
        '"' => "&quot;".into(),
        '\'' => "&#39;".into(),
        c => c.to_string(),
    }
}

/// HTML-escape text or an attribute value.
pub fn esc(s: &str) -> String {
    s.chars().map(esc_char).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn html(md: &str) -> String {
        html_with(md, None)
    }

    fn html_with(md: &str, vars: Option<&Vars>) -> String {
        render(md, vars)
            .blocks
            .into_iter()
            .map(|b| match b {
                MdBlock::Para(p) => format!("<p>{p}</p>"),
                MdBlock::List(items) => {
                    let li: String = items.iter().map(|i| format!("<li>{i}</li>")).collect();
                    format!("<ul>{li}</ul>")
                }
            })
            .collect()
    }

    fn vars(first: &str, team: &str) -> Vars {
        Vars {
            first_name: first.into(),
            team_name: team.into(),
            plan: "Team".into(),
        }
    }

    #[test]
    fn markdown_escapes_html() {
        let out = html("<script>alert(1)</script> & \"q\" 'a'");
        assert_eq!(
            out,
            "<p>&lt;script&gt;alert(1)&lt;/script&gt; &amp; &quot;q&quot; &#39;a&#39;</p>"
        );
        assert_eq!(
            render("<script>x</script>", None).text,
            "<script>x</script>"
        );
    }

    #[test]
    fn paragraphs_breaks_bold_italic() {
        assert_eq!(
            html("Hi **there** and *you*\nsecond line\r\n\r\n\n  Next  "),
            "<p>Hi <strong>there</strong> and <em>you</em><br>second line</p><p>Next</p>"
        );
        assert_eq!(
            render("Hi **there** and *you*\nline\n\nNext", None).text,
            "Hi there and you\nline\n\nNext"
        );
        assert_eq!(
            html("*a **b** c*"),
            "<p><em>a <strong>b</strong> c</em></p>"
        );
    }

    #[test]
    fn lists() {
        assert_eq!(
            html("Intro\n- one\n- **two**\n\nAfter"),
            "<p>Intro</p><ul><li>one</li><li><strong>two</strong></li></ul><p>After</p>"
        );
        assert_eq!(
            render("Intro\n- one\n- two\n\nAfter", None).text,
            "Intro\n- one\n- two\n\nAfter"
        );
    }

    #[test]
    fn https_and_mailto_links() {
        let out = html(
            "[Open the dashboard](https://example.com/a?b=1&c=2) or [mail](mailto:hi@example.com)",
        );
        assert!(
            out.contains("<a href=\"https://example.com/a?b=1&amp;c=2\" "),
            "{out}"
        );
        assert!(out.contains(">Open the dashboard</a>"), "{out}");
        assert!(out.contains("<a href=\"mailto:hi@example.com\" "), "{out}");
        assert_eq!(
            render(
                "[Docs](https://example.com) [https://x.io](https://x.io)",
                None
            )
            .text,
            "Docs (https://example.com) https://x.io"
        );
    }

    #[test]
    fn hostile_links_stay_text() {
        for md in [
            "[x](javascript:alert(1))",
            "[x](JAVASCRIPT:alert(1))",
            "[x](http://example.com)",
            "[x](data:text/html,<b>)",
            "[x](https://a\"onmouseover=\"1)",
            "[x](https://a b)",
            "[x](https://)",
            "[x](//example.com)",
        ] {
            let out = html(md);
            assert!(!out.contains("<a"), "{md}: {out}");
            assert!(!out.contains('"') || out.contains("&quot;"), "{md}: {out}");
        }
    }

    #[test]
    fn nested_brackets() {
        let out = html("[a [b] c](https://example.com)");
        assert!(out.contains(">a [b] c</a>"), "{out}");
        // A link inside link text is not a second link.
        let out = html("[[in](https://a.example)](https://b.example)");
        assert_eq!(out.matches("<a ").count(), 1, "{out}");
        assert!(out.contains("href=\"https://b.example\""), "{out}");
    }

    #[test]
    fn unclosed_markers_are_literal() {
        assert_eq!(html("**bold"), "<p>**bold</p>");
        assert_eq!(html("*it"), "<p>*it</p>");
        assert_eq!(html("a ** b"), "<p>a ** b</p>");
        assert_eq!(html("[text](https://x.io"), "<p>[text](https://x.io</p>");
        assert_eq!(html("[text]"), "<p>[text]</p>");
        assert_eq!(html("{{first_name"), "<p>{{first_name</p>");
    }

    #[test]
    fn variables_are_escaped_and_not_markdown() {
        let v = vars("<img src=x>", "**Acme** [x](https://evil.example)");
        let out = html_with("Hi {{first_name}}, {{ team_name }} on {{plan}}", Some(&v));
        assert!(out.contains("Hi &lt;img src=x&gt;,"), "{out}");
        assert!(!out.contains("<strong>") && !out.contains("<a "), "{out}");
        assert!(out.contains("on Team"), "{out}");
        let text = render("Hi {{first_name}}", Some(&v)).text;
        assert_eq!(text, "Hi <img src=x>");
        // Unknown names, and every name without values, stay as typed.
        assert_eq!(html_with("{{nope}}", Some(&v)), "<p>{{nope}}</p>");
        assert_eq!(html("{{first_name}}"), "<p>{{first_name}}</p>");
        assert_eq!(
            substitute("News for {{first_name}}", Some(&vars("Ada", "Acme"))),
            "News for Ada"
        );
        assert_eq!(substitute("{{plan}}", None), "{{plan}}");
    }

    #[test]
    fn pathological_input_renders_quickly() {
        let v = vars("Ada", "Acme");
        let n = 20_000;
        let inputs = [
            "{{".repeat(n / 2),
            "[".repeat(n),
            "[a](".repeat(n / 4),
            "*a".repeat(n / 2),
            "** ".repeat(n / 3),
            "[[[]".repeat(n / 4) + "](https://x.io)",
            "{{plan}".repeat(n / 7),
        ];
        let start = std::time::Instant::now();
        for input in &inputs {
            let out = render(input, Some(&v));
            assert!(!out.text.is_empty());
            let _ = substitute(input, Some(&v));
        }
        let took = start.elapsed();
        assert!(took < std::time::Duration::from_millis(500), "{took:?}");
    }
}
