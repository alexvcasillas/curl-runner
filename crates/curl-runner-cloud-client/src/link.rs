//! Share links: the `curl-runner://open` deep link and the landing page URL.

use crate::client::{Client, check_server};
use serde::Deserialize;

const HOSTED_LANDING_HOST: &str = "cloud.curl-runner.com";
const HOSTED_API: &str = "https://api.curl-runner.com";

/// A parsed link: the API base to call, or a landing page that must be asked
/// where its API is.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Link {
    Api { server: String, token: String },
    Landing { origin: String, token: String },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LinkError {
    /// Not a share link, or a link to an insecure server.
    Malformed,
    /// The link names a server that is plain http and not loopback.
    InsecureServer(String),
    /// The landing page didn't say where its API is.
    NoApi,
}

impl std::fmt::Display for LinkError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            LinkError::Malformed => "That isn't a curl-runner share link.",
            LinkError::InsecureServer(_) => "That link points to a server that isn't https.",
            LinkError::NoApi => "This link's server didn't say where its API is.",
        })
    }
}

impl std::error::Error for LinkError {}

/// A share token goes into a URL path, so it is plain URL-safe characters.
pub fn valid_token(t: &str) -> bool {
    !t.is_empty()
        && t.len() <= 256
        && t.chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
}

/// Parse a deep link (`curl-runner://open[/team/collection]?server=&share=`)
/// or a landing URL (`https://host/s/{token}`).
pub fn parse(text: &str) -> Result<Link, LinkError> {
    let u = url::Url::parse(text).map_err(|_| LinkError::Malformed)?;
    if u.scheme() == "curl-runner" {
        if u.host_str() != Some("open") {
            return Err(LinkError::Malformed);
        }
        let get = |k: &str| {
            u.query_pairs()
                .find(|(name, _)| name == k)
                .map(|(_, v)| v.into_owned())
        };
        let (Some(server), Some(token)) = (get("server"), get("share")) else {
            return Err(LinkError::Malformed);
        };
        let server = server.trim_end_matches('/').to_string();
        check_server(&server).map_err(|_| LinkError::InsecureServer(server.clone()))?;
        if !valid_token(&token) {
            return Err(LinkError::Malformed);
        }
        return Ok(Link::Api { server, token });
    }
    check_server(text).map_err(|_| LinkError::InsecureServer(text.to_string()))?;
    let segments: Vec<&str> = u
        .path_segments()
        .map(|s| s.filter(|p| !p.is_empty()).collect())
        .unwrap_or_default();
    let ["s", token] = segments.as_slice() else {
        return Err(LinkError::Malformed);
    };
    if !valid_token(token) {
        return Err(LinkError::Malformed);
    }
    let host = u.host_str().ok_or(LinkError::Malformed)?;
    let origin = match u.port() {
        Some(p) if host.contains(':') => format!("{}://[{host}]:{p}", u.scheme()),
        Some(p) => format!("{}://{host}:{p}", u.scheme()),
        None if host.contains(':') => format!("{}://[{host}]", u.scheme()),
        None => format!("{}://{host}", u.scheme()),
    };
    Ok(Link::Landing {
        origin,
        token: token.to_string(),
    })
}

#[derive(Deserialize)]
struct WellKnown {
    api: String,
}

