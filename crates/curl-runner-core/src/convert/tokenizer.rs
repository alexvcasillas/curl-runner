//! Shell-safe tokenizer for curl commands (`convert/tokenizer.ts`): single
//! quotes, double quotes, escapes and backslash line continuations.

/// Tokenize a shell command string.
pub fn tokenize(input: &str) -> Vec<String> {
    let c: Vec<char> = input.chars().collect();
    let len = c.len();
    let mut tokens = Vec::new();
    let mut cur = String::new();
    let mut i = 0;

    while i < len {
        let ch = c[i];

        if ch == '\\' && i + 1 < len && (c[i + 1] == '\n' || c[i + 1] == '\r') {
            i += 2;
            if i < len && c[i] == '\n' {
                i += 1;
            }
            while i < len && (c[i] == ' ' || c[i] == '\t') {
                i += 1;
            }
            continue;
        }

        if matches!(ch, ' ' | '\t' | '\n' | '\r') {
            if !cur.is_empty() {
                tokens.push(std::mem::take(&mut cur));
            }
            i += 1;
            continue;
        }

        if ch == '\'' {
            i += 1;
            while i < len && c[i] != '\'' {
                cur.push(c[i]);
                i += 1;
            }
            if i < len {
                i += 1;
            }
            continue;
        }

        if ch == '"' {
            i += 1;
            while i < len && c[i] != '"' {
                if c[i] == '\\' && i + 1 < len {
                    let next = c[i + 1];
                    if matches!(next, '"' | '\\' | '$' | '`' | '\n') {
                        if next != '\n' {
                            cur.push(next);
                        }
                        i += 2;
                        continue;
                    }
                }
                cur.push(c[i]);
                i += 1;
            }
            if i < len {
                i += 1;
            }
            continue;
        }

        if ch == '\\' && i + 1 < len {
            cur.push(c[i + 1]);
            i += 2;
            continue;
        }

        cur.push(ch);
        i += 1;
    }

    if !cur.is_empty() {
        tokens.push(cur);
    }
    tokens
}

#[cfg(test)]
mod tests {
    use super::tokenize;

    fn t(s: &str) -> Vec<String> {
        tokenize(s)
    }

    #[test]
    fn simple_command() {
        assert_eq!(
            t("curl https://example.com"),
            ["curl", "https://example.com"]
        );
    }

    #[test]
    fn single_quoted_strings() {
        assert_eq!(
            t("curl -d '{\"key\":\"value\"}'"),
            ["curl", "-d", "{\"key\":\"value\"}"]
        );
    }

    #[test]
    fn double_quoted_strings() {
        assert_eq!(
            t("curl -H \"Content-Type: application/json\""),
            ["curl", "-H", "Content-Type: application/json"]
        );
    }

    #[test]
    fn escaped_characters_in_double_quotes() {
        assert_eq!(
            t("curl -d \"hello \\\"world\\\"\""),
            ["curl", "-d", "hello \"world\""]
        );
    }

    #[test]
    fn backslash_line_continuation() {
        assert_eq!(
            t("curl \\\n  -X POST \\\n  https://example.com"),
            ["curl", "-X", "POST", "https://example.com"]
        );
    }

    #[test]
    fn mixed_quoting() {
        assert_eq!(
            t("curl -H 'Accept: */*' -d \"test\""),
            ["curl", "-H", "Accept: */*", "-d", "test"]
        );
    }

    #[test]
    fn empty_input() {
        assert!(t("").is_empty());
    }

    #[test]
    fn tabs_and_multiple_spaces() {
        assert_eq!(
            t("curl   -X   GET\thttps://example.com"),
            ["curl", "-X", "GET", "https://example.com"]
        );
    }

    #[test]
    fn backslash_escape_outside_quotes() {
        assert_eq!(t("curl hello\\ world"), ["curl", "hello world"]);
    }

    #[test]
    fn windows_line_endings_in_continuation() {
        assert_eq!(t("curl \\\r\n  -X POST"), ["curl", "-X", "POST"]);
    }

    #[test]
    fn adjacent_quoted_segments() {
        assert_eq!(t("curl 'hello'\"world\""), ["curl", "helloworld"]);
    }

    #[test]
    fn ansi_c_quotes_are_not_special() {
        assert_eq!(t("curl -d $'a\\nb'"), ["curl", "-d", "$a\\nb"]);
    }
}
