#![cfg(feature = "db-tests")]

mod common;

use axum::http::{Method, StatusCode};
use chrono::Duration;
use common::{TestApp, test_app, test_app_with, test_config};
use serde_json::{Value, json};
use sqlx::PgPool;

async fn user_id(app: &TestApp, session: &str) -> String {
    let (_, me) = app
        .request(Method::GET, "/v1/me", None, Some(session))
        .await;
    me["user"]["id"].as_str().unwrap().to_owned()
}

async fn invite(app: &TestApp, session: &str, team: &str, email: &str) -> (StatusCode, Value) {
    app.request(
        Method::POST,
        &format!("/v1/teams/{team}/invites"),
        Some(json!({"email": email, "role": "member"})),
        Some(session),
    )
    .await
}

/// The invite token from the newest invite mail to `email`.
async fn invite_token(app: &TestApp, email: &str) -> String {
    let body: String = sqlx::query_scalar(
        "SELECT text_body FROM outbox_emails WHERE to_addr = $1 ORDER BY ctid DESC LIMIT 1",
    )
    .bind(email)
    .fetch_one(&app.state.db)
    .await
    .unwrap();
    let rest = body.split("/invites/").nth(1).expect("invite link in mail");
    rest.split(|c: char| !(c.is_ascii_alphanumeric() || c == '_' || c == '-'))
        .next()
        .unwrap()
        .to_owned()
}

async fn accept(app: &TestApp, session: &str, token: &str) -> (StatusCode, Value) {
    app.request(
        Method::POST,
        &format!("/v1/invites/{token}/accept"),
        None,
        Some(session),
    )
    .await
}

async fn set_plan(app: &TestApp, session: &str, team: &str, plan: &str) -> (StatusCode, Value) {
    app.request(
        Method::PUT,
        &format!("/v1/admin/teams/{team}/plan"),
        Some(json!({"plan": plan})),
        Some(session),
    )
    .await
}

#[sqlx::test]
async fn create_team_makes_creator_owner(pool: PgPool) {
    let app = test_app(pool).await;
    let s = app.sign_in("ada@example.com").await;
    let (status, body) = app
        .request(
            Method::POST,
            "/v1/teams",
            Some(json!({"slug": "acme", "name": "Acme"})),
            Some(&s),
        )
        .await;
    assert_eq!(status, StatusCode::CREATED, "{body}");
    assert_eq!(
        body,
        json!({"slug": "acme", "name": "Acme", "plan": "free"})
    );
    let (status, team) = app
        .request(Method::GET, "/v1/teams/acme", None, Some(&s))
        .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(team["members"][0]["role"], "owner");
    assert_eq!(team["members"][0]["email"], "ada@example.com");
    let (status, body) = app
        .request(
            Method::POST,
            "/v1/teams",
            Some(json!({"slug": "acme", "name": "Other"})),
            Some(&s),
        )
        .await;
    assert_eq!(status, StatusCode::CONFLICT, "{body}");
    let (status, body) = app
        .request(
            Method::POST,
            "/v1/teams",
            Some(json!({"slug": "Bad Slug", "name": "x"})),
            Some(&s),
        )
        .await;
    assert_eq!(status, StatusCode::BAD_REQUEST, "{body}");
}

#[sqlx::test]
async fn create_team_closed_signup_403(pool: PgPool) {
    let app = test_app_with(pool, test_config(&[("SIGNUP", "closed")])).await;
    let s = app.sign_in("ada@example.com").await;
    let (status, body) = app
        .request(
            Method::POST,
            "/v1/teams",
            Some(json!({"slug": "acme", "name": "Acme"})),
            Some(&s),
        )
        .await;
    assert_eq!(status, StatusCode::FORBIDDEN);
    assert_eq!(body["error"], "signup_closed");
}

