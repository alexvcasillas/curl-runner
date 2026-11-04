#![cfg(feature = "db-tests")]

mod common;

use axum::http::{Method, StatusCode};
use common::{test_app, test_app_with, test_config};
use serde_json::json;
use sqlx::PgPool;

#[sqlx::test]
async fn info_lists_email_only_without_oauth(pool: PgPool) {
    let app = test_app_with(pool, test_config(&[("SIGNUP", "domains")])).await;
    let (status, body) = app.request(Method::GET, "/v1/info", None, None).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(
        body,
        json!({
            "version": env!("CARGO_PKG_VERSION"),
            "sign_in": ["email"],
            "signup": "domains",
            "billing": false,
        })
    );
}

#[sqlx::test]
async fn info_lists_github_when_configured(pool: PgPool) {
    let config = test_config(&[
        ("GITHUB_CLIENT_ID", "gh-id"),
        ("GITHUB_CLIENT_SECRET", "gh-secret"),
        ("SIGNUP", "open"),
    ]);
    let app = test_app_with(pool, config).await;
    let (status, body) = app.request(Method::GET, "/v1/info", None, None).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["sign_in"], json!(["email", "github"]));
    assert_eq!(body["signup"], "open");
}

#[sqlx::test]
async fn info_lists_google_after_github(pool: PgPool) {
    let config = test_config(&[
        ("GOOGLE_CLIENT_ID", "g-id"),
        ("GOOGLE_CLIENT_SECRET", "g-secret"),
        ("GITHUB_CLIENT_ID", "gh-id"),
        ("GITHUB_CLIENT_SECRET", "gh-secret"),
    ]);
    let app = test_app_with(pool, config).await;
    let (_, body) = app.request(Method::GET, "/v1/info", None, None).await;
    assert_eq!(body["sign_in"], json!(["email", "github", "google"]));
}

#[sqlx::test]
async fn unknown_route_is_404_json(pool: PgPool) {
    let app = test_app(pool).await;
    let (status, body) = app.request(Method::GET, "/v1/nope", None, None).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    assert_eq!(body["error"], "not_found");
    assert!(body["message"].as_str().is_some_and(|m| !m.is_empty()));
}

#[sqlx::test]
async fn migrations_create_every_table(pool: PgPool) {
    let tables: Vec<String> = sqlx::query_scalar(
        "SELECT table_name::text FROM information_schema.tables \
         WHERE table_schema = 'public' AND table_name <> '_sqlx_migrations' ORDER BY 1",
    )
    .fetch_all(&pool)
    .await
    .unwrap();
    assert_eq!(
        tables,
        [
            "access_entries",
            "auth_handoffs",
            "blobs",
            "collections",
            "email_codes",
            "identities",
            "member_invites",
            "memberships",
            "oauth_states",
            "outbox_emails",
            "sessions",
            "settings",
            "share_opens",
            "teams",
            "tokens",
            "users",
            "version_files",
            "versions",
        ]
    );
}

#[sqlx::test]
async fn users_email_is_case_insensitive(pool: PgPool) {
    let now = chrono::Utc::now();
    sqlx::query("INSERT INTO users (email, created_at) VALUES ($1, $2)")
        .bind("Jane@Example.com")
        .bind(now)
        .execute(&pool)
        .await
        .unwrap();
    let dup = sqlx::query("INSERT INTO users (email, created_at) VALUES ($1, $2)")
        .bind("jane@example.com")
        .bind(now)
        .execute(&pool)
        .await;
    assert!(dup.is_err(), "citext unique must reject a case variant");
}
