#![cfg(feature = "db-tests")]

mod common;

use axum::body::Body;
use axum::http::{Method, Request, StatusCode, header};
use common::{TestApp, test_app_with, test_config};
use http_body_util::BodyExt;
use serde_json::{Value, json};
use sqlx::PgPool;
use tower::ServiceExt;

const OWNER: &str = "ada@example.com";

async fn app_with(pool: PgPool) -> TestApp {
    test_app_with(pool, test_config(&[("OPERATOR_EMAILS", OWNER)])).await
}

async fn req(
    app: &TestApp,
    method: &str,
    path: &str,
    body: Option<Value>,
    bearer: &str,
) -> (StatusCode, Value) {
    app.request(method.parse::<Method>().unwrap(), path, body, Some(bearer))
        .await
}

async fn uid(app: &TestApp, session: &str) -> String {
    let (_, me) = req(app, "GET", "/v1/me", None, session).await;
    me["user"]["id"].as_str().unwrap().to_owned()
}

async fn raw_get(app: &TestApp, path: &str, bearer: &str) -> (StatusCode, String, String) {
    let res = curl_runner_server::app(app.state.clone())
        .oneshot(
            Request::builder()
                .uri(path)
                .header(header::AUTHORIZATION, format!("Bearer {bearer}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    let status = res.status();
    let disposition = res
        .headers()
        .get(header::CONTENT_DISPOSITION)
        .map(|v| v.to_str().unwrap().to_owned())
        .unwrap_or_default();
    let bytes = res.into_body().collect().await.unwrap().to_bytes();
    (
        status,
        disposition,
        String::from_utf8_lossy(&bytes).into_owned(),
    )
}

async fn scalar(app: &TestApp, sql: &str, bind: &str) -> i64 {
    sqlx::query_scalar(sql)
        .bind(bind)
        .fetch_one(&app.state.db)
        .await
        .unwrap()
}

/// Sign in with GitHub as `subject`/`email`; returns the callback's answer.
async fn github_sign_in(app: &TestApp, subject: &str, email: &str) -> (StatusCode, Value) {
    let (status, start) = app
        .request(
            Method::POST,
            "/v1/auth/oauth/github/start",
            Some(json!({"client": "web"})),
            None,
        )
        .await;
    assert_eq!(status, StatusCode::OK, "{start}");
    let url = reqwest::Url::parse(start["url"].as_str().unwrap()).unwrap();
    let state = url
        .query_pairs()
        .find(|(k, _)| k == "state")
        .unwrap()
        .1
        .into_owned();
    app.providers.set_identity(subject, email, Some("Jane"));
    app.request(
        Method::POST,
        "/v1/auth/oauth/github/callback",
        Some(json!({"code": "c", "state": state, "binding": start["binding"]})),
        None,
    )
    .await
}

fn suspended_error() -> Value {
    json!({"error": "account_suspended", "message": "This account is suspended. Contact support."})
}

#[sqlx::test]
async fn suspended_user_cannot_sign_in_any_method(pool: PgPool) {
    let app = app_with(pool).await;
    let ada = app.sign_in(OWNER).await;
    let (status, _) = github_sign_in(&app, "gh-1", "jane@example.com").await;
    assert_eq!(status, StatusCode::OK);
    let jane = app.sign_in("jane@example.com").await;
    let jane_id = uid(&app, &jane).await;
    // A hand-off code minted before the suspension.
    let (_, code) = req(
        &app,
        "POST",
        "/v1/auth/handoff",
        Some(json!({"session_label": "studio"})),
        &jane,
    )
    .await;

    let (status, body) = req(
        &app,
        "POST",
        &format!("/v1/admin/users/{jane_id}/suspend"),
        Some(json!({"reason": "chargeback", "notify": false})),
        &ada,
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(body["status"], "suspended");

    // Email code.
    let (code_in_mail, _) = app.start_email("jane@example.com", "web").await;
    let (status, body) = app
        .request(
            Method::POST,
            "/v1/auth/email/verify",
            Some(json!({"email": "jane@example.com", "code": code_in_mail})),
            None,
        )
        .await;
    assert_eq!((status, body), (StatusCode::FORBIDDEN, suspended_error()));
    // A wrong code says nothing about the account.
    let (status, body) = app
        .request(
            Method::POST,
            "/v1/auth/email/verify",
            Some(json!({"email": "jane@example.com", "code": "000000"})),
            None,
        )
        .await;
    assert_eq!(status, StatusCode::UNAUTHORIZED, "{body}");
    // OAuth.
    let (status, body) = github_sign_in(&app, "gh-1", "jane@example.com").await;
    assert_eq!((status, body), (StatusCode::FORBIDDEN, suspended_error()));
    // Hand-off redeem.
    let (status, _) = app
        .request(
            Method::POST,
            "/v1/auth/handoff/redeem",
            Some(json!({"code": code["code"]})),
            None,
        )
        .await;
    assert_ne!(status, StatusCode::OK);
    // The old session is gone, and nothing was left behind.
    let (status, _) = req(&app, "GET", "/v1/me", None, &jane).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
    assert_eq!(
        scalar(
            &app,
            "SELECT count(*) FROM sessions WHERE user_id = $1::uuid",
            &jane_id
        )
        .await,
        0
    );
}

#[sqlx::test]
async fn suspend_deletes_sessions_tokens_keep_publishing(pool: PgPool) {
    let app = app_with(pool).await;
    let ada = app.sign_in(OWNER).await;
    let jane = app.sign_in("jane@example.com").await;
    app.create_team(&jane, "acme").await;
    let (status, token) = req(
        &app,
        "POST",
        "/v1/teams/acme/tokens",
        Some(json!({"name": "ci"})),
        &jane,
    )
    .await;
    assert_eq!(status, StatusCode::CREATED, "{token}");
    let token = token["token"].as_str().unwrap().to_owned();
    let jane_id = uid(&app, &jane).await;

    let (status, _) = req(
        &app,
        "POST",
        &format!("/v1/admin/users/{jane_id}/suspend"),
        Some(json!({"reason": "abuse", "notify": true})),
        &ada,
    )
    .await;
    assert_eq!(status, StatusCode::OK);

    // The token still publishes.
    let files = vec![curl_runner_cloud_proto::BundleFile {
        path: "a.yaml".into(),
        text: "request:\n  url: https://example.com\n".into(),
    }];
    let (status, body) = req(
        &app,
        "PUT",
        "/v1/teams/acme/collections/api/versions",
        Some(json!({"hash": curl_runner_cloud_proto::bundle_hash(&files), "files": files, "inputs": []})),
        &token,
    )
    .await;
    assert_eq!(status, StatusCode::CREATED, "{body}");
    let (status, _) = req(&app, "GET", "/v1/teams/acme", None, &jane).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
    // The team is untouched.
    assert_eq!(
        scalar(
            &app,
            "SELECT count(*) FROM memberships WHERE user_id = $1::uuid",
            &jane_id
        )
        .await,
        1
    );
    assert_eq!(
        scalar(
            &app,
            "SELECT count(*) FROM tokens WHERE revoked_at IS NULL AND name = $1",
            "ci"
        )
        .await,
        1
    );
    // The notice is queued with the reason, under its own kind.
    let (subject, text, kind, user_id): (String, String, String, Option<uuid::Uuid>) =
        sqlx::query_as(
            "SELECT subject, text_body, kind, user_id FROM outbox_emails \
             WHERE to_addr = 'jane@example.com' AND subject LIKE '%suspended%'",
        )
        .fetch_one(&app.state.db)
        .await
        .unwrap();
    assert!(text.contains("abuse"), "{text}");
    assert!(!subject.contains('\u{2014}') && !text.contains('\u{2014}'));
    assert_eq!(kind, "account_suspended");
    assert_eq!(user_id.map(|u| u.to_string()), Some(jane_id.clone()));
    // Logged, with the target.
    let (action, label): (String, String) = sqlx::query_as(
        "SELECT action, target_label FROM admin_events WHERE action = 'user.suspend'",
    )
    .fetch_one(&app.state.db)
    .await
    .unwrap();
    assert_eq!(
        (action.as_str(), label.as_str()),
        ("user.suspend", "jane@example.com")
    );
}

#[sqlx::test]
async fn unsuspend_restores_sign_in(pool: PgPool) {
    let app = app_with(pool).await;
    let ada = app.sign_in(OWNER).await;
    let jane = app.sign_in("jane@example.com").await;
    let jane_id = uid(&app, &jane).await;
    let suspend = format!("/v1/admin/users/{jane_id}/suspend");
    let (status, _) = req(&app, "POST", &suspend, Some(json!({"notify": false})), &ada).await;
    assert_eq!(status, StatusCode::OK);
    let (status, _) = req(&app, "POST", &suspend, Some(json!({"notify": false})), &ada).await;
    assert_eq!(status, StatusCode::CONFLICT, "already suspended");
    let (status, body) = req(
        &app,
        "POST",
        &format!("/v1/admin/users/{jane_id}/unsuspend"),
        None,
        &ada,
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(body["status"], "active");
    app.sign_in("jane@example.com").await;
    assert_eq!(
        scalar(
            &app,
            "SELECT count(*) FROM admin_events WHERE action = $1",
            "user.unsuspend"
        )
        .await,
        1
    );
}

#[sqlx::test]
async fn owners_and_self_cannot_be_suspended_or_deleted(pool: PgPool) {
    let app = app_with(pool).await;
    let ada = app.sign_in(OWNER).await;
    let ada_id = uid(&app, &ada).await;
    for (method, path, body) in [
        (
            "POST",
            format!("/v1/admin/users/{ada_id}/suspend"),
            Some(json!({"notify": false})),
        ),
        ("DELETE", format!("/v1/admin/users/{ada_id}"), None),
    ] {
        let (status, out) = req(&app, method, &path, body, &ada).await;
        assert_eq!(status, StatusCode::CONFLICT, "{method} {path}: {out}");
    }
}

#[sqlx::test]
async fn delete_sole_owner_409(pool: PgPool) {
    let app = app_with(pool).await;
    let ada = app.sign_in(OWNER).await;
    let jane = app.sign_in("jane@example.com").await;
    app.create_team(&jane, "acme").await;
    let jane_id = uid(&app, &jane).await;
    let (status, body) = req(
        &app,
        "DELETE",
        &format!("/v1/admin/users/{jane_id}"),
        None,
        &ada,
    )
    .await;
    assert_eq!(status, StatusCode::CONFLICT);
    assert_eq!(body["error"], "conflict");
    assert_eq!(
        body["message"],
        "jane@example.com is the only owner of acme. Make someone else an owner, or delete acme first."
    );
    assert_eq!(
        scalar(
            &app,
            "SELECT count(*) FROM users WHERE email = $1",
            "jane@example.com"
        )
        .await,
        1
    );
    assert_eq!(
        scalar(
            &app,
            "SELECT count(*) FROM admin_events WHERE action = $1",
            "user.delete"
        )
        .await,
        0
    );
}

#[sqlx::test]
async fn delete_removes_identities_sessions_access_matches(pool: PgPool) {
    let app = app_with(pool).await;
    let ada = app.sign_in(OWNER).await;
    let jane = app.sign_in("jane@example.com").await;
    app.create_team(&jane, "acme").await;
    // A second owner, so jane may go.
    let bob = app.sign_in("bob@example.com").await;
    let bob_id = uid(&app, &bob).await;
    let (status, _) = req(
        &app,
        "POST",
        "/v1/teams/acme/invites",
        Some(json!({"email": "bob@example.com", "role": "owner"})),
        &jane,
    )
    .await;
    assert_eq!(status, StatusCode::CREATED);
    let token: String = sqlx::query_scalar("SELECT text_body FROM outbox_emails WHERE to_addr = 'bob@example.com' AND subject LIKE '%invited%'")
        .fetch_one(&app.state.db)
        .await
        .unwrap();
    let token = token
        .split("/invites/")
        .nth(1)
        .unwrap()
        .split(['&', '\n', ' ', '?'])
        .next()
        .unwrap()
        .to_owned();
    let (status, body) = req(
        &app,
        "POST",
        &format!("/v1/invites/{token}/accept"),
        None,
        &bob,
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{body}");
    let _ = bob_id;
    // A publish token jane made, a collection with a mix of access entries.
    let (_, tok) = req(
        &app,
        "POST",
        "/v1/teams/acme/tokens",
        Some(json!({"name": "ci"})),
        &jane,
    )
    .await;
    assert!(tok["token"].as_str().is_some(), "{tok}");
    let team_id: uuid::Uuid = sqlx::query_scalar("SELECT id FROM teams WHERE slug = 'acme'")
        .fetch_one(&app.state.db)
        .await
        .unwrap();
    let coll: uuid::Uuid = sqlx::query_scalar(
        "INSERT INTO collections (team_id, slug, name, access) VALUES ($1, 'api', 'api', 'restricted') RETURNING id",
    )
    .bind(team_id)
    .fetch_one(&app.state.db)
    .await
    .unwrap();
    for (entry, kind) in [
        ("jane@example.com", "email"),
        ("*@example.com", "domain"),
        ("other@example.com", "email"),
    ] {
        sqlx::query("INSERT INTO access_entries (collection_id, entry, kind, invited_at) VALUES ($1, $2, $3, now())")
            .bind(coll)
            .bind(entry)
            .bind(kind)
            .execute(&app.state.db)
            .await
            .unwrap();
    }
    github_sign_in(&app, "gh-jane", "jane@example.com").await;
    let jane_id = uid(&app, &jane).await;
    sqlx::query("UPDATE teams SET seats_synced = 2 WHERE id = $1")
        .bind(team_id)
        .execute(&app.state.db)
        .await
        .unwrap();

    let (status, body) = req(
        &app,
        "DELETE",
        &format!("/v1/admin/users/{jane_id}"),
        None,
        &ada,
    )
    .await;
    assert_eq!(status, StatusCode::NO_CONTENT, "{body}");
    for (table, col) in [
        ("identities", "user_id"),
        ("sessions", "user_id"),
        ("memberships", "user_id"),
    ] {
        assert_eq!(
            scalar(
                &app,
                &format!("SELECT count(*) FROM {table} WHERE {col} = $1::uuid"),
                &jane_id
            )
            .await,
            0,
            "{table}"
        );
    }
    assert_eq!(
        scalar(
            &app,
            "SELECT count(*) FROM users WHERE id = $1::uuid",
            &jane_id
        )
        .await,
        0
    );
    // The exact email entry went, the domain and the other email stayed.
    let left: Vec<String> =
        sqlx::query_scalar("SELECT entry::text FROM access_entries ORDER BY entry")
            .fetch_all(&app.state.db)
            .await
            .unwrap();
    assert_eq!(left, ["*@example.com", "other@example.com"]);
    // The team and its token stay, with no creator. The seat count is dirty.
    assert_eq!(
        scalar(
            &app,
            "SELECT count(*) FROM tokens WHERE name = $1 AND created_by IS NULL",
            "ci"
        )
        .await,
        1
    );
    let seats: Option<i32> = sqlx::query_scalar("SELECT seats_synced FROM teams WHERE id = $1")
        .bind(team_id)
        .fetch_one(&app.state.db)
        .await
        .unwrap();
    assert_eq!(seats, None);
    // The log names the person even though the account is gone.
    let (label, id): (String, String) = sqlx::query_as(
        "SELECT target_label, target_id FROM admin_events WHERE action = 'user.delete'",
    )
    .fetch_one(&app.state.db)
    .await
    .unwrap();
    assert_eq!(
        (label.as_str(), id.as_str()),
        ("jane@example.com", jane_id.as_str())
    );
    let admin_email: String =
        sqlx::query_scalar("SELECT admin_email FROM admin_events WHERE action = 'user.delete'")
            .fetch_one(&app.state.db)
            .await
            .unwrap();
    assert_eq!(admin_email, OWNER);
}

#[sqlx::test]
async fn unlink_github_then_email_relinks(pool: PgPool) {
    let app = app_with(pool).await;
    let ada = app.sign_in(OWNER).await;
    github_sign_in(&app, "gh-1", "jane@example.com").await;
    let jane = app.sign_in("jane@example.com").await;
    let jane_id = uid(&app, &jane).await;
    let base = format!("/v1/admin/users/{jane_id}/identities");

    let (status, body) = req(&app, "DELETE", &format!("{base}/email"), None, &ada).await;
    assert_eq!(status, StatusCode::BAD_REQUEST, "{body}");
    assert_eq!(body["error"], "invalid_request");
    let (status, _) = req(&app, "DELETE", &format!("{base}/google"), None, &ada).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    let (status, _) = req(&app, "DELETE", &format!("{base}/github"), None, &ada).await;
    assert_eq!(status, StatusCode::NO_CONTENT);
    assert_eq!(
        scalar(
            &app,
            "SELECT count(*) FROM identities WHERE user_id = $1::uuid",
            &jane_id
        )
        .await,
        0
    );

    // The next GitHub sign-in links to the same account by email.
    let (status, body) = github_sign_in(&app, "gh-1", "jane@example.com").await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(body["user"]["id"], jane_id);
    assert_eq!(
        scalar(
            &app,
            "SELECT count(*) FROM identities WHERE user_id = $1::uuid",
            &jane_id
        )
        .await,
        1
    );
    assert_eq!(
        scalar(
            &app,
            "SELECT count(*) FROM users WHERE email = $1",
            "jane@example.com"
        )
        .await,
        1
    );
}

#[sqlx::test]
async fn search_by_email_and_name(pool: PgPool) {
    let app = app_with(pool).await;
    let ada = app.sign_in(OWNER).await;
    app.sign_in("grace@example.com").await;
    app.sign_in("linus@example.com").await;
    sqlx::query("UPDATE users SET name = 'Grace Hopper' WHERE email = 'grace@example.com'")
        .execute(&app.state.db)
        .await
        .unwrap();
    let emails = |body: &Value| -> Vec<String> {
        body["items"]
            .as_array()
            .unwrap()
            .iter()
            .map(|u| u["email"].as_str().unwrap().to_owned())
            .collect()
    };
    for (q, expect) in [
        ("grace", vec!["grace@example.com"]),
        ("GRACE@EX", vec!["grace@example.com"]),
        ("hopper", vec!["grace@example.com"]),
        ("inus", vec!["linus@example.com"]),
        ("nobody", vec![]),
        ("100%", vec![]),
        ("_", vec![]),
    ] {
        let (status, body) = req(&app, "GET", &format!("/v1/admin/users?q={q}"), None, &ada).await;
        assert_eq!(status, StatusCode::OK, "{q}: {body}");
        assert_eq!(emails(&body), expect, "{q}");
        assert_eq!(body["total"], expect.len(), "{q}");
    }
}

#[sqlx::test]
async fn customers_filter_is_users_with_no_team(pool: PgPool) {
    let app = app_with(pool).await;
    let ada = app.sign_in(OWNER).await;
    let jane = app.sign_in("jane@example.com").await;
    app.create_team(&jane, "acme").await;
    app.sign_in("cust@example.com").await;
    let rosa = app.sign_in("rosa@example.com").await;
    let rosa_id = uid(&app, &rosa).await;
    sqlx::query(
        "INSERT INTO admins (user_id, permissions, added_at) VALUES ($1::uuid, '{users}', now())",
    )
    .bind(&rosa_id)
    .execute(&app.state.db)
    .await
    .unwrap();
    let suspended = uid(&app, &app.sign_in("sus@example.com").await).await;
    let (status, _) = req(
        &app,
        "POST",
        &format!("/v1/admin/users/{suspended}/suspend"),
        Some(json!({"notify": false})),
        &ada,
    )
    .await;
    assert_eq!(status, StatusCode::OK);

    let list = |filter: &'static str| {
        let app = &app;
        let ada = &ada;
        async move {
            let (status, body) = req(
                app,
                "GET",
                &format!("/v1/admin/users?filter={filter}"),
                None,
                ada,
            )
            .await;
            assert_eq!(status, StatusCode::OK, "{filter}: {body}");
            let mut emails: Vec<String> = body["items"]
                .as_array()
                .unwrap()
                .iter()
                .map(|u| u["email"].as_str().unwrap().to_owned())
                .collect();
            emails.sort();
            emails
        }
    };
    assert_eq!(list("members").await, ["jane@example.com"]);
    assert_eq!(
        list("customers").await,
        [
            "ada@example.com",
            "cust@example.com",
            "rosa@example.com",
            "sus@example.com"
        ]
    );
    assert_eq!(list("suspended").await, ["sus@example.com"]);
    assert_eq!(
        list("admins").await,
        ["ada@example.com", "rosa@example.com"]
    );
    assert_eq!(list("all").await.len(), 5);
    let (status, _) = req(&app, "GET", "/v1/admin/users?filter=nope", None, &ada).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);

    // Row shape.
    let (_, body) = req(&app, "GET", "/v1/admin/users?q=jane", None, &ada).await;
    let row = &body["items"][0];
    assert_eq!(row["status"], "active");
    assert_eq!(row["methods"], json!(["email"]));
    assert_eq!(
        row["teams"],
        json!([{"slug": "acme", "name": "acme", "role": "owner"}])
    );
    assert_eq!(row["shared_in"], 0);
    assert!(row["last_active"].is_string());
}

#[sqlx::test]
async fn list_pages_by_cursor_and_exports_csv(pool: PgPool) {
    let app = app_with(pool).await;
    let ada = app.sign_in(OWNER).await;
    for i in 0..4 {
        app.clock.advance(chrono::Duration::minutes(1));
        app.sign_in(&format!("u{i}@example.com")).await;
    }
    app.clock.advance(chrono::Duration::minutes(1));
    app.sign_in("=cmd@example.com").await;
    let mut seen = Vec::new();
    let mut cursor = String::new();
    loop {
        let (status, body) = req(
            &app,
            "GET",
            &format!("/v1/admin/users?limit=2&cursor={cursor}"),
            None,
            &ada,
        )
        .await;
        assert_eq!(status, StatusCode::OK, "{body}");
        assert_eq!(body["total"], 6);
        for u in body["items"].as_array().unwrap() {
            seen.push(u["email"].as_str().unwrap().to_owned());
        }
        match body["next_cursor"].as_str() {
            Some(c) => cursor = c.to_owned(),
            None => break,
        }
    }
    assert_eq!(seen.len(), 6);
    let mut unique = seen.clone();
    unique.sort();
    unique.dedup();
    assert_eq!(unique.len(), 6);

    let (status, disposition, csv) = raw_get(&app, "/v1/admin/users.csv?q=example", &ada).await;
    assert_eq!(status, StatusCode::OK);
    assert!(
        disposition.contains("users-2026-11-04.csv"),
        "{disposition}"
    );
    let lines: Vec<&str> = csv.lines().collect();
    assert_eq!(lines.len(), 7, "{csv}");
    assert!(lines[0].starts_with("email,name,"), "{csv}");
    assert!(
        csv.contains("'=cmd@example.com"),
        "formula cells are escaped: {csv}"
    );
}

#[sqlx::test]
async fn sign_out_everywhere_and_revoke_one_session(pool: PgPool) {
    let app = app_with(pool).await;
    let ada = app.sign_in(OWNER).await;
    let jane = app.sign_in("jane@example.com").await;
    let phone = app.sign_in("jane@example.com").await;
    let laptop = app.sign_in("jane@example.com").await;
    let jane_id = uid(&app, &jane).await;
    let sid: String = sqlx::query_scalar(
        "SELECT s.id::text FROM sessions s WHERE s.user_id = $1::uuid ORDER BY created_at LIMIT 1",
    )
    .bind(&jane_id)
    .fetch_one(&app.state.db)
    .await
    .unwrap();
    // Another user's session id is a 404, not a revoke.
    let ada_sid: String =
        sqlx::query_scalar("SELECT id::text FROM sessions WHERE user_id <> $1::uuid LIMIT 1")
            .bind(&jane_id)
            .fetch_one(&app.state.db)
            .await
            .unwrap();
    let (status, _) = req(
        &app,
        "DELETE",
        &format!("/v1/admin/users/{jane_id}/sessions/{ada_sid}"),
        None,
        &ada,
    )
    .await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    let (status, _) = req(
        &app,
        "DELETE",
        &format!("/v1/admin/users/{jane_id}/sessions/{sid}"),
        None,
        &ada,
    )
    .await;
    assert_eq!(status, StatusCode::NO_CONTENT);
    assert_eq!(
        req(&app, "GET", "/v1/me", None, &jane).await.0,
        StatusCode::UNAUTHORIZED
    );
    assert_eq!(
        req(&app, "GET", "/v1/me", None, &phone).await.0,
        StatusCode::OK
    );

    let (status, body) = req(
        &app,
        "POST",
        &format!("/v1/admin/users/{jane_id}/sign-out-everywhere"),
        None,
        &ada,
    )
    .await;
    assert_eq!(status, StatusCode::NO_CONTENT, "{body}");
    assert_eq!(
        req(&app, "GET", "/v1/me", None, &phone).await.0,
        StatusCode::UNAUTHORIZED
    );
    assert_eq!(
        req(&app, "GET", "/v1/me", None, &laptop).await.0,
        StatusCode::UNAUTHORIZED
    );
    assert_eq!(
        req(&app, "GET", "/v1/me", None, &ada).await.0,
        StatusCode::OK
    );
    let (_, d) = req(
        &app,
        "GET",
        &format!("/v1/admin/users/{jane_id}"),
        None,
        &ada,
    )
    .await;
    assert_eq!(d["sessions"], json!([]));
    assert_eq!(
        scalar(
            &app,
            "SELECT count(*) FROM admin_events WHERE action = $1",
            "user.sign_out_everywhere"
        )
        .await,
        1
    );
    assert_eq!(
        scalar(
            &app,
            "SELECT count(*) FROM admin_events WHERE action = $1",
            "user.session.revoke"
        )
        .await,
        1
    );
}

#[sqlx::test]
async fn bulk_sign_out(pool: PgPool) {
    let app = app_with(pool).await;
    let ada = app.sign_in(OWNER).await;
    let jane = app.sign_in("jane@example.com").await;
    let bob = app.sign_in("bob@example.com").await;
    let ada_id = uid(&app, &ada).await;
    let ids = vec![uid(&app, &jane).await, uid(&app, &bob).await];

    let (status, body) = req(
        &app,
        "POST",
        "/v1/admin/users/bulk",
        Some(json!({"ids": ids, "action": "sign_out"})),
        &ada,
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(body["done"], 2);
    assert_eq!(body["skipped"], json!([]));
    // The caller is skipped, so an admin never signs themselves out.
    let mut with_self = ids.clone();
    with_self.push(ada_id.clone());
    let (status, body) = req(
        &app,
        "POST",
        "/v1/admin/users/bulk",
        Some(json!({"ids": with_self, "action": "sign_out"})),
        &ada,
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(body["done"], 2);
    assert_eq!(body["skipped"], json!([ada_id]));
    assert_eq!(
        req(&app, "GET", "/v1/me", None, &ada).await.0,
        StatusCode::OK
    );
    assert_eq!(
        req(&app, "GET", "/v1/me", None, &jane).await.0,
        StatusCode::UNAUTHORIZED
    );
    assert_eq!(
        req(&app, "GET", "/v1/me", None, &bob).await.0,
        StatusCode::UNAUTHORIZED
    );

    // Bulk suspend skips the owner and the caller; one event for the batch.
    let mut all = ids.clone();
    all.push(ada_id.clone());
    let (status, body) = req(
        &app,
        "POST",
        "/v1/admin/users/bulk",
        Some(json!({"ids": all, "action": "suspend", "reason": "spam"})),
        &ada,
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(body["done"], 2);
    assert_eq!(body["skipped"], json!([ada_id]));
    assert_eq!(
        scalar(
            &app,
            "SELECT count(*) FROM users WHERE suspended_reason = $1",
            "spam"
        )
        .await,
        2
    );
    assert_eq!(
        scalar(
            &app,
            "SELECT count(*) FROM admin_events WHERE action = $1",
            "users.bulk.sign_out"
        )
        .await,
        2
    );
    // The suspension filter of the activity log finds the bulk suspend.
    let (_, act) = req(
        &app,
        "GET",
        "/v1/admin/activity?action_group=suspension",
        None,
        &ada,
    )
    .await;
    let actions: Vec<&str> = act["items"]
        .as_array()
        .unwrap()
        .iter()
        .map(|e| e["action"].as_str().unwrap())
        .collect();
    assert_eq!(actions, ["users.bulk.suspend"]);

    // Validation.
    for body in [
        json!({"ids": [], "action": "sign_out"}),
        json!({"ids": ["nope"], "action": "sign_out"}),
        json!({"ids": ids, "action": "delete"}),
    ] {
        let (status, out) = req(
            &app,
            "POST",
            "/v1/admin/users/bulk",
            Some(body.clone()),
            &ada,
        )
        .await;
        assert_eq!(status, StatusCode::BAD_REQUEST, "{body}: {out}");
    }
    // The batch shows in each person's recent events.
    let (_, d) = req(
        &app,
        "GET",
        &format!("/v1/admin/users/{}", ids[0]),
        None,
        &ada,
    )
    .await;
    assert_eq!(d["events"].as_array().unwrap().len(), 3, "{d}");
}

#[sqlx::test]
async fn detail_has_everything(pool: PgPool) {
    let app = app_with(pool).await;
    let ada = app.sign_in(OWNER).await;
    let jane = app.sign_in("jane@example.com").await;
    app.create_team(&jane, "acme").await;
    let owner = app.sign_in("owner@other.com").await;
    app.create_team(&owner, "initech").await;
    github_sign_in(&app, "gh-77", "jane@example.com").await;
    let jane_id = uid(&app, &jane).await;
    // initech shares a restricted collection with jane's email.
    let team_id: uuid::Uuid = sqlx::query_scalar("SELECT id FROM teams WHERE slug = 'initech'")
        .fetch_one(&app.state.db)
        .await
        .unwrap();
    let enc = curl_runner_server::crypto::encrypt_link_token(
        app.state.config.secret_key.as_bytes(),
        "crl_x",
    );
    let coll: uuid::Uuid = sqlx::query_scalar(
        "INSERT INTO collections (team_id, slug, name, access, link_hash, link_token_enc, current_version) \
         VALUES ($1, 'sandbox', 'Billing sandbox', 'restricted', 'h', $2, 1) RETURNING id",
    )
    .bind(team_id)
    .bind(&enc)
    .fetch_one(&app.state.db)
    .await
    .unwrap();
    sqlx::query("INSERT INTO versions (collection_id, number, hash, published_by, created_at) VALUES ($1, 1, 'x', 'o', now())")
        .bind(coll)
        .execute(&app.state.db)
        .await
        .unwrap();
    sqlx::query("INSERT INTO access_entries (collection_id, entry, kind, invited_at) VALUES ($1, 'jane@example.com', 'email', now())")
        .bind(coll)
        .execute(&app.state.db)
        .await
        .unwrap();
    req(
        &app,
        "POST",
        &format!("/v1/admin/users/{jane_id}/suspend"),
        Some(json!({"reason": "r", "notify": false})),
        &ada,
    )
    .await;
    app.clock.advance(chrono::Duration::minutes(1));
    req(
        &app,
        "POST",
        &format!("/v1/admin/users/{jane_id}/unsuspend"),
        None,
        &ada,
    )
    .await;
    let _ = app.sign_in("jane@example.com").await;

    let (status, d) = req(
        &app,
        "GET",
        &format!("/v1/admin/users/{jane_id}"),
        None,
        &ada,
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{d}");
    assert_eq!(d["user"]["email"], "jane@example.com");
    assert_eq!(d["user"]["status"], "active");
    assert_eq!(d["teams"][0]["slug"], "acme");
    assert_eq!(d["teams"][0]["role"], "owner");
    assert_eq!(d["teams"][0]["plan_source"], "none");
    let providers: Vec<&str> = d["identities"]
        .as_array()
        .unwrap()
        .iter()
        .map(|i| i["provider"].as_str().unwrap())
        .collect();
    assert_eq!(providers, ["email", "github"]);
    assert_eq!(d["identities"][1]["subject"], "gh-77");
    assert_eq!(d["sessions"].as_array().unwrap().len(), 1);
    assert_eq!(d["shared"].as_array().unwrap().len(), 1);
    assert_eq!(d["shared"][0]["collection"], "sandbox");
    assert!(
        d["shared"][0].get("link").is_none(),
        "no secrets in the admin UI: {d}"
    );
    let actions: Vec<&str> = d["events"]
        .as_array()
        .unwrap()
        .iter()
        .map(|e| e["action"].as_str().unwrap())
        .collect();
    assert_eq!(actions, ["user.unsuspend", "user.suspend"]);
    // The shared count shows in the list too.
    let (_, list) = req(&app, "GET", "/v1/admin/users?q=jane", None, &ada).await;
    assert_eq!(list["items"][0]["shared_in"], 1);
    let (status, _) = req(
        &app,
        "GET",
        &format!("/v1/admin/users/{}", uuid::Uuid::new_v4()),
        None,
        &ada,
    )
    .await;
    assert_eq!(status, StatusCode::NOT_FOUND);
}

#[sqlx::test]
async fn email_user_needs_emails_perm_and_escapes(pool: PgPool) {
    let app = app_with(pool).await;
    let ada = app.sign_in(OWNER).await;
    let rosa = app.sign_in("rosa@example.com").await;
    let jane = app.sign_in("jane@example.com").await;
    let jane_id = uid(&app, &jane).await;
    let rosa_id = uid(&app, &rosa).await;
    let (status, _) = req(
        &app,
        "POST",
        "/v1/admin/admins",
        Some(json!({"email": "rosa@example.com", "permissions": ["users"]})),
        &ada,
    )
    .await;
    assert_eq!(status, StatusCode::CREATED);
    let path = format!("/v1/admin/users/{jane_id}/email");
    let msg = json!({"subject": "About your account\r\nBcc: x@y.com", "body_md": "Hi <b>Jane</b> & co\n\nSecond paragraph"});
    let (status, body) = req(&app, "POST", &path, Some(msg.clone()), &rosa).await;
    assert_eq!(status, StatusCode::FORBIDDEN, "{body}");
    let _ = rosa_id;
    let (status, body) = req(&app, "POST", &path, Some(msg), &ada).await;
    assert_eq!(status, StatusCode::ACCEPTED, "{body}");
    let (subject, text, html, kind, user_id): (String, String, String, String, Option<uuid::Uuid>) =
        sqlx::query_as(
            "SELECT subject, text_body, html_body, kind, user_id FROM outbox_emails \
         WHERE to_addr = 'jane@example.com' AND kind = 'admin'",
        )
        .fetch_one(&app.state.db)
        .await
        .unwrap();
    assert!(!subject.contains(['\r', '\n']));
    assert!(text.contains("Second paragraph"));
    assert!(
        html.contains("Hi &lt;b&gt;Jane&lt;/b&gt; &amp; co"),
        "{html}"
    );
    assert_eq!(html.matches("Second paragraph").count(), 1);
    assert_eq!(kind, "admin");
    assert_eq!(user_id.map(|u| u.to_string()), Some(jane_id));
    for (subject, body) in [("", "x"), ("x", "  ")] {
        let (status, _) = req(
            &app,
            "POST",
            &path,
            Some(json!({"subject": subject, "body_md": body})),
            &ada,
        )
        .await;
        assert_eq!(status, StatusCode::BAD_REQUEST);
    }
    assert_eq!(
        scalar(
            &app,
            "SELECT count(*) FROM admin_events WHERE action = $1",
            "user.email"
        )
        .await,
        1
    );
}

#[sqlx::test]
async fn writes_need_the_users_perm(pool: PgPool) {
    let app = app_with(pool).await;
    let ada = app.sign_in(OWNER).await;
    let rosa = app.sign_in("rosa@example.com").await;
    let jane = app.sign_in("jane@example.com").await;
    let jane_id = uid(&app, &jane).await;
    let (status, _) = req(
        &app,
        "POST",
        "/v1/admin/admins",
        Some(json!({"email": "rosa@example.com", "permissions": ["billing"]})),
        &ada,
    )
    .await;
    assert_eq!(status, StatusCode::CREATED);
    for (method, path, body) in [
        (
            "POST",
            format!("/v1/admin/users/{jane_id}/suspend"),
            Some(json!({"notify": false})),
        ),
        ("POST", format!("/v1/admin/users/{jane_id}/unsuspend"), None),
        (
            "POST",
            format!("/v1/admin/users/{jane_id}/sign-out-everywhere"),
            None,
        ),
        (
            "DELETE",
            format!(
                "/v1/admin/users/{jane_id}/sessions/{}",
                uuid::Uuid::new_v4()
            ),
            None,
        ),
        (
            "DELETE",
            format!("/v1/admin/users/{jane_id}/identities/github"),
            None,
        ),
        ("DELETE", format!("/v1/admin/users/{jane_id}"), None),
        (
            "POST",
            "/v1/admin/users/bulk".to_owned(),
            Some(json!({"ids": [jane_id], "action": "sign_out"})),
        ),
    ] {
        let (status, out) = req(&app, method, &path, body, &rosa).await;
        assert_eq!(status, StatusCode::FORBIDDEN, "{method} {path}: {out}");
    }
    // Reads are open to every admin.
    for path in [
        "/v1/admin/users".to_owned(),
        format!("/v1/admin/users/{jane_id}"),
    ] {
        assert_eq!(
            req(&app, "GET", &path, None, &rosa).await.0,
            StatusCode::OK,
            "{path}"
        );
    }
    assert_eq!(
        scalar(
            &app,
            "SELECT count(*) FROM admin_events WHERE action <> $1",
            "admin.add"
        )
        .await,
        0
    );
}

#[sqlx::test]
async fn delete_clears_exact_email_opens(pool: PgPool) {
    let app = app_with(pool).await;
    let ada = app.sign_in(OWNER).await;
    let jane = app.sign_in("jane@example.com").await;
    let jane_id = uid(&app, &jane).await;
    let owner = app.sign_in("owner@other.com").await;
    app.create_team(&owner, "initech").await;
    let team_id: uuid::Uuid = sqlx::query_scalar("SELECT id FROM teams WHERE slug = 'initech'")
        .fetch_one(&app.state.db)
        .await
        .unwrap();
    let coll: uuid::Uuid = sqlx::query_scalar(
        "INSERT INTO collections (team_id, slug, name, access) VALUES ($1, 'c', 'c', 'restricted') RETURNING id",
    )
    .bind(team_id)
    .fetch_one(&app.state.db)
    .await
    .unwrap();
    for entry in ["jane@example.com", "*@example.com"] {
        sqlx::query("INSERT INTO share_opens (collection_id, user_id, entry, opened_at) VALUES ($1, $2::uuid, $3, now())")
            .bind(coll)
            .bind(&jane_id)
            .bind(entry)
            .execute(&app.state.db)
            .await
            .unwrap();
    }
    let (status, _) = req(
        &app,
        "DELETE",
        &format!("/v1/admin/users/{jane_id}"),
        None,
        &ada,
    )
    .await;
    assert_eq!(status, StatusCode::NO_CONTENT);
    assert_eq!(
        scalar(
            &app,
            "SELECT count(*) FROM share_opens WHERE entry = $1",
            "jane@example.com"
        )
        .await,
        0
    );
    // The domain-matched open is not theirs to clear; both opens still count.
    assert_eq!(
        scalar(
            &app,
            "SELECT count(*) FROM share_opens WHERE entry = $1",
            "*@example.com"
        )
        .await,
        1
    );
    assert_eq!(
        scalar(
            &app,
            "SELECT count(*) FROM share_opens WHERE collection_id = $1::uuid",
            &coll.to_string()
        )
        .await,
        2
    );
}