#[sqlx::test]
async fn create_team_domains_allows_listed_domain(pool: PgPool) {
    let app = test_app_with(
        pool,
        test_config(&[("SIGNUP", "domains"), ("SIGNUP_DOMAINS", "acme.io")]),
    )
    .await;
    let outsider = app.sign_in("eve@other.com").await;
    let (status, _) = app
        .request(
            Method::POST,
            "/v1/teams",
            Some(json!({"slug": "x", "name": "X"})),
            Some(&outsider),
        )
        .await;
    assert_eq!(status, StatusCode::FORBIDDEN);
    let insider = app.sign_in("Ada@ACME.io").await;
    app.create_team(&insider, "acme").await;
}

#[sqlx::test]
async fn first_user_on_empty_domains_becomes_operator(pool: PgPool) {
    let app = test_app_with(pool, test_config(&[("SIGNUP", "domains")])).await;
    let first = app.sign_in("ada@example.com").await;
    let second = app.sign_in("bob@example.com").await;
    // The second account cannot, even if it asks first.
    let (status, _) = app
        .request(
            Method::POST,
            "/v1/teams",
            Some(json!({"slug": "bob", "name": "Bob"})),
            Some(&second),
        )
        .await;
    assert_eq!(status, StatusCode::FORBIDDEN);
    app.create_team(&first, "acme").await;
    let value: String =
        sqlx::query_scalar("SELECT value FROM settings WHERE key = 'first_operator'")
            .fetch_one(&app.state.db)
            .await
            .unwrap();
    assert_eq!(value, "ada@example.com");
    let (status, _) = set_plan(&app, &first, "acme", "free").await;
    assert_eq!(status, StatusCode::OK);
}

#[sqlx::test]
async fn operator_can_create_when_closed(pool: PgPool) {
    let app = test_app_with(
        pool,
        test_config(&[("SIGNUP", "closed"), ("OPERATOR_EMAILS", "Ops@example.com")]),
    )
    .await;
    let s = app.sign_in("ops@example.com").await;
    app.create_team(&s, "acme").await;
}

#[sqlx::test]
async fn new_team_plan_follows_signup_mode(pool: PgPool) {
    for (mode, plan) in [("domains", "team"), ("closed", "free"), ("open", "free")] {
        let cfg = test_config(&[
            ("SIGNUP", mode),
            ("SIGNUP_DOMAINS", "example.com"),
            ("OPERATOR_EMAILS", "ada@example.com"),
        ]);
        let app = test_app_with(pool.clone(), cfg).await;
        let s = app.sign_in("ada@example.com").await;
        let slug = format!("t-{mode}");
        let (status, body) = app
            .request(
                Method::POST,
                "/v1/teams",
                Some(json!({"slug": slug, "name": "T"})),
                Some(&s),
            )
            .await;
        assert_eq!(status, StatusCode::CREATED, "{mode}: {body}");
        assert_eq!(body["plan"], plan, "{mode}");
    }
}

#[sqlx::test]
async fn free_team_fourth_invite_402(pool: PgPool) {
    let app = test_app(pool).await;
    let owner = app.sign_in("ada@example.com").await;
    app.create_team(&owner, "acme").await;
    let (status, body) = invite(&app, &owner, "acme", "bob@example.com").await;
    assert_eq!(status, StatusCode::CREATED, "{body}");
    let token = invite_token(&app, "bob@example.com").await;
    assert!(token.starts_with("cri_"));
    let bob = app.sign_in("bob@example.com").await;
    assert_eq!(accept(&app, &bob, &token).await.0, StatusCode::OK);
    // 2 members + 1 pending = 3.
    assert_eq!(
        invite(&app, &owner, "acme", "cy@example.com").await.0,
        StatusCode::CREATED
    );
    let (status, body) = invite(&app, &owner, "acme", "di@example.com").await;
    assert_eq!(status, StatusCode::PAYMENT_REQUIRED, "{body}");
    assert_eq!(body["error"], "plan_limit");
    // An expired pending invite stops counting.
    app.clock.advance(Duration::days(8));
    assert_eq!(
        invite(&app, &owner, "acme", "di@example.com").await.0,
        StatusCode::CREATED
    );
}

