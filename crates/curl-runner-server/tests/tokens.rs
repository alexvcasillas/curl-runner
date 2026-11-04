#![cfg(feature = "db-tests")]

mod common;

use axum::Router;
use axum::body::Body;
use axum::http::{Method, Request, StatusCode, header};
use axum::routing::get;
use chrono::Duration;
use common::{TestApp, test_app};
use curl_runner_server::tokens::PublishAuth;
use http_body_util::BodyExt;
use serde_json::{Value, json};
use sqlx::PgPool;
use tower::ServiceExt;

/// Test-only route: answers with the publisher label and team id.
async fn probe(auth: PublishAuth) -> String {
    format!("{}|{}", auth.publisher.label(), auth.team_id)
}

async fn publish_probe(app: &TestApp, team: &str, bearer: Option<&str>) -> (StatusCode, String) {
    let router = Router::new()
        .route("/v1/teams/{team}/probe", get(probe))
        .with_state(app.state.clone());
    let mut req = Request::builder().uri(format!("/v1/teams/{team}/probe"));
    if let Some(b) = bearer {
        req = req.header(header::AUTHORIZATION, format!("Bearer {b}"));
    }
    let res = router
        .oneshot(req.body(Body::empty()).unwrap())
        .await
        .unwrap();
    let status = res.status();
    let bytes = res.into_body().collect().await.unwrap().to_bytes();
    (status, String::from_utf8(bytes.to_vec()).unwrap())
}

async fn mint(app: &TestApp, session: &str, team: &str, extra: Value) -> (StatusCode, Value) {
    let mut body = json!({"name": "ci"});
    body.as_object_mut()
        .unwrap()
        .extend(extra.as_object().unwrap().clone());
    app.request(
        Method::POST,
        &format!("/v1/teams/{team}/tokens"),
        Some(body),
        Some(session),
    )
    .await
}

async fn token(app: &TestApp, session: &str, team: &str) -> String {
    let (status, body) = mint(app, session, team, json!({})).await;
    assert_eq!(status, StatusCode::CREATED, "{body}");
    assert!(body["id"].is_string());
    body["token"].as_str().unwrap().to_owned()
}

#[sqlx::test]
async fn token_shown_once_and_hashed(pool: PgPool) {
    let app = test_app(pool).await;
    let s = app.sign_in("a@example.com").await;
    app.create_team(&s, "acme").await;
    let clear = token(&app, &s, "acme").await;
    assert!(clear.starts_with("crt_"));

    let stored: Vec<String> = sqlx::query_scalar("SELECT token_hash FROM tokens")
        .fetch_all(&app.state.db)
        .await
        .unwrap();
    assert_eq!(stored.len(), 1);
    assert!(!stored[0].contains(&clear) && !clear.contains(&stored[0]));

    let (status, list) = app
        .request(Method::GET, "/v1/teams/acme/tokens", None, Some(&s))
        .await;
    assert_eq!(status, StatusCode::OK);
    let text = list.to_string();
    assert!(!text.contains(&clear) && !text.contains(&stored[0]));
    assert!(!text.contains("token_hash") && !text.contains("\"token\""));
    assert_eq!(list[0]["name"], "ci");
    assert_eq!(list[0]["created_by"], "a@example.com");
    assert!(list[0]["last_used_at"].is_null());

    let (status, body) = publish_probe(&app, "acme", Some(&clear)).await;
    assert_eq!(status, StatusCode::OK);
    assert!(body.starts_with("token ci|"));
    let (_, list) = app
        .request(Method::GET, "/v1/teams/acme/tokens", None, Some(&s))
        .await;
    assert!(list[0]["last_used_at"].is_string());
}

#[sqlx::test]
async fn token_for_other_team_404(pool: PgPool) {
    let app = test_app(pool).await;
    let s = app.sign_in("a@example.com").await;
    app.create_team(&s, "acme").await;
    app.create_team(&s, "other").await;
    let clear = token(&app, &s, "acme").await;
    assert_eq!(
        publish_probe(&app, "other", Some(&clear)).await.0,
        StatusCode::NOT_FOUND
    );
    assert_eq!(
        publish_probe(&app, "acme", Some(&clear)).await.0,
        StatusCode::OK
    );
}

