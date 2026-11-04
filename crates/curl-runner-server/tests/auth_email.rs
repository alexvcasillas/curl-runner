#![cfg(feature = "db-tests")]

mod common;

use axum::http::{Method, StatusCode};
use chrono::Duration;
use common::{TestApp, test_app};
use serde_json::{Value, json};
use sqlx::PgPool;

async fn verify(app: &TestApp, body: Value) -> (StatusCode, Value) {
    app.request(Method::POST, "/v1/auth/email/verify", Some(body), None)
        .await
}

async fn me(app: &TestApp, token: &str) -> (StatusCode, Value) {
    app.request(Method::GET, "/v1/me", None, Some(token)).await
}

fn wrong(code: &str) -> String {
    if code == "000000" { "000001" } else { "000000" }.to_owned()
}

#[sqlx::test]
async fn start_always_202_and_queues_mail(pool: PgPool) {
    let app = test_app(pool).await;
    for email in ["new@example.com", "new@example.com"] {
        let (status, body) = app
            .request(
                Method::POST,
                "/v1/auth/email/start",
                Some(json!({"email": email, "client": "web"})),
                None,
            )
            .await;
        assert_eq!((status, body), (StatusCode::ACCEPTED, json!({})));
    }
    let (code, token) = app.start_email("new@example.com", "web").await;
    let token = token.expect("web mail has a link");
    assert_eq!(code.len(), 6);
    assert!(code.bytes().all(|b| b.is_ascii_digit()));
    let link: String =
        sqlx::query_scalar("SELECT text_body FROM outbox_emails ORDER BY ctid DESC LIMIT 1")
            .fetch_one(&app.state.db)
            .await
            .unwrap();
    assert!(link.contains(&format!(
        "http://localhost:3000/sign-in/email?token={token}&client=web"
    )));
    // Only hashes are stored.
    let stored: String = sqlx::query_scalar("SELECT code_hash FROM email_codes")
        .fetch_one(&app.state.db)
        .await
        .unwrap();
    assert_ne!(stored, code);
    assert_eq!(stored.len(), 64);
}

#[sqlx::test]
async fn start_rejects_bad_input(pool: PgPool) {
    let app = test_app(pool).await;
    let (status, body) = app
        .request(
            Method::POST,
            "/v1/auth/email/start",
            Some(json!({"email": "nope", "client": "web"})),
            None,
        )
        .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["error"], "invalid_request");
    let (status, body) = app
        .request(
            Method::POST,
            "/v1/auth/email/start",
            Some(json!({"email": "a@example.com", "client": "tv"})),
            None,
        )
        .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["error"], "invalid_request");
}

#[sqlx::test]
async fn verify_with_code_creates_user_and_session(pool: PgPool) {
    let app = test_app(pool).await;
    let (code, _) = app.start_email("jane@example.com", "web").await;
    let (status, body) = verify(&app, json!({"email": "jane@example.com", "code": code})).await;
    assert_eq!(status, StatusCode::OK, "{body}");
    let token = body["session"].as_str().unwrap();
    assert!(token.starts_with("crs_"));
    assert_eq!(body["user"]["email"], "jane@example.com");
    assert!(body["user"]["id"].is_string());

    let (status, who) = me(&app, token).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(who["user"]["email"], "jane@example.com");
    assert_eq!(who["teams"], json!([]));

    // The session is stored hashed, and the code is single use.
    let stored: String = sqlx::query_scalar("SELECT token_hash FROM sessions")
        .fetch_one(&app.state.db)
        .await
        .unwrap();
    assert_ne!(stored, token);
    let (status, body) = verify(&app, json!({"email": "jane@example.com", "code": code})).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
    assert_eq!(body["error"], "code_invalid");
}