#[sqlx::test]
async fn invite_accept_rechecks_limit(pool: PgPool) {
    let ops = test_config(&[("OPERATOR_EMAILS", "ada@example.com")]);
    let app = test_app_with(pool, ops).await;
    let owner = app.sign_in("ada@example.com").await;
    app.create_team(&owner, "acme").await;
    set_plan(&app, &owner, "acme", "team").await;
    let emails = ["b@example.com", "c@example.com", "d@example.com"];
    for e in emails {
        assert_eq!(invite(&app, &owner, "acme", e).await.0, StatusCode::CREATED);
    }
    set_plan(&app, &owner, "acme", "free").await;
    let mut results = vec![];
    for e in emails {
        let token = invite_token(&app, e).await;
        let s = app.sign_in(e).await;
        results.push(accept(&app, &s, &token).await.0);
    }
    assert_eq!(
        results,
        [StatusCode::OK, StatusCode::OK, StatusCode::PAYMENT_REQUIRED]
    );
}

#[sqlx::test]
async fn invite_expires_after_7_days(pool: PgPool) {
    let app = test_app(pool).await;
    let owner = app.sign_in("ada@example.com").await;
    app.create_team(&owner, "acme").await;
    invite(&app, &owner, "acme", "bob@example.com").await;
    let token = invite_token(&app, "bob@example.com").await;
    let bob = app.sign_in("bob@example.com").await;
    app.clock.advance(Duration::days(7) + Duration::seconds(1));
    let (status, body) = accept(&app, &bob, &token).await;
    assert_eq!(status, StatusCode::NOT_FOUND, "{body}");
    // The invite is stored hashed, and accepting twice is a 404.
    let leaked: i64 =
        sqlx::query_scalar("SELECT count(*) FROM member_invites WHERE token_hash = $1")
            .bind(&token)
            .fetch_one(&app.state.db)
            .await
            .unwrap();
    assert_eq!(leaked, 0);
}

#[sqlx::test]
async fn accepted_invite_cannot_be_reused_and_can_use_any_email(pool: PgPool) {
    let app = test_app(pool).await;
    let owner = app.sign_in("ada@example.com").await;
    app.create_team(&owner, "acme").await;
    invite(&app, &owner, "acme", "bob@example.com").await;
    let token = invite_token(&app, "bob@example.com").await;
    let other = app.sign_in("zed@example.com").await;
    let (status, body) = accept(&app, &other, &token).await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(body["slug"], "acme");
    let third = app.sign_in("yan@example.com").await;
    assert_eq!(accept(&app, &third, &token).await.0, StatusCode::NOT_FOUND);
    let (_, team) = app
        .request(Method::GET, "/v1/teams/acme", None, Some(&owner))
        .await;
    assert_eq!(team["members"].as_array().unwrap().len(), 2);
    assert_eq!(team["pending_invites"].as_array().unwrap().len(), 0);
}

#[sqlx::test]
async fn non_member_gets_404(pool: PgPool) {
    let app = test_app(pool).await;
    let owner = app.sign_in("ada@example.com").await;
    app.create_team(&owner, "acme").await;
    let eve = app.sign_in("eve@example.com").await;
    let owner_id = user_id(&app, &owner).await;
    let (status, _) = app
        .request(Method::GET, "/v1/teams/acme", None, Some(&eve))
        .await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    assert_eq!(
        invite(&app, &eve, "acme", "x@example.com").await.0,
        StatusCode::NOT_FOUND
    );
    let (status, _) = app
        .request(
            Method::DELETE,
            &format!("/v1/teams/acme/members/{owner_id}"),
            None,
            Some(&eve),
        )
        .await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    let (status, _) = app
        .request(Method::GET, "/v1/teams/nope", None, Some(&eve))
        .await;
    assert_eq!(status, StatusCode::NOT_FOUND);
}

