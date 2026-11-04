#![cfg(feature = "db-tests")]

mod common;

use axum::http::{Method, StatusCode};
use chrono::Duration;
use common::{TestApp, test_app};
use curl_runner_server::auth::oauth::pkce_challenge;
use curl_runner_server::auth::providers::Provider;
use serde_json::{Value, json};
use sqlx::PgPool;

async fn post(app: &TestApp, path: &str, body: Value, bearer: Option<&str>) -> (StatusCode, Value) {
    app.request(Method::POST, path, Some(body), bearer).await
}

/// Start sign-in and return (state, consent url).
async fn start(app: &TestApp, provider: &str, client: &str) -> (String, String) {
    let (status, body) = post(
        app,
        &format!("/v1/auth/oauth/{provider}/start"),
        json!({"client": client}),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{body}");
    let url = body["url"].as_str().unwrap().to_owned();
    (query_param(&url, "state"), url)
}

fn query_param(url: &str, name: &str) -> String {
    reqwest::Url::parse(url)
        .unwrap()
        .query_pairs()
        .find(|(k, _)| k == name)
        .map(|(_, v)| v.into_owned())
        .unwrap_or_else(|| panic!("no {name} in {url}"))
}

async fn callback(app: &TestApp, provider: &str, state: &str) -> (StatusCode, Value) {
    post(
        app,
        &format!("/v1/auth/oauth/{provider}/callback"),
        json!({"code": "provider-code", "state": state}),
        None,
    )
    .await
}

async fn count(app: &TestApp, table: &str) -> i64 {
    sqlx::query_scalar(&format!("SELECT count(*) FROM {table}"))
        .fetch_one(&app.state.db)
        .await
        .unwrap()
}

#[sqlx::test]
async fn start_unconfigured_provider_404(pool: PgPool) {
    let app = test_app(pool).await;
    app.providers.only_configured(&[Provider::Github]);
    let (status, body) = post(
        &app,
        "/v1/auth/oauth/google/start",
        json!({"client": "web"}),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    assert_eq!(body["error"], "not_found");
    let (status, _) = post(
        &app,
        "/v1/auth/oauth/gitlab/start",
        json!({"client": "web"}),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    assert_eq!(count(&app, "oauth_states").await, 0);
    start(&app, "github", "web").await;
}

#[sqlx::test]
async fn start_builds_url_and_stores_hashed_state(pool: PgPool) {
    let app = test_app(pool).await;
    let (state, url) = start(&app, "github", "device").await;
    assert_eq!(
        query_param(&url, "redirect_uri"),
        "http://localhost:3000/sign-in/github/callback"
    );
    let (hash, verifier, client): (String, String, String) =
        sqlx::query_as("SELECT state_hash, verifier, client FROM oauth_states")
            .fetch_one(&app.state.db)
            .await
            .unwrap();
    assert_ne!(hash, state);
    assert_eq!(client, "device");
    assert_eq!(
        query_param(&url, "code_challenge"),
        pkce_challenge(&verifier)
    );
}

#[sqlx::test]
async fn start_validates_return_to_and_client(pool: PgPool) {
    let app = test_app(pool).await;
    for bad in ["//evil.example.com", "https://evil.example.com", "teams"] {
        let (status, body) = post(
            &app,
            "/v1/auth/oauth/github/start",
            json!({"client": "web", "return_to": bad}),
            None,
        )
        .await;
        assert_eq!(status, StatusCode::BAD_REQUEST, "{bad}");
        assert_eq!(body["error"], "invalid_request");
    }
    let (status, _) = post(
        &app,
        "/v1/auth/oauth/github/start",
        json!({"client": "tv"}),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    let (status, _) = post(
        &app,
        "/v1/auth/oauth/github/start",
        json!({"client": "web", "return_to": "/teams/acme?tab=1"}),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
}

#[sqlx::test]
async fn start_and_callback_are_rate_limited_per_ip(pool: PgPool) {
    let app = test_app(pool).await;
    for _ in 0..20 {
        let (status, _) = app
            .request_with_ip(
                "/v1/auth/oauth/github/start",
                json!({"client": "web"}),
                "9.9.9.9",
            )
            .await;
        assert_eq!(status, StatusCode::OK);
    }
    let (status, body) = app
        .request_with_ip(
            "/v1/auth/oauth/github/start",
            json!({"client": "web"}),
            "9.9.9.9",
        )
        .await;
    assert_eq!(status, StatusCode::TOO_MANY_REQUESTS);
    assert_eq!(body["error"], "rate_limited");
    let (status, _) = app
        .request_with_ip(
            "/v1/auth/oauth/github/start",
            json!({"client": "web"}),
            "8.8.8.8",
        )
        .await;
    assert_eq!(status, StatusCode::OK);
}

#[sqlx::test]
async fn callback_creates_user_and_identity(pool: PgPool) {
    let app = test_app(pool).await;
    app.providers
        .set_identity("583231", "Jane@Example.com", Some("Jane Doe"));
    let (state, url) = post(
        &app,
        "/v1/auth/oauth/github/start",
        json!({"client": "web", "return_to": "/teams/acme"}),
        None,
    )
    .await
    .1["url"]
        .as_str()
        .map(|u| (query_param(u, "state"), u.to_owned()))
        .unwrap();
    let (status, body) = callback(&app, "github", &state).await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(body["user"]["email"], "jane@example.com");
    assert_eq!(body["user"]["name"], "Jane Doe");
    assert_eq!(body["return_to"], "/teams/acme");
    let token = body["session"].as_str().unwrap();
    assert!(token.starts_with("crs_"));

    let (provider, subject, email): (String, String, String) =
        sqlx::query_as("SELECT provider, subject, email::text FROM identities")
            .fetch_one(&app.state.db)
            .await
            .unwrap();
    assert_eq!(
        (provider.as_str(), subject.as_str(), email.as_str()),
        ("github", "583231", "jane@example.com")
    );
    let kind: String = sqlx::query_scalar("SELECT kind FROM sessions")
        .fetch_one(&app.state.db)
        .await
        .unwrap();
    assert_eq!(kind, "web");
    let (status, me) = app.request(Method::GET, "/v1/me", None, Some(token)).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(me["user"]["email"], "jane@example.com");

    // The exchange got the redirect URI and a verifier matching the challenge.
    let seen = app.providers.last_exchange().unwrap();
    assert_eq!(seen.code, "provider-code");
    assert_eq!(seen.redirect_uri, query_param(&url, "redirect_uri"));
    assert_eq!(
        pkce_challenge(&seen.verifier),
        query_param(&url, "code_challenge")
    );
}

#[sqlx::test]
async fn device_client_gets_a_device_session(pool: PgPool) {
    let app = test_app(pool).await;
    app.providers.set_identity("1088", "jane@example.com", None);
    let (state, _) = start(&app, "google", "device").await;
    let (status, body) = callback(&app, "google", &state).await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(body["return_to"], Value::Null);
    let kind: String = sqlx::query_scalar("SELECT kind FROM sessions")
        .fetch_one(&app.state.db)
        .await
        .unwrap();
    assert_eq!(kind, "device");
}

#[sqlx::test]
async fn callback_links_to_existing_email_user(pool: PgPool) {
    let app = test_app(pool).await;
    app.sign_in("jane@example.com").await;
    app.providers
        .set_identity("583231", "jane@example.com", Some("Jane"));
    let (state, _) = start(&app, "github", "web").await;
    let (status, body) = callback(&app, "github", &state).await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(count(&app, "users").await, 1);
    assert_eq!(count(&app, "identities").await, 1);

    app.providers.set_identity("1088", "jane@example.com", None);
    let (state, _) = start(&app, "google", "web").await;
    let (status, _) = callback(&app, "google", &state).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(count(&app, "users").await, 1);
    assert_eq!(count(&app, "identities").await, 2);
}

#[sqlx::test]
async fn known_identity_signs_in_even_if_its_email_changed(pool: PgPool) {
    let app = test_app(pool).await;
    app.providers
        .set_identity("583231", "jane@example.com", None);
    let (state, _) = start(&app, "github", "web").await;
    let first = callback(&app, "github", &state).await.1;
    app.providers
        .set_identity("583231", "jane@new.example.com", None);
    let (state, _) = start(&app, "github", "web").await;
    let (status, second) = callback(&app, "github", &state).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(second["user"]["id"], first["user"]["id"]);
    assert_eq!(count(&app, "users").await, 1);
    assert_eq!(count(&app, "identities").await, 1);
}

#[sqlx::test]
async fn state_single_use_and_expires(pool: PgPool) {
    let app = test_app(pool).await;
    app.providers.set_identity("1", "jane@example.com", None);
    let (state, _) = start(&app, "github", "web").await;
    assert_eq!(callback(&app, "github", &state).await.0, StatusCode::OK);
    let (status, body) = callback(&app, "github", &state).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["error"], "invalid_request");

    let (state, _) = start(&app, "github", "web").await;
    app.clock
        .advance(Duration::minutes(10) + Duration::seconds(1));
    let (status, body) = callback(&app, "github", &state).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["error"], "invalid_request");
    assert_eq!(count(&app, "oauth_states").await, 0);

    // A state is bound to the provider it was issued for.
    let (state, _) = start(&app, "github", "web").await;
    let (status, _) = callback(&app, "google", &state).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    let (status, _) = callback(&app, "github", "not-a-state").await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
}

#[sqlx::test]
async fn unverified_email_rejected(pool: PgPool) {
    let app = test_app(pool).await;
    app.providers
        .set_error("github has no primary verified email");
    let (state, _) = start(&app, "github", "web").await;
    let (status, body) = callback(&app, "github", &state).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
    assert_eq!(body["error"], "unauthorized");
    assert_eq!(count(&app, "users").await, 0);
    assert_eq!(count(&app, "sessions").await, 0);
    // The state is spent either way.
    app.providers.set_identity("1", "jane@example.com", None);
    assert_eq!(
        callback(&app, "github", &state).await.0,
        StatusCode::BAD_REQUEST
    );
}

#[sqlx::test]
async fn handoff_code_single_use_2_minutes(pool: PgPool) {
    let app = test_app(pool).await;
    let web = app.sign_in("jane@example.com").await;
    let (status, body) = post(
        &app,
        "/v1/auth/handoff",
        json!({"session_label": "Studio on laptop"}),
        Some(&web),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{body}");
    let code = body["code"].as_str().unwrap().to_owned();
    assert!(code.starts_with("crh_"));
    let stored: String = sqlx::query_scalar("SELECT code_hash FROM auth_handoffs")
        .fetch_one(&app.state.db)
        .await
        .unwrap();
    assert_ne!(stored, code);

    let (status, body) = post(&app, "/v1/auth/handoff/redeem", json!({"code": code}), None).await;
    assert_eq!(status, StatusCode::OK, "{body}");
    let device = body["session"].as_str().unwrap();
    assert!(device.starts_with("crs_"));
    assert_ne!(device, web);
    assert_eq!(body["user"]["email"], "jane@example.com");
    let (kind, label): (String, Option<String>) =
        sqlx::query_as("SELECT kind, label FROM sessions ORDER BY created_at DESC, kind LIMIT 1")
            .fetch_one(&app.state.db)
            .await
            .unwrap();
    assert_eq!(
        (kind.as_str(), label.as_deref()),
        ("device", Some("Studio on laptop"))
    );
    let (status, _) = app.request(Method::GET, "/v1/me", None, Some(device)).await;
    assert_eq!(status, StatusCode::OK);

    // Single use.
    let (status, body) = post(&app, "/v1/auth/handoff/redeem", json!({"code": code}), None).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
    assert_eq!(body["error"], "code_invalid");

    // Expiry at 2 minutes.
    let (_, body) = post(&app, "/v1/auth/handoff", json!({}), Some(&web)).await;
    let late = body["code"].as_str().unwrap().to_owned();
    app.clock
        .advance(Duration::minutes(2) + Duration::seconds(1));
    let (status, body) = post(&app, "/v1/auth/handoff/redeem", json!({"code": late}), None).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
    assert_eq!(body["error"], "code_expired");

    let (_, body) = post(&app, "/v1/auth/handoff", json!({}), Some(&web)).await;
    let fresh = body["code"].as_str().unwrap().to_owned();
    app.clock.advance(Duration::minutes(1));
    let (status, _) = post(
        &app,
        "/v1/auth/handoff/redeem",
        json!({"code": fresh}),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
}

#[sqlx::test]
async fn handoff_needs_a_live_web_session(pool: PgPool) {
    let app = test_app(pool).await;
    let (status, _) = post(&app, "/v1/auth/handoff", json!({}), None).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);

    // A device session cannot hand off.
    let web = app.sign_in("jane@example.com").await;
    let (_, body) = post(&app, "/v1/auth/handoff", json!({}), Some(&web)).await;
    let (_, body) = post(
        &app,
        "/v1/auth/handoff/redeem",
        json!({"code": body["code"]}),
        None,
    )
    .await;
    let device = body["session"].as_str().unwrap();
    let (status, body) = post(&app, "/v1/auth/handoff", json!({}), Some(device)).await;
    assert_eq!(status, StatusCode::FORBIDDEN);
    assert_eq!(body["error"], "forbidden");

    // Signing the web session out voids its pending codes.
    let (_, body) = post(&app, "/v1/auth/handoff", json!({}), Some(&web)).await;
    let code = body["code"].clone();
    app.request(Method::POST, "/v1/auth/sign-out", None, Some(&web))
        .await;
    let (status, _) = post(&app, "/v1/auth/handoff/redeem", json!({"code": code}), None).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
}
