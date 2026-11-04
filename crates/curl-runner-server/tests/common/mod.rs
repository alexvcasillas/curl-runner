//! Shared harness for the DB-backed integration tests. Each test file uses a
//! subset of it, hence `dead_code`.
#![allow(dead_code)]

use std::collections::HashMap;
use std::sync::Arc;

use axum::Router;
use axum::body::Body;
use axum::http::{Method, Request, StatusCode, header};
use chrono::{DateTime, Utc};
use curl_runner_server::auth::providers::FakeProviders;
use curl_runner_server::clock::FixedClock;
use curl_runner_server::config::Config;
use curl_runner_server::{AppState, app};
use http_body_util::BodyExt;
use serde_json::Value;
use sqlx::PgPool;
use tower::ServiceExt;

/// The fixed "now" every test starts at.
pub const NOW: &str = "2026-11-04T10:00:00Z";

pub struct TestApp {
    pub state: AppState,
    pub clock: Arc<FixedClock>,
    /// What the OAuth providers answer; set per test.
    pub providers: Arc<FakeProviders>,
    router: Router,
}

/// A config with only the required values, plus `overrides`.
pub fn test_config(overrides: &[(&str, &str)]) -> Config {
    let mut env: HashMap<&str, &str> = HashMap::from([
        ("DATABASE_URL", "postgres://test@localhost/test"),
        ("SECRET_KEY", "test-secret-key-test-secret-key-0123"),
        // Open by default so any signed-in test user can create a team.
        ("SIGNUP", "open"),
        // Free limits by default, since most tests exercise them.
        ("DEFAULT_PLAN", "free"),
    ]);
    env.extend(overrides.iter().copied());
    Config::from_env(|k| env.get(k).map(|v| (*v).to_owned())).expect("test config")
}

/// The app with [`test_config`] defaults. `pool` comes from `#[sqlx::test]`,
/// which has already run the migrations.
pub async fn test_app(pool: PgPool) -> TestApp {
    test_app_with(pool, test_config(&[])).await
}

/// The app with a custom config. The clock starts at [`NOW`]; the mailer only
/// writes to the outbox table, so tests read sent mail from there.
pub async fn test_app_with(pool: PgPool, config: Config) -> TestApp {
    let start: DateTime<Utc> = NOW.parse().unwrap();
    let clock = Arc::new(FixedClock::new(start));
    let mut state = AppState::new(pool, config, clock.clone());
    let providers = Arc::new(FakeProviders::new());
    state.oauth = providers.clone();
    TestApp {
        providers,
        router: app(state.clone()),
        state,
        clock,
    }
}

impl TestApp {
    /// Send one request. The body is parsed as JSON; an empty body is `Null`.
    pub async fn request(
        &self,
        method: Method,
        path: &str,
        json: Option<Value>,
        bearer: Option<&str>,
    ) -> (StatusCode, Value) {
        let mut req = Request::builder().method(method).uri(path);
        if let Some(token) = bearer {
            req = req.header(header::AUTHORIZATION, format!("Bearer {token}"));
        }
        let body = match json {
            Some(value) => {
                req = req.header(header::CONTENT_TYPE, "application/json");
                Body::from(value.to_string())
            }
            None => Body::empty(),
        };
        let res = self
            .router
            .clone()
            .oneshot(req.body(body).unwrap())
            .await
            .unwrap();
        let status = res.status();
        let bytes = res.into_body().collect().await.unwrap().to_bytes();
        let value = if bytes.is_empty() {
            Value::Null
        } else {
            serde_json::from_slice(&bytes)
                .unwrap_or_else(|_| Value::String(String::from_utf8_lossy(&bytes).into_owned()))
        };
        (status, value)
    }

    /// Ask for a sign-in code and read it, and the link token (web only), from
    /// the newest queued mail to `email`. Mail bodies are intact until the worker runs.
    pub async fn start_email(&self, email: &str, client: &str) -> (String, Option<String>) {
        let (status, _) = self
            .request(
                Method::POST,
                "/v1/auth/email/start",
                Some(serde_json::json!({"email": email, "client": client})),
                None,
            )
            .await;
        assert_eq!(status, StatusCode::ACCEPTED);
        let body: String = sqlx::query_scalar(
            "SELECT text_body FROM outbox_emails WHERE to_addr = $1 ORDER BY ctid DESC LIMIT 1",
        )
        .bind(email.to_lowercase())
        .fetch_one(&self.state.db)
        .await
        .unwrap();
        let code = body
            .split("sign-in code is ")
            .nth(1)
            .and_then(|r| r.get(..6))
            .expect("code in mail")
            .to_owned();
        let token = body
            .split("token=")
            .nth(1)
            .and_then(|r| r.split('&').next())
            .map(str::to_owned);
        (code, token)
    }

    /// Sign `email` in (start, read the code, verify) and return the session token.
    pub async fn sign_in(&self, email: &str) -> String {
        let (code, _) = self.start_email(email, "web").await;
        let (status, body) = self
            .request(
                Method::POST,
                "/v1/auth/email/verify",
                Some(serde_json::json!({"email": email, "code": code})),
                None,
            )
            .await;
        assert_eq!(status, StatusCode::OK, "{body}");
        body["session"].as_str().expect("session token").to_owned()
    }

    /// POST a JSON body with an `X-Forwarded-For` header.
    pub async fn request_with_ip(
        &self,
        path: &str,
        json: Value,
        forwarded: &str,
    ) -> (StatusCode, Value) {
        let req = Request::builder()
            .method(Method::POST)
            .uri(path)
            .header(header::CONTENT_TYPE, "application/json")
            .header("x-forwarded-for", forwarded)
            .body(Body::from(json.to_string()))
            .unwrap();
        self.send(req).await
    }

    /// POST a raw (possibly malformed) body labelled as JSON.
    pub async fn request_raw(&self, method: Method, path: &str, body: &str) -> (StatusCode, Value) {
        let req = Request::builder()
            .method(method)
            .uri(path)
            .header(header::CONTENT_TYPE, "application/json")
            .body(Body::from(body.to_owned()))
            .unwrap();
        self.send(req).await
    }

    async fn send(&self, req: Request<Body>) -> (StatusCode, Value) {
        let res = self.router.clone().oneshot(req).await.unwrap();
        let status = res.status();
        let bytes = res.into_body().collect().await.unwrap().to_bytes();
        let value = if bytes.is_empty() {
            Value::Null
        } else {
            serde_json::from_slice(&bytes)
                .unwrap_or_else(|_| Value::String(String::from_utf8_lossy(&bytes).into_owned()))
        };
        (status, value)
    }

    /// GET with a raw `Authorization` header value.
    pub async fn request_with_auth(&self, path: &str, authorization: &str) -> (StatusCode, Value) {
        let req = Request::builder()
            .uri(path)
            .header(header::AUTHORIZATION, authorization)
            .body(Body::empty())
            .unwrap();
        self.send(req).await
    }

    /// Create team `slug` (named after it) as the user behind `session`.
    /// Needs a config under which that user may create teams (the default).
    pub async fn create_team(&self, session: &str, slug: &str) {
        let (status, body) = self
            .request(
                Method::POST,
                "/v1/teams",
                Some(serde_json::json!({"slug": slug, "name": slug})),
                Some(session),
            )
            .await;
        assert_eq!(status, StatusCode::CREATED, "{body}");
    }
}
