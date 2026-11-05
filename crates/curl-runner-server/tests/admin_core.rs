#![cfg(feature = "db-tests")]

mod common;

use std::borrow::Cow;

use axum::body::Body;
use axum::http::{HeaderMap, Request, StatusCode, header};
use chrono::{DateTime, Duration, Utc};
use common::{NOW, TestApp, test_app_with, test_config};
use curl_runner_server::clock::Clock;
use http_body_util::BodyExt;
use serde_json::{Value, json};
use sqlx::PgPool;
use sqlx::migrate::Migrator;
use tower::ServiceExt;
use uuid::Uuid;

const OWNER: &str = "ada@example.com";
const ROSA: &str = "rosa@example.com";
const NOT_FOUND: &str = r#"{"error":"not_found","message":"Not found."}"#;
const ALL_PERMS: [&str; 6] = ["users", "teams", "billing", "emails", "settings", "admins"];

async fn owner_app(pool: PgPool) -> TestApp {
    test_app_with(pool, test_config(&[("OPERATOR_EMAILS", OWNER)])).await
}

/// One raw request: any method, body and headers. Returns the status, the
/// headers and the body as text.
async fn send(
    app: &TestApp,
    method: &str,
    path: &str,
    body: Option<&str>,
    bearer: Option<&str>,
    ip: Option<&str>,
) -> (StatusCode, HeaderMap, String) {
    let mut req = Request::builder().method(method).uri(path);
    if let Some(token) = bearer {
        req = req.header(header::AUTHORIZATION, format!("Bearer {token}"));
    }
    if let Some(ip) = ip {
        req = req.header("x-forwarded-for", ip);
    }
    let body = match body {
        Some(b) => {
            req = req.header(header::CONTENT_TYPE, "application/json");
            Body::from(b.to_owned())
        }
        None => Body::empty(),
    };
    let res = curl_runner_server::app(app.state.clone())
        .oneshot(req.body(body).unwrap())
        .await
        .unwrap();
    let status = res.status();
    let headers = res.headers().clone();
    let bytes = res.into_body().collect().await.unwrap().to_bytes();
    (
        status,
        headers,
        String::from_utf8_lossy(&bytes).into_owned(),
    )
}

async fn json_req(
    app: &TestApp,
    method: &str,
    path: &str,
    body: Option<Value>,
    bearer: &str,
) -> (StatusCode, Value) {
    let text = body.map(|b| b.to_string());
    let (status, _, out) = send(app, method, path, text.as_deref(), Some(bearer), None).await;
    let value = if out.is_empty() {
        Value::Null
    } else {
        serde_json::from_str(&out).unwrap_or(Value::String(out))
    };
    (status, value)
}

async fn user_id(app: &TestApp, session: &str) -> String {
    let (_, me) = json_req(app, "GET", "/v1/me", None, session).await;
    me["user"]["id"].as_str().unwrap().to_owned()
}

async fn add_admin(app: &TestApp, by: &str, email: &str, perms: Value) -> (StatusCode, Value) {
    json_req(
        app,
        "POST",
        "/v1/admin/admins",
        Some(json!({"email": email, "permissions": perms})),
        by,
    )
    .await
}

async fn event_count(app: &TestApp) -> i64 {
    sqlx::query_scalar("SELECT count(*) FROM admin_events")
        .fetch_one(&app.state.db)
        .await
        .unwrap()
}

