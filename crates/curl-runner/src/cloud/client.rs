//! HTTP client for the curl-runner Cloud API, over libcurl with the same TLS
//! and redirect setup as the update paths (`net::easy_for`).

use super::Env;
use crate::net;
use crate::version_check::{HOOKS_ENABLED, hook};
use curl::easy::List;
use curl_runner_cloud_proto::{ErrorBody, ErrorCode, Plan, Role};
use flate2::{Compression, write::GzEncoder};
use serde::{Deserialize, Serialize, de::DeserializeOwned};
use std::io::Write;
use std::time::Duration;

pub const DEFAULT_SERVER: &str = "https://api.curl-runner.com";
pub const SERVER_ENV: &str = "CURL_RUNNER_SERVER";
pub const TEST_SERVER_ENV: &str = "CURL_RUNNER_TEST_SERVER";
const TIMEOUT: Duration = Duration::from_secs(30);
/// A publish can be large on a slow uplink: a long total limit, but give up
/// when the transfer stalls.
const UPLOAD_TIMEOUT: Duration = Duration::from_secs(300);
const MAX_RESPONSE: usize = 64 << 20;

/// A transport failure (no HTTP response).
#[derive(Debug, PartialEq, Eq)]
pub struct NetError(pub String);

impl std::fmt::Display for NetError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

/// One HTTP answer.
pub struct Reply {
    pub status: u16,
    pub body: Vec<u8>,
    /// `Retry-After` in seconds, when the server sent one as a number.
    pub retry_after: Option<u64>,
}

#[derive(Debug, PartialEq)]
pub enum CloudError {
    /// The server answered with an error body and this status.
    Api(ErrorBody, u16),
    /// No usable answer: unreachable, or a reply that is not the API's.
    Unreachable(String),
}

/// The API base URL: `--server`, else the test hook (debug and `test-hooks`
/// builds), else `CURL_RUNNER_SERVER`, else the hosted service.
pub fn resolve_server(flag: Option<&str>, env: Env) -> String {
    let url = flag
        .map(str::to_string)
        .or_else(|| hook(env, TEST_SERVER_ENV, HOOKS_ENABLED))
        .or_else(|| env(SERVER_ENV).filter(|s| !s.is_empty()))
        .unwrap_or_else(|| DEFAULT_SERVER.to_string());
    url.trim_end_matches('/').to_string()
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)] // id and name are read by publish and pull
pub struct User {
    pub id: String,
    pub email: String,
    pub name: Option<String>,
}

#[derive(Deserialize)]
pub struct Verified {
    pub session: String,
    pub user: User,
}

impl std::fmt::Debug for Verified {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Verified")
            .field("session", &"<redacted>")
            .field("user", &self.user)
            .finish()
    }
}

/// Plain http is only for a loopback host.
pub fn check_server(url: &str) -> Result<(), String> {
    const MSG: &str = "Use an https:// server. Plain http is only allowed for localhost.";
    let u = url::Url::parse(url).map_err(|_| format!("{url} is not a valid server URL."))?;
    match u.scheme() {
        "https" => Ok(()),
        "http" => match u.host() {
            Some(url::Host::Domain("localhost")) => Ok(()),
            Some(url::Host::Ipv4(a)) if a.is_loopback() => Ok(()),
            Some(url::Host::Ipv6(a)) if a.is_loopback() => Ok(()),
            _ => Err(MSG.into()),
        },
        _ => Err(MSG.into()),
    }
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)] // used by publish and pull
pub struct TeamInfo {
    pub slug: String,
    pub name: String,
    pub role: Role,
    pub plan: Plan,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)] // used by publish and pull
pub struct Me {
    pub user: User,
    pub teams: Vec<TeamInfo>,
}

pub struct Client {
    base: String,
    auth: Option<String>,
}

impl Client {
    pub fn new(base: &str, auth: Option<String>) -> Client {
        Client {
            base: base.trim_end_matches('/').to_string(),
            auth,
        }
    }

    /// One request. `gzip` compresses `body` and labels it. Any HTTP status
    /// is a success here; callers decide what a status means.
    pub fn send(
        &self,
        method: &str,
        path: &str,
        body: Option<&[u8]>,
        gzip: bool,
    ) -> Result<(u16, Vec<u8>), NetError> {
        self.send_full(method, path, body, gzip)
            .map(|r| (r.status, r.body))
    }

