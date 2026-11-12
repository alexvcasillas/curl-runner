//! HTTP client for the curl-runner Cloud API, over libcurl with the same TLS
//! and redirect setup as the update paths (`net::easy_for`).

use crate::escape_control;
use crate::net;
use curl::easy::List;
use curl_runner_cloud_proto::{
    ErrorBody, ErrorCode, Plan, PublishRequest, PublishResponse, Role, ServerInfo, SharedInfo,
    Snapshot,
};
use flate2::{Compression, write::GzEncoder};
use serde::{Deserialize, Serialize, de::DeserializeOwned};
use std::io::Write;
use std::time::Duration;

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

#[derive(Debug, Deserialize)]
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

/// Where a collection's versions are published.
pub fn versions_path(team: &str, collection: &str) -> String {
    format!("/v1/teams/{team}/collections/{collection}/versions")
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
pub struct TeamInfo {
    pub slug: String,
    pub name: String,
    pub role: Role,
    pub plan: Plan,
}

#[derive(Debug, Deserialize)]
pub struct Me {
    pub user: User,
    pub teams: Vec<TeamInfo>,
}

/// The latest published version of a shared collection.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct Latest {
    pub version: u32,
    pub hash: String,
}

/// A team's collection as `GET /v1/teams/{team}/collections` lists it.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct CollectionRow {
    pub slug: String,
    pub current_version: Option<u32>,
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
        self.call_gz(method, path, body, false)
    }

    fn call_gz(
        &self,
        method: &str,
        path: &str,
        body: Option<&[u8]>,
        gzip: bool,
    ) -> Result<(u16, Vec<u8>), CloudError> {
        let (status, resp) = self
            .send(method, path, body, gzip)
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

    pub fn info(&self) -> Result<ServerInfo, CloudError> {
        self.json("GET", "/v1/info", None::<&()>)
    }

    pub fn handoff_redeem(&self, code: &str) -> Result<Verified, CloudError> {
        let body = serde_json::json!({"code": code});
        self.json("POST", "/v1/auth/handoff/redeem", Some(&body))
    }

    pub fn sign_out(&self) -> Result<(), CloudError> {
        self.empty("POST", "/v1/auth/sign-out", None)
    }

    pub fn me(&self) -> Result<Me, CloudError> {
        self.json("GET", "/v1/me", None::<&()>)
    }

    /// A shared collection's card. `token` is a validated share token.
    pub fn shared_info(&self, token: &str) -> Result<SharedInfo, CloudError> {
        self.json("GET", &format!("/v1/shared/{token}"), None::<&()>)
    }

    /// The files and inputs of a shared collection's current version.
    pub fn snapshot(&self, token: &str) -> Result<Snapshot, CloudError> {
        self.json("GET", &format!("/v1/shared/{token}/snapshot"), None::<&()>)
    }

    /// A team's collections. `team` is a validated slug.
    pub fn collections(&self, team: &str) -> Result<Vec<CollectionRow>, CloudError> {
        self.json("GET", &format!("/v1/teams/{team}/collections"), None::<&()>)
    }

    /// Publish a new version of `team/collection` (gzipped). `201` is a new
    /// version, `200` an unchanged one.
    pub fn publish(
        &self,
        team: &str,
        collection: &str,
        req: &PublishRequest,
    ) -> Result<PublishResponse, CloudError> {
        let body = serde_json::to_vec(req).map_err(|e| CloudError::Unreachable(e.to_string()))?;
        let (status, resp) =
            self.call_gz("PUT", &versions_path(team, collection), Some(&body), true)?;
        serde_json::from_slice(&resp).map_err(|_| not_the_api(status))
    }

    /// The current version number and content hash, without the files.
    pub fn latest(&self, token: &str) -> Result<Latest, CloudError> {
        self.json(
            "GET",
            &format!("/v1/shared/{token}/versions/latest"),
            None::<&()>,
        )
    }
}

const REDIRECT: &str = "The server answered with a redirect. Check the --server URL";

fn not_the_api(status: u16) -> CloudError {
    CloudError::Unreachable(format!("unexpected reply (HTTP {status})"))
}

