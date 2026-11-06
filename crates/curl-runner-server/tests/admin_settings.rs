#![cfg(feature = "db-tests")]

mod common;

use axum::http::{Method, StatusCode};
use chrono::Duration;
use common::{BILLING_ENV, TestApp, test_app_with, test_config};
use curl_runner_cloud_proto::{BundleFile, bundle_hash};
use curl_runner_server::clock::Clock;
use serde_json::{Value, json};
use sqlx::PgPool;

const OWNER: &str = "ada@example.com";
const PAUSED: &str = "Publishing is paused for maintenance. Try again in a few minutes.";

async fn owner_app(pool: PgPool, extra: &[(&str, &str)]) -> (TestApp, String) {
    let mut env = vec![("OPERATOR_EMAILS", OWNER)];
    env.extend_from_slice(extra);
    let app = test_app_with(pool, test_config(&env)).await;
    let ada = app.sign_in(OWNER).await;
    (app, ada)
}

async fn get_settings(app: &TestApp, s: &str) -> Value {
    let (status, body) = app
        .request(Method::GET, "/v1/admin/settings", None, Some(s))
        .await;
    assert_eq!(status, StatusCode::OK, "{body}");
    body
}

async fn put(app: &TestApp, s: &str, section: &str, body: Value) -> (StatusCode, Value) {
    app.request(
        Method::PUT,
        &format!("/v1/admin/settings/{section}"),
        Some(body),
        Some(s),
    )
    .await
}

async fn publish(app: &TestApp, s: &str, slug: &str) -> (StatusCode, Value) {
    let files = vec![BundleFile {
        path: "a.yaml".into(),
        text: "request:\n  url: https://api.example.com/a".into(),
    }];
    let body = json!({"hash": bundle_hash(&files), "files": files, "inputs": []});
    app.request(
        Method::PUT,
        &format!("/v1/teams/acme/collections/{slug}/versions"),
        Some(body),
        Some(s),
    )
    .await
}

async fn info(app: &TestApp, bearer: Option<&str>) -> (StatusCode, Value) {
    app.request(Method::GET, "/v1/info", None, bearer).await
}

async fn invite(app: &TestApp, s: &str, email: &str) -> (StatusCode, Value) {
    app.request(
        Method::POST,
        "/v1/teams/acme/invites",
        Some(json!({"email": email, "role": "member"})),
        Some(s),
    )
    .await
}

async fn event_actions(app: &TestApp) -> Vec<String> {
    sqlx::query_scalar("SELECT action FROM admin_events ORDER BY at, id")
        .fetch_all(&app.state.db)
        .await
        .unwrap()
}

#[sqlx::test]
async fn seeded_from_env_once(pool: PgPool) {
    let (app, ada) = owner_app(
        pool.clone(),
        &[
            ("SIGNUP", "domains"),
            ("SIGNUP_DOMAINS", " Example.com ,example.org"),
            ("DEFAULT_PLAN", "free"),
        ],
    )
    .await;
    let s = get_settings(&app, &ada).await;
    assert_eq!(s["signup_mode"], "domains");
    assert_eq!(s["signup_domains"], json!(["example.com", "example.org"]));
    assert_eq!(s["default_plan"], "free");
    assert_eq!(s["free_member_limit"], 3);
    assert_eq!(s["free_restricted"], false);
    assert_eq!(s["grace_days"], 14);
    assert_eq!(s["publishing_paused"], false);
    assert_eq!(s["banner_message"], Value::Null);
    assert_eq!(s["env_seed"]["signup"], "domains");
    let rows: i64 = sqlx::query_scalar("SELECT count(*) FROM server_settings")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(rows, 1);

    // A second start with other env values keeps the stored row.
    let other = test_config(&[("SIGNUP", "closed"), ("DEFAULT_PLAN", "team")]);
    let outcome = curl_runner_server::settings::seed(&pool, &other, app.clock.now())
        .await
        .unwrap();
    assert!(!outcome.inserted);
    let s = get_settings(&app, &ada).await;
    assert_eq!(s["signup_mode"], "domains");
    assert_eq!(s["default_plan"], "free");
}

