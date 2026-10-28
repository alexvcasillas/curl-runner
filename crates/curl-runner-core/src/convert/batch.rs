//! Batch script parser: extracts curl commands from shell scripts
//! (`convert/batch-parser.ts`). Heuristic: handles backslash continuations,
//! curl after `&&`, `||`, `;`, `$(` and backticks, and strips pipes.

use crate::jsval::{is_js_space, trim};
use regex::Regex;
use std::sync::LazyLock;

/// `/(?:^|&&\s*|;\s*|\|\|\s*|\$\(\s*|`\s*)curl\s/` with JS `\s`.
static CURL_AFTER_OPERATOR: LazyLock<Regex> = LazyLock::new(|| {
    let ws = r"[\t\n\x0B\f\r \u{a0}\u{1680}\u{2000}-\u{200a}\u{2028}\u{2029}\u{202f}\u{205f}\u{3000}\u{feff}]";
    Regex::new(&format!(
        r"(?:^|&&{ws}*|;{ws}*|\|\|{ws}*|\$\({ws}*|`{ws}*)curl{ws}"
    ))
    .expect("static regex")
});

/// Extract curl command strings from a shell script (`extractCurlCommands`).
pub fn extract_curl_commands(script: &str) -> Vec<String> {
    let mut commands = Vec::new();
    let lines: Vec<&str> = script.split('\n').collect();
    let mut i = 0;

    while i < lines.len() {
        let trimmed = trim(lines[i]);
        if trimmed.is_empty() || trimmed.starts_with('#') || trimmed.starts_with("//") {
            i += 1;
            continue;
        }

        if let Some(start) = find_curl_start(trimmed) {
            let mut cmd = trimmed[start..].to_string();
            while cmd.ends_with('\\') && i + 1 < lines.len() {
                cmd.pop();
                i += 1;
                cmd.push(' ');
                cmd.push_str(trim(lines[i]));
            }

            if let Some(pipe) = find_unquoted_pipe(&cmd).filter(|p| *p > 0) {
                cmd = trim(&cmd[..pipe]).to_string();
            }

            // `.replace(/;\s*$/, '').trim()`
            let stripped = cmd.trim_end_matches(is_js_space);
            let cmd = trim(stripped.strip_suffix(';').unwrap_or(&cmd)).to_string();

            if cmd.encode_utf16().count() > 4 {
                commands.push(cmd);
            }
        }
        i += 1;
    }
    commands
}

/// Index of `curl` at the start of a line or after a shell operator.
fn find_curl_start(line: &str) -> Option<usize> {
    if line.starts_with("curl ") || line == "curl" {
        return Some(0);
    }
    let m = CURL_AFTER_OPERATOR.find(line)?;
    line[m.start()..].find("curl").map(|o| m.start() + o)
}

/// Byte index of the first pipe outside quotes.
fn find_unquoted_pipe(s: &str) -> Option<usize> {
    let (mut single, mut double) = (false, false);
    let mut it = s.char_indices();
    while let Some((i, ch)) = it.next() {
        match ch {
            '\\' if !single => {
                it.next();
            }
            '\'' if !double => single = !single,
            '"' if !single => double = !double,
            '|' if !single && !double => return Some(i),
            _ => {}
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::CURL_AFTER_OPERATOR;
    use super::extract_curl_commands as x;

    #[test]
    fn source_has_no_invisible_unicode_literals() {
        assert!(include_str!("batch.rs").is_ascii());
    }

    #[test]
    fn curl_after_operator_takes_every_js_space() {
        for c in [
            '\t', '\n', '\u{b}', '\u{c}', '\r', ' ', '\u{a0}', '\u{1680}', '\u{2000}', '\u{2005}',
            '\u{200a}', '\u{2028}', '\u{2029}', '\u{202f}', '\u{205f}', '\u{3000}', '\u{feff}',
        ] {
            assert!(
                CURL_AFTER_OPERATOR.is_match(&format!("a &&{c}curl{c}x")),
                "U+{:04X}",
                c as u32
            );
        }
        assert!(!CURL_AFTER_OPERATOR.is_match("a &&\u{85}curl x"));
        assert!(!CURL_AFTER_OPERATOR.is_match("a &&\u{200b}curl x"));
    }

    #[test]
    fn extracts_curl_from_script() {
        let c = x(
            "#!/bin/bash\n# API tests\ncurl https://api.example.com/health\ncurl -X POST https://api.example.com/users -d '{\"name\":\"test\"}'\n",
        );
        assert_eq!(c.len(), 2);
        assert!(c[0].contains("https://api.example.com/health"));
        assert!(c[1].contains("-X POST"));
    }

    #[test]
    fn handles_multiline_curl_commands() {
        let c = x(
            "curl -X POST \\\n  -H \"Content-Type: application/json\" \\\n  -d '{\"test\":true}' \\\n  https://api.example.com",
        );
        assert_eq!(c.len(), 1);
        assert!(c[0].contains("-X POST") && c[0].contains("https://api.example.com"));
    }

    #[test]
    fn skips_comments_and_empty_lines() {
        assert_eq!(
            x("# This is a comment\n\n# Another comment\ncurl https://example.com\n").len(),
            1
        );
    }

    #[test]
    fn strips_piped_output() {
        let c = x("curl https://api.example.com | jq .");
        assert_eq!(c.len(), 1);
        assert!(!c[0].contains("jq"));
    }

    #[test]
    fn handles_curl_after_operator() {
        let c = x("echo \"test\" && curl https://example.com");
        assert_eq!(c.len(), 1);
        assert!(c[0].contains("https://example.com"));
    }

    #[test]
    fn strips_trailing_semicolon_and_handles_subshell() {
        assert_eq!(x("curl https://a.com;"), ["curl https://a.com"]);
        assert_eq!(x("R=$(curl https://a.com)"), ["curl https://a.com)"]);
    }
}