#[sqlx::test]
async fn verify_with_link_token_works(pool: PgPool) {
    let app = test_app(pool).await;
    let (_, token) = app.start_email("jane@example.com", "web").await;
    let token = token.expect("web mail has a link");
    let (status, body) = verify(&app, json!({"token": token})).await;
    assert_eq!(status, StatusCode::OK, "{body}");
    let kind: String = sqlx::query_scalar("SELECT kind FROM sessions")
        .fetch_one(&app.state.db)
        .await
        .unwrap();
    assert_eq!(kind, "web");
    let (status, body) = verify(&app, json!({"token": token})).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
    assert_eq!(body["error"], "code_invalid");
}

#[sqlx::test]
async fn device_mail_has_code_and_no_link(pool: PgPool) {
    let app = test_app(pool).await;
    let (code, token) = app.start_email("jane@example.com", "device").await;
    assert_eq!(token, None);
    let (text, html): (String, String) =
        sqlx::query_as("SELECT text_body, html_body FROM outbox_emails")
            .fetch_one(&app.state.db)
            .await
            .unwrap();
    for body in [&text, &html] {
        assert!(body.contains(&code));
        assert!(!body.contains("http"), "{body}");
    }
    let link_hash: Option<String> = sqlx::query_scalar("SELECT link_hash FROM email_codes")
        .fetch_one(&app.state.db)
        .await
        .unwrap();
    assert_eq!(link_hash, None);
    let (status, body) = verify(&app, json!({"email": "jane@example.com", "code": code})).await;
    assert_eq!(status, StatusCode::OK, "{body}");
    let kind: String = sqlx::query_scalar("SELECT kind FROM sessions")
        .fetch_one(&app.state.db)
        .await
        .unwrap();
    assert_eq!(kind, "device");
}

#[sqlx::test]
async fn link_token_cannot_spend_a_device_code(pool: PgPool) {
    let app = test_app(pool).await;
    let (code, _) = app.start_email("jane@example.com", "device").await;
    // Give the device row a link anyway, as if one had been mailed.
    let key = app.state.config.secret_key.as_bytes();
    sqlx::query("UPDATE email_codes SET link_hash = $1")
        .bind(curl_runner_server::hash::secret_hash(key, "crl_forged"))
        .execute(&app.state.db)
        .await
        .unwrap();
    let (status, body) = verify(&app, json!({"token": "crl_forged"})).await;
    assert_eq!(status, StatusCode::BAD_REQUEST, "{body}");
    assert_eq!(body["error"], "invalid_request");
    // The code is still live for the terminal.
    let (status, body) = verify(&app, json!({"email": "jane@example.com", "code": code})).await;
    assert_eq!(status, StatusCode::OK, "{body}");
}

#[sqlx::test]
async fn verify_needs_code_or_token(pool: PgPool) {
    let app = test_app(pool).await;
    for body in [
        json!({}),
        json!({"email": "a@example.com"}),
        json!({"code": "123456"}),
    ] {
        let (status, body) = verify(&app, body).await;
        assert_eq!(status, StatusCode::BAD_REQUEST);
        assert_eq!(body["error"], "invalid_request");
    }
}

#[sqlx::test]
async fn device_label_is_stored_and_listed(pool: PgPool) {
    let app = test_app(pool).await;
    let (status, _) = app
        .request(
            Method::POST,
            "/v1/auth/email/start",
            Some(
                json!({"email": "jane@example.com", "client": "device", "label": "Jane's laptop"}),
            ),
            None,
        )
        .await;
    assert_eq!(status, StatusCode::ACCEPTED);
    let body: String = sqlx::query_scalar("SELECT text_body FROM outbox_emails")
        .fetch_one(&app.state.db)
        .await
        .unwrap();
    let code = body
        .split("sign-in code is ")
        .nth(1)
        .unwrap()
        .get(..6)
        .unwrap();
    let (_, v) = verify(&app, json!({"email": "jane@example.com", "code": code})).await;
    let (status, list) = app
        .request(Method::GET, "/v1/sessions", None, v["session"].as_str())
        .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(list[0]["label"], "Jane's laptop");
    assert_eq!(list[0]["kind"], "device");
    assert_eq!(list[0]["current"], true);
}