#[sqlx::test]
async fn env_change_after_seed_is_ignored_with_warn(pool: PgPool) {
    let (app, ada) = owner_app(pool.clone(), &[("SIGNUP", "open")]).await;
    get_settings(&app, &ada).await;
    let now = app.clock.now();

    let same = test_config(&[("SIGNUP", "open"), ("OPERATOR_EMAILS", OWNER)]);
    let outcome = curl_runner_server::settings::seed(&pool, &same, now)
        .await
        .unwrap();
    assert!(outcome.drifted.is_empty(), "{:?}", outcome.drifted);

    let changed = test_config(&[
        ("SIGNUP", "closed"),
        ("SIGNUP_DOMAINS", "example.com"),
        ("DEFAULT_PLAN", "team"),
    ]);
    let outcome = curl_runner_server::settings::seed(&pool, &changed, now)
        .await
        .unwrap();
    assert!(!outcome.inserted);
    assert_eq!(
        outcome.drifted,
        ["SIGNUP", "SIGNUP_DOMAINS", "DEFAULT_PLAN"]
    );
    let s = get_settings(&app, &ada).await;
    assert_eq!(s["signup_mode"], "open");
    assert_eq!(s["default_plan"], "free");
}

#[sqlx::test]
async fn signup_domains_mode(pool: PgPool) {
    let (app, ada) = owner_app(pool, &[]).await;
    // Validation first.
    for bad in [
        json!({"mode": "domains", "domains": ["not a host"]}),
        json!({"mode": "domains", "domains": ["nodot"]}),
        json!({"mode": "domains", "domains": ["-x.example.com"]}),
        json!({"mode": "maybe", "domains": []}),
        json!({"mode": "domains"}),
    ] {
        let (status, body) = put(&app, &ada, "signup", bad.clone()).await;
        assert_eq!(status, StatusCode::BAD_REQUEST, "{bad}: {body}");
        assert_eq!(body["error"], "invalid_request");
    }
    let (status, body) = put(
        &app,
        &ada,
        "signup",
        json!({"mode": "domains", "domains": [" Example.COM ", "example.com", "Example.org"]}),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(
        body["signup_domains"],
        json!(["example.com", "example.org"])
    );

    // The change applies at once, with no wait for the cache.
    let ok = app.sign_in("rosa@example.com").await;
    let (status, body) = app
        .request(
            Method::POST,
            "/v1/teams",
            Some(json!({"slug": "rosa", "name": "Rosa"})),
            Some(&ok),
        )
        .await;
    assert_eq!(status, StatusCode::CREATED, "{body}");
    let no = app.sign_in("sam@other.example.org").await;
    let (status, body) = app
        .request(
            Method::POST,
            "/v1/teams",
            Some(json!({"slug": "sam", "name": "Sam"})),
            Some(&no),
        )
        .await;
    assert_eq!(status, StatusCode::FORBIDDEN, "{body}");
    assert_eq!(body["error"], "signup_closed");

    let (status, _) = put(
        &app,
        &ada,
        "signup",
        json!({"mode": "closed", "domains": []}),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let (status, _) = app
        .request(
            Method::POST,
            "/v1/teams",
            Some(json!({"slug": "rosa2", "name": "Rosa"})),
            Some(&ok),
        )
        .await;
    assert_eq!(status, StatusCode::FORBIDDEN);
    let (_, i) = info(&app, None).await;
    assert_eq!(i["signup"], "closed");
    assert_eq!(
        event_actions(&app).await,
        ["settings.update", "settings.update"]
    );
}

#[sqlx::test]
async fn lower_member_limit_removes_nothing_blocks_new_invites(pool: PgPool) {
    let (app, ada) = owner_app(pool, &[]).await;
    app.create_team(&ada, "acme").await;
    for email in ["a@example.com", "b@example.com"] {
        let (status, body) = invite(&app, &ada, email).await;
        assert_eq!(status, StatusCode::CREATED, "{body}");
    }
    let (status, body) = invite(&app, &ada, "c@example.com").await;
    assert_eq!(status, StatusCode::PAYMENT_REQUIRED, "{body}");
    assert_eq!(
        body["message"],
        "The Free plan allows 3 members. Upgrade to add more."
    );

    // Raise it: one more fits.
    let plans = |limit: u32| json!({"default_plan": "free", "free_member_limit": limit, "free_restricted": false, "grace_days": 14});
    let (status, _) = put(&app, &ada, "plans", plans(4)).await;
    assert_eq!(status, StatusCode::OK);
    let (status, body) = invite(&app, &ada, "c@example.com").await;
    assert_eq!(status, StatusCode::CREATED, "{body}");

    // Lower it below current use: nothing goes, new invites are refused.
    let (status, _) = put(&app, &ada, "plans", plans(2)).await;
    assert_eq!(status, StatusCode::OK);
    let pending: i64 = sqlx::query_scalar("SELECT count(*) FROM member_invites")
        .fetch_one(&app.state.db)
        .await
        .unwrap();
    let members: i64 = sqlx::query_scalar("SELECT count(*) FROM memberships")
        .fetch_one(&app.state.db)
        .await
        .unwrap();
    assert_eq!((members, pending), (1, 3));
    let (status, body) = invite(&app, &ada, "d@example.com").await;
    assert_eq!(status, StatusCode::PAYMENT_REQUIRED, "{body}");
    assert_eq!(
        body["message"],
        "The Free plan allows 2 members. Upgrade to add more."
    );

    // The bounds.
    for bad in [0, 101] {
        let (status, body) = put(&app, &ada, "plans", plans(bad)).await;
        assert_eq!(status, StatusCode::BAD_REQUEST, "{bad}: {body}");
    }
    let (status, _) = put(&app, &ada, "plans", plans(1)).await;
    assert_eq!(status, StatusCode::OK);
    let (status, _) = put(&app, &ada, "plans", plans(100)).await;
    assert_eq!(status, StatusCode::OK);
}

#[sqlx::test]
async fn free_restricted_toggle(pool: PgPool) {
    let (app, ada) = owner_app(pool, &[]).await;
    app.create_team(&ada, "acme").await;
    let (status, body) = publish(&app, &ada, "api").await;
    assert_eq!(status, StatusCode::CREATED, "{body}");
    let share = "/v1/teams/acme/collections/api/share";
    let restrict = || json!({"access": "restricted"});
    let (status, body) = app
        .request(Method::PATCH, share, Some(restrict()), Some(&ada))
        .await;
    assert_eq!(status, StatusCode::PAYMENT_REQUIRED, "{body}");

    let plans = |allowed: bool| json!({"default_plan": "free", "free_member_limit": 3, "free_restricted": allowed, "grace_days": 14});
    let (status, _) = put(&app, &ada, "plans", plans(true)).await;
    assert_eq!(status, StatusCode::OK);
    let (status, body) = app
        .request(Method::PATCH, share, Some(restrict()), Some(&ada))
        .await;
    assert_eq!(status, StatusCode::OK, "{body}");
    // A restricted publish on Free is allowed too, and so is adding people.
    let (status, body) = publish(&app, &ada, "api").await;
    assert_eq!(status, StatusCode::OK, "{body}");

    // Turning it off removes nothing, and blocks anything new.
    let (status, _) = put(&app, &ada, "plans", plans(false)).await;
    assert_eq!(status, StatusCode::OK);
    let (status, body) = app
        .request(
            Method::POST,
            &format!("{share}/entries"),
            Some(json!({"entry": "x@example.com", "notify": false})),
            Some(&ada),
        )
        .await;
    assert_eq!(status, StatusCode::PAYMENT_REQUIRED, "{body}");
}

#[sqlx::test]
async fn pause_publishing_503_pull_still_works(pool: PgPool) {
    let (app, ada) = owner_app(pool, &[]).await;
    app.create_team(&ada, "acme").await;
    let (status, body) = publish(&app, &ada, "api").await;
    assert_eq!(status, StatusCode::CREATED, "{body}");
    let (_, share) = app
        .request(
            Method::GET,
            "/v1/teams/acme/collections/api/share",
            None,
            Some(&ada),
        )
        .await;
    let token = share["link"]
        .as_str()
        .unwrap()
        .split("/s/")
        .nth(1)
        .unwrap()
        .to_owned();
    let (status, _) = app
        .request(
            Method::PATCH,
            "/v1/teams/acme/collections/api/share",
            Some(json!({"access": "public"})),
            Some(&ada),
        )
        .await;
    assert_eq!(status, StatusCode::OK);

    let (status, body) = put(&app, &ada, "publishing", json!({"paused": true})).await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(body["publishing_paused"], true);
    let (status, body) = publish(&app, &ada, "api").await;
    assert_eq!(status, StatusCode::SERVICE_UNAVAILABLE, "{body}");
    assert_eq!(body["error"], "unavailable");
    assert_eq!(body["message"], PAUSED);
    // A new collection is refused the same way, and nothing was created.
    let (status, body) = publish(&app, &ada, "other").await;
    assert_eq!(status, StatusCode::SERVICE_UNAVAILABLE, "{body}");
    let made: i64 = sqlx::query_scalar("SELECT count(*) FROM collections")
        .fetch_one(&app.state.db)
        .await
        .unwrap();
    assert_eq!(made, 1);
    // Share and pull keep working.
    let (status, _) = app
        .request(
            Method::GET,
            &format!("/v1/shared/{token}/snapshot"),
            None,
            None,
        )
        .await;
    assert_eq!(status, StatusCode::OK);
    let (status, _) = app
        .request(
            Method::GET,
            "/v1/teams/acme/collections/api/versions",
            None,
            Some(&ada),
        )
        .await;
    assert_eq!(status, StatusCode::OK);

    let (status, _) = put(&app, &ada, "publishing", json!({"paused": false})).await;
    assert_eq!(status, StatusCode::OK);
    let (status, body) = publish(&app, &ada, "other").await;
    assert_eq!(status, StatusCode::CREATED, "{body}");
}

#[sqlx::test]
async fn banner_audience_signed_in(pool: PgPool) {
    let (app, ada) = owner_app(pool, &[]).await;
    let (_, i) = info(&app, None).await;
    assert_eq!(i["banner"], Value::Null);

    let banner = |audience: &str| {
        json!({"message": " Maintenance on Friday. ", "tone": "warning",
               "audience": audience, "link": "https://status.example.com/"})
    };
    let (status, body) = put(&app, &ada, "banner", banner("signed_in")).await;
    assert_eq!(status, StatusCode::OK, "{body}");
    let (_, anon) = info(&app, None).await;
    assert_eq!(anon["banner"], Value::Null);
    // A stale or foreign bearer reads as anonymous: older CLIs send tokens here.
    for bearer in ["crs_bogus", "crt_bogus"] {
        let (status, body) = info(&app, Some(bearer)).await;
        assert_eq!(status, StatusCode::OK, "{bearer}");
        assert_eq!(body["banner"], Value::Null, "{bearer}");
    }
    let (status, signed) = info(&app, Some(&ada)).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(
        signed["banner"],
        json!({"message": "Maintenance on Friday.", "tone": "warning",
               "link": "https://status.example.com/", "version": 1})
    );

    let (status, _) = put(&app, &ada, "banner", banner("everyone")).await;
    assert_eq!(status, StatusCode::OK);
    let (_, anon) = info(&app, None).await;
    assert_eq!(anon["banner"]["message"], "Maintenance on Friday.");

    // An empty message clears it.
    let (status, _) = put(
        &app,
        &ada,
        "banner",
        json!({"message": "  ", "tone": "info", "audience": "everyone", "link": null}),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let (_, anon) = info(&app, None).await;
    assert_eq!(anon["banner"], Value::Null);
}

#[sqlx::test]
async fn banner_validation(pool: PgPool) {
    let (app, ada) = owner_app(pool, &[]).await;
    let ok = json!({"message": "Hi", "tone": "info", "audience": "everyone", "link": null});
    for (field, value) in [
        ("tone", json!("loud")),
        ("audience", json!("nobody")),
        ("link", json!("javascript:alert(1)")),
        ("link", json!("https://example.com/ b")),
        ("message", json!("x".repeat(281))),
    ] {
        let mut body = ok.clone();
        body[field] = value.clone();
        let (status, out) = put(&app, &ada, "banner", body).await;
        assert_eq!(status, StatusCode::BAD_REQUEST, "{field}={value}: {out}");
    }
    let (status, _) = put(&app, &ada, "nope", json!({})).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
}

#[sqlx::test]
async fn banner_version_bumps(pool: PgPool) {
    let (app, ada) = owner_app(pool, &[]).await;
    let s = get_settings(&app, &ada).await;
    assert_eq!(s["banner_version"], 0);
    for expected in 1..=3 {
        let (status, body) = put(
            &app,
            &ada,
            "banner",
            json!({"message": format!("Note {expected}"), "tone": "info",
                   "audience": "everyone", "link": null}),
        )
        .await;
        assert_eq!(status, StatusCode::OK, "{body}");
        assert_eq!(body["banner_version"], expected);
    }
    // Other sections leave it alone.
    let (_, body) = put(&app, &ada, "publishing", json!({"paused": true})).await;
    assert_eq!(body["banner_version"], 3);
}

#[sqlx::test]
async fn env_listing_hides_secrets(pool: PgPool) {
    let mut extra: Vec<(&str, &str)> = BILLING_ENV.to_vec();
    extra.extend([
        ("SMTP_HOST", "smtp.example.com"),
        ("SMTP_PORT", "2525"),
        ("SMTP_USER", "smtp-user-secret"),
        ("SMTP_PASSWORD", "smtp-pass-secret"),
        ("GITHUB_CLIENT_ID", "gh-id-secret"),
        ("GITHUB_CLIENT_SECRET", "gh-client-secret"),
        ("PUBLIC_APP_URL", "https://cloud.example.com"),
        ("PUBLIC_API_URL", "https://api.example.com"),
        ("MAIL_FROM", "curl-runner <no-reply@example.com>"),
        ("TRUSTED_PROXY_HOPS", "2"),
    ]);
    let (app, ada) = owner_app(pool, &extra).await;
    let s = get_settings(&app, &ada).await;
    let env = s["env"].as_array().unwrap();
    let find = |key: &str| {
        env.iter()
            .find(|e| e["key"] == key)
            .unwrap_or_else(|| panic!("{key}"))
    };

    let shown = [
        ("PUBLIC_APP_URL", "https://cloud.example.com"),
        ("PUBLIC_API_URL", "https://api.example.com"),
        ("SMTP_HOST", "smtp.example.com"),
        ("SMTP_PORT", "2525"),
        ("MAIL_FROM", "curl-runner <no-reply@example.com>"),
        ("STRIPE_PRICE_MONTHLY", "price_monthly"),
        ("STRIPE_PRICE_ANNUAL", "price_annual"),
        ("TRUSTED_PROXY_HOPS", "2"),
    ];
    for (key, value) in shown {
        assert_eq!(
            find(key),
            &json!({"key": key, "value": value, "secret": false, "set": true})
        );
    }
    // Every other variable config.rs reads is listed by name only.
    for key in [
        "DATABASE_URL",
        "DB_HOST",
        "DB_PORT",
        "DB_USER",
        "DB_PASSWORD",
        "DB_NAME",
        "SECRET_KEY",
        "SMTP_USER",
        "SMTP_PASSWORD",
        "GITHUB_CLIENT_ID",
        "GITHUB_CLIENT_SECRET",
        "GOOGLE_CLIENT_ID",
        "GOOGLE_CLIENT_SECRET",
        "SIGNUP",
        "SIGNUP_DOMAINS",
        "DEFAULT_PLAN",
        "OPERATOR_EMAILS",
        "STRIPE_SECRET_KEY",
        "STRIPE_WEBHOOK_SECRET",
    ] {
        let e = find(key);
        assert_eq!(e["value"], json!(null), "{key}");
        assert_eq!(e["secret"], json!(true), "{key}");
        assert!(e["set"].is_boolean(), "{key}");
    }
    // `set` follows what the server holds, and a secret never carries a value.
    for (key, set) in [
        ("SMTP_PASSWORD", true),
        ("GITHUB_CLIENT_ID", true),
        ("GITHUB_CLIENT_SECRET", true),
        ("STRIPE_SECRET_KEY", true),
        ("GOOGLE_CLIENT_ID", false),
        ("GOOGLE_CLIENT_SECRET", false),
    ] {
        assert_eq!(find(key)["set"], json!(set), "{key}");
    }
    let text = s.to_string();
    for secret in [
        "smtp-user-secret",
        "smtp-pass-secret",
        "gh-id-secret",
        "gh-client-secret",
        "sk_test_fake",
        "whsec_fake",
        "test-secret-key",
        "postgres://",
        OWNER,
    ] {
        assert!(!text.contains(secret), "leaked {secret}");
    }
}

#[sqlx::test]
async fn settings_cache_expires_after_five_seconds(pool: PgPool) {
    let (app, ada) = owner_app(pool, &[]).await;
    get_settings(&app, &ada).await;
    assert_eq!(app.state.settings().await.unwrap().grace_days, 14);

    // A write by another process: this one sees it only after the TTL.
    sqlx::query("UPDATE server_settings SET grace_days = 30")
        .execute(&app.state.db)
        .await
        .unwrap();
    app.clock.advance(Duration::seconds(4));
    assert_eq!(app.state.settings().await.unwrap().grace_days, 14);
    app.clock.advance(Duration::seconds(2));
    assert_eq!(app.state.settings().await.unwrap().grace_days, 30);

    // A write through the API shows at once, inside the TTL.
    let (status, _) = put(
        &app,
        &ada,
        "plans",
        json!({"default_plan": "free", "free_member_limit": 3, "free_restricted": false, "grace_days": 7}),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(app.state.settings().await.unwrap().grace_days, 7);
}

#[sqlx::test]
async fn permissions_and_audit(pool: PgPool) {
    let (app, ada) = owner_app(pool, &[]).await;
    let rosa = app.sign_in("rosa@example.com").await;
    let (status, body) = app
        .request(
            Method::POST,
            "/v1/admin/admins",
            Some(json!({"email": "rosa@example.com", "permissions": ["users"]})),
            Some(&ada),
        )
        .await;
    assert_eq!(status, StatusCode::CREATED, "{body}");
    // Any admin reads.
    get_settings(&app, &rosa).await;
    // Without the permission, writes are 403 and change nothing.
    let (status, body) = put(&app, &rosa, "publishing", json!({"paused": true})).await;
    assert_eq!(status, StatusCode::FORBIDDEN, "{body}");
    let (status, _) = app
        .request(
            Method::POST,
            "/v1/admin/settings/test-email",
            None,
            Some(&rosa),
        )
        .await;
    assert_eq!(status, StatusCode::FORBIDDEN);
    assert_eq!(get_settings(&app, &ada).await["publishing_paused"], false);

    // A write logs who, what and the change.
    let (status, _) = put(&app, &ada, "publishing", json!({"paused": true})).await;
    assert_eq!(status, StatusCode::OK);
    let (action, kind, id, detail): (String, String, String, Value) = sqlx::query_as(
        "SELECT action, target_kind, target_id, detail FROM admin_events \
         WHERE action = 'settings.update'",
    )
    .fetch_one(&app.state.db)
    .await
    .unwrap();
    assert_eq!(
        (action.as_str(), kind.as_str(), id.as_str()),
        ("settings.update", "settings", "publishing")
    );
    assert_eq!(detail["section"], "publishing");
    assert_eq!(detail["from"], json!({"paused": false}));
    assert_eq!(detail["to"], json!({"paused": true}));
    let (by, at): (Option<uuid::Uuid>, Option<chrono::DateTime<chrono::Utc>>) =
        sqlx::query_as("SELECT updated_by, updated_at FROM server_settings")
            .fetch_one(&app.state.db)
            .await
            .unwrap();
    assert!(by.is_some() && at.is_some());
}

#[sqlx::test]
async fn test_email_goes_to_the_calling_admin(pool: PgPool) {
    let (app, ada) = owner_app(pool, &[]).await;
    let (status, body) = app
        .request(
            Method::POST,
            "/v1/admin/settings/test-email",
            None,
            Some(&ada),
        )
        .await;
    assert_eq!(status, StatusCode::ACCEPTED, "{body}");
    assert_eq!(body, json!({"to": OWNER}));
    let (to, subject, html): (String, String, String) = sqlx::query_as(
        "SELECT to_addr, subject, html_body FROM outbox_emails \
         WHERE subject LIKE '%test email%'",
    )
    .fetch_one(&app.state.db)
    .await
    .unwrap();
    assert_eq!(to, OWNER);
    assert!(subject.starts_with("curl-runner test email"), "{subject}");
    assert!(html.contains("curl-runner</td>"));
    assert_eq!(event_actions(&app).await, ["settings.test_email"]);
}
