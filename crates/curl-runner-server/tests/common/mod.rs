//! Shared harness for the DB-backed integration tests. Each test file uses a
//! subset of it, hence `dead_code`.
#![allow(dead_code)]

use std::collections::HashMap;
use std::sync::Arc;

use axum::Router;
use axum::body::Body;
use axum::http::{Method, Request, StatusCode, header};
use chrono::{DateTime, Utc};
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
    router: Router,
}

/// A config with only the required values, plus `overrides`.
pub fn test_config(overrides: &[(&str, &str)]) -> Config {
    let mut env: HashMap<&str, &str> = HashMap::from([
        ("DATABASE_URL", "postgres://test@localhost/test"),
        ("SECRET_KEY", "test-secret-key-test-secret-key-0123"),
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
    let state = AppState::new(pool, config, clock.clone());
    TestApp {
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
}