#[sqlx::test]
async fn wrong_code_five_times_then_expired(pool: PgPool) {
    let app = test_app(pool).await;
    let (code, _) = app.start_email("jane@example.com", "web").await;
    for _ in 0..5 {
        let (status, body) = verify(
            &app,
            json!({"email": "jane@example.com", "code": wrong(&code)}),
        )
        .await;
        assert_eq!(status, StatusCode::UNAUTHORIZED);
        assert_eq!(body["error"], "code_invalid");
    }
    let (status, body) = verify(
        &app,
        json!({"email": "jane@example.com", "code": wrong(&code)}),
    )
    .await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
    assert_eq!(body["error"], "code_expired");
    // Even the right code is dead now.
    let (_, body) = verify(&app, json!({"email": "jane@example.com", "code": code})).await;
    assert_eq!(body["error"], "code_expired");
}

#[sqlx::test]
async fn code_expires_after_10_minutes(pool: PgPool) {
    let app = test_app(pool).await;
    let (code, _) = app.start_email("jane@example.com", "web").await;
    app.clock
        .advance(Duration::minutes(10) + Duration::seconds(1));
    let (status, body) = verify(&app, json!({"email": "jane@example.com", "code": code})).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
    assert_eq!(body["error"], "code_expired");
}

#[sqlx::test]
async fn code_still_valid_at_10_minutes_minus_a_second(pool: PgPool) {
    let app = test_app(pool).await;
    let (code, _) = app.start_email("jane@example.com", "web").await;
    app.clock
        .advance(Duration::minutes(10) - Duration::seconds(1));
    let (status, _) = verify(&app, json!({"email": "jane@example.com", "code": code})).await;
    assert_eq!(status, StatusCode::OK);
}

#[sqlx::test]
async fn only_latest_code_valid(pool: PgPool) {
    let app = test_app(pool).await;
    let (first, first_token) = app.start_email("jane@example.com", "web").await;
    let (second, _) = app.start_email("jane@example.com", "web").await;
    if first != second {
        let (status, body) =
            verify(&app, json!({"email": "jane@example.com", "code": first})).await;
        assert_eq!(status, StatusCode::UNAUTHORIZED);
        assert_eq!(body["error"], "code_invalid");
    }
    let (_, body) = verify(&app, json!({"token": first_token.unwrap()})).await;
    assert_eq!(body["error"], "code_invalid");
    let (status, _) = verify(&app, json!({"email": "jane@example.com", "code": second})).await;
    assert_eq!(status, StatusCode::OK);
}

#[sqlx::test]
async fn email_is_case_insensitive(pool: PgPool) {
    let app = test_app(pool).await;
    let (code, _) = app.start_email("Jane@Example.com", "web").await;
    let (status, first) = verify(&app, json!({"email": "jane@example.com", "code": code})).await;
    assert_eq!(status, StatusCode::OK, "{first}");
    let (code, _) = app.start_email("JANE@example.com", "web").await;
    let (_, second) = verify(&app, json!({"email": "Jane@Example.com", "code": code})).await;
    assert_eq!(first["user"]["id"], second["user"]["id"]);
    let users: i64 = sqlx::query_scalar("SELECT count(*) FROM users")
        .fetch_one(&app.state.db)
        .await
        .unwrap();
    assert_eq!(users, 1);
}

#[sqlx::test]
async fn start_rate_limited_after_5(pool: PgPool) {
    let app = test_app(pool).await;
    let start = |email: &'static str| {
        app.request(
            Method::POST,
            "/v1/auth/email/start",
            Some(json!({"email": email, "client": "web"})),
            None,
        )
    };
    for _ in 0..5 {
        assert_eq!(start("jane@example.com").await.0, StatusCode::ACCEPTED);
    }
    let (status, body) = start("JANE@example.com").await;
    assert_eq!(status, StatusCode::TOO_MANY_REQUESTS);
    assert_eq!(body["error"], "rate_limited");
    // Another address is unaffected, and the window resets.
    assert_eq!(start("ada@example.com").await.0, StatusCode::ACCEPTED);
    app.clock.advance(Duration::minutes(15));
    assert_eq!(start("jane@example.com").await.0, StatusCode::ACCEPTED);
}