/// Every `/v1/admin` route, with a body that would be valid for an admin.
/// Later tasks add their routes here.
fn admin_routes(some_user: &str) -> Vec<(&'static str, String, Option<String>)> {
    let perms = json!({"permissions": ["users"]}).to_string();
    vec![
        ("GET", "/v1/admin/me".into(), None),
        ("GET", "/v1/admin/admins".into(), None),
        (
            "POST",
            "/v1/admin/admins".into(),
            Some(json!({"email": ROSA, "permissions": []}).to_string()),
        ),
        (
            "PATCH",
            format!("/v1/admin/admins/{some_user}"),
            Some(perms.clone()),
        ),
        ("DELETE", format!("/v1/admin/admins/{some_user}"), None),
        ("GET", "/v1/admin/activity".into(), None),
        (
            "GET",
            "/v1/admin/activity?action_group=admins&since=2026-01-01T00:00:00Z".into(),
            None,
        ),
        ("GET", "/v1/admin/activity.csv".into(), None),
        ("GET", "/v1/admin/settings".into(), None),
        (
            "PUT",
            "/v1/admin/settings/signup".into(),
            Some(json!({"mode": "closed", "domains": []}).to_string()),
        ),
        (
            "PUT",
            "/v1/admin/settings/plans".into(),
            Some(
                json!({"default_plan": "free", "free_member_limit": 3,
                       "free_restricted": false, "grace_days": 14})
                .to_string(),
            ),
        ),
        (
            "PUT",
            "/v1/admin/settings/banner".into(),
            Some(
                json!({"message": "Hi", "tone": "info", "audience": "everyone", "link": null})
                    .to_string(),
            ),
        ),
        (
            "PUT",
            "/v1/admin/settings/publishing".into(),
            Some(json!({"paused": true}).to_string()),
        ),
        ("PUT", "/v1/admin/settings/nope".into(), Some("{}".into())),
        ("POST", "/v1/admin/settings/test-email".into(), None),
        (
            "PUT",
            "/v1/admin/teams/acme/plan".into(),
            Some(json!({"plan": "team"}).to_string()),
        ),
        ("GET", "/v1/admin/users".into(), None),
        ("GET", "/v1/admin/users?q=ada&filter=admins".into(), None),
        ("GET", "/v1/admin/users.csv".into(), None),
        ("GET", format!("/v1/admin/users/{some_user}"), None),
        (
            "POST",
            format!("/v1/admin/users/{some_user}/suspend"),
            Some(json!({"reason": "x", "notify": true}).to_string()),
        ),
        (
            "POST",
            format!("/v1/admin/users/{some_user}/unsuspend"),
            None,
        ),
        (
            "POST",
            format!("/v1/admin/users/{some_user}/sign-out-everywhere"),
            None,
        ),
        (
            "DELETE",
            format!("/v1/admin/users/{some_user}/sessions/{some_user}"),
            None,
        ),
        (
            "DELETE",
            format!("/v1/admin/users/{some_user}/identities/github"),
            None,
        ),
        ("DELETE", format!("/v1/admin/users/{some_user}"), None),
        (
            "POST",
            format!("/v1/admin/users/{some_user}/email"),
            Some(json!({"subject": "Hi", "body_md": "Hello"}).to_string()),
        ),
        (
            "POST",
            "/v1/admin/users/bulk".into(),
            Some(json!({"ids": [some_user], "action": "sign_out"}).to_string()),
        ),
        ("GET", "/v1/admin/teams".into(), None),
        ("GET", "/v1/admin/teams?q=acme&filter=granted".into(), None),
        ("GET", "/v1/admin/teams.csv".into(), None),
        ("GET", "/v1/admin/teams/acme".into(), None),
        ("DELETE", "/v1/admin/teams/acme".into(), None),
        (
            "PUT",
            "/v1/admin/teams/acme/override".into(),
            Some(json!({"plan": "team", "reason": "x", "notify": true}).to_string()),
        ),
        ("DELETE", "/v1/admin/teams/acme/override".into(), None),
        (
            "POST",
            "/v1/admin/teams/acme/suspend".into(),
            Some(json!({"reason": "x", "notify": true}).to_string()),
        ),
        ("POST", "/v1/admin/teams/acme/unsuspend".into(), None),
        (
            "PATCH",
            format!("/v1/admin/teams/acme/members/{some_user}"),
            Some(json!({"role": "member"}).to_string()),
        ),
        ("POST", "/v1/admin/teams/acme/stripe-sync".into(), None),
        ("POST", "/v1/admin/teams/acme/resend-invoice".into(), None),
        (
            "POST",
            "/v1/admin/teams/acme/email-owners".into(),
            Some(json!({"subject": "Hi", "body_md": "Hello"}).to_string()),
        ),
    ]
}

#[sqlx::test]
async fn non_admin_gets_404_everywhere(pool: PgPool) {
    let app = owner_app(pool).await;
    let jane = app.sign_in("jane@example.com").await;
    app.create_team(&jane, "acme").await;
    let jane_id = user_id(&app, &jane).await;
    let mut probes = admin_routes(&jane_id);
    // Malformed input must not answer 400 before the admin check.
    probes.extend([
        ("POST", "/v1/admin/admins".into(), Some("{nope".into())),
        (
            "PATCH",
            "/v1/admin/admins/not-a-uuid".into(),
            Some("{}".into()),
        ),
        ("GET", "/v1/admin/activity?since=yesterday".into(), None),
        ("GET", "/v1/admin/activity?cursor=nope&limit=0".into(), None),
        // A wrong method, and a route that does not exist.
        ("POST", "/v1/admin/me".into(), None),
        ("DELETE", "/v1/admin/activity".into(), None),
        (
            "PUT",
            "/v1/admin/settings/plans".into(),
            Some("{nope".into()),
        ),
        ("PUT", "/v1/admin/settings".into(), None),
        ("GET", "/v1/admin/settings/test-email".into(), None),
        ("GET", "/v1/admin/nope".into(), None),
        ("GET", "/v1/admin/users?filter=nope&limit=0".into(), None),
        ("GET", "/v1/admin/users/not-a-uuid".into(), None),
        ("POST", "/v1/admin/users/bulk".into(), Some("{nope".into())),
        ("PUT", format!("/v1/admin/users/{jane_id}"), None),
        ("GET", "/v1/admin/teams?filter=nope&limit=0".into(), None),
        (
            "PUT",
            "/v1/admin/teams/acme/override".into(),
            Some("{nope".into()),
        ),
        (
            "PATCH",
            "/v1/admin/teams/acme/members/not-a-uuid".into(),
            Some("{}".into()),
        ),
        ("PUT", "/v1/admin/teams/acme/suspend".into(), None),
    ]);
    let callers: [(&str, Option<&str>); 4] = [
        ("anonymous", None),
        ("bogus session", Some("crs_bogus")),
        ("publish token", Some("crt_bogus")),
        ("signed-in non-admin", Some(&jane)),
    ];
    for (who, bearer) in callers {
        for (method, path, body) in &probes {
            let (status, _, out) = send(&app, method, path, body.as_deref(), bearer, None).await;
            assert_eq!(
                (status, out.as_str()),
                (StatusCode::NOT_FOUND, NOT_FOUND),
                "{who}: {method} {path}"
            );
        }
    }
    // The team's plan is untouched and nothing was logged.
    let (_, team) = json_req(&app, "GET", "/v1/teams/acme", None, &jane).await;
    assert_eq!(team["plan"], "free");
    assert_eq!(event_count(&app).await, 0);
    let (_, me) = json_req(&app, "GET", "/v1/me", None, &jane).await;
    assert_eq!(me["admin"], false);
}

