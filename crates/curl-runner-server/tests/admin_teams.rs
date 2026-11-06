#![cfg(feature = "db-tests")]

mod common;

use std::sync::Arc;
use std::sync::atomic::Ordering;

use axum::body::Body;
use axum::http::{Method, Request, StatusCode, header};
use chrono::{DateTime, Duration, Utc};
use common::{BILLING_ENV, FakeBilling, NOW, TestApp, test_app_billing_with, test_config};
use curl_runner_cloud_proto::{BundleFile, bundle_hash};
use curl_runner_server::billing::sync::tick;
use curl_runner_server::clock::Clock;
use http_body_util::BodyExt;
use serde_json::{Value, json};
use sqlx::PgPool;
use tower::ServiceExt;
use uuid::Uuid;

/// A server owner (every admin permission).
const ROOT: &str = "root@example.com";
/// The owner of team `acme`.
const ADA: &str = "ada@example.com";
const TEAM: &str = "/v1/admin/teams/acme";

struct Ctx {
    app: TestApp,
    fake: Arc<FakeBilling>,
    root: String,
    ada: String,
    team_id: Uuid,
}

/// Billing on, `ROOT` an owner, and `acme` (Free, no Stripe) owned by `ADA`.
async fn setup(pool: PgPool) -> Ctx {
    let mut env = BILLING_ENV.to_vec();
    env.push(("OPERATOR_EMAILS", ROOT));
    let (app, fake) = test_app_billing_with(pool, test_config(&env)).await;
    let root = app.sign_in(ROOT).await;
    let ada = app.sign_in(ADA).await;
    app.create_team(&ada, "acme").await;
    let team_id = team_id(&app, "acme").await;
    fake.subscription.lock().unwrap().team_id = Some(team_id);
    Ctx {
        app,
        fake,
        root,
        ada,
        team_id,
    }
}

async fn team_id(app: &TestApp, slug: &str) -> Uuid {
    sqlx::query_scalar("SELECT id FROM teams WHERE slug = $1")
        .bind(slug)
        .fetch_one(&app.state.db)
        .await
        .unwrap()
}

fn now() -> DateTime<Utc> {
    NOW.parse().unwrap()
}

async fn req(
    app: &TestApp,
    method: &str,
    path: &str,
    body: Option<Value>,
    bearer: Option<&str>,
) -> (StatusCode, Value) {
    app.request(method.parse::<Method>().unwrap(), path, body, bearer)
        .await
}

async fn sql(app: &TestApp, query: &str) {
    sqlx::query(query).execute(&app.state.db).await.unwrap();
}

/// `acme` paying through Stripe: `sub_1` of `cus_1`, monthly, active.
async fn pay(c: &Ctx) {
    sqlx::query(
        "UPDATE teams SET plan = 'team', stripe_customer_id = 'cus_1', \
         stripe_subscription_id = 'sub_1', billing_status = 'active', \
         billing_interval = 'monthly', seats_synced = 1, current_period_end = $2 \
         WHERE id = $1",
    )
    .bind(c.team_id)
    .bind(now() + Duration::days(20))
    .execute(&c.app.state.db)
    .await
    .unwrap();
}

/// `acme`'s plan as `/v1/me` reports it to its owner.
async fn me_plan(c: &Ctx) -> Value {
    let (status, me) = req(&c.app, "GET", "/v1/me", None, Some(&c.ada)).await;
    assert_eq!(status, StatusCode::OK, "{me}");
    me["teams"][0]["plan"].clone()
}

async fn summary(c: &Ctx) -> Value {
    let (status, body) = req(&c.app, "GET", "/v1/teams/acme/billing", None, Some(&c.ada)).await;
    assert_eq!(status, StatusCode::OK, "{body}");
    body
}

async fn grant(c: &Ctx, body: Value) -> (StatusCode, Value) {
    req(
        &c.app,
        "PUT",
        &format!("{TEAM}/override"),
        Some(body),
        Some(&c.root),
    )
    .await
}

async fn mails(app: &TestApp, subject_like: &str) -> Vec<String> {
    sqlx::query_scalar("SELECT to_addr FROM outbox_emails WHERE subject LIKE $1 ORDER BY to_addr")
        .bind(subject_like)
        .fetch_all(&app.state.db)
        .await
        .unwrap()
}

async fn actions(app: &TestApp) -> Vec<String> {
    sqlx::query_scalar("SELECT action FROM admin_events ORDER BY at, action")
        .fetch_all(&app.state.db)
        .await
        .unwrap()
}

/// `(source, plan)` of each `plan_history` row of `acme`, oldest first.
async fn history(c: &Ctx) -> Vec<(String, String)> {
    sqlx::query_as("SELECT source, plan FROM plan_history WHERE team_id = $1 ORDER BY at, ctid")
        .bind(c.team_id)
        .fetch_all(&c.app.state.db)
        .await
        .unwrap()
}

/// An admin (not an owner) with `perms`, signed in.
async fn admin_with(c: &Ctx, email: &str, perms: &[&str]) -> String {
    let session = c.app.sign_in(email).await;
    sqlx::query(
        "INSERT INTO admins (user_id, permissions, added_at) \
         SELECT id, $2, now() FROM users WHERE email = $1",
    )
    .bind(email)
    .bind(perms.iter().map(|p| (*p).to_owned()).collect::<Vec<_>>())
    .execute(&c.app.state.db)
    .await
    .unwrap();
    session
}

async fn publish(c: &Ctx, bearer: &str, text: &str) -> (StatusCode, Value) {
    let files = vec![
        BundleFile {
            path: "curl-runner.yaml".into(),
            text: "name: payments".into(),
        },
        BundleFile {
            path: "charge.yaml".into(),
            text: text.into(),
        },
    ];
    let body = json!({"hash": bundle_hash(&files), "files": files, "inputs": []});
    req(
        &c.app,
        "PUT",
        "/v1/teams/acme/collections/payments-api/versions",
        Some(body),
        Some(bearer),
    )
    .await
}