#[sqlx::test]
async fn start_rate_limited_per_ip(pool: PgPool) {
    let app = test_app(pool).await;
    for i in 0..21 {
        let (status, body) = app
            .request_with_ip(
                "/v1/auth/email/start",
                json!({"email": format!("u{i}@example.com"), "client": "web"}),
                "10.9.9.9, 203.0.113.7",
            )
            .await;
        if i < 20 {
            assert_eq!(status, StatusCode::ACCEPTED);
        } else {
            assert_eq!(status, StatusCode::TOO_MANY_REQUESTS, "{body}");
        }
    }
}

#[sqlx::test]
async fn session_slides_and_expires_after_90_days_idle(pool: PgPool) {
    let app = test_app(pool).await;
    let token = app.sign_in("jane@example.com").await;
    app.clock.advance(Duration::days(60));
    assert_eq!(me(&app, &token).await.0, StatusCode::OK);
    // 120 days after sign-in but only 60 idle: the slide kept it alive.
    app.clock.advance(Duration::days(60));
    assert_eq!(me(&app, &token).await.0, StatusCode::OK);
    app.clock.advance(Duration::days(90) + Duration::seconds(1));
    let (status, body) = me(&app, &token).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
    assert_eq!(body["error"], "unauthorized");
}

#[sqlx::test]
async fn last_used_is_touched_at_most_once_a_minute(pool: PgPool) {
    let app = test_app(pool).await;
    let token = app.sign_in("jane@example.com").await;
    let last = |pool: PgPool| async move {
        sqlx::query_scalar::<_, chrono::DateTime<chrono::Utc>>("SELECT last_used_at FROM sessions")
            .fetch_one(&pool)
            .await
            .unwrap()
    };
    let created = last(app.state.db.clone()).await;
    app.clock.advance(Duration::seconds(30));
    me(&app, &token).await;
    assert_eq!(last(app.state.db.clone()).await, created);
    app.clock.advance(Duration::seconds(31));
    me(&app, &token).await;
    assert_eq!(
        last(app.state.db.clone()).await,
        created + Duration::seconds(61)
    );
}

#[sqlx::test]
async fn missing_or_bad_bearer_is_401(pool: PgPool) {
    let app = test_app(pool).await;
    for token in [None, Some("crs_nope"), Some("not-a-session")] {
        let (status, body) = app.request(Method::GET, "/v1/me", None, token).await;
        assert_eq!(status, StatusCode::UNAUTHORIZED);
        assert_eq!(body["error"], "unauthorized");
    }
}

#[sqlx::test]
async fn revoked_session_is_401(pool: PgPool) {
    let app = test_app(pool).await;
    let token = app.sign_in("jane@example.com").await;
    let (status, body) = app
        .request(Method::POST, "/v1/auth/sign-out", None, Some(&token))
        .await;
    assert_eq!((status, body), (StatusCode::NO_CONTENT, Value::Null));
    let (status, body) = me(&app, &token).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
    assert_eq!(body["error"], "unauthorized");
}

#[sqlx::test]
async fn sessions_list_and_revoke_only_own(pool: PgPool) {
    let app = test_app(pool).await;
    let jane1 = app.sign_in("jane@example.com").await;
    let jane2 = app.sign_in("jane@example.com").await;
    let ada = app.sign_in("ada@example.com").await;

    let (status, list) = app
        .request(Method::GET, "/v1/sessions", None, Some(&jane1))
        .await;
    assert_eq!(status, StatusCode::OK);
    let list = list.as_array().unwrap();
    assert_eq!(list.len(), 2);
    let current: Vec<_> = list.iter().filter(|s| s["current"] == true).collect();
    assert_eq!(current.len(), 1);
    let other = list.iter().find(|s| s["current"] == false).unwrap()["id"]
        .as_str()
        .unwrap()
        .to_owned();

    // Ada cannot touch Jane's session.
    let (status, body) = app
        .request(
            Method::DELETE,
            &format!("/v1/sessions/{other}"),
            None,
            Some(&ada),
        )
        .await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    assert_eq!(body["error"], "not_found");
    assert_eq!(me(&app, &jane2).await.0, StatusCode::OK);

    let (status, _) = app
        .request(
            Method::DELETE,
            &format!("/v1/sessions/{other}"),
            None,
            Some(&jane1),
        )
        .await;
    assert_eq!(status, StatusCode::NO_CONTENT);
    assert_eq!(me(&app, &jane2).await.0, StatusCode::UNAUTHORIZED);
    let (_, list) = app
        .request(Method::GET, "/v1/sessions", None, Some(&jane1))
        .await;
    assert_eq!(list.as_array().unwrap().len(), 1);
}