    /// [`Client::send`] that also reports the `Retry-After` header.
    pub fn send_full(
        &self,
        method: &str,
        path: &str,
        body: Option<&[u8]>,
        gzip: bool,
    ) -> Result<Reply, NetError> {
        let url = format!("{}{}", self.base, path);
        let e = |e: curl::Error| NetError(e.to_string());
        let upload = gzip && body.is_some();
        let mut easy =
            net::easy_for(&url, if upload { UPLOAD_TIMEOUT } else { TIMEOUT }).map_err(e)?;
        if upload {
            easy.low_speed_limit(1024).map_err(e)?;
            easy.low_speed_time(Duration::from_secs(30)).map_err(e)?;
        }
        // Never follow a redirect: it would replay the body and the token.
        easy.follow_location(false).map_err(e)?;
        easy.custom_request(method).map_err(e)?;
        let mut headers = List::new();
        headers.append("Accept: application/json").map_err(e)?;
        if let Some(token) = &self.auth {
            headers
                .append(&format!("Authorization: Bearer {token}"))
                .map_err(e)?;
        }
        let payload = match body {
            Some(b) => {
                headers
                    .append("Content-Type: application/json")
                    .map_err(e)?;
                if gzip {
                    headers.append("Content-Encoding: gzip").map_err(e)?;
                    let mut gz = GzEncoder::new(Vec::new(), Compression::default());
                    gz.write_all(b).map_err(|x| NetError(x.to_string()))?;
                    Some(gz.finish().map_err(|x| NetError(x.to_string()))?)
                } else {
                    Some(b.to_vec())
                }
            }
            None => None,
        };
        if let Some(p) = &payload {
            easy.post_fields_copy(p).map_err(e)?;
        }
        easy.http_headers(headers).map_err(e)?;
        let mut out = Vec::new();
        let mut retry_after = None;
        {
            let mut t = easy.transfer();
            t.header_function(|h| {
                if let Ok(line) = std::str::from_utf8(h)
                    && let Some((name, value)) = line.split_once(':')
                    && name.trim().eq_ignore_ascii_case("retry-after")
                {
                    retry_after = value.trim().parse::<u64>().ok();
                }
                true
            })
            .map_err(e)?;
            t.write_function(|d| {
                if out.len() + d.len() > MAX_RESPONSE {
                    return Ok(0);
                }
                out.extend_from_slice(d);
                Ok(d.len())
            })
            .map_err(e)?;
            t.perform().map_err(e)?;
        }
        let code = easy.response_code().map_err(e)?;
        Ok(Reply {
            status: code as u16,
            body: out,
            retry_after,
        })
    }

    /// JSON in, JSON out. Non-2xx statuses become [`CloudError::Api`].
    pub fn json<T: DeserializeOwned>(
        &self,
        method: &str,
        path: &str,
        body: Option<&impl Serialize>,
    ) -> Result<T, CloudError> {
        let bytes = body
            .map(serde_json::to_vec)
            .transpose()
            .map_err(|e| CloudError::Unreachable(e.to_string()))?;
        let (status, resp) = self.call(method, path, bytes.as_deref())?;
        serde_json::from_slice(&resp).map_err(|_| not_the_api(status))
    }

    /// Like [`Client::json`] for replies without a body (`202 {}`, `204`).
    fn empty(&self, method: &str, path: &str, body: Option<&[u8]>) -> Result<(), CloudError> {
        self.call(method, path, body).map(|_| ())
    }

    fn call(
        &self,
        method: &str,
        path: &str,
        body: Option<&[u8]>,
    ) -> Result<(u16, Vec<u8>), CloudError> {
        let (status, resp) = self
            .send(method, path, body, false)
            .map_err(|e| CloudError::Unreachable(e.0))?;
        if (200..300).contains(&status) {
            return Ok((status, resp));
        }
        if (300..400).contains(&status) {
            return Err(CloudError::Unreachable(REDIRECT.into()));
        }
        match serde_json::from_slice::<ErrorBody>(&resp) {
            Ok(b) => Err(CloudError::Api(b, status)),
            Err(_) => Err(not_the_api(status)),
        }
    }

    pub fn email_start(&self, email: &str, label: &str) -> Result<(), CloudError> {
        let body = serde_json::json!({"email": email, "client": "device", "label": label});
        self.empty(
            "POST",
            "/v1/auth/email/start",
            Some(body.to_string().as_bytes()),
        )
    }

    pub fn email_verify(&self, email: &str, code: &str) -> Result<Verified, CloudError> {
        let body = serde_json::json!({"email": email, "code": code});
        self.json("POST", "/v1/auth/email/verify", Some(&body))
    }

    pub fn sign_out(&self) -> Result<(), CloudError> {
        self.empty("POST", "/v1/auth/sign-out", None)
    }

    #[allow(dead_code)] // used by publish and pull
    pub fn me(&self) -> Result<Me, CloudError> {
        self.json("GET", "/v1/me", None::<&()>)
    }
}

const REDIRECT: &str = "The server answered with a redirect. Check the --server URL";

fn not_the_api(status: u16) -> CloudError {
    CloudError::Unreachable(format!("unexpected reply (HTTP {status})"))
}

