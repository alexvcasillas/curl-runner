//! URL rules (TS `validateURL`, `validateProxyURL`).

use super::issues::{Ctx, Fix, FixValue, Severity};
use crate::jsval::parse_int;
use crate::model::Json;

/// TS `URL_TYPOS`, in order (the first match wins).
const URL_TYPOS: &[(&str, &str)] = &[
    ("htpp", "http"),
    ("htps", "https"),
    ("htp", "http"),
    ("http:/", "http://"),
    ("https:/", "https://"),
    ("locahost", "localhost"),
    ("localhsot", "localhost"),
    ("loaclhost", "localhost"),
    ("localost", "localhost"),
    ("127.0.0.0", "127.0.0.1"),
];

fn set(desc: impl Into<String>, v: String) -> Option<Fix> {
    Some(Fix {
        description: desc.into(),
        value: FixValue::Set(Json::String(v)),
    })
}

/// `new RegExp(typo, 'gi')` matches at byte `i` (typos are ASCII; `.` is a
/// regex wildcard for any char but a line terminator). Returns the match end.
fn typo_match_at(url: &str, i: usize, typo: &str) -> Option<usize> {
    let mut chars = url[i..].char_indices();
    for t in typo.bytes() {
        let (_, c) = chars.next()?;
        let ok = if t == b'.' {
            !matches!(c, '\n' | '\r' | '\u{2028}' | '\u{2029}')
        } else {
            c.is_ascii() && (c as u8).eq_ignore_ascii_case(&t)
        };
        if !ok {
            return None;
        }
    }
    Some(chars.next().map_or(url.len(), |(j, _)| i + j))
}

/// `url.replace(new RegExp(typo, 'gi'), fix)`.
fn replace_typo(url: &str, typo: &str, fix: &str) -> String {
    let mut out = String::new();
    let mut i = 0;
    while i < url.len() {
        if let Some(end) = typo_match_at(url, i, typo) {
            out.push_str(fix);
            i = end;
        } else {
            let c = url[i..].chars().next().unwrap_or_default();
            out.push(c);
            i += c.len_utf8();
        }
    }
    out
}

/// `/{typo}(?!\/)/i` for `http:/` and `https:/`.
fn has_unslashed(url: &str, typo: &str) -> bool {
    (0..url.len()).any(|i| {
        url.is_char_boundary(i)
            && typo_match_at(url, i, typo).is_some_and(|end| !url[end..].starts_with('/'))
    })
}

/// At byte `i`: `https?:\/\/[^/]+` followed by one or more `/`. Returns the
/// end of the host part and the end of the slash run.
fn scheme_host_slashes(url: &str, i: usize) -> Option<(usize, usize)> {
    let rest = &url[i..];
    let scheme = if rest.starts_with("https://") {
        8
    } else if rest.starts_with("http://") {
        7
    } else {
        return None;
    };
    let host_start = i + scheme;
    let host_len = url[host_start..].find('/')?;
    if host_len == 0 {
        return None;
    }
    let host_end = host_start + host_len;
    let slashes = url[host_end..].bytes().take_while(|b| *b == b'/').count();
    Some((host_end, host_end + slashes))
}

/// `/https?:\/\/[^/]+\/\/+/` test.
fn has_double_slash(url: &str) -> bool {
    (0..url.len()).any(|i| {
        url.is_char_boundary(i) && scheme_host_slashes(url, i).is_some_and(|(h, e)| e - h >= 2)
    })
}

/// `url.replace(/(https?:\/\/[^/]+)(\/+)/g, '$1/')`.
fn collapse_slashes(url: &str) -> String {
    let mut out = String::new();
    let mut i = 0;
    while i < url.len() {
        if let Some((h, e)) = scheme_host_slashes(url, i) {
            out.push_str(&url[i..h]);
            out.push('/');
            i = e;
        } else {
            let c = url[i..].chars().next().unwrap_or_default();
            out.push(c);
            i += c.len_utf8();
        }
    }
    out
}