#[sqlx::test]
async fn members_cannot_invite_or_change_roles(pool: PgPool) {
    let app = test_app(pool).await;
    let owner = app.sign_in("ada@example.com").await;
    app.create_team(&owner, "acme").await;
    invite(&app, &owner, "acme", "bob@example.com").await;
    let token = invite_token(&app, "bob@example.com").await;
    let bob = app.sign_in("bob@example.com").await;
    accept(&app, &bob, &token).await;
    let owner_id = user_id(&app, &owner).await;
    let bob_id = user_id(&app, &bob).await;
    assert_eq!(
        invite(&app, &bob, "acme", "x@example.com").await.0,
        StatusCode::FORBIDDEN
    );
    let (status, _) = app
        .request(
            Method::PATCH,
            &format!("/v1/teams/acme/members/{bob_id}"),
            Some(json!({"role": "owner"})),
            Some(&bob),
        )
        .await;
    assert_eq!(status, StatusCode::FORBIDDEN);
    let (status, _) = app
        .request(
            Method::DELETE,
            &format!("/v1/teams/acme/members/{owner_id}"),
            None,
            Some(&bob),
        )
        .await;
    assert_eq!(status, StatusCode::FORBIDDEN);
    // A member can leave.
    let (status, _) = app
        .request(
            Method::DELETE,
            &format!("/v1/teams/acme/members/{bob_id}"),
            None,
            Some(&bob),
        )
        .await;
    assert_eq!(status, StatusCode::NO_CONTENT);
}

#[sqlx::test]
async fn cannot_remove_last_owner(pool: PgPool) {
    let app = test_app(pool).await;
    let owner = app.sign_in("ada@example.com").await;
    app.create_team(&owner, "acme").await;
    invite(&app, &owner, "acme", "bob@example.com").await;
    let token = invite_token(&app, "bob@example.com").await;
    let bob = app.sign_in("bob@example.com").await;
    accept(&app, &bob, &token).await;
    let owner_id = user_id(&app, &owner).await;
    let bob_id = user_id(&app, &bob).await;
    let remove = format!("/v1/teams/acme/members/{owner_id}");
    let (status, body) = app
        .request(Method::DELETE, &remove, None, Some(&owner))
        .await;
    assert_eq!(status, StatusCode::CONFLICT, "{body}");
    let patch = |role: &'static str| json!({ "role": role });
    let (status, _) = app
        .request(
            Method::PATCH,
            &format!("/v1/teams/acme/members/{owner_id}"),
            Some(patch("member")),
            Some(&owner),
        )
        .await;
    assert_eq!(status, StatusCode::CONFLICT);
    // Promote bob, then the first owner can leave.
    let (status, body) = app
        .request(
            Method::PATCH,
            &format!("/v1/teams/acme/members/{bob_id}"),
            Some(patch("owner")),
            Some(&owner),
        )
        .await;
    assert_eq!(status, StatusCode::OK, "{body}");
    let (status, _) = app
        .request(Method::DELETE, &remove, None, Some(&owner))
        .await;
    assert_eq!(status, StatusCode::NO_CONTENT);
    let (status, _) = app
        .request(Method::GET, "/v1/teams/acme", None, Some(&owner))
        .await;
    assert_eq!(status, StatusCode::NOT_FOUND);
}

#[sqlx::test]
async fn admin_plan_endpoint_operator_only(pool: PgPool) {
    let app = test_app_with(pool, test_config(&[("OPERATOR_EMAILS", "ops@example.com")])).await;
    let owner = app.sign_in("ada@example.com").await;
    app.create_team(&owner, "acme").await;
    let ops = app.sign_in("ops@example.com").await;
    // A team owner who is not an operator gets 404.
    assert_eq!(
        set_plan(&app, &owner, "acme", "team").await.0,
        StatusCode::NOT_FOUND
    );
    let (status, body) = set_plan(&app, &ops, "acme", "team").await;
    assert_eq!(status, StatusCode::OK, "{body}");
    let (_, team) = app
        .request(Method::GET, "/v1/teams/acme", None, Some(&owner))
        .await;
    assert_eq!(team["plan"], "team");
    assert_eq!(
        set_plan(&app, &ops, "missing", "team").await.0,
        StatusCode::NOT_FOUND
    );
}