#[sqlx::test]
async fn revoked_token_401(pool: PgPool) {
    let app = test_app(pool).await;
    let s = app.sign_in("a@example.com").await;
    app.create_team(&s, "acme").await;
    let (_, created) = mint(&app, &s, "acme", json!({})).await;
    let clear = created["token"].as_str().unwrap();
    let id = created["id"].as_str().unwrap();
    let path = format!("/v1/teams/acme/tokens/{id}");
    let (status, _) = app.request(Method::DELETE, &path, None, Some(&s)).await;
    assert_eq!(status, StatusCode::NO_CONTENT);
    assert_eq!(
        publish_probe(&app, "acme", Some(clear)).await.0,
        StatusCode::UNAUTHORIZED
    );
    let (status, _) = app.request(Method::DELETE, &path, None, Some(&s)).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    let (_, list) = app
        .request(Method::GET, "/v1/teams/acme/tokens", None, Some(&s))
        .await;
    assert_eq!(list, json!([]));
}

#[sqlx::test]
async fn expired_token_401(pool: PgPool) {
    let app = test_app(pool).await;
    let s = app.sign_in("a@example.com").await;
    app.create_team(&s, "acme").await;
    let (status, created) = mint(&app, &s, "acme", json!({"expires_in_days": 1})).await;
    assert_eq!(status, StatusCode::CREATED);
    assert!(created["expires_at"].is_string());
    let clear = created["token"].as_str().unwrap();
    assert_eq!(
        publish_probe(&app, "acme", Some(clear)).await.0,
        StatusCode::OK
    );
    app.clock.advance(Duration::days(1) + Duration::seconds(1));
    assert_eq!(
        publish_probe(&app, "acme", Some(clear)).await.0,
        StatusCode::UNAUTHORIZED
    );
}

#[sqlx::test]
async fn session_publisher_needs_membership(pool: PgPool) {
    let app = test_app(pool).await;
    let a = app.sign_in("a@example.com").await;
    let b = app.sign_in("b@example.com").await;
    app.create_team(&a, "acme").await;
    let (status, body) = publish_probe(&app, "acme", Some(&a)).await;
    assert_eq!(status, StatusCode::OK);
    assert!(body.starts_with("a@example.com|"));
    assert_eq!(
        publish_probe(&app, "acme", Some(&b)).await.0,
        StatusCode::NOT_FOUND
    );
    assert_eq!(
        publish_probe(&app, "acme", None).await.0,
        StatusCode::UNAUTHORIZED
    );
    assert_eq!(
        publish_probe(&app, "acme", Some("junk")).await.0,
        StatusCode::UNAUTHORIZED
    );
    assert_eq!(
        publish_probe(&app, "acme", Some("crt_nope")).await.0,
        StatusCode::UNAUTHORIZED
    );
}

#[sqlx::test]
async fn token_endpoints_need_membership_and_valid_input(pool: PgPool) {
    let app = test_app(pool).await;
    let a = app.sign_in("a@example.com").await;
    let b = app.sign_in("b@example.com").await;
    app.create_team(&a, "acme").await;
    assert_eq!(
        mint(&app, &b, "acme", json!({})).await.0,
        StatusCode::NOT_FOUND
    );
    let (status, _) = app
        .request(Method::GET, "/v1/teams/acme/tokens", None, Some(&b))
        .await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    assert_eq!(
        mint(&app, &a, "acme", json!({"name": " "})).await.0,
        StatusCode::BAD_REQUEST
    );
    assert_eq!(
        mint(&app, &a, "acme", json!({"expires_in_days": 0}))
            .await
            .0,
        StatusCode::BAD_REQUEST
    );
    // A publish token cannot manage tokens.
    let clear = token(&app, &a, "acme").await;
    let (status, _) = app
        .request(Method::GET, "/v1/teams/acme/tokens", None, Some(&clear))
        .await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
}