/// Publish `payments-api`, make it public and return its share token.
async fn shared_collection(c: &Ctx) -> String {
    let (status, body) = publish(c, &c.ada, "request:\n  url: https://x.example.com/1").await;
    assert_eq!(status, StatusCode::CREATED, "{body}");
    let share = "/v1/teams/acme/collections/payments-api/share";
    let (status, body) = req(
        &c.app,
        "PATCH",
        share,
        Some(json!({"access": "public"})),
        Some(&c.ada),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{body}");
    body["link"]
        .as_str()
        .unwrap()
        .split("/s/")
        .nth(1)
        .unwrap()
        .to_owned()
}

#[sqlx::test]
async fn override_team_beats_stripe_free(pool: PgPool) {
    let c = setup(pool).await;
    // Paid once; the subscription is gone, so Stripe says Free.
    sql(
        &c.app,
        "UPDATE teams SET stripe_customer_id = 'cus_1', billing_status = 'canceled'",
    )
    .await;
    assert_eq!(me_plan(&c).await, "free");
    let until = now() + Duration::days(30);
    let (status, body) = grant(
        &c,
        json!({"plan": "team", "until": until, "reason": "Design partner", "notify": true}),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(body["plan"], "team");
    assert_eq!(body["plan_source"], "override");
    assert_eq!(body["override"]["plan"], "team");
    assert_eq!(body["override"]["reason"], "Design partner");
    assert_eq!(body["override"]["by"], ROOT);

    assert_eq!(me_plan(&c).await, "team");
    let (_, team) = req(&c.app, "GET", "/v1/teams/acme", None, Some(&c.ada)).await;
    assert_eq!(team["plan"], "team");
    // Free allows 3 members with pending invites; Team has no cap.
    for n in 0..4 {
        let (status, body) = req(
            &c.app,
            "POST",
            "/v1/teams/acme/invites",
            Some(json!({"email": format!("p{n}@example.com"), "role": "member"})),
            Some(&c.ada),
        )
        .await;
        assert_eq!(status, StatusCode::CREATED, "{n}: {body}");
    }
    assert_eq!(
        mails(&c.app, "curl-runner gave acme the Team plan").await,
        [ADA]
    );
    assert_eq!(actions(&c.app).await, ["team.override.grant"]);
}

#[sqlx::test]
async fn override_free_beats_stripe_team(pool: PgPool) {
    let c = setup(pool).await;
    pay(&c).await;
    let (status, body) = publish(&c, &c.ada, "request:\n  url: https://x.example.com/1").await;
    assert_eq!(status, StatusCode::CREATED, "{body}");
    assert_eq!(me_plan(&c).await, "team");

    let (status, body) = grant(
        &c,
        json!({"plan": "free", "reason": "Abuse review", "notify": false}),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(body["override"]["until"], Value::Null);
    // Stripe still bills: the board warns about it.
    assert_eq!(body["stripe_conflict"], true);

    assert_eq!(me_plan(&c).await, "free");
    let s = summary(&c).await;
    assert_eq!(
        (&s["plan"], &s["plan_source"]),
        (&json!("free"), &json!("override"))
    );
    let (status, body) = req(
        &c.app,
        "PATCH",
        "/v1/teams/acme/collections/payments-api/share",
        Some(json!({"access": "restricted"})),
        Some(&c.ada),
    )
    .await;
    assert_eq!(status, StatusCode::PAYMENT_REQUIRED, "{body}");
    // The stored Stripe state is untouched.
    let plan: String = sqlx::query_scalar("SELECT plan FROM teams WHERE id = $1")
        .bind(c.team_id)
        .fetch_one(&c.app.state.db)
        .await
        .unwrap();
    assert_eq!(plan, "team");
    assert!(mails(&c.app, "%plan%").await.is_empty());
}

#[sqlx::test]
async fn override_expiry_falls_back_to_stripe(pool: PgPool) {
    let c = setup(pool).await;
    pay(&c).await;
    let until = now() + Duration::days(20);
    let (status, body) = grant(
        &c,
        json!({"plan": "free", "until": until, "reason": "Trial pause", "notify": false}),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(me_plan(&c).await, "free");

    c.app.clock.advance(Duration::days(21));
    // Past `until` the override no longer counts, even before the worker runs.
    assert_eq!(me_plan(&c).await, "team");
    tick(&c.app.state, c.app.clock.now()).await;

    assert_eq!(me_plan(&c).await, "team");
    let s = summary(&c).await;
    assert_eq!(s["plan_source"], "stripe");
    assert_eq!(s["override_until"], Value::Null);
    let cleared: (Option<String>, Option<DateTime<Utc>>, Option<String>) = sqlx::query_as(
        "SELECT override_plan, override_until, stripe_subscription_id FROM teams WHERE id = $1",
    )
    .bind(c.team_id)
    .fetch_one(&c.app.state.db)
    .await
    .unwrap();
    assert_eq!(cleared, (None, None, Some("sub_1".into())));
    assert_eq!(
        history(&c).await.last().unwrap(),
        &("expired".to_owned(), "team".to_owned())
    );
    assert!(mails(&c.app, "%Free plan%").await.is_empty());
}

#[sqlx::test]
async fn override_expiry_without_stripe_ends_on_the_column_plan(pool: PgPool) {
    let c = setup(pool).await;
    let until = now() + Duration::days(3);
    let (status, _) = grant(
        &c,
        json!({"plan": "team", "until": until, "reason": "Pilot", "notify": false}),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    c.app.clock.advance(Duration::days(4));
    tick(&c.app.state, c.app.clock.now()).await;
    assert_eq!(me_plan(&c).await, "free");
    assert_eq!(
        history(&c).await.last().unwrap(),
        &("expired".to_owned(), "free".to_owned())
    );
    // Team to Free: the owners hear about it.
    assert_eq!(mails(&c.app, "acme moved to the Free plan").await, [ADA]);
}

#[sqlx::test]
async fn override_reminder_14_days_once(pool: PgPool) {
    let c = setup(pool).await;
    let until = now() + Duration::days(30);
    let (status, _) = grant(
        &c,
        json!({"plan": "team", "until": until, "reason": "Pilot", "notify": false}),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let ending = "%granted plan ends%";

    tick(&c.app.state, c.app.clock.now()).await;
    assert!(mails(&c.app, ending).await.is_empty());

    // 13 days left.
    c.app.clock.advance(Duration::days(17));
    tick(&c.app.state, c.app.clock.now()).await;
    assert_eq!(mails(&c.app, ending).await, [ADA]);
    for _ in 0..2 {
        c.app.clock.advance(Duration::days(1));
        tick(&c.app.state, c.app.clock.now()).await;
    }
    assert_eq!(mails(&c.app, ending).await, [ADA]);

    // A new end date is a new reminder.
    let (status, _) = grant(
        &c,
        json!({"plan": "team", "until": until + Duration::days(60), "reason": "Pilot", "notify": false}),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    tick(&c.app.state, c.app.clock.now()).await;
    assert_eq!(mails(&c.app, ending).await, [ADA]);
    // Into the new end date's 14-day window: the second reminder, once.
    c.app.clock.advance(Duration::days(58));
    tick(&c.app.state, c.app.clock.now()).await;
    assert_eq!(mails(&c.app, ending).await, [ADA, ADA]);
    c.app.clock.advance(Duration::days(1));
    tick(&c.app.state, c.app.clock.now()).await;
    assert_eq!(mails(&c.app, ending).await, [ADA, ADA]);

    // No end date: never reminded.
    let (status, _) = grant(
        &c,
        json!({"plan": "team", "reason": "Forever", "notify": false}),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    c.app.clock.advance(Duration::days(365));
    tick(&c.app.state, c.app.clock.now()).await;
    assert_eq!(mails(&c.app, ending).await, [ADA, ADA]);
}

#[sqlx::test]
async fn notified_edit_inside_the_window_is_not_reminded_again(pool: PgPool) {
    let c = setup(pool).await;
    let until = now() + Duration::days(10);
    let body = |notify: bool, reason: &str| json!({"plan": "team", "until": until, "reason": reason, "notify": notify});
    let (status, _) = grant(&c, body(false, "Pilot")).await;
    assert_eq!(status, StatusCode::OK);
    // Same end date, owners told now: PlanGranted, and no OverrideEnding after it.
    let (status, res) = grant(&c, body(true, "Pilot, extended scope")).await;
    assert_eq!(status, StatusCode::OK, "{res}");
    assert!(res["override"]["reminded_at"].is_string(), "{res}");
    tick(&c.app.state, c.app.clock.now()).await;
    assert_eq!(
        mails(&c.app, "curl-runner gave acme the Team plan").await,
        [ADA]
    );
    assert!(mails(&c.app, "%granted plan ends%").await.is_empty());
}

#[sqlx::test]
async fn regrant_over_an_uncleared_ended_override_records_the_expiry(pool: PgPool) {
    let c = setup(pool).await;
    let until = now() + Duration::days(2);
    grant(
        &c,
        json!({"plan": "team", "until": until, "reason": "Pilot", "notify": false}),
    )
    .await;
    c.app.clock.advance(Duration::days(3));
    // The worker has not run: the ended override is still on the row.
    let (status, _) = grant(
        &c,
        json!({"plan": "team", "reason": "Signed", "notify": false}),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let got: Vec<(String, String)> = history(&c).await;
    let got: Vec<(&str, &str)> = got.iter().map(|(s, p)| (s.as_str(), p.as_str())).collect();
    assert_eq!(
        got,
        [
            ("default", "free"),
            ("override", "team"),
            ("expired", "free"),
            ("override", "team")
        ]
    );
    // A fresh grant, not an edit of the ended one.
    assert_eq!(
        actions(&c.app).await,
        ["team.override.grant", "team.override.grant"]
    );
}

#[sqlx::test]
async fn stripe_cancel_under_an_override_keeps_the_team(pool: PgPool) {
    let c = setup(pool).await;
    pay(&c).await;
    let (status, _) = grant(
        &c,
        json!({"plan": "team", "reason": "Partner", "notify": false}),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    c.fake.subscription.lock().unwrap().status = "canceled".into();
    let (status, body) = req(
        &c.app,
        "POST",
        &format!("{TEAM}/stripe-sync"),
        None,
        Some(&c.root),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(me_plan(&c).await, "team");
    let s = summary(&c).await;
    assert_eq!(
        (&s["plan"], &s["plan_source"]),
        (&json!("team"), &json!("override"))
    );
    assert_eq!(s["has_subscription"], false);
    assert!(mails(&c.app, "%moved to the Free plan%").await.is_empty());
    let h = history(&c).await;
    let tail: Vec<(&str, &str)> = h[h.len() - 2..]
        .iter()
        .map(|(s, p)| (s.as_str(), p.as_str()))
        .collect();
    assert_eq!(tail, [("override", "team"), ("stripe", "free")]);
}

#[sqlx::test]
async fn payment_failed_is_not_sent_under_a_free_override(pool: PgPool) {
    let c = setup(pool).await;
    pay(&c).await;
    let (status, _) = grant(
        &c,
        json!({"plan": "free", "reason": "Review", "notify": false}),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    c.fake.subscription.lock().unwrap().status = "past_due".into();
    let (status, body) = req(
        &c.app,
        "POST",
        &format!("{TEAM}/stripe-sync"),
        None,
        Some(&c.root),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert!(mails(&c.app, "We couldn't take%").await.is_empty());
    // The clock still starts, so the grace period runs as usual.
    let since: Option<DateTime<Utc>> =
        sqlx::query_scalar("SELECT past_due_since FROM teams WHERE id = $1")
            .bind(c.team_id)
            .fetch_one(&c.app.state.db)
            .await
            .unwrap();
    assert!(since.is_some());
}

#[sqlx::test]
async fn summary_has_subscription_under_an_override(pool: PgPool) {
    let c = setup(pool).await;
    assert_eq!(summary(&c).await["has_subscription"], false);
    pay(&c).await;
    assert_eq!(summary(&c).await["has_subscription"], true);
    let (status, _) = grant(
        &c,
        json!({"plan": "free", "reason": "Review", "notify": false}),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let s = summary(&c).await;
    assert_eq!(s["plan_source"], "override");
    assert_eq!(s["has_subscription"], true);
    // A canceled subscription id left on the row is not one.
    sql(&c.app, "UPDATE teams SET billing_status = 'canceled'").await;
    assert_eq!(summary(&c).await["has_subscription"], false);
}

#[sqlx::test]
async fn members_see_the_suspension_without_admin_notes(pool: PgPool) {
    let c = setup(pool).await;
    let (_, team) = req(&c.app, "GET", "/v1/teams/acme", None, Some(&c.ada)).await;
    assert_eq!(team["suspended"], false);
    assert_eq!(team["suspended_at"], Value::Null);
    let (_, me) = req(&c.app, "GET", "/v1/me", None, Some(&c.ada)).await;
    assert_eq!(me["teams"][0]["suspended"], false);
    let (status, _) = req(
        &c.app,
        "POST",
        &format!("{TEAM}/suspend"),
        Some(json!({"reason": "Secret admin note", "notify": false})),
        Some(&c.root),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let (status, team) = req(&c.app, "GET", "/v1/teams/acme", None, Some(&c.ada)).await;
    assert_eq!(status, StatusCode::OK, "{team}");
    assert_eq!(team["suspended"], true);
    assert_eq!(
        team["suspended_at"]
            .as_str()
            .unwrap()
            .parse::<DateTime<Utc>>()
            .unwrap(),
        now()
    );
    let (_, me) = req(&c.app, "GET", "/v1/me", None, Some(&c.ada)).await;
    assert_eq!(me["teams"][0]["suspended"], true);
    for body in [team.to_string(), me.to_string()] {
        assert!(!body.contains("Secret admin note"), "{body}");
        assert!(!body.contains(ROOT), "{body}");
        assert!(!body.contains("suspended_reason"), "{body}");
        assert!(!body.contains("suspended_by"), "{body}");
    }
}

#[sqlx::test]
async fn summary_plan_source_override(pool: PgPool) {
    let c = setup(pool).await;
    pay(&c).await;
    assert_eq!(summary(&c).await["plan_source"], "stripe");
    let until = now() + Duration::days(90);
    let (status, _) = grant(
        &c,
        json!({"plan": "team", "until": until, "reason": "Partner", "notify": false}),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let s = summary(&c).await;
    assert_eq!(s["plan_source"], "override");
    assert_eq!(s["plan"], "team");
    assert_eq!(
        s["override_until"]
            .as_str()
            .unwrap()
            .parse::<DateTime<Utc>>()
            .unwrap(),
        until
    );

    let (status, body) = req(
        &c.app,
        "DELETE",
        &format!("{TEAM}/override"),
        None,
        Some(&c.root),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(body["plan_source"], "stripe");
    let s = summary(&c).await;
    assert_eq!(s["plan_source"], "stripe");
    assert_eq!(s["override_until"], Value::Null);
    let (status, _) = req(
        &c.app,
        "DELETE",
        &format!("{TEAM}/override"),
        None,
        Some(&c.root),
    )
    .await;
    assert_eq!(status, StatusCode::CONFLICT);
    assert_eq!(
        actions(&c.app).await,
        ["team.override.grant", "team.override.remove"]
    );
}

#[sqlx::test]
async fn override_input_and_permissions(pool: PgPool) {
    let c = setup(pool).await;
    let past = now() - Duration::days(1);
    let (status, body) = grant(
        &c,
        json!({"plan": "team", "until": past, "reason": "x", "notify": false}),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST, "{body}");
    let (status, _) = req(
        &c.app,
        "PUT",
        "/v1/admin/teams/nope/override",
        Some(json!({"plan": "team", "reason": "x", "notify": false})),
        Some(&c.root),
    )
    .await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    let rosa = admin_with(&c, "rosa@example.com", &["teams"]).await;
    let (status, body) = req(
        &c.app,
        "PUT",
        &format!("{TEAM}/override"),
        Some(json!({"plan": "team", "reason": "x", "notify": false})),
        Some(&rosa),
    )
    .await;
    assert_eq!(status, StatusCode::FORBIDDEN, "{body}");
    assert!(actions(&c.app).await.is_empty());
}

#[sqlx::test]
async fn suspended_team_publish_403_and_share_404(pool: PgPool) {
    let c = setup(pool).await;
    let t = shared_collection(&c).await;
    let bob = c.app.sign_in("bob@example.com").await;
    let (status, _) = req(&c.app, "GET", &format!("/v1/shared/{t}"), None, Some(&bob)).await;
    assert_eq!(status, StatusCode::OK);
    let (_, mine) = req(&c.app, "GET", "/v1/me/shared", None, Some(&bob)).await;
    assert_eq!(mine.as_array().unwrap().len(), 1);
    let (status, token) = req(
        &c.app,
        "POST",
        "/v1/teams/acme/tokens",
        Some(json!({"name": "ci"})),
        Some(&c.ada),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED, "{token}");
    let ci = token["token"].as_str().unwrap().to_owned();

    let (status, body) = req(
        &c.app,
        "POST",
        &format!("{TEAM}/suspend"),
        Some(json!({"reason": "Phishing pages", "notify": true})),
        Some(&c.root),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{body}");

    for bearer in [&c.ada, &ci] {
        let (status, body) = publish(&c, bearer, "request:\n  url: https://x.example.com/2").await;
        assert_eq!(status, StatusCode::FORBIDDEN, "{body}");
        assert_eq!(body["error"], "team_suspended");
    }
    for suffix in ["", "/snapshot", "/versions/latest"] {
        for bearer in [None, Some(bob.as_str()), Some(c.ada.as_str())] {
            let (status, body) = req(
                &c.app,
                "GET",
                &format!("/v1/shared/{t}{suffix}"),
                None,
                bearer,
            )
            .await;
            assert_eq!(status, StatusCode::NOT_FOUND, "{suffix} {bearer:?}");
            assert_eq!(body["error"], "not_found");
        }
    }
    let (_, mine) = req(&c.app, "GET", "/v1/me/shared", None, Some(&bob)).await;
    assert_eq!(mine, json!([]));
    // The admin view of bob agrees.
    let bob_id: Uuid = sqlx::query_scalar("SELECT id FROM users WHERE email = 'bob@example.com'")
        .fetch_one(&c.app.state.db)
        .await
        .unwrap();
    let (_, detail) = req(
        &c.app,
        "GET",
        &format!("/v1/admin/users/{bob_id}"),
        None,
        Some(&c.root),
    )
    .await;
    assert_eq!(detail["shared"], json!([]));

    // Members still read their own team.
    for path in [
        "/v1/teams/acme",
        "/v1/teams/acme/collections",
        "/v1/teams/acme/collections/payments-api/versions",
        "/v1/teams/acme/billing",
    ] {
        let (status, body) = req(&c.app, "GET", path, None, Some(&c.ada)).await;
        assert_eq!(status, StatusCode::OK, "{path}: {body}");
    }
    assert_eq!(
        mails(&c.app, "acme is suspended on curl-runner").await,
        [ADA]
    );
    let (status, _) = req(
        &c.app,
        "POST",
        &format!("{TEAM}/suspend"),
        Some(json!({"reason": "again", "notify": false})),
        Some(&c.root),
    )
    .await;
    assert_eq!(status, StatusCode::CONFLICT);
    let (_, d) = req(&c.app, "GET", TEAM, None, Some(&c.root)).await;
    assert_eq!(d["suspension"]["reason"], "Phishing pages");
    assert_eq!(d["suspension"]["by"], ROOT);
    assert_eq!(actions(&c.app).await, ["team.suspend"]);
}

#[sqlx::test]
async fn unsuspend_restores(pool: PgPool) {
    let c = setup(pool).await;
    let t = shared_collection(&c).await;
    let bob = c.app.sign_in("bob@example.com").await;
    req(&c.app, "GET", &format!("/v1/shared/{t}"), None, Some(&bob)).await;
    let (status, _) = req(
        &c.app,
        "POST",
        &format!("{TEAM}/suspend"),
        Some(json!({"reason": "Review", "notify": false})),
        Some(&c.root),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert!(mails(&c.app, "%suspended%").await.is_empty());
    let (status, body) = req(
        &c.app,
        "POST",
        &format!("{TEAM}/unsuspend"),
        None,
        Some(&c.root),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{body}");

    let (status, body) = publish(&c, &c.ada, "request:\n  url: https://x.example.com/2").await;
    assert_eq!(status, StatusCode::CREATED, "{body}");
    let (status, _) = req(&c.app, "GET", &format!("/v1/shared/{t}"), None, None).await;
    assert_eq!(status, StatusCode::OK);
    let (_, mine) = req(&c.app, "GET", "/v1/me/shared", None, Some(&bob)).await;
    assert_eq!(mine.as_array().unwrap().len(), 1);
    let (_, d) = req(&c.app, "GET", TEAM, None, Some(&c.root)).await;
    assert_eq!(d["suspension"], Value::Null);
    let (status, _) = req(
        &c.app,
        "POST",
        &format!("{TEAM}/unsuspend"),
        None,
        Some(&c.root),
    )
    .await;
    assert_eq!(status, StatusCode::CONFLICT);
    assert_eq!(actions(&c.app).await, ["team.suspend", "team.unsuspend"]);
}

#[sqlx::test]
async fn plan_history_records_each_change(pool: PgPool) {
    let c = setup(pool).await;
    let step = || async { c.app.clock.advance(Duration::minutes(1)) };
    // Created on the default plan.
    step().await;
    let (status, _) = req(
        &c.app,
        "PUT",
        &format!("{TEAM}/plan"),
        Some(json!({"plan": "team"})),
        Some(&c.root),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    step().await;
    grant(
        &c,
        json!({"plan": "free", "reason": "Review", "notify": false}),
    )
    .await;
    step().await;
    let (status, _) = grant(
        &c,
        json!({"plan": "team", "reason": "Cleared", "notify": false}),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    step().await;
    req(
        &c.app,
        "DELETE",
        &format!("{TEAM}/override"),
        None,
        Some(&c.root),
    )
    .await;
    step().await;
    // Stripe: canceled, then active again.
    sql(&c.app, "UPDATE teams SET stripe_subscription_id = 'sub_1'").await;
    c.fake.subscription.lock().unwrap().status = "canceled".into();
    let (status, body) = req(
        &c.app,
        "POST",
        &format!("{TEAM}/stripe-sync"),
        None,
        Some(&c.root),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{body}");
    step().await;
    sql(&c.app, "UPDATE teams SET stripe_subscription_id = 'sub_1'").await;
    c.fake.subscription.lock().unwrap().status = "active".into();
    req(
        &c.app,
        "POST",
        &format!("{TEAM}/stripe-sync"),
        None,
        Some(&c.root),
    )
    .await;
    step().await;
    // A sync that changes nothing adds nothing.
    req(
        &c.app,
        "POST",
        &format!("{TEAM}/stripe-sync"),
        None,
        Some(&c.root),
    )
    .await;
    // Past due beyond the grace period.
    sqlx::query("UPDATE teams SET past_due_since = $1")
        .bind(c.app.clock.now() - Duration::days(15))
        .execute(&c.app.state.db)
        .await
        .unwrap();
    step().await;
    tick(&c.app.state, c.app.clock.now()).await;

    let got = history(&c).await;
    let want = [
        ("default", "free"),
        ("admin", "team"),
        ("override", "free"),
        ("override", "team"),
        ("override_removed", "team"),
        ("stripe", "free"),
        ("stripe", "team"),
        ("past_due", "free"),
    ];
    let got: Vec<(&str, &str)> = got.iter().map(|(s, p)| (s.as_str(), p.as_str())).collect();
    assert_eq!(got, want);

    let (status, d) = req(&c.app, "GET", TEAM, None, Some(&c.root)).await;
    assert_eq!(status, StatusCode::OK, "{d}");
    let rows = d["plan_history"].as_array().unwrap();
    assert_eq!(rows.len(), want.len());
    // Newest first, with the admin and reason where there is one.
    assert_eq!(rows[0]["source"], "past_due");
    assert_eq!(rows[4]["source"], "override");
    assert_eq!(rows[4]["reason"], "Cleared");
    assert_eq!(rows[4]["admin"], ROOT);
    assert_eq!(rows[7]["admin"], Value::Null);
    assert!(
        actions(&c.app)
            .await
            .contains(&"team.override.update".to_owned())
    );
}

#[sqlx::test]
async fn stripe_sync_reapplies(pool: PgPool) {
    let c = setup(pool).await;
    // The webhook was missed: the row still says past due on Free.
    sql(
        &c.app,
        "UPDATE teams SET stripe_subscription_id = 'sub_1', stripe_customer_id = 'cus_1', \
         billing_status = 'past_due'",
    )
    .await;
    c.fake.subscription.lock().unwrap().quantity = 3;
    let (status, body) = req(
        &c.app,
        "POST",
        &format!("{TEAM}/stripe-sync"),
        None,
        Some(&c.root),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(body["plan"], "team");
    assert_eq!(body["stripe"]["status"], "active");
    let row: (String, Option<String>, Option<i32>, Option<String>) = sqlx::query_as(
        "SELECT plan, billing_status, seats_synced, billing_interval FROM teams WHERE id = $1",
    )
    .bind(c.team_id)
    .fetch_one(&c.app.state.db)
    .await
    .unwrap();
    assert_eq!(
        row,
        (
            "team".into(),
            Some("active".into()),
            Some(3),
            Some("monthly".into())
        )
    );
    assert_eq!(actions(&c.app).await, ["team.stripe_sync"]);

    // Stripe down: 503, nothing logged.
    c.fake.fail.store(true, Ordering::SeqCst);
    let (status, _) = req(
        &c.app,
        "POST",
        &format!("{TEAM}/stripe-sync"),
        None,
        Some(&c.root),
    )
    .await;
    assert_eq!(status, StatusCode::SERVICE_UNAVAILABLE);
    c.fake.fail.store(false, Ordering::SeqCst);

    // Stripe no longer knows it: Free, and the subscription is dropped.
    c.fake.missing.store(true, Ordering::SeqCst);
    let (status, body) = req(
        &c.app,
        "POST",
        &format!("{TEAM}/stripe-sync"),
        None,
        Some(&c.root),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(body["plan"], "free");
    assert_eq!(body["stripe"], Value::Null);

    // No subscription to sync.
    let (status, _) = req(
        &c.app,
        "POST",
        &format!("{TEAM}/stripe-sync"),
        None,
        Some(&c.root),
    )
    .await;
    assert_eq!(status, StatusCode::CONFLICT);
    let rosa = admin_with(&c, "rosa@example.com", &["teams"]).await;
    let (status, _) = req(
        &c.app,
        "POST",
        &format!("{TEAM}/stripe-sync"),
        None,
        Some(&rosa),
    )
    .await;
    assert_eq!(status, StatusCode::FORBIDDEN);
    assert_eq!(
        actions(&c.app).await,
        ["team.stripe_sync", "team.stripe_sync"]
    );
}

#[sqlx::test]
async fn resend_invoice_calls_billing(pool: PgPool) {
    let c = setup(pool).await;
    let path = format!("{TEAM}/resend-invoice");
    let (status, _) = req(&c.app, "POST", &path, None, Some(&c.root)).await;
    assert_eq!(status, StatusCode::CONFLICT, "no customer yet");
    pay(&c).await;
    let (status, body) = req(&c.app, "POST", &path, None, Some(&c.root)).await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(body["invoice_id"], "in_1");
    assert_eq!(*c.fake.invoices_sent.lock().unwrap(), ["cus_1"]);
    c.fake.no_invoice.store(true, Ordering::SeqCst);
    let (status, _) = req(&c.app, "POST", &path, None, Some(&c.root)).await;
    assert_eq!(status, StatusCode::CONFLICT);
    let detail: Value =
        sqlx::query_scalar("SELECT detail FROM admin_events WHERE action = 'team.invoice.resend'")
            .fetch_one(&c.app.state.db)
            .await
            .unwrap();
    assert_eq!(detail["invoice_id"], "in_1");
    assert_eq!(actions(&c.app).await, ["team.invoice.resend"]);
}

#[sqlx::test]
async fn admin_delete_team_cancels_stripe(pool: PgPool) {
    let c = setup(pool).await;
    pay(&c).await;
    let rosa = admin_with(&c, "rosa@example.com", &["billing"]).await;
    let (status, _) = req(&c.app, "DELETE", TEAM, None, Some(&rosa)).await;
    assert_eq!(status, StatusCode::FORBIDDEN);

    // Stripe down: the team stays, so it is never left charging.
    c.fake.fail.store(true, Ordering::SeqCst);
    let (status, _) = req(&c.app, "DELETE", TEAM, None, Some(&c.root)).await;
    assert_eq!(status, StatusCode::SERVICE_UNAVAILABLE);
    assert!(c.fake.cancelled.lock().unwrap().is_empty());
    assert!(actions(&c.app).await.is_empty());
    c.fake.fail.store(false, Ordering::SeqCst);

    let (status, body) = req(&c.app, "DELETE", TEAM, None, Some(&c.root)).await;
    assert_eq!(status, StatusCode::NO_CONTENT, "{body}");
    assert_eq!(*c.fake.cancelled.lock().unwrap(), ["sub_1"]);
    let left: i64 = sqlx::query_scalar("SELECT count(*) FROM teams")
        .fetch_one(&c.app.state.db)
        .await
        .unwrap();
    assert_eq!(left, 0);
    let (label, id): (String, String) = sqlx::query_as(
        "SELECT target_label, target_id FROM admin_events WHERE action = 'team.delete'",
    )
    .fetch_one(&c.app.state.db)
    .await
    .unwrap();
    assert_eq!((label.as_str(), id), ("acme", c.team_id.to_string()));
    let (status, _) = req(&c.app, "GET", TEAM, None, Some(&c.root)).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
}

#[sqlx::test]
async fn list_filters_search_and_csv(pool: PgPool) {
    let c = setup(pool).await;
    pay(&c).await;
    for slug in ["beta", "gamma", "delta", "epsilon"] {
        c.app.clock.advance(Duration::seconds(1));
        c.app.create_team(&c.ada, slug).await;
    }
    let (status, _) = req(
        &c.app,
        "PUT",
        "/v1/admin/teams/beta/override",
        Some(json!({"plan": "team", "reason": "Partner", "notify": false})),
        Some(&c.root),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let (status, _) = req(
        &c.app,
        "POST",
        "/v1/admin/teams/delta/suspend",
        Some(json!({"reason": "Spam", "notify": false})),
        Some(&c.root),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    sqlx::query("UPDATE teams SET past_due_since = $1 WHERE slug = 'epsilon'")
        .bind(now())
        .execute(&c.app.state.db)
        .await
        .unwrap();

    let slugs = |v: &Value| -> Vec<String> {
        v["items"]
            .as_array()
            .unwrap()
            .iter()
            .map(|t| t["slug"].as_str().unwrap().to_owned())
            .collect()
    };
    let list = |q: &'static str| {
        let c = &c;
        async move {
            let (status, body) = req(
                &c.app,
                "GET",
                &format!("/v1/admin/teams{q}"),
                None,
                Some(&c.root),
            )
            .await;
            assert_eq!(status, StatusCode::OK, "{q}: {body}");
            body
        }
    };
    let all = list("").await;
    assert_eq!(all["total"], 5);
    // Newest first.
    assert_eq!(slugs(&all), ["epsilon", "delta", "gamma", "beta", "acme"]);
    assert_eq!(slugs(&list("?filter=stripe").await), ["acme"]);
    assert_eq!(slugs(&list("?filter=granted").await), ["beta"]);
    assert_eq!(slugs(&list("?filter=free").await), ["epsilon", "gamma"]);
    assert_eq!(slugs(&list("?filter=suspended").await), ["delta"]);
    assert_eq!(slugs(&list("?filter=past_due").await), ["epsilon"]);
    assert_eq!(slugs(&list("?q=gam").await), ["gamma"]);
    assert_eq!(list("?q=ada@").await["total"], 5);

    let acme = &all["items"][4];
    assert_eq!(acme["plan"], "team");
    assert_eq!(acme["plan_source"], "stripe");
    assert_eq!(acme["owner"]["email"], ADA);
    assert_eq!(acme["members"], 1);
    // One seat at 9.00 a month.
    assert_eq!(acme["mrr_cents"], 900);
    let beta = &all["items"][3];
    assert_eq!(
        (&beta["plan"], &beta["plan_source"]),
        (&json!("team"), &json!("override"))
    );
    assert_eq!(beta["mrr_cents"], 0);
    assert_eq!(all["items"][1]["suspended"], true);

    // Paging.
    let (_, page) = req(
        &c.app,
        "GET",
        "/v1/admin/teams?limit=2",
        None,
        Some(&c.root),
    )
    .await;
    assert_eq!(slugs(&page), ["epsilon", "delta"]);
    let cursor = page["next_cursor"].as_str().unwrap();
    let (_, page) = req(
        &c.app,
        "GET",
        &format!("/v1/admin/teams?limit=2&cursor={cursor}"),
        None,
        Some(&c.root),
    )
    .await;
    assert_eq!(slugs(&page), ["gamma", "beta"]);
    let (status, _) = req(
        &c.app,
        "GET",
        "/v1/admin/teams?filter=nope",
        None,
        Some(&c.root),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);

    let res = curl_runner_server::app(c.app.state.clone())
        .oneshot(
            Request::builder()
                .uri("/v1/admin/teams.csv?filter=stripe")
                .header(header::AUTHORIZATION, format!("Bearer {}", c.root))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    assert_eq!(
        res.headers()[header::CONTENT_DISPOSITION],
        "attachment; filename=\"teams-2026-11-04.csv\""
    );
    let body =
        String::from_utf8(res.into_body().collect().await.unwrap().to_bytes().to_vec()).unwrap();
    let lines: Vec<&str> = body.split("\r\n").filter(|l| !l.is_empty()).collect();
    assert_eq!(lines.len(), 2, "{body}");
    assert!(lines[0].starts_with("slug,name,owner"), "{body}");
    assert!(lines[1].starts_with("acme,acme,ada@example.com"), "{body}");

    // An override on top of a live subscription: still `stripe`, and not
    // `granted`, since the team still pays.
    let (status, _) = req(
        &c.app,
        "PUT",
        "/v1/admin/teams/acme/override",
        Some(json!({"plan": "team", "reason": "Partner", "notify": false})),
        Some(&c.root),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(slugs(&list("?filter=stripe").await), ["acme"]);
    assert_eq!(slugs(&list("?filter=granted").await), ["beta"]);
    // A Free override is no grant.
    let (status, _) = req(
        &c.app,
        "PUT",
        "/v1/admin/teams/gamma/override",
        Some(json!({"plan": "free", "reason": "Abuse", "notify": false})),
        Some(&c.root),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(slugs(&list("?filter=granted").await), ["beta"]);
}

#[sqlx::test]
async fn detail_shows_stripe_override_members_and_usage(pool: PgPool) {
    let c = setup(pool).await;
    pay(&c).await;
    sql(
        &c.app,
        "UPDATE teams SET billing_interval = 'annual', seats_synced = 3",
    )
    .await;
    let (status, _) = publish(&c, &c.ada, "request:\n  url: https://x.example.com/1").await;
    assert_eq!(status, StatusCode::CREATED);
    let (status, _) = req(
        &c.app,
        "POST",
        "/v1/teams/acme/invites",
        Some(json!({"email": "dana@example.com", "role": "member"})),
        Some(&c.ada),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED);

    let (status, d) = req(&c.app, "GET", TEAM, None, Some(&c.root)).await;
    assert_eq!(status, StatusCode::OK, "{d}");
    assert_eq!(d["team"]["slug"], "acme");
    assert_eq!(d["team"]["plan"], "team");
    assert_eq!(d["team"]["plan_source"], "stripe");
    assert_eq!(
        d["team"]["limits"],
        json!({"members": null, "restricted": true})
    );
    let stripe = &d["stripe"];
    assert_eq!(stripe["interval"], "annual");
    assert_eq!(stripe["seats"], 3);
    // Three seats at 90.00 a year.
    assert_eq!(stripe["amount_cents"], 27_000);
    assert_eq!(stripe["currency"], "usd");
    assert_eq!(stripe["subscription_id"], "sub_1");
    assert_eq!(stripe["customer_id"], "cus_1");
    assert!(stripe["renews"].is_string());
    // The subscription read does not carry the card.
    assert!(stripe.get("card_last4").is_none(), "{stripe}");
    assert_eq!(d["override"], Value::Null);
    assert_eq!(d["members"][0]["email"], ADA);
    assert_eq!(d["members"][0]["role"], "owner");
    assert_eq!(d["pending"][0]["email"], "dana@example.com");
    assert_eq!(d["collections"][0]["slug"], "payments-api");
    assert_eq!(d["usage"]["publishes_this_month"], 1);
    assert_eq!(d["usage"]["tokens_used_this_month"], 0);
    assert_eq!(d["suspension"], Value::Null);
    assert_eq!(d["plan_history"][0]["source"], "default");

    let (status, _) = req(&c.app, "GET", "/v1/admin/teams/nope", None, Some(&c.root)).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
}

#[sqlx::test]
async fn member_role_changes_keep_an_owner(pool: PgPool) {
    let c = setup(pool).await;
    c.app.sign_in("bob@example.com").await;
    let bob_id: Uuid = sqlx::query_scalar("SELECT id FROM users WHERE email = 'bob@example.com'")
        .fetch_one(&c.app.state.db)
        .await
        .unwrap();
    let ada_id: Uuid = sqlx::query_scalar("SELECT id FROM users WHERE email = $1")
        .bind(ADA)
        .fetch_one(&c.app.state.db)
        .await
        .unwrap();
    sqlx::query(
        "INSERT INTO memberships (team_id, user_id, role, created_at) VALUES ($1, $2, 'member', now())",
    )
    .bind(c.team_id)
    .bind(bob_id)
    .execute(&c.app.state.db)
    .await
    .unwrap();
    let path = |id: Uuid| format!("{TEAM}/members/{id}");

    let (status, body) = req(
        &c.app,
        "PATCH",
        &path(ada_id),
        Some(json!({"role": "member"})),
        Some(&c.root),
    )
    .await;
    assert_eq!(status, StatusCode::CONFLICT, "{body}");
    let (status, body) = req(
        &c.app,
        "PATCH",
        &path(bob_id),
        Some(json!({"role": "owner"})),
        Some(&c.root),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(body["role"], "owner");
    let (status, _) = req(
        &c.app,
        "PATCH",
        &path(ada_id),
        Some(json!({"role": "member"})),
        Some(&c.root),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let (status, _) = req(
        &c.app,
        "PATCH",
        &path(Uuid::new_v4()),
        Some(json!({"role": "member"})),
        Some(&c.root),
    )
    .await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    let detail: Value = sqlx::query_scalar(
        "SELECT detail FROM admin_events WHERE action = 'team.member.role' \
         AND detail->>'to' = 'owner'",
    )
    .fetch_one(&c.app.state.db)
    .await
    .unwrap();
    assert_eq!(
        detail,
        json!({"user_id": bob_id, "email": "bob@example.com", "from": "member", "to": "owner"})
    );
    assert_eq!(
        actions(&c.app).await,
        ["team.member.role", "team.member.role"]
    );
}

#[sqlx::test]
async fn email_owners_queues_one_message_each(pool: PgPool) {
    let c = setup(pool).await;
    c.app.sign_in("grace@example.com").await;
    sqlx::query(
        "INSERT INTO memberships (team_id, user_id, role, created_at) \
         SELECT $1, id, 'owner', now() FROM users WHERE email = 'grace@example.com'",
    )
    .bind(c.team_id)
    .execute(&c.app.state.db)
    .await
    .unwrap();
    let path = format!("{TEAM}/email-owners");
    let body = json!({"subject": "Your pilot", "body_md": "Hello\n\nThanks for trying it."});
    let rosa = admin_with(&c, "rosa@example.com", &["teams", "billing"]).await;
    let (status, _) = req(&c.app, "POST", &path, Some(body.clone()), Some(&rosa)).await;
    assert_eq!(status, StatusCode::FORBIDDEN);
    let (status, _) = req(
        &c.app,
        "POST",
        &path,
        Some(json!({"subject": "", "body_md": "x"})),
        Some(&c.root),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    let (status, res) = req(&c.app, "POST", &path, Some(body), Some(&c.root)).await;
    assert_eq!(status, StatusCode::ACCEPTED, "{res}");
    let rows: Vec<(String, String)> = sqlx::query_as(
        "SELECT to_addr, kind FROM outbox_emails WHERE subject = 'Your pilot' ORDER BY to_addr",
    )
    .fetch_all(&c.app.state.db)
    .await
    .unwrap();
    assert_eq!(
        rows,
        [
            (ADA.to_owned(), "admin".to_owned()),
            ("grace@example.com".to_owned(), "admin".to_owned())
        ]
    );
    let detail: Value =
        sqlx::query_scalar("SELECT detail FROM admin_events WHERE action = 'team.email_owners'")
            .fetch_one(&c.app.state.db)
            .await
            .unwrap();
    assert_eq!(detail, json!({"subject": "Your pilot", "recipients": 2}));
}

#[sqlx::test]
async fn activity_groups_cover_team_actions(pool: PgPool) {
    let c = setup(pool).await;
    grant(&c, json!({"plan": "team", "reason": "x", "notify": false})).await;
    req(
        &c.app,
        "POST",
        &format!("{TEAM}/suspend"),
        Some(json!({"reason": "x", "notify": false})),
        Some(&c.root),
    )
    .await;
    req(
        &c.app,
        "POST",
        &format!("{TEAM}/email-owners"),
        Some(json!({"subject": "Hi", "body_md": "Hello"})),
        Some(&c.root),
    )
    .await;
    for (group, action) in [
        ("override", "team.override.grant"),
        ("suspension", "team.suspend"),
        ("email", "team.email_owners"),
    ] {
        let (status, body) = req(
            &c.app,
            "GET",
            &format!("/v1/admin/activity?action_group={group}"),
            None,
            Some(&c.root),
        )
        .await;
        assert_eq!(status, StatusCode::OK, "{body}");
        let got: Vec<&str> = body["items"]
            .as_array()
            .unwrap()
            .iter()
            .map(|i| i["action"].as_str().unwrap())
            .collect();
        assert_eq!(got, [action], "{group}");
    }
}

/// The sync takes the webhook's per-subscription lock before it reads
/// Stripe, so a read never lands under a webhook still applying an older one.
#[sqlx::test]
async fn stripe_sync_reads_under_the_subscription_lock(pool: PgPool) {
    let c = setup(pool).await;
    pay(&c).await;
    let mut held = c.app.state.db.begin().await.unwrap();
    sqlx::query("SELECT pg_advisory_xact_lock(hashtext('sub_1'))")
        .execute(&mut *held)
        .await
        .unwrap();
    let app = curl_runner_server::app(c.app.state.clone());
    let root = c.root.clone();
    let sync = tokio::spawn(async move {
        app.oneshot(
            Request::builder()
                .method(Method::POST)
                .uri(format!("{TEAM}/stripe-sync"))
                .header(header::AUTHORIZATION, format!("Bearer {root}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap()
        .status()
    });
    tokio::time::sleep(std::time::Duration::from_millis(300)).await;
    assert_eq!(
        c.fake.subscription_calls.load(Ordering::SeqCst),
        0,
        "read Stripe while the lock was held"
    );
    held.rollback().await.unwrap();
    assert_eq!(sync.await.unwrap(), StatusCode::OK);
    assert_eq!(c.fake.subscription_calls.load(Ordering::SeqCst), 1);
}
