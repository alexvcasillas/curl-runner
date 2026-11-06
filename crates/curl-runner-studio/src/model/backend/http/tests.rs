use super::*;
use crate::model::backend::{FOREIGN_CALLBACK, LinkAccess, SharedVar, SignInMethod};
use crate::model::secrets::MemorySecrets;
use crate::model::sessions::{AccountCache, MemorySessions};
use curl_runner_cloud_client::CloudError;
use curl_runner_cloud_proto::{ErrorBody, ErrorCode};
use futures::executor::block_on;
use std::sync::Mutex;
use testkit::{Request, Response, TestServer};

type Log = Arc<Mutex<Vec<(String, String, Option<String>)>>>;

fn json(status: u16, body: &str) -> Response {
    Response::from_data(body.as_bytes().to_vec())
        .with_status_code(status)
        .with_header("Content-Type", "application/json")
}

fn info_body(app_url: Option<&str>) -> String {
    let app = app_url.map_or(String::new(), |a| format!(r#","app_url":"{a}""#));
    format!(
        r#"{{"version":"2.1.1","sign_in":["email","github"],"signup":"open","billing":false{app}}}"#
    )
}

fn verified(email: &str) -> String {
    format!(r#"{{"session":"crs_new","user":{{"id":"u1","email":"{email}","name":"Ada"}}}}"#)
}

/// A server that answers info, redeem, me and sign-out, and logs requests
/// as (method, path, bearer).
fn server(me_status: u16) -> (TestServer, Log) {
    let log: Log = Log::default();
    let l = log.clone();
    let srv = TestServer::with_handler(move |r: Request| {
        l.lock().unwrap().push((
            r.method.clone(),
            r.url.clone(),
            r.header("authorization").map(str::to_string),
        ));
        match (r.method.as_str(), r.url.as_str()) {
            ("GET", "/v1/info") => json(200, &info_body(Some("https://app.example.test"))),
            ("POST", "/v1/auth/handoff/redeem") => json(200, &verified("ada@example.test")),
            ("GET", "/v1/me") if me_status == 200 => json(
                200,
                r#"{"user":{"id":"u1","email":"ada@example.test","name":"Ada"},"teams":[]}"#,
            ),
            ("GET", "/v1/me") => json(me_status, r#"{"error":"unauthorized","message":"No."}"#),
            ("POST", "/v1/auth/sign-out") => Response::from_data(Vec::new()).with_status_code(204),
            _ => json(404, r#"{"error":"not_found","message":"No."}"#),
        }
    });
    (srv, log)
}

fn backend(state: Option<&Path>) -> (HttpBackend, Arc<MemorySessions>) {
    let sessions = Arc::new(MemorySessions::new(MemorySecrets::default(), state));
    (HttpBackend::open(state, sessions.clone()), sessions)
}

fn own(url: &str) -> Server {
    Server::SelfHosted { url: url.into() }
}

fn callback(server: &str) -> AuthCallback {
    AuthCallback {
        server: server.into(),
        code: "crh_abc".into(),
    }
}

#[test]
fn test_connection_reports_methods() {
    let (srv, _) = server(200);
    let (b, _) = backend(None);
    let info = block_on(b.test_connection(&own(&srv.base_url))).unwrap();
    assert_eq!(info.version, "2.1.1");
    assert_eq!(info.sign_in, [SignInMethod::Email, SignInMethod::Github]);
    assert!(info.latency_ms < 5_000);
    let bad = block_on(b.test_connection(&own("http://example.com")));
    assert!(matches!(bad, Err(BackendError::Invalid(m)) if m.contains("https")));
    let down = block_on(b.test_connection(&own("http://127.0.0.1:1")));
    assert_eq!(down.unwrap_err(), BackendError::Offline);
}

#[test]
fn sign_in_url_comes_from_the_dashboard() {
    let (srv, _) = server(200);
    let (b, _) = backend(None);
    let url = block_on(b.sign_in_url(&own(&srv.base_url))).unwrap();
    assert_eq!(url, "https://app.example.test/sign-in?client=studio");
}

#[test]
fn sign_in_url_needs_an_app_url() {
    let srv = TestServer::with_handler(|_| json(200, &info_body(None)));
    let (b, _) = backend(None);
    let err = block_on(b.sign_in_url(&own(&srv.base_url))).unwrap_err();
    assert_eq!(
        err,
        BackendError::Invalid("This server has no sign-in page.".into())
    );
    // Nothing pending: its callback is foreign.
    let err = block_on(b.complete_sign_in(&callback(&srv.base_url))).unwrap_err();
    assert_eq!(err, BackendError::Invalid(FOREIGN_CALLBACK.into()));
}

#[test]
fn sign_in_url_refuses_plain_http_off_loopback() {
    let (b, _) = backend(None);
    let err = block_on(b.sign_in_url(&own("http://curl.example.com"))).unwrap_err();
    assert!(
        matches!(&err, BackendError::Invalid(m) if m.contains("https")),
        "{err:?}"
    );
    assert_ne!(err, BackendError::Invalid(FOREIGN_CALLBACK.into()));
}

#[test]
fn callback_without_a_started_sign_in_sends_nothing() {
    let (srv, log) = server(200);
    let (b, sessions) = backend(None);
    let err = block_on(b.complete_sign_in(&callback(&srv.base_url))).unwrap_err();
    assert_eq!(err, BackendError::Invalid(FOREIGN_CALLBACK.into()));
    assert!(log.lock().unwrap().is_empty());
    assert_eq!(sessions.get(&srv.base_url).unwrap(), None);
}

#[test]
fn complete_sign_in_stores_session_and_account() {
    let d = tempfile::tempdir().unwrap();
    let (srv, _) = server(200);
    let (b, sessions) = backend(Some(d.path()));
    block_on(b.sign_in_url(&own(&srv.base_url))).unwrap();
    // The dashboard's callback carries the server with a trailing slash.
    let account = block_on(b.complete_sign_in(&callback(&format!("{}/", srv.base_url)))).unwrap();
    assert_eq!(account.email, "ada@example.test");
    assert_eq!(account.name, "Ada");
    assert_eq!(
        sessions.get(&srv.base_url).unwrap().as_deref(),
        Some("crs_new")
    );
    assert_eq!(b.account_on(&srv.base_url), Some(account));
    assert_eq!(sessions.servers(), std::slice::from_ref(&srv.base_url));
    let text = std::fs::read_to_string(d.path().join("accounts.json")).unwrap();
    assert!(
        text.contains("ada@example.test") && !text.contains("crs_"),
        "{text}"
    );
    // The pending sign-in is used up.
    let again = block_on(b.complete_sign_in(&callback(&srv.base_url))).unwrap_err();
    assert_eq!(again, BackendError::Invalid(FOREIGN_CALLBACK.into()));
    // A new HttpBackend on the same state still knows who is signed in.
    let (b2, _) = backend(Some(d.path()));
    assert_eq!(
        b2.account_on(&srv.base_url).unwrap().email,
        "ada@example.test"
    );
}

#[test]
fn the_account_is_the_current_servers() {
    let (srv, _) = server(200);
    let (b, _) = backend(None);
    block_on(b.set_server(own(&srv.base_url))).unwrap();
    assert_eq!(b.server(), own(&srv.base_url));
    assert_eq!(b.account(), None);
    block_on(b.sign_in_url(&own(&srv.base_url))).unwrap();
    let a = block_on(b.complete_sign_in(&callback(&srv.base_url))).unwrap();
    assert_eq!(b.account(), Some(a));
    block_on(b.set_server(Server::Cloud)).unwrap();
    assert_eq!(b.account(), None);
}

#[test]
fn server_choice_persists() {
    let d = tempfile::tempdir().unwrap();
    let (b, _) = backend(Some(d.path()));
    assert_eq!(b.server(), Server::Cloud);
    block_on(b.set_server(own("https://curl.example.com"))).unwrap();
    let (b2, _) = backend(Some(d.path()));
    assert_eq!(b2.server(), own("https://curl.example.com"));
}

#[test]
fn unauthorized_drops_only_that_servers_session() {
    let d = tempfile::tempdir().unwrap();
    let (a, _) = server(401);
    let (c, log_c) = server(200);
    let (b, sessions) = backend(Some(d.path()));
    for srv in [&a, &c] {
        block_on(b.sign_in_url(&own(&srv.base_url))).unwrap();
        block_on(b.complete_sign_in(&callback(&srv.base_url))).unwrap();
    }
    assert_eq!(sessions.servers().len(), 2);
    let err = block_on(b.refresh_account(&a.base_url)).unwrap_err();
    assert_eq!(err, BackendError::SignedOut);
    assert_eq!(sessions.get(&a.base_url).unwrap(), None);
    assert_eq!(b.account_on(&a.base_url), None);
    assert_eq!(sessions.servers(), std::slice::from_ref(&c.base_url));
    assert_eq!(AccountCache::load(Some(d.path())).0.len(), 1);
    // The other server keeps its session and answers with it.
    assert!(sessions.get(&c.base_url).unwrap().is_some());
    let acct = block_on(b.refresh_account(&c.base_url)).unwrap();
    assert_eq!(acct.email, "ada@example.test");
    assert!(b.account_on(&c.base_url).is_some());
    let log = log_c.lock().unwrap();
    assert!(log.iter().any(|(m, p, auth)| m == "GET"
        && p == "/v1/me"
        && auth.as_deref() == Some("Bearer crs_new")));
}

#[test]
fn refresh_without_a_session_is_signed_out() {
    let (srv, log) = server(200);
    let (b, _) = backend(None);
    let err = block_on(b.refresh_account(&srv.base_url)).unwrap_err();
    assert_eq!(err, BackendError::SignedOut);
    assert!(log.lock().unwrap().is_empty());
}

#[test]
fn a_locked_keychain_is_not_signed_out() {
    struct Locked;
    impl SessionStore for Locked {
        fn get(&self, _: &str) -> std::io::Result<Option<String>> {
            Err(std::io::Error::other("locked"))
        }
        fn set(&self, _: &str, _: &str) -> std::io::Result<()> {
            Ok(())
        }
        fn delete(&self, _: &str) -> std::io::Result<()> {
            Ok(())
        }
        fn servers(&self) -> Vec<String> {
            Vec::new()
        }
    }
    let (srv, log) = server(200);
    let b = HttpBackend::open(None, Arc::new(Locked));
    let err = block_on(b.refresh_account(&srv.base_url)).unwrap_err();
    assert_eq!(
        err,
        BackendError::Invalid("Couldn't read your sign-in from the Keychain.".into())
    );
    assert!(log.lock().unwrap().is_empty());
}

#[test]
fn sign_out_forgets_session_when_offline() {
    let d = tempfile::tempdir().unwrap();
    let down = "http://127.0.0.1:1";
    let (b, sessions) = backend(Some(d.path()));
    sessions.set(down, "crs_x").unwrap();
    let mut cache = AccountCache::default();
    cache.0.insert(
        down.into(),
        crate::model::sessions::CachedAccount {
            name: None,
            email: "a@example.test".into(),
        },
    );
    cache.save(Some(d.path())).unwrap();
    let (b2, _) = (HttpBackend::open(Some(d.path()), sessions.clone()), ());
    drop(b);
    assert!(b2.account_on(down).is_some());
    block_on(b2.sign_out(down)).unwrap();
    assert_eq!(sessions.get(down).unwrap(), None);
    assert_eq!(b2.account_on(down), None);
}

#[test]
fn sign_out_tells_the_server_with_the_session() {
    let (srv, log) = server(200);
    let (b, sessions) = backend(None);
    block_on(b.sign_in_url(&own(&srv.base_url))).unwrap();
    block_on(b.complete_sign_in(&callback(&srv.base_url))).unwrap();
    block_on(b.sign_out(&format!("{}/", srv.base_url))).unwrap();
    assert_eq!(sessions.get(&srv.base_url).unwrap(), None);
    let log = log.lock().unwrap();
    assert!(log.iter().any(|(m, p, auth)| m == "POST"
        && p == "/v1/auth/sign-out"
        && auth.as_deref() == Some("Bearer crs_new")));
}

#[test]
fn error_mapping() {
    let api = |code: ErrorCode, status: u16| {
        CloudError::Api(
            ErrorBody {
                error: code,
                message: "Server words.".into(),
            },
            status,
        )
    };
    let (b, sessions) = backend(None);
    let server = "https://api.example.test";
    let invalid = |s: &str| BackendError::Invalid(s.into());
    let table = [
        (CloudError::Unreachable("x".into()), BackendError::Offline),
        (api(ErrorCode::Unauthorized, 401), BackendError::SignedOut),
        (api(ErrorCode::Forbidden, 403), BackendError::Unauthorized),
        (api(ErrorCode::NotFound, 404), BackendError::NotFound),
        (api(ErrorCode::LinkDisabled, 404), BackendError::NotFound),
        (api(ErrorCode::CodeExpired, 400), BackendError::Expired),
        (
            api(ErrorCode::CodeInvalid, 400),
            invalid("That code is not right. Check it and try again."),
        ),
        (api(ErrorCode::Unavailable, 503), BackendError::Unavailable),
        (api(ErrorCode::RateLimited, 429), BackendError::Unavailable),
        (api(ErrorCode::PlanLimit, 402), invalid("Server words.")),
        (
            api(ErrorCode::InvalidRequest, 400),
            invalid("Server words."),
        ),
    ];
    for (err, want) in table {
        let shown = format!("{err:?}");
        sessions.set(server, "crs_x").unwrap();
        assert_eq!(b.map_error(server, err), want, "{shown}");
        let kept = sessions.get(server).unwrap().is_some();
        assert_eq!(kept, want != BackendError::SignedOut, "{shown}");
    }
}

#[test]
fn team_side_calls_are_unavailable_until_3b() {
    let (b, _) = backend(None);
    let key = CollectionKey {
        id: "c".into(),
        name: "C".into(),
    };
    let un = |e: BackendError| assert_eq!(e, BackendError::Unavailable);
    un(block_on(b.sync(&key, &[], &[])).unwrap_err());
    un(block_on(b.share_settings(&key)).unwrap_err());
    un(block_on(b.update_share(&key, ShareUpdate::default())).unwrap_err());
    un(block_on(b.invite(&key, "a@b.c")).unwrap_err());
    un(block_on(b.remove_person(&key, "a@b.c")).unwrap_err());
    assert_eq!(b.sync_status(&key), SyncStatus::Off);
    assert!(!b.team_sync());
    assert!(b.dev().is_none());
}

#[test]
fn a_failed_redeem_keeps_the_existing_session() {
    let srv = TestServer::with_handler(|r: Request| match r.url.as_str() {
        "/v1/info" => json(200, &info_body(Some("https://app.example.test"))),
        _ => json(401, r#"{"error":"unauthorized","message":"No."}"#),
    });
    let (b, sessions) = backend(None);
    sessions.set(&srv.base_url, "crs_old").unwrap();
    block_on(b.sign_in_url(&own(&srv.base_url))).unwrap();
    let err = block_on(b.complete_sign_in(&callback(&srv.base_url))).unwrap_err();
    assert_eq!(err, BackendError::Unauthorized);
    assert_eq!(
        sessions.get(&srv.base_url).unwrap().as_deref(),
        Some("crs_old")
    );
}

#[test]
fn http_accounts_carry_no_provider() {
    let (srv, _) = server(200);
    let (b, _) = backend(None);
    block_on(b.sign_in_url(&own(&srv.base_url))).unwrap();
    let a = block_on(b.complete_sign_in(&callback(&srv.base_url))).unwrap();
    assert_eq!(a.provider, None);
}

const TOKEN: &str = "crsl_Ab-9_";

fn shared_info(access: &str, version: u32) -> String {
    format!(
        r#"{{"team":"Acme","team_slug":"acme","collection":"shipping-api","name":"Shipping API","description":null,"version":{version},"updated_at":"2026-11-01T10:00:00Z","request_count":2,"access":"{access}"}}"#
    )
}

/// A server sharing one collection with `access`. A restricted one
/// answers only `Bearer crs_ok`; the log is (method, path, bearer).
fn shared_server(access: &'static str) -> (TestServer, Log) {
    let log: Log = Log::default();
    let l = log.clone();
    let srv = TestServer::with_handler(move |r: Request| {
        let auth = r.header("authorization").map(str::to_string);
        l.lock()
            .unwrap()
            .push((r.method.clone(), r.url.clone(), auth.clone()));
        let allowed = access == "public" || auth.as_deref() == Some("Bearer crs_ok");
        if !allowed {
            return json(
                401,
                r#"{"error":"unauthorized","message":"Sign in to open this collection."}"#,
            );
        }
        let info = shared_info(access, 4);
        match r.url.strip_prefix(&format!("/v1/shared/{TOKEN}")) {
            Some("") => json(200, &info),
            Some("/snapshot") => json(
                200,
                &format!(
                    r#"{{"info":{info},"files":[{{"path":"rates/get.yaml","text":"a: 1\n"}},{{"path":"labels/make.yaml","text":"b: 2\n"}}],"inputs":[{{"name":"API_TOKEN","description":"Your own token.","secret":true,"required":true,"locked":false,"default":null}},{{"name":"CURRENCY","description":null,"secret":false,"required":false,"locked":true,"default":"EUR"}}]}}"#
                ),
            ),
            Some("/versions/latest") => json(200, r#"{"version":5,"hash":"abc"}"#),
            _ => json(404, r#"{"error":"not_found","message":"No."}"#),
        }
    });
    (srv, log)
}

fn link(srv: &TestServer) -> ShareLink {
    ShareLink {
        server: srv.base_url.clone(),
        token: TOKEN.into(),
    }
}

#[test]
fn resolve_public_without_session() {
    let (srv, log) = shared_server("public");
    let (b, _) = backend(None);
    let c = block_on(b.resolve_link(&link(&srv))).unwrap();
    assert_eq!(c.link, link(&srv));
    assert_eq!(c.owner_org, "Acme");
    assert_eq!(c.team_slug, "acme");
    assert_eq!(c.name, "Shipping API");
    assert_eq!(c.version, 4);
    assert_eq!(c.access, LinkAccess::Public);
    assert_eq!(c.request_count, 2);
    assert_eq!(c.summary, "rates, labels");
    assert_eq!(c.variables.len(), 2);
    let log = log.lock().unwrap();
    assert!(
        log.iter()
            .any(|(m, p, _)| m == "GET" && p == &format!("/v1/shared/{TOKEN}")),
        "{log:?}"
    );
    assert!(log.iter().all(|(_, _, auth)| auth.is_none()), "{log:?}");
}

#[test]
fn resolve_restricted_needs_sign_in() {
    let (srv, log) = shared_server("restricted");
    let (b, sessions) = backend(None);
    let err = block_on(b.resolve_link(&link(&srv))).unwrap_err();
    assert_eq!(err, BackendError::SignedOut);
    assert_eq!(log.lock().unwrap().len(), 1, "it stops at the card");
    // Signed in to that server, the session goes with it.
    sessions.set(&srv.base_url, "crs_ok").unwrap();
    let c = block_on(b.resolve_link(&link(&srv))).unwrap();
    assert_eq!(c.access, LinkAccess::Restricted);
    assert!(
        log.lock()
            .unwrap()
            .iter()
            .skip(1)
            .all(|(_, _, auth)| auth.as_deref() == Some("Bearer crs_ok"))
    );
}

#[test]
fn a_rejected_session_on_a_link_signs_out_of_that_server_only() {
    let (srv, _) = shared_server("restricted");
    let (b, sessions) = backend(None);
    sessions.set(&srv.base_url, "crs_stale").unwrap();
    sessions
        .set("https://other.example.test", "crs_other")
        .unwrap();
    let err = block_on(b.fetch_shared(&link(&srv))).unwrap_err();
    assert_eq!(err, BackendError::SignedOut);
    assert_eq!(sessions.get(&srv.base_url).unwrap(), None);
    assert!(
        sessions
            .get("https://other.example.test")
            .unwrap()
            .is_some()
    );
}

#[test]
fn fetch_shared_maps_inputs_to_variables() {
    let (srv, _) = shared_server("public");
    let (b, _) = backend(None);
    let snap = block_on(b.fetch_shared(&link(&srv))).unwrap();
    assert_eq!(snap.version, 4);
    assert_eq!(
        snap.published_at,
        "2026-11-01T10:00:00Z"
            .parse::<chrono::DateTime<chrono::Utc>>()
            .unwrap()
    );
    assert_eq!(
        snap.files,
        [
            SyncFile::new("rates/get.yaml", "a: 1\n"),
            SyncFile::new("labels/make.yaml", "b: 2\n"),
        ]
    );
    assert_eq!(
        snap.variables,
        [
            SharedVar {
                name: "API_TOKEN".into(),
                description: "Your own token.".into(),
                required: true,
                secret: true,
                default: None,
                locked: false,
            },
            SharedVar {
                name: "CURRENCY".into(),
                description: String::new(),
                required: false,
                secret: false,
                default: Some("EUR".into()),
                locked: true,
            },
        ]
    );
}

#[test]
fn latest_version_asks_without_the_files() {
    let (srv, log) = shared_server("public");
    let (b, _) = backend(None);
    assert_eq!(block_on(b.latest_version(&link(&srv))).unwrap(), 5);
    let log = log.lock().unwrap();
    assert_eq!(log.len(), 1);
    assert_eq!(log[0].1, format!("/v1/shared/{TOKEN}/versions/latest"));
}

#[test]
fn shared_calls_read_a_locked_keychain_as_an_error() {
    struct Locked;
    impl SessionStore for Locked {
        fn get(&self, _: &str) -> std::io::Result<Option<String>> {
            Err(std::io::Error::other("locked"))
        }
        fn set(&self, _: &str, _: &str) -> std::io::Result<()> {
            Ok(())
        }
        fn delete(&self, _: &str) -> std::io::Result<()> {
            Ok(())
        }
        fn servers(&self) -> Vec<String> {
            Vec::new()
        }
    }
    let (srv, log) = shared_server("public");
    let b = HttpBackend::open(None, Arc::new(Locked));
    let err = block_on(b.resolve_link(&link(&srv))).unwrap_err();
    assert_eq!(
        err,
        BackendError::Invalid("Couldn't read your sign-in from the Keychain.".into())
    );
    assert!(log.lock().unwrap().is_empty());
}

fn record(token: &str, version: u32) -> ReceivedShare {
    ReceivedShare {
        link: ShareLink {
            server: "https://api.example.test".into(),
            token: token.into(),
        },
        team: "Acme".into(),
        name: "Shipping API".into(),
        version,
        added: chrono::NaiveDate::from_ymd_opt(2026, 11, 6).unwrap(),
        files: [(PathBuf::from("rates/get.yaml"), 42)].into(),
    }
}

#[test]
fn received_shares_persist_by_token() {
    let d = tempfile::tempdir().unwrap();
    let (b, _) = backend(Some(d.path()));
    assert!(b.received().is_empty());
    block_on(b.add_received(record("crsl_a", 1))).unwrap();
    block_on(b.add_received(record("crsl_b", 1))).unwrap();
    block_on(b.add_received(record("crsl_a", 2))).unwrap();
    block_on(b.update_received(record("crsl_b", 3))).unwrap();
    let want = [record("crsl_b", 3), record("crsl_a", 2)];
    let mut got = b.received();
    got.sort_by(|x, y| y.link.token.cmp(&x.link.token));
    assert_eq!(got, want);
    let text = std::fs::read_to_string(d.path().join("received-shares.json")).unwrap();
    assert!(
        text.contains("crsl_a") && text.contains("rates/get.yaml"),
        "{text}"
    );
    let (b2, _) = backend(Some(d.path()));
    let mut again = b2.received();
    again.sort_by(|x, y| y.link.token.cmp(&x.link.token));
    assert_eq!(again, want);
}

/// A public collection on a server that refuses a stale session (401 on
/// any bearer but `crs_ok`) yet answers without one.
fn stale_public_server() -> (TestServer, Log) {
    let log: Log = Log::default();
    let l = log.clone();
    let srv = TestServer::with_handler(move |r: Request| {
        let auth = r.header("authorization").map(str::to_string);
        l.lock()
            .unwrap()
            .push((r.method.clone(), r.url.clone(), auth.clone()));
        if auth.is_some_and(|a| a != "Bearer crs_ok") {
            return json(
                401,
                r#"{"error":"unauthorized","message":"Session expired."}"#,
            );
        }
        let info = shared_info("public", 4);
        match r.url.strip_prefix(&format!("/v1/shared/{TOKEN}")) {
            Some("") => json(200, &info),
            Some("/snapshot") => json(200, &format!(r#"{{"info":{info},"files":[],"inputs":[]}}"#)),
            Some("/versions/latest") => json(200, r#"{"version":5,"hash":"abc"}"#),
            _ => json(404, r#"{"error":"not_found","message":"No."}"#),
        }
    });
    (srv, log)
}

#[test]
fn a_stale_session_on_a_public_link_retries_without_it() {
    let (srv, log) = stale_public_server();
    let (b, sessions) = backend(None);
    sessions.set(&srv.base_url, "crs_stale").unwrap();
    let c = block_on(b.resolve_link(&link(&srv))).unwrap();
    assert_eq!(c.name, "Shipping API");
    // The stale session is forgotten; the retry carried none.
    assert_eq!(sessions.get(&srv.base_url).unwrap(), None);
    {
        let log = log.lock().unwrap();
        assert_eq!(log[0].2.as_deref(), Some("Bearer crs_stale"));
        assert!(log[1..].iter().all(|(_, _, a)| a.is_none()), "{log:?}");
    }
    // fetch_shared and latest_version get the same retry.
    for call in 0..2 {
        sessions.set(&srv.base_url, "crs_stale").unwrap();
        match call {
            0 => assert!(block_on(b.fetch_shared(&link(&srv))).is_ok()),
            _ => assert_eq!(block_on(b.latest_version(&link(&srv))).unwrap(), 5),
        }
        assert_eq!(sessions.get(&srv.base_url).unwrap(), None);
    }
}

#[test]
fn a_restricted_link_refused_twice_is_signed_out() {
    let (srv, log) = shared_server("restricted");
    let (b, sessions) = backend(None);
    sessions.set(&srv.base_url, "crs_stale").unwrap();
    let err = block_on(b.resolve_link(&link(&srv))).unwrap_err();
    assert_eq!(err, BackendError::SignedOut);
    let log = log.lock().unwrap();
    // Once with the session, once without: then it stops.
    assert_eq!(log.len(), 2, "{log:?}");
    assert_eq!(log[0].2.as_deref(), Some("Bearer crs_stale"));
    assert_eq!(log[1].2, None);
}

/// A server whose `/v1/info` names `app_url` as its dashboard.
fn info_server(app_url: &'static str) -> TestServer {
    TestServer::with_handler(move |_| json(200, &info_body(Some(app_url))))
}

#[test]
fn sign_in_url_refuses_a_dashboard_off_https() {
    let check = "Use an https:// server. Plain http is only allowed for localhost.";
    for app in [
        "smb://host/share",
        "curl-runner://auth/callback?code=crh_x&server=https://evil.example",
        "http://app.example.test",
    ] {
        let srv = info_server(app);
        let (b, _) = backend(None);
        let err = block_on(b.sign_in_url(&own(&srv.base_url))).unwrap_err();
        assert_eq!(err, BackendError::Invalid(check.into()), "{app}");
        // Nothing pending: no callback for it is redeemed.
        let err = block_on(b.complete_sign_in(&callback(&srv.base_url))).unwrap_err();
        assert_eq!(err, BackendError::Invalid(FOREIGN_CALLBACK.into()), "{app}");
    }
    // A dashboard on loopback http is fine (a local server).
    let srv = info_server("http://127.0.0.1:3000");
    let (b, _) = backend(None);
    let url = block_on(b.sign_in_url(&own(&srv.base_url))).unwrap();
    assert_eq!(url, "http://127.0.0.1:3000/sign-in?client=studio");
}

#[test]
fn info_calls_never_sign_out() {
    let srv =
        TestServer::with_handler(|_| json(401, r#"{"error":"unauthorized","message":"No."}"#));
    let (b, sessions) = backend(None);
    sessions.set(&srv.base_url, "crs_kept").unwrap();
    let err = block_on(b.test_connection(&own(&srv.base_url))).unwrap_err();
    assert_eq!(err, BackendError::SignedOut);
    assert!(block_on(b.sign_in_url(&own(&srv.base_url))).is_err());
    assert_eq!(
        sessions.get(&srv.base_url).unwrap().as_deref(),
        Some("crs_kept")
    );
}