#[sqlx::test]
async fn owner_from_env_has_all_perms(pool: PgPool) {
    let app = owner_app(pool).await;
    let ada = app.sign_in("Ada@Example.com").await;
    let (status, me) = json_req(&app, "GET", "/v1/admin/me", None, &ada).await;
    assert_eq!(status, StatusCode::OK, "{me}");
    assert_eq!(me, json!({"role": "owner", "permissions": ALL_PERMS}));
    let (_, me) = json_req(&app, "GET", "/v1/me", None, &ada).await;
    assert_eq!(me["admin"], true);

    // The recorded first operator is an owner too.
    let first = app.sign_in("first@example.com").await;
    sqlx::query("INSERT INTO settings (key, value) VALUES ('first_operator', 'first@example.com')")
        .execute(&app.state.db)
        .await
        .unwrap();
    let (_, me) = json_req(&app, "GET", "/v1/admin/me", None, &first).await;
    assert_eq!(me, json!({"role": "owner", "permissions": ALL_PERMS}));

    // Owners come first in the list, with no `added_by`.
    let (status, list) = json_req(&app, "GET", "/v1/admin/admins", None, &ada).await;
    assert_eq!(status, StatusCode::OK, "{list}");
    let roles: Vec<&str> = list
        .as_array()
        .unwrap()
        .iter()
        .map(|a| a["role"].as_str().unwrap())
        .collect();
    assert_eq!(roles, ["owner", "owner"]);
    assert_eq!(list[0]["email"], OWNER);
    assert_eq!(list[0]["permissions"], json!(ALL_PERMS));
    assert_eq!(list[0]["added_by"], Value::Null);
}