/// A one-sentence message for a failed call.
pub fn describe(err: &CloudError, server: &str) -> String {
    match err {
        CloudError::Unreachable(why) => {
            format!("Cannot reach {}: {why}.", escape_control(server))
        }
        CloudError::Api(b, _) => match b.error {
            ErrorCode::CodeInvalid => "That code is not right. Check it and try again.".into(),
            ErrorCode::CodeExpired => {
                "That code has expired. Run curl-runner login to get a new one.".into()
            }
            ErrorCode::Unauthorized => "You are not signed in. Run curl-runner login.".into(),
            _ => escape_control(&b.message),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Arc, Mutex};
    use testkit::{Response, TestServer};

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

    #[test]
    fn snapshot_decodes_files_and_inputs() {
        let seen = Arc::new(Mutex::new(Vec::new()));
        let s2 = seen.clone();
        let srv = TestServer::with_handler(move |r| {
            s2.lock()
                .unwrap()
                .push((r.url.clone(), r.header("authorization").map(str::to_string)));
            Response::from_data(
                br#"{"info":{"team":"Acme","team_slug":"acme","collection":"pay","name":"Pay","description":null,"version":3,"updated_at":"2026-11-01T10:00:00Z","request_count":1,"access":"public"},
"files":[{"path":"pay/a.yaml","text":"request: {}"}],
"inputs":[{"name":"API_KEY","description":null,"secret":true,"required":true,"locked":false,"default":null}]}"#
                    .to_vec(),
            )
        });
        let c = Client::new(&srv.base_url, None);
        let snap = c.snapshot("crsl_a-b_c").unwrap();
        assert_eq!(snap.files[0].path, "pay/a.yaml");
        assert_eq!(snap.inputs[0].name, "API_KEY");
        assert!(snap.inputs[0].secret);
        assert_eq!(snap.info.version, 3);
        let seen = seen.lock().unwrap();
        assert_eq!(seen[0].0, "/v1/shared/crsl_a-b_c/snapshot");
        assert_eq!(seen[0].1, None);
    }

    #[test]
    fn latest_reads_version_and_hash() {
        let srv = TestServer::with_handler(|r| {
            assert_eq!(r.url, "/v1/shared/tok/versions/latest");
            Response::from_data(br#"{"version":7,"hash":"abc"}"#.to_vec())
        });
        let l = Client::new(&srv.base_url, None).latest("tok").unwrap();
        assert_eq!(
            l,
            Latest {
                version: 7,
                hash: "abc".into()
            }
        );
    }

    #[test]
    fn api_error_keeps_code_and_status() {
        let srv = TestServer::with_handler(|_| {
            Response::from_data(br#"{"error":"not_found","message":"No such link."}"#.to_vec())
                .with_status_code(404)
        });
        match Client::new(&srv.base_url, None).shared_info("tok") {
            Err(CloudError::Api(b, 404)) => assert_eq!(b.error, ErrorCode::NotFound),
            other => panic!("{other:?}"),
        }
    }

    fn publish_reply(status: u16, body: &'static str) -> Result<PublishResponse, CloudError> {
        let srv = TestServer::with_handler(move |r| {
            assert_eq!(
                (r.method.as_str(), r.url.as_str()),
                ("PUT", "/v1/teams/acme/collections/pay/versions")
            );
            assert_eq!(r.header("content-encoding"), Some("gzip"));
            Response::from_data(body.as_bytes().to_vec()).with_status_code(status)
        });
        let req = PublishRequest {
            files: Vec::new(),
            hash: "h".into(),
            inputs: Vec::new(),
            name: None,
            description: None,
            git: None,
        };
        Client::new(&srv.base_url, Some("crs_x".into())).publish("acme", "pay", &req)
    }

    #[test]
    fn publish_created_and_unchanged() {
        use curl_runner_cloud_proto::PublishStatus;
        let ok = |status: &str| {
            format!(
                r#"{{"collection":"pay","version":14,"status":"{status}","changed":{{"added":["a"],"changed":[],"removed":[]}}}}"#
            )
            .leak() as &'static str
        };
        let r = publish_reply(201, ok("created")).unwrap();
        assert_eq!((r.version, r.status), (14, PublishStatus::Created));
        assert_eq!(r.changed.added, ["a"]);
        let r = publish_reply(200, ok("unchanged")).unwrap();
        assert_eq!(r.status, PublishStatus::Unchanged);
    }

    #[test]
    fn publish_refusals_keep_their_code() {
        for (status, code, want) in [
            (409, "wrong_source", ErrorCode::WrongSource),
            (402, "plan_limit", ErrorCode::PlanLimit),
        ] {
            let body = format!(r#"{{"error":"{code}","message":"No."}}"#).leak();
            match publish_reply(status, body) {
                Err(CloudError::Api(b, s)) => assert_eq!((b.error, s), (want, status)),
                other => panic!("{other:?}"),
            }
        }
    }

    #[test]
    fn collections_lists_slugs_and_versions() {
        let srv = TestServer::with_handler(|r| {
            assert_eq!(r.url, "/v1/teams/acme/collections");
            Response::from_data(
                br#"[{"slug":"pay","current_version":3,"name":"Pay"},{"slug":"new","current_version":null}]"#
                    .to_vec(),
            )
        });
        let rows = Client::new(&srv.base_url, None)
            .collections("acme")
            .unwrap();
        assert_eq!(
            rows,
            [
                CollectionRow {
                    slug: "pay".into(),
                    current_version: Some(3)
                },
                CollectionRow {
                    slug: "new".into(),
                    current_version: None
                }
            ]
        );
    }
}
