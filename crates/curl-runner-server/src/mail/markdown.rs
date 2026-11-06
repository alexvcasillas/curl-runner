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
    let chars: Vec<char> = s.chars().collect();
    let mut out = String::new();
    let mut i = 0;
    while i < chars.len() {
        if let Some((value, next)) = variable(&chars, i, vars) {
            out.push_str(value);
            i = next;
        } else {
            out.push(chars[i]);
            i += 1;
        }
    }
    out
}

fn inline_str(s: &str, vars: Option<&Vars>, links: bool) -> (String, String) {
    let chars: Vec<char> = s.chars().collect();
    let mut html = String::new();
    let mut text = String::new();
    inline(&chars, vars, links, &mut html, &mut text);
    (html, text)
}

/// The value of a `{{name}}` starting at `i`, and the index after it.
fn variable<'a>(chars: &[char], i: usize, vars: Option<&'a Vars>) -> Option<(&'a str, usize)> {
    let vars = vars?;
    if !starts_with(chars, i, "{{") {
        return None;
    }
    let close = find(chars, i + 2, "}}")?;
    let name: String = chars[i + 2..close].iter().collect();
    Some((vars.get(name.trim())?, close + 2))
}

fn starts_with(chars: &[char], i: usize, pat: &str) -> bool {
    (i..)
        .zip(pat.chars())
        .all(|(j, p)| chars.get(j) == Some(&p))
}

/// The first index at or after `from` where `pat` starts.
fn find(chars: &[char], from: usize, pat: &str) -> Option<usize> {
    (from..chars.len()).find(|&j| starts_with(chars, j, pat))
}

/// A lone `*` at or after `from`: not part of `**`, not right after a space.
fn find_single_star(chars: &[char], from: usize) -> Option<usize> {
    (from..chars.len()).find(|&j| {
        chars[j] == '*'
            && chars.get(j + 1) != Some(&'*')
            && (j == 0 || chars[j - 1] != '*')
            && (j == 0 || !chars[j - 1].is_whitespace())
    })
}

/// The `]` closing the `[` at `open`, counting nested brackets.
fn matching_bracket(chars: &[char], open: usize) -> Option<usize> {
    let mut depth = 0usize;
    for (j, &c) in chars.iter().enumerate().skip(open) {
        match c {
            '[' => depth += 1,
            ']' => {
                depth -= 1;
                if depth == 0 {
                    return Some(j);
                }
            }
            _ => {}
        }
    }
    None
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

fn inline(chars: &[char], vars: Option<&Vars>, links: bool, html: &mut String, text: &mut String) {
    let mut i = 0;
    while i < chars.len() {
        if let Some((value, next)) = variable(chars, i, vars) {
            html.push_str(&esc(value));
            text.push_str(value);
            i = next;
            continue;
        }
        if starts_with(chars, i, "**")
            && let Some(close) = find(chars, i + 2, "**")
            && close > i + 2
            && !chars[i + 2].is_whitespace()
        {
            html.push_str("<strong>");
            inline(&chars[i + 2..close], vars, links, html, text);
            html.push_str("</strong>");
            i = close + 2;
            continue;
        }
        if chars[i] == '*'
            && chars
                .get(i + 1)
                .is_some_and(|c| *c != '*' && !c.is_whitespace())
            && let Some(close) = find_single_star(chars, i + 1)
        {
            html.push_str("<em>");
            inline(&chars[i + 1..close], vars, links, html, text);
            html.push_str("</em>");
            i = close + 1;
            continue;
        }
        if chars[i] == '['
            && links
            && let Some(close) = matching_bracket(chars, i)
            && chars.get(close + 1) == Some(&'(')
            && let Some(end) = (close + 2..chars.len()).find(|&j| chars[j] == ')')
        {
            let url: String = chars[close + 2..end].iter().collect();
            if safe_url(&url) {
                let mut label_html = String::new();
                let mut label_text = String::new();
                inline(
                    &chars[i + 1..close],
                    vars,
                    false,
                    &mut label_html,
                    &mut label_text,
                );
                html.push_str(&format!(
                    "<a href=\"{}\" style=\"{LINK_STYLE}\">{label_html}</a>",
                    esc(&url)
                ));
                if label_text == url {
                    text.push_str(&url);
                } else {
                    text.push_str(&format!("{label_text} ({url})"));
                }
                i = end + 1;
                continue;
            }
        }
        html.push_str(&esc_char(chars[i]));
        text.push(chars[i]);
        i += 1;
    }
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
}