#[sqlx::test]
async fn admin_without_perm_403(pool: PgPool) {
    let app = owner_app(pool).await;
    let ada = app.sign_in(OWNER).await;
    app.create_team(&ada, "acme").await;
    let rosa = app.sign_in(ROSA).await;
    let (status, body) = add_admin(&app, &ada, ROSA, json!(["users"])).await;
    assert_eq!(status, StatusCode::CREATED, "{body}");

    let (status, me) = json_req(&app, "GET", "/v1/admin/me", None, &rosa).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(me, json!({"role": "admin", "permissions": ["users"]}));
    let (_, me) = json_req(&app, "GET", "/v1/me", None, &rosa).await;
    assert_eq!(me["admin"], true);

    // Every admin reads everything.
    for path in ["/v1/admin/admins", "/v1/admin/activity"] {
        let (status, _) = json_req(&app, "GET", path, None, &rosa).await;
        assert_eq!(status, StatusCode::OK, "{path}");
    }
    let (status, _, _) = send(
        &app,
        "GET",
        "/v1/admin/activity.csv",
        None,
        Some(&rosa),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK);

    // Writes need their permission.
    let forbidden =
        json!({"error": "forbidden", "message": "You don't have permission to do that."});
    let ada_id = user_id(&app, &ada).await;
    let rosa_id = user_id(&app, &rosa).await;
    let writes = [
        (
            "POST",
            "/v1/admin/admins".to_owned(),
            Some(json!({"email": "jane@example.com", "permissions": []})),
        ),
        (
            "PATCH",
            format!("/v1/admin/admins/{rosa_id}"),
            Some(json!({"permissions": ALL_PERMS})),
        ),
        ("DELETE", format!("/v1/admin/admins/{ada_id}"), None),
        (
            "PUT",
            "/v1/admin/teams/acme/plan".to_owned(),
            Some(json!({"plan": "team"})),
        ),
    ];
    for (method, path, body) in writes {
        let (status, out) = json_req(&app, method, &path, body, &rosa).await;
        assert_eq!(
            (status, &out),
            (StatusCode::FORBIDDEN, &forbidden),
            "{method} {path}"
        );
    }
    assert_eq!(event_count(&app).await, 1, "only the add was logged");

    // With `billing`, the plan route works as before.
    let (status, _) = json_req(
        &app,
        "PATCH",
        &format!("/v1/admin/admins/{rosa_id}"),
        Some(json!({"permissions": ["billing"]})),
        &ada,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let (status, body) = json_req(
        &app,
        "PUT",
        "/v1/admin/teams/acme/plan",
        Some(json!({"plan": "team"})),
        &rosa,
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(body, json!({"slug": "acme", "plan": "team"}));
    let (status, _) = json_req(
        &app,
        "PUT",
        "/v1/admin/teams/missing/plan",
        Some(json!({"plan": "team"})),
        &rosa,
    )
    .await;
    assert_eq!(status, StatusCode::NOT_FOUND);
}

#[sqlx::test]
async fn add_admin_requires_existing_user(pool: PgPool) {
    let app = owner_app(pool).await;
    let ada = app.sign_in(OWNER).await;
    let (status, body) = add_admin(&app, &ada, ROSA, json!(["users"])).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    assert_eq!(
        body["message"],
        "No account with that email yet. Ask them to sign in once."
    );
    assert_eq!(event_count(&app).await, 0);

    let rosa = app.sign_in(ROSA).await;
    let rosa_id = user_id(&app, &rosa).await;
    // Case-insensitive, deduplicated and in a stable order.
    let (status, body) = add_admin(
        &app,
        &ada,
        " Rosa@Example.com ",
        json!(["emails", "users", "emails"]),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED, "{body}");
    assert_eq!(body["user_id"], rosa_id.as_str());
    assert_eq!(body["email"], ROSA);
    assert_eq!(body["role"], "admin");
    assert_eq!(body["permissions"], json!(["users", "emails"]));
    assert_eq!(body["added_by"]["email"], OWNER);
    assert_eq!(body["added_at"], "2026-11-04T10:00:00Z");

    // Already an admin, or an owner: 409.
    let (status, _) = add_admin(&app, &ada, ROSA, json!([])).await;
    assert_eq!(status, StatusCode::CONFLICT);
    let (status, body) = add_admin(&app, &ada, OWNER, json!([])).await;
    assert_eq!(status, StatusCode::CONFLICT);
    assert_eq!(
        body["message"],
        "Owners come from OPERATOR_EMAILS and can't be changed here."
    );
    // An unknown permission is a bad request.
    let (status, body) = add_admin(&app, &ada, "jane@example.com", json!(["root"])).await;
    assert_eq!(status, StatusCode::BAD_REQUEST, "{body}");

    // The list shows owners first, then admins.
    let (_, list) = json_req(&app, "GET", "/v1/admin/admins", None, &ada).await;
    let emails: Vec<&str> = list
        .as_array()
        .unwrap()
        .iter()
        .map(|a| a["email"].as_str().unwrap())
        .collect();
    assert_eq!(emails, [OWNER, ROSA]);
    assert_eq!(list[1]["name"], Value::Null);
    assert_eq!(list[1]["last_active"], "2026-11-04T10:00:00Z");
    assert_eq!(list[1]["added_by"]["email"], OWNER);
}

#[sqlx::test]
async fn owner_cannot_be_removed(pool: PgPool) {
    let app = owner_app(pool).await;
    let ada = app.sign_in(OWNER).await;
    let ada_id = user_id(&app, &ada).await;
    let rosa = app.sign_in(ROSA).await;
    let rosa_id = user_id(&app, &rosa).await;
    add_admin(&app, &ada, ROSA, json!(["admins"])).await;

    let owner_msg = "Owners come from OPERATOR_EMAILS and can't be changed here.";
    let (status, body) = json_req(
        &app,
        "PATCH",
        &format!("/v1/admin/admins/{ada_id}"),
        Some(json!({"permissions": []})),
        &rosa,
    )
    .await;
    assert_eq!(
        (status, body["message"].as_str()),
        (StatusCode::CONFLICT, Some(owner_msg))
    );
    let (status, body) = json_req(
        &app,
        "DELETE",
        &format!("/v1/admin/admins/{ada_id}"),
        None,
        &rosa,
    )
    .await;
    assert_eq!(
        (status, body["message"].as_str()),
        (StatusCode::CONFLICT, Some(owner_msg))
    );
    let (_, me) = json_req(&app, "GET", "/v1/admin/me", None, &ada).await;
    assert_eq!(me["role"], "owner");

    // A user who is not an admin: 404.
    let jane = app.sign_in("jane@example.com").await;
    let jane_id = user_id(&app, &jane).await;
    let (status, _) = json_req(
        &app,
        "DELETE",
        &format!("/v1/admin/admins/{jane_id}"),
        None,
        &ada,
    )
    .await;
    assert_eq!(status, StatusCode::NOT_FOUND);

    // An admin can be removed; they lose the panel and keep their account.
    let (status, _) = json_req(
        &app,
        "DELETE",
        &format!("/v1/admin/admins/{rosa_id}"),
        None,
        &ada,
    )
    .await;
    assert_eq!(status, StatusCode::NO_CONTENT);
    let (status, _) = json_req(&app, "GET", "/v1/admin/me", None, &rosa).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    let (status, me) = json_req(&app, "GET", "/v1/me", None, &rosa).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(me["admin"], false);
}

/// The event a write should record.
struct Expect {
    action: &'static str,
    kind: &'static str,
    target_id: String,
    label: &'static str,
    detail: Value,
}

#[derive(sqlx::FromRow)]
struct EventRow {
    admin_user_id: Uuid,
    action: String,
    target_kind: Option<String>,
    target_id: Option<String>,
    target_label: Option<String>,
    detail: Value,
    ip: Option<String>,
    at: DateTime<Utc>,
}

#[sqlx::test]
async fn every_write_records_one_event(pool: PgPool) {
    let app = owner_app(pool).await;
    let ada = app.sign_in(OWNER).await;
    let ada_id = user_id(&app, &ada).await;
    app.create_team(&ada, "acme").await;
    let rosa = app.sign_in(ROSA).await;
    let rosa_id = user_id(&app, &rosa).await;
    let ip = Some("203.0.113.7");

    let team_id: Uuid = sqlx::query_scalar("SELECT id FROM teams WHERE slug = 'acme'")
        .fetch_one(&app.state.db)
        .await
        .unwrap();
    let team_id = team_id.to_string();

    let user = |action, detail| Expect {
        action,
        kind: "user",
        target_id: rosa_id.clone(),
        label: ROSA,
        detail,
    };
    let writes = [
        (
            "POST",
            "/v1/admin/admins".to_owned(),
            Some(json!({"email": ROSA, "permissions": ["users"]})),
            user("admin.add", json!({"permissions": ["users"]})),
        ),
        (
            "PATCH",
            format!("/v1/admin/admins/{rosa_id}"),
            Some(json!({"permissions": ["users", "emails"]})),
            user(
                "admin.update",
                json!({"from": ["users"], "to": ["users", "emails"]}),
            ),
        ),
        (
            "PUT",
            "/v1/admin/teams/acme/plan".to_owned(),
            Some(json!({"plan": "team"})),
            Expect {
                action: "team.plan.set",
                kind: "team",
                target_id: team_id,
                label: "acme",
                detail: json!({"from": "free", "to": "team"}),
            },
        ),
        (
            "DELETE",
            format!("/v1/admin/admins/{rosa_id}"),
            None,
            user("admin.remove", json!({"permissions": ["users", "emails"]})),
        ),
    ];
    for (i, (method, path, body, want)) in writes.into_iter().enumerate() {
        let text = body.map(|b| b.to_string());
        let (status, _, out) = send(&app, method, &path, text.as_deref(), Some(&ada), ip).await;
        assert!(status.is_success(), "{method} {path}: {status} {out}");
        assert_eq!(event_count(&app).await, i as i64 + 1, "{}", want.action);
        let row: EventRow = sqlx::query_as(
            "SELECT admin_user_id, action, target_kind, target_id, target_label, detail, ip, at \
             FROM admin_events ORDER BY at DESC LIMIT 1",
        )
        .fetch_one(&app.state.db)
        .await
        .unwrap();
        assert_eq!(row.admin_user_id.to_string(), ada_id);
        assert_eq!(row.action, want.action);
        assert_eq!(row.target_kind.as_deref(), Some(want.kind));
        assert_eq!(row.target_id, Some(want.target_id));
        assert_eq!(row.target_label.as_deref(), Some(want.label));
        assert_eq!(row.detail, want.detail, "{}", want.action);
        assert_eq!(row.ip.as_deref(), Some("203.0.113.7"));
        assert_eq!(row.at, app.clock.now(), "the app clock's time");
        app.clock.advance(Duration::seconds(1));
    }

    // Refused writes record nothing.
    let before = event_count(&app).await;
    add_admin(&app, &ada, "nobody@example.com", json!([])).await;
    json_req(
        &app,
        "DELETE",
        &format!("/v1/admin/admins/{ada_id}"),
        None,
        &ada,
    )
    .await;
    json_req(
        &app,
        "PUT",
        "/v1/admin/teams/missing/plan",
        Some(json!({"plan": "team"})),
        &ada,
    )
    .await;
    assert_eq!(event_count(&app).await, before);
}

/// `n` events by `admin`, one second apart from the clock's now, actions cycling.
async fn seed_events(app: &TestApp, admin: Uuid, actions: &[&str], start: DateTime<Utc>) {
    for (i, action) in actions.iter().enumerate() {
        sqlx::query(
            "INSERT INTO admin_events (id, admin_user_id, admin_email, action, target_kind, target_id, target_label, detail, ip, at) \
             VALUES ($1, $2, 'snapshot@example.com', $3, 'team', 't', 'acme', '{}', '203.0.113.7', $4)",
        )
        .bind(Uuid::new_v4())
        .bind(admin)
        .bind(action)
        .bind(start + Duration::seconds(i as i64))
        .execute(&app.state.db)
        .await
        .unwrap();
    }
}

#[sqlx::test]
async fn activity_filters_and_cursor(pool: PgPool) {
    let app = owner_app(pool).await;
    let ada = app.sign_in(OWNER).await;
    let ada_id: Uuid = user_id(&app, &ada).await.parse().unwrap();
    let rosa = app.sign_in(ROSA).await;
    let rosa_id: Uuid = user_id(&app, &rosa).await.parse().unwrap();
    let now: DateTime<Utc> = NOW.parse().unwrap();
    let old = now - Duration::days(40);
    seed_events(
        &app,
        ada_id,
        &["team.override.grant", "user.suspend", "settings.update"],
        old,
    )
    .await;
    seed_events(
        &app,
        rosa_id,
        &[
            "broadcast.send",
            "collection.link.disable",
            "team.unsuspend",
            "user.email",
            "admin.add",
            "team.override.remove",
        ],
        now - Duration::days(1),
    )
    .await;

    let actions = |body: &Value| -> Vec<String> {
        body["items"]
            .as_array()
            .unwrap()
            .iter()
            .map(|i| i["action"].as_str().unwrap().to_owned())
            .collect()
    };

    // Newest first, with the admin and every field.
    let all = list_by(&app, &ada, "").await;
    assert_eq!(all["items"].as_array().unwrap().len(), 9);
    assert_eq!(all["total"], 9);
    assert_eq!(all["next_cursor"], Value::Null);
    let first = &all["items"][0];
    assert_eq!(first["action"], "team.override.remove");
    assert_eq!(first["admin"]["email"], ROSA);
    assert_eq!(first["admin"]["user_id"], rosa_id.to_string());
    assert_eq!(first["target_kind"], "team");
    assert_eq!(first["target_id"], "t");
    assert_eq!(first["target_label"], "acme");
    assert_eq!(first["detail"], json!({}));
    assert_eq!(first["ip"], "203.0.113.7");
    assert!(first["at"].is_string() && first["id"].is_string());

    // By admin.
    let body = list_by(&app, &ada, &format!("?admin={ada_id}")).await;
    assert_eq!(
        actions(&body),
        ["settings.update", "user.suspend", "team.override.grant"]
    );

    // By group.
    let groups = [
        (
            "override",
            vec!["team.override.remove", "team.override.grant"],
        ),
        (
            "suspension",
            vec!["team.unsuspend", "collection.link.disable", "user.suspend"],
        ),
        ("email", vec!["user.email", "broadcast.send"]),
        ("settings", vec!["settings.update"]),
        ("admins", vec!["admin.add"]),
    ];
    for (group, want) in groups {
        let body = list_by(&app, &ada, &format!("?action_group={group}")).await;
        assert_eq!(actions(&body), want, "{group}");
    }

    // Since: the last 7 days leaves out the old ones.
    let since = (now - Duration::days(7)).to_rfc3339_opts(chrono::SecondsFormat::Secs, true);
    let body = list_by(&app, &ada, &format!("?since={since}")).await;
    assert_eq!(body["items"].as_array().unwrap().len(), 6);
    assert_eq!(body["total"], 6);

    // Combined filters.
    let body = list_by(
        &app,
        &ada,
        &format!("?admin={ada_id}&action_group=override&since={since}"),
    )
    .await;
    assert_eq!(body["items"], json!([]));

    // The cursor walks every row once, newest first.
    let mut seen = Vec::new();
    let mut cursor: Option<String> = None;
    loop {
        let q = match &cursor {
            Some(c) => format!("?limit=4&cursor={c}"),
            None => "?limit=4".to_owned(),
        };
        let body = list_by(&app, &ada, &q).await;
        assert_eq!(body["total"], 9);
        seen.extend(actions(&body));
        match body["next_cursor"].as_str() {
            Some(c) => cursor = Some(c.to_owned()),
            None => break,
        }
    }
    assert_eq!(seen, actions(&all));

    // Bad input is a 400.
    for q in [
        "?action_group=everything",
        "?since=yesterday",
        "?cursor=nope",
        "?admin=nope",
        "?limit=0",
    ] {
        let (status, body) =
            json_req(&app, "GET", &format!("/v1/admin/activity{q}"), None, &ada).await;
        assert_eq!(status, StatusCode::BAD_REQUEST, "{q}: {body}");
        assert_eq!(body["error"], "invalid_request");
    }
}

async fn list_by(app: &TestApp, session: &str, q: &str) -> Value {
    let (status, body) =
        json_req(app, "GET", &format!("/v1/admin/activity{q}"), None, session).await;
    assert_eq!(status, StatusCode::OK, "{q}: {body}");
    body
}

#[sqlx::test]
async fn csv_escapes_formulas(pool: PgPool) {
    let app = owner_app(pool).await;
    let ada = app.sign_in(OWNER).await;
    let ada_id: Uuid = user_id(&app, &ada).await.parse().unwrap();
    let now: DateTime<Utc> = NOW.parse().unwrap();
    for (i, label) in [
        "=HYPERLINK(\"http://evil.example\",\"x\")",
        "+1+1",
        "-2",
        "@SUM(A1)",
        "plain, with comma",
    ]
    .iter()
    .enumerate()
    {
        sqlx::query(
            "INSERT INTO admin_events (id, admin_user_id, admin_email, action, target_kind, target_id, target_label, detail, ip, at) \
             VALUES ($1, $2, 'snapshot@example.com', 'team.suspend', 'team', 't', $3, $4, '203.0.113.7', $5)",
        )
        .bind(Uuid::new_v4())
        .bind(ada_id)
        .bind(label)
        .bind(json!({"reason": "multi\nline"}))
        .bind(now - Duration::seconds(i as i64))
        .execute(&app.state.db)
        .await
        .unwrap();
    }
    let (status, headers, body) = send(
        &app,
        "GET",
        "/v1/admin/activity.csv",
        None,
        Some(&ada),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(headers[header::CONTENT_TYPE], "text/csv; charset=utf-8");
    assert_eq!(
        headers[header::CONTENT_DISPOSITION],
        "attachment; filename=\"activity-2026-11-04.csv\""
    );
    let lines: Vec<&str> = body.split("\r\n").collect();
    assert_eq!(
        lines[0],
        "at,admin_email,action,target_kind,target_id,target_label,detail,ip"
    );
    assert!(
        lines[1].starts_with(&format!(
            "2026-11-04T10:00:00Z,{OWNER},team.suspend,team,t,\"'=HYPERLINK(\"\"http://evil.example\"\",\"\"x\"\")\",\"{{\"\"reason\"\":\"\"multi\\nline\"\"}}\",203.0.113.7"
        )),
        "{}",
        lines[1]
    );
    assert!(body.contains(",'+1+1,"), "{body}");
    assert!(body.contains(",'-2,"), "{body}");
    assert!(body.contains(",'@SUM(A1),"), "{body}");
    assert!(body.contains(",\"plain, with comma\","), "{body}");
    // Header, five rows and the trailing line break.
    assert_eq!(lines.len(), 7);
    assert_eq!(lines[6], "");

    // Filters apply to the export.
    let (_, _, body) = send(
        &app,
        "GET",
        "/v1/admin/activity.csv?action_group=override",
        None,
        Some(&ada),
        None,
    )
    .await;
    assert_eq!(body.split("\r\n").count(), 2);
}

#[sqlx::test]
async fn retention_deletes_events_older_than_2_years(pool: PgPool) {
    let app = owner_app(pool).await;
    let now: DateTime<Utc> = NOW.parse().unwrap();
    let admin = Uuid::new_v4();
    let two_years = now - chrono::Months::new(24);
    for (action, at) in [
        ("too.old", two_years - Duration::days(1)),
        ("just.kept", two_years + Duration::hours(1)),
        ("recent", now),
    ] {
        seed_events(&app, admin, &[action], at).await;
    }
    curl_runner_server::billing::sync::tick(&app.state, now).await;
    let left: Vec<String> = sqlx::query_scalar("SELECT action FROM admin_events ORDER BY at")
        .fetch_all(&app.state.db)
        .await
        .unwrap();
    assert_eq!(left, ["just.kept", "recent"]);
}

/// 0004 applies to a database that already has 0001 to 0003 data.
#[sqlx::test(migrations = false)]
async fn migration_0004_applies_to_populated_db(pool: PgPool) {
    let all: Migrator = sqlx::migrate!("./migrations");
    let before = Migrator {
        migrations: Cow::Owned(
            all.migrations
                .iter()
                .filter(|m| m.version < 4)
                .cloned()
                .collect(),
        ),
        ..Migrator::DEFAULT
    };
    assert_eq!(before.migrations.len(), 3);
    before.run(&pool).await.unwrap();

    let now: DateTime<Utc> = NOW.parse().unwrap();
    let db = &pool;
    let user: Uuid = sqlx::query_scalar(
        "INSERT INTO users (email, name, created_at) VALUES ('ada@example.com', 'Ada', $1) RETURNING id",
    )
    .bind(now)
    .fetch_one(db)
    .await
    .unwrap();
    let team: Uuid = sqlx::query_scalar(
        "INSERT INTO teams (slug, name, plan, created_at, stripe_subscription_id, billing_interval) \
         VALUES ('acme', 'Acme', 'team', $1, 'sub_1', 'monthly') RETURNING id",
    )
    .bind(now)
    .fetch_one(db)
    .await
    .unwrap();
    sqlx::query(
        "INSERT INTO memberships (team_id, user_id, role, created_at) VALUES ($1, $2, 'owner', $3)",
    )
    .bind(team)
    .bind(user)
    .bind(now)
    .execute(db)
    .await
    .unwrap();
    sqlx::query("INSERT INTO collections (team_id, slug, name, access) VALUES ($1, 'payments-api', 'Payments API', 'public')")
        .bind(team)
        .execute(db)
        .await
        .unwrap();
    sqlx::query(
        "INSERT INTO outbox_emails (to_addr, subject, text_body, next_attempt_at) \
         VALUES ('ada@example.com', 'Hi', 'Hello', $1)",
    )
    .bind(now)
    .execute(db)
    .await
    .unwrap();
    sqlx::query(
        "INSERT INTO stripe_events (id, type, received_at) VALUES ('evt_1', 'invoice.paid', $1)",
    )
    .bind(now)
    .execute(db)
    .await
    .unwrap();
    sqlx::query("INSERT INTO settings (key, value) VALUES ('first_operator', 'ada@example.com')")
        .execute(db)
        .await
        .unwrap();

    all.run(&pool).await.unwrap();

    // Existing rows take the defaults.
    let (product_emails, suspended): (bool, Option<DateTime<Utc>>) =
        sqlx::query_as("SELECT product_emails, suspended_at FROM users")
            .fetch_one(db)
            .await
            .unwrap();
    assert!(product_emails);
    assert!(suspended.is_none());
    let (plan, override_plan): (String, Option<String>) =
        sqlx::query_as("SELECT plan, override_plan FROM teams")
            .fetch_one(db)
            .await
            .unwrap();
    assert_eq!((plan.as_str(), override_plan), ("team", None));
    let disabled: Option<DateTime<Utc>> =
        sqlx::query_scalar("SELECT link_disabled_at FROM collections")
            .fetch_one(db)
            .await
            .unwrap();
    assert!(disabled.is_none());
    let (kind, broadcast): (String, Option<Uuid>) =
        sqlx::query_as("SELECT kind, broadcast_id FROM outbox_emails")
            .fetch_one(db)
            .await
            .unwrap();
    assert_eq!((kind.as_str(), broadcast), ("transactional", None));
    let (created, failed, recipient): (Option<DateTime<Utc>>, Option<DateTime<Utc>>, Option<Uuid>) =
        sqlx::query_as("SELECT created_at, failed_at, user_id FROM outbox_emails")
            .fetch_one(db)
            .await
            .unwrap();
    assert_eq!(
        (created, failed, recipient),
        (None, None, None),
        "existing rows"
    );
    let (team_id, summary): (Option<Uuid>, Option<String>) =
        sqlx::query_as("SELECT team_id, summary FROM stripe_events")
            .fetch_one(db)
            .await
            .unwrap();
    assert_eq!((team_id, summary), (None, None));
    let settings: i64 = sqlx::query_scalar("SELECT count(*) FROM server_settings")
        .fetch_one(db)
        .await
        .unwrap();
    assert_eq!(settings, 0, "the server seeds the row from env on start");

    // server_settings holds one row at most, with the documented defaults.
    sqlx::query("INSERT INTO server_settings DEFAULT VALUES")
        .execute(db)
        .await
        .unwrap();
    let row: (String, Vec<String>, i32, bool, i32, i32, bool) = sqlx::query_as(
        "SELECT signup_mode, signup_domains, free_member_limit, free_restricted, grace_days, \
                banner_version, publishing_paused FROM server_settings",
    )
    .fetch_one(db)
    .await
    .unwrap();
    assert_eq!(row, ("domains".into(), vec![], 3, false, 14, 0, false));
    assert!(
        sqlx::query("INSERT INTO server_settings DEFAULT VALUES")
            .execute(db)
            .await
            .is_err()
    );
    assert!(
        sqlx::query("INSERT INTO server_settings (singleton) VALUES (false)")
            .execute(db)
            .await
            .is_err()
    );

    // One outbox row per broadcast recipient.
    let broadcast: Uuid = sqlx::query_scalar(
        "INSERT INTO broadcasts (subject, body_md, audience, created_by, created_at, updated_at) \
         VALUES ('News', 'Hi', 'everyone', $1, $2, $2) RETURNING id",
    )
    .bind(user)
    .bind(now)
    .fetch_one(db)
    .await
    .unwrap();
    let insert = |to: &'static str| {
        sqlx::query(
            "INSERT INTO outbox_emails (to_addr, subject, text_body, next_attempt_at, kind, broadcast_id, user_id) \
             VALUES ($3, 'News', 'Hi', $1, 'broadcast', $2, $4)",
        )
        .bind(now)
        .bind(broadcast)
        .bind(to)
        .bind(user)
        .execute(db)
    };
    insert("ada@example.com").await.unwrap();
    // Keyed on the user, not the address: a changed email is still one row.
    assert!(insert("ada.new@example.com").await.is_err());
    // New rows get created_at from the column default.
    let created: Option<DateTime<Utc>> =
        sqlx::query_scalar("SELECT created_at FROM outbox_emails WHERE broadcast_id IS NOT NULL")
            .fetch_one(db)
            .await
            .unwrap();
    assert!(created.is_some());

    // A user delete still cascades through the new tables.
    sqlx::query("INSERT INTO admins (user_id, permissions, added_at) VALUES ($1, '{users}', $2)")
        .bind(user)
        .bind(now)
        .execute(db)
        .await
        .unwrap();
    sqlx::query(
        "INSERT INTO plan_history (team_id, at, source, plan) VALUES ($1, $2, 'stripe', 'team')",
    )
    .bind(team)
    .bind(now)
    .execute(db)
    .await
    .unwrap();
    sqlx::query("DELETE FROM teams").execute(db).await.unwrap();
    sqlx::query("DELETE FROM users").execute(db).await.unwrap();
    let left: i64 = sqlx::query_scalar(
        "SELECT (SELECT count(*) FROM admins) + (SELECT count(*) FROM plan_history)",
    )
    .fetch_one(db)
    .await
    .unwrap();
    assert_eq!(left, 0);
}

#[sqlx::test]
async fn deleted_admin_still_named_in_log(pool: PgPool) {
    let app = owner_app(pool).await;
    let ada = app.sign_in(OWNER).await;
    let rosa = app.sign_in(ROSA).await;
    let rosa_id = user_id(&app, &rosa).await;
    let jane = app.sign_in("jane@example.com").await;
    add_admin(&app, &ada, ROSA, json!(["admins"])).await;
    app.clock.advance(Duration::seconds(1));
    let (status, _) = add_admin(&app, &rosa, "jane@example.com", json!([])).await;
    assert_eq!(status, StatusCode::CREATED);
    let _ = jane;
    sqlx::query("DELETE FROM users WHERE id = $1::uuid")
        .bind(&rosa_id)
        .execute(&app.state.db)
        .await
        .unwrap();

    let body = list_by(&app, &ada, "").await;
    let by_rosa = &body["items"][0];
    assert_eq!(by_rosa["target_label"], "jane@example.com");
    assert_eq!(by_rosa["admin"]["user_id"], rosa_id.as_str());
    assert_eq!(by_rosa["admin"]["email"], ROSA);
    assert_eq!(by_rosa["admin"]["name"], Value::Null);
    let (_, _, csv) = send(
        &app,
        "GET",
        "/v1/admin/activity.csv",
        None,
        Some(&ada),
        None,
    )
    .await;
    let line = csv.split("\r\n").nth(1).unwrap();
    assert!(line.contains(&format!(",{ROSA},admin.add,")), "{line}");
}

#[sqlx::test]
async fn cursor_walks_rows_sharing_a_time(pool: PgPool) {
    let app = owner_app(pool).await;
    let ada = app.sign_in(OWNER).await;
    let ada_id: Uuid = user_id(&app, &ada).await.parse().unwrap();
    let at: DateTime<Utc> = NOW.parse().unwrap();
    for _ in 0..5 {
        seed_events(&app, ada_id, &["admin.add"], at).await;
    }
    seed_events(&app, ada_id, &["admin.remove"], at - Duration::seconds(1)).await;
    let mut ids = Vec::new();
    let mut q = "?limit=1".to_owned();
    loop {
        let body = list_by(&app, &ada, &q).await;
        let items = body["items"].as_array().unwrap();
        assert_eq!(items.len(), 1, "{body}");
        ids.push(items[0]["id"].as_str().unwrap().to_owned());
        match body["next_cursor"].as_str() {
            Some(c) => q = format!("?limit=1&cursor={c}"),
            None => break,
        }
    }
    let all = list_by(&app, &ada, "").await;
    let want: Vec<String> = all["items"]
        .as_array()
        .unwrap()
        .iter()
        .map(|i| i["id"].as_str().unwrap().to_owned())
        .collect();
    assert_eq!(want.len(), 6);
    assert_eq!(ids, want, "every row once, in order");
}

#[sqlx::test]
async fn email_group_is_anchored(pool: PgPool) {
    let app = owner_app(pool).await;
    let ada = app.sign_in(OWNER).await;
    let ada_id: Uuid = user_id(&app, &ada).await.parse().unwrap();
    let at = NOW.parse::<DateTime<Utc>>().unwrap() - Duration::minutes(1);
    seed_events(
        &app,
        ada_id,
        &[
            "broadcast.send",
            "user.email",
            "team.email_owners",
            "settings.emailing",
            "user.email_change",
            "admin.broadcast.x",
        ],
        at,
    )
    .await;
    let body = list_by(&app, &ada, "?action_group=email").await;
    let actions: Vec<&str> = body["items"]
        .as_array()
        .unwrap()
        .iter()
        .map(|i| i["action"].as_str().unwrap())
        .collect();
    assert_eq!(
        actions,
        ["team.email_owners", "user.email", "broadcast.send"]
    );
}

#[sqlx::test]
async fn admin_bad_path_uuid_is_api_400(pool: PgPool) {
    let app = owner_app(pool).await;
    let ada = app.sign_in(OWNER).await;
    for (method, body) in [
        ("PATCH", Some(json!({"permissions": []}))),
        ("DELETE", None),
    ] {
        let (status, out) = json_req(&app, method, "/v1/admin/admins/not-a-uuid", body, &ada).await;
        assert_eq!(status, StatusCode::BAD_REQUEST, "{method}");
        assert_eq!(out["error"], "invalid_request", "{method}: {out}");
    }
}