/// The API base for a landing page: the test hook, else the host's
/// `/.well-known/curl-runner.json` (no redirects), else, for the hosted
/// landing host only, the hosted API.
pub fn api_for_landing(origin: &str, test_hook: Option<&str>) -> Result<String, LinkError> {
    if let Some(s) = test_hook {
        let s = s.trim_end_matches('/').to_string();
        check_server(&s).map_err(|_| LinkError::InsecureServer(s.clone()))?;
        return Ok(s);
    }
    let host = url::Url::parse(origin)
        .ok()
        .and_then(|u| u.host_str().map(str::to_string))
        .unwrap_or_default();
    let found = Client::new(origin, None)
        .send("GET", "/.well-known/curl-runner.json", None, false)
        .ok()
        .filter(|(status, _)| *status == 200)
        .and_then(|(_, body)| serde_json::from_slice::<WellKnown>(&body).ok())
        .map(|w| w.api.trim_end_matches('/').to_string())
        .filter(|api| !api.chars().any(char::is_control) && check_server(api).is_ok());
    match found {
        Some(api) => Ok(api),
        None if host == HOSTED_LANDING_HOST => Ok(HOSTED_API.to_string()),
        None => Err(LinkError::NoApi),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use testkit::{Response, TestServer};

    #[test]
    fn parse_accepts_both_deep_link_forms() {
        let want = Link::Api {
            server: "https://api.x.test".into(),
            token: "crsl_Ab-9_".into(),
        };
        for text in [
            "curl-runner://open?server=https%3A%2F%2Fapi.x.test&share=crsl_Ab-9_",
            "curl-runner://open/acme/pay?share=crsl_Ab-9_&server=https%3A%2F%2Fapi.x.test",
        ] {
            assert_eq!(parse(text).unwrap(), want, "{text}");
        }
    }

    #[test]
    fn parse_landing() {
        assert_eq!(
            parse("https://cloud.x.test/s/crsl_Ab-9_").unwrap(),
            Link::Landing {
                origin: "https://cloud.x.test".into(),
                token: "crsl_Ab-9_".into()
            }
        );
        assert_eq!(
            parse("http://127.0.0.1:9/s/tok/").unwrap(),
            Link::Landing {
                origin: "http://127.0.0.1:9".into(),
                token: "tok".into()
            }
        );
    }

    #[test]
    fn parse_rejects_bad_tokens() {
        let long = "a".repeat(257);
        let ok = "a".repeat(256);
        assert!(parse(&format!("https://h.x.test/s/{ok}")).is_ok());
        for bad in [
            format!("https://h.x.test/s/{long}"),
            "https://h.x.test/s/a/b".to_string(),
            "https://h.x.test/s/".to_string(),
            format!("curl-runner://open?server=https%3A%2F%2Fapi.x.test&share={long}"),
            "curl-runner://open?server=https%3A%2F%2Fapi.x.test&share=".to_string(),
            "curl-runner://close?server=https%3A%2F%2Fapi.x.test&share=t".to_string(),
            "nonsense".to_string(),
        ] {
            assert_eq!(parse(&bad), Err(LinkError::Malformed), "{bad}");
        }
    }

    #[test]
    fn insecure_server_is_checked_in_the_old_order() {
        // Not a share link at all: Malformed wins over the server check.
        for text in [
            "curl-runner://open?server=http://evil.example.com",
            "curl-runner://close?server=http://evil.example.com&share=t",
        ] {
            assert_eq!(parse(text), Err(LinkError::Malformed), "{text}");
        }
        assert_eq!(
            parse("curl-runner://open?server=http%3A%2F%2Fevil.example.com&share=t"),
            Err(LinkError::InsecureServer("http://evil.example.com".into()))
        );
        assert_eq!(
            parse("http://h.example.com/s/tok"),
            Err(LinkError::InsecureServer(
                "http://h.example.com/s/tok".into()
            ))
        );
    }

    #[test]
    fn landing_without_well_known_is_no_api() {
        let srv =
            TestServer::with_handler(|_| Response::from_data(b"{}".to_vec()).with_status_code(404));
        assert_eq!(api_for_landing(&srv.base_url, None), Err(LinkError::NoApi));
    }

    #[test]
    fn landing_reads_well_known_and_honors_the_hook() {
        let srv = TestServer::with_handler(|r| {
            assert_eq!(r.url, "/.well-known/curl-runner.json");
            Response::from_data(br#"{"api":"https://api.x.test/"}"#.to_vec())
        });
        assert_eq!(
            api_for_landing(&srv.base_url, None).unwrap(),
            "https://api.x.test"
        );
        assert_eq!(
            api_for_landing("https://nowhere.invalid", Some("http://127.0.0.1:5/")).unwrap(),
            "http://127.0.0.1:5"
        );
    }

    #[test]
    fn messages_match_the_studio_copy() {
        assert_eq!(
            LinkError::Malformed.to_string(),
            "That isn't a curl-runner share link."
        );
        assert_eq!(
            LinkError::NoApi.to_string(),
            "This link's server didn't say where its API is."
        );
    }
}