/// `url.match(/:(\d+)/)` → `parseInt(digits)`.
fn port(url: &str) -> Option<f64> {
    let b = url.as_bytes();
    (0..b.len()).find_map(|i| {
        (b[i] == b':' && b.get(i + 1).is_some_and(u8::is_ascii_digit))
            .then(|| parse_int(&url[i + 1..]))
    })
}

pub fn validate_url(cx: &mut Ctx, url: &Json, path: &str) {
    let Some(url) = url.as_str() else {
        cx.add(path, Severity::Error, "URL must be a string");
        return;
    };
    let has_vars = url.contains("${");

    let mut has_typo = false;
    let lower = url.to_lowercase();
    for (typo, fix) in URL_TYPOS {
        let matches = if typo.ends_with(":/") {
            has_unslashed(url, typo)
        } else {
            lower.contains(typo)
        };
        if matches {
            has_typo = true;
            cx.add_fix(
                path,
                Severity::Error,
                format!("URL contains typo \"{typo}\""),
                set(format!("Fix to \"{fix}\""), replace_typo(url, typo, fix)),
            );
            break;
        }
    }

    if !has_typo && !url.starts_with("http") && !has_vars {
        cx.add_fix(
            path,
            Severity::Warning,
            "URL should start with http:// or https://",
            set("Prepend https://", format!("https://{url}")),
        );
    }

    if !has_vars && has_double_slash(url) {
        cx.add_fix(
            path,
            Severity::Warning,
            "URL contains double slashes in path",
            set("Remove extra slashes", collapse_slashes(url)),
        );
    }

    if url.contains(' ') {
        cx.add_fix(
            path,
            Severity::Error,
            "URL contains spaces",
            set("Encode spaces as %20", url.replace(' ', "%20")),
        );
    }

    if url.starts_with("http://") && !url.contains("localhost") && !url.contains("127.0.0.1") {
        cx.add(
            path,
            Severity::Info,
            "Using HTTP instead of HTTPS; consider using HTTPS for security",
        );
    }

    if !has_vars
        && let Some(p) = port(url)
        && !(1.0..=65535.0).contains(&p)
    {
        cx.add(
            path,
            Severity::Error,
            format!(
                "Invalid port number {}. Must be 1-65535",
                crate::jsval::num_to_string(p)
            ),
        );
    }
}

pub fn validate_proxy_url(cx: &mut Ctx, proxy: &Json, path: &str) {
    let Some(proxy) = proxy.as_str() else {
        cx.add(path, Severity::Error, "Proxy must be a string URL");
        return;
    };
    let lower = proxy.to_ascii_lowercase();
    let ok = ["http://", "https://", "socks://", "socks4://", "socks5://"]
        .iter()
        .any(|p| lower.starts_with(p));
    if !proxy.contains("${") && !ok {
        cx.add(
            path,
            Severity::Error,
            "Proxy URL should start with http://, https://, socks4://, or socks5://",
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn regex_helpers() {
        assert_eq!(
            replace_typo("HTPP://a/htpp", "htpp", "http"),
            "http://a/http"
        );
        assert_eq!(
            replace_typo("127x0y0z0", "127.0.0.0", "127.0.0.1"),
            "127.0.0.1"
        );
        assert!(has_unslashed("http:/a", "http:/"));
        assert!(!has_unslashed("http://a", "http:/"));
        assert!(has_unslashed("HTTP://a/http:/b", "http:/"));
        assert!(has_double_slash("https://a.com//b"));
        assert!(!has_double_slash("https://a.com/b//c"));
        assert!(has_double_slash("x http://h///"));
        assert_eq!(
            collapse_slashes("https://a.com//b//c"),
            "https://a.com/b//c"
        );
        assert_eq!(
            collapse_slashes("http://a//x http://b///y"),
            "http://a/x http://b/y"
        );
        assert_eq!(port("https://a.com:x:080/"), Some(80.0));
        assert_eq!(port("https://a.com"), None);
    }
}
