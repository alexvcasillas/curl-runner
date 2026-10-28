//! URL protocol allow-list guard (port of `core/security/url-guard.ts`).

use super::GuardError;

/// Check `url`'s scheme against `allowed_protocols`. Call after
/// interpolation so store-injected URLs are also checked.
pub fn check(url: &str, allowed_protocols: &[String]) -> Result<(), GuardError> {
    let parsed =
        url::Url::parse(url).map_err(|_| GuardError::new(format!("Invalid URL: \"{url}\"")))?;
    // `Url::scheme` is already lowercase (WHATWG), like `URL.protocol`.
    let proto = parsed.scheme().to_ascii_lowercase();
    if !allowed_protocols.contains(&proto) {
        return Err(GuardError::new(format!(
            "Blocked protocol \"{proto}\" (allowed: {}). Use --allow-protocol to permit it.",
            allowed_protocols.join(", ")
        )));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::DEFAULT_ALLOWED_PROTOCOLS;

    fn def() -> Vec<String> {
        DEFAULT_ALLOWED_PROTOCOLS
            .iter()
            .map(|s| s.to_string())
            .collect()
    }
    fn sv(v: &[&str]) -> Vec<String> {
        v.iter().map(|s| s.to_string()).collect()
    }
    fn err(url: &str, allowed: &[String]) -> String {
        check(url, allowed).unwrap_err().to_string()
    }

    #[test]
    fn default_protocols_are_http_and_https() {
        assert_eq!(def(), sv(&["http", "https"]));
    }

    #[test]
    fn allowed_protocols_pass() {
        for u in [
            "http://example.com/api",
            "https://api.example.com/users",
            "https://localhost:3000/health",
            "http://example.com/search?q=test&limit=10",
        ] {
            assert_eq!(check(u, &def()), Ok(()), "{u}");
        }
    }

    #[test]
    fn blocked_protocols_are_rejected() {
        for (u, p) in [
            ("file:///etc/passwd", "file"),
            ("gopher://evil.example.com/", "gopher"),
            ("ftp://files.example.com/data.txt", "ftp"),
            ("dict://attacker.example.com/", "dict"),
            ("scp://user@host:/path", "scp"),
        ] {
            let e = err(u, &def());
            assert!(e.contains(&format!("Blocked protocol \"{p}\"")), "{e}");
        }
    }

    #[test]
    fn blocked_message_is_exact() {
        assert_eq!(
            err("ftp://example.com", &def()),
            "Blocked protocol \"ftp\" (allowed: http, https). Use --allow-protocol to permit it."
        );
    }

    #[test]
    fn custom_allowed_protocols() {
        assert_eq!(
            check(
                "ftp://files.example.com/data.txt",
                &sv(&["http", "https", "ftp"])
            ),
            Ok(())
        );
        let e = err("http://example.com", &sv(&["https"]));
        assert!(e.contains("Blocked protocol \"http\""), "{e}");
        assert!(e.contains("allowed: https"), "{e}");
        assert_eq!(check("https://example.com", &sv(&["https"])), Ok(()));
    }

    #[test]
    fn scheme_is_lowercased() {
        assert_eq!(check("HTTPS://example.com", &def()), Ok(()));
    }

    #[test]
    fn malformed_urls_are_rejected() {
        for u in [
            "",
            "example.com/api",
            "http://exa mple.com",
            "https://",
            "not-a-url",
        ] {
            let e = err(u, &def());
            assert_eq!(e, format!("Invalid URL: \"{u}\""));
        }
    }
}