/// A one-sentence message for a failed call.
pub fn describe(err: &CloudError, server: &str) -> String {
    match err {
        CloudError::Unreachable(why) => format!("Cannot reach {server}: {why}."),
        CloudError::Api(b, _) => match b.error {
            ErrorCode::CodeInvalid => "That code is not right. Check it and try again.".into(),
            ErrorCode::CodeExpired => {
                "That code has expired. Run curl-runner login to get a new one.".into()
            }
            ErrorCode::Unauthorized => "You are not signed in. Run curl-runner login.".into(),
            _ => super::escape_control(&b.message),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use testkit::{Response, TestServer};

    fn env_of(pairs: &'static [(&'static str, &'static str)]) -> impl Fn(&str) -> Option<String> {
        move |k| {
            pairs
                .iter()
                .find(|(a, _)| *a == k)
                .map(|(_, v)| v.to_string())
        }
    }

    #[test]
    fn server_resolution_order() {
        let env = env_of(&[
            (SERVER_ENV, "https://env.example.com/"),
            (TEST_SERVER_ENV, "http://t/"),
        ]);
        assert_eq!(
            resolve_server(Some("https://flag.example.com/"), &env),
            "https://flag.example.com"
        );
        assert_eq!(resolve_server(None, &env), "http://t");
        let env = env_of(&[(SERVER_ENV, "https://env.example.com/")]);
        assert_eq!(resolve_server(None, &env), "https://env.example.com");
        assert_eq!(resolve_server(None, &env_of(&[])), DEFAULT_SERVER);
    }

    #[test]
    fn api_errors_parse_and_transport_errors_do_not() {
        let srv = TestServer::with_handler(|_| {
            Response::from_data(br#"{"error":"code_invalid","message":"Wrong code."}"#.to_vec())
                .with_status_code(400)
        });
        let c = Client::new(&srv.base_url, None);
        match c.email_verify("a@example.com", "000000") {
            Err(CloudError::Api(b, 400)) => assert_eq!(b.error, ErrorCode::CodeInvalid),
            other => panic!("{other:?}"),
        }
        let c = Client::new("http://127.0.0.1:1", None);
        assert!(matches!(c.sign_out(), Err(CloudError::Unreachable(_))));
    }

    #[test]
    fn sends_bearer_and_gzip() {
        let seen = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
        let s2 = seen.clone();
        let srv = TestServer::with_handler(move |r| {
            s2.lock().unwrap().push((
                r.header("authorization").map(str::to_string),
                r.header("content-encoding").map(str::to_string),
                r.body.clone(),
            ));
            Response::from_data(b"{}".to_vec())
        });
        let c = Client::new(&srv.base_url, Some("crs_x".into()));
        let (status, _) = c.send("POST", "/p", Some(b"hello"), true).unwrap();
        assert_eq!(status, 200);
        let seen = seen.lock().unwrap();
        assert_eq!(seen[0].0.as_deref(), Some("Bearer crs_x"));
        assert_eq!(seen[0].1.as_deref(), Some("gzip"));
        assert_eq!(&seen[0].2[..2], &[0x1f, 0x8b]);
    }

    #[test]
    fn plain_http_only_for_loopback() {
        for ok in [
            "https://api.example.com",
            "http://localhost:8080",
            "http://127.0.0.1:1",
            "http://[::1]:3000",
        ] {
            assert!(check_server(ok).is_ok(), "{ok}");
        }
        for bad in [
            "http://api.example.com",
            "ftp://x",
            "http://127.0.0.1.example.com",
        ] {
            assert!(check_server(bad).is_err(), "{bad}");
        }
    }

    #[test]
    fn redirects_are_not_followed() {
        let hits = std::sync::Arc::new(std::sync::Mutex::new(0));
        let h2 = hits.clone();
        let target = TestServer::with_handler(move |_| {
            *h2.lock().unwrap() += 1;
            Response::from_data(b"{}".to_vec())
        });
        let t2 = target.base_url.clone();
        let first = TestServer::with_handler(move |_| {
            Response::from_data(Vec::new())
                .with_status_code(302)
                .with_header("Location", &format!("{t2}/v1/auth/sign-out"))
        });
        let c = Client::new(&first.base_url, Some("crs_x".into()));
        match c.sign_out() {
            Err(CloudError::Unreachable(m)) => assert!(m.contains("redirect")),
            other => panic!("{other:?}"),
        }
        assert_eq!(*hits.lock().unwrap(), 0);
    }

    #[test]
    fn verified_debug_hides_the_session() {
        let v: Verified = serde_json::from_str(
            r#"{"session":"crs_secret","user":{"id":"1","email":"a@example.com","name":null}}"#,
        )
        .unwrap();
        assert!(!format!("{v:?}").contains("crs_secret"));
    }

    #[test]
    fn describe_messages_have_no_em_dash() {
        let b = |error| {
            CloudError::Api(
                ErrorBody {
                    error,
                    message: "Server says so.".into(),
                },
                400,
            )
        };
        assert!(describe(&b(ErrorCode::CodeInvalid), "s").contains("not right"));
        assert_eq!(describe(&b(ErrorCode::RateLimited), "s"), "Server says so.");
        assert!(describe(&CloudError::Unreachable("boom".into()), "http://s").contains("http://s"));
    }
}