#[sqlx::test]
async fn me_lists_teams(pool: PgPool) {
    let app = test_app(pool).await;
    let token = app.sign_in("jane@example.com").await;
    sqlx::query(
        "WITH t AS (INSERT INTO teams (slug, name, created_at) VALUES ('acme', 'Acme', now()) RETURNING id) \
         INSERT INTO memberships (team_id, user_id, role, created_at) \
         SELECT t.id, u.id, 'owner', now() FROM t, users u",
    )
    .execute(&app.state.db)
    .await
    .unwrap();
    let (_, who) = me(&app, &token).await;
    assert_eq!(
        who["teams"],
        json!([{"slug": "acme", "name": "Acme", "role": "owner", "plan": "free"}])
    );
}

#[sqlx::test]
async fn malformed_json_is_400_invalid_request(pool: PgPool) {
    let app = test_app(pool).await;
    let (status, body) = app
        .request_raw(Method::POST, "/v1/auth/email/start", "{nope")
        .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["error"], "invalid_request");
}

#[sqlx::test]
async fn start_for_existing_user_is_also_202_empty(pool: PgPool) {
    let app = test_app(pool).await;
    app.sign_in("jane@example.com").await;
    let (status, body) = app
        .request(
            Method::POST,
            "/v1/auth/email/start",
            Some(json!({"email": "jane@example.com", "client": "web"})),
            None,
        )
        .await;
    assert_eq!((status, body), (StatusCode::ACCEPTED, json!({})));
}

#[sqlx::test]
async fn bearer_scheme_is_case_insensitive(pool: PgPool) {
    let app = test_app(pool).await;
    let token = app.sign_in("jane@example.com").await;
    let (status, _) = app
        .request_with_auth("/v1/me", &format!("bearer {token}"))
        .await;
    assert_eq!(status, StatusCode::OK);
}

#[sqlx::test]
async fn concurrent_starts_leave_one_live_code(pool: PgPool) {
    let app = test_app(pool).await;
    let start = || {
        app.request(
            Method::POST,
            "/v1/auth/email/start",
            Some(json!({"email": "jane@example.com", "client": "web"})),
            None,
        )
    };
    let (a, b, c, d) = tokio::join!(start(), start(), start(), start());
    for (status, _) in [a, b, c, d] {
        assert_eq!(status, StatusCode::ACCEPTED);
    }
    let live: i64 = sqlx::query_scalar("SELECT count(*) FROM email_codes WHERE used_at IS NULL")
        .fetch_one(&app.state.db)
        .await
        .unwrap();
    assert_eq!(live, 1);
}

#[sqlx::test]
async fn forwarded_for_spoofing_cannot_dodge_ip_limit(pool: PgPool) {
    let app = test_app(pool).await;
    // One proxy hop: the rightmost entry counts, the spoofed left one is ignored.
    for i in 0..21 {
        let (status, _) = app
            .request_with_ip(
                "/v1/auth/email/start",
                json!({"email": format!("s{i}@example.com"), "client": "web"}),
                &format!("spoof-{i}, 198.51.100.1"),
            )
            .await;
        assert_eq!(status == StatusCode::TOO_MANY_REQUESTS, i == 20, "{i}");
    }
}
