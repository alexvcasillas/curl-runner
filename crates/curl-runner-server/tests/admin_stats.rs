#![cfg(feature = "db-tests")]

mod common;

use std::sync::Arc;

use axum::http::{Method, StatusCode};
use chrono::{DateTime, Duration, Utc};
use common::{
    BILLING_ENV, FakeBilling, NOW, TestApp, test_app_billing_with, test_app_with, test_config,
};
use curl_runner_server::billing::sync::tick;
use curl_runner_server::clock::Clock;
use curl_runner_server::mail::worker::fan_out_due;
use hmac::{Hmac, Mac};
use serde_json::{Value, json};
use sha2::Sha256;
use sqlx::PgPool;
use uuid::Uuid;

const ROOT: &str = "root@example.com";
const ADA: &str = "ada@example.com";

fn now() -> DateTime<Utc> {
    NOW.parse().unwrap()
}

/// Billing off.
async fn plain(pool: PgPool) -> TestApp {
    test_app_with(pool, test_config(&[("OPERATOR_EMAILS", ROOT)])).await
}

/// Billing on, behind a fake Stripe: monthly 900, annual 9000 (750 a month).
async fn billing(pool: PgPool) -> (TestApp, Arc<FakeBilling>) {
    let mut env = BILLING_ENV.to_vec();
    env.push(("OPERATOR_EMAILS", ROOT));
    test_app_billing_with(pool, test_config(&env)).await
}

async fn get(app: &TestApp, path: &str, bearer: &str) -> (StatusCode, Value) {
    app.request(Method::GET, path, None, Some(bearer)).await
}

async fn ok(app: &TestApp, path: &str, bearer: &str) -> Value {
    let (status, body) = get(app, path, bearer).await;
    assert_eq!(status, StatusCode::OK, "{path}: {body}");
    body
}

async fn team_id(app: &TestApp, slug: &str) -> Uuid {
    sqlx::query_scalar("SELECT id FROM teams WHERE slug = $1")
        .bind(slug)
        .fetch_one(&app.state.db)
        .await
        .unwrap()
}

/// Make `slug` a paying team: Stripe subscription, `interval`, `seats`.
async fn pay(app: &TestApp, slug: &str, interval: &str, seats: i32) {
    sqlx::query(
        "UPDATE teams SET plan = 'team', stripe_subscription_id = $2, billing_status = 'active', \
         billing_interval = $3, seats_synced = $4 WHERE slug = $1",
    )
    .bind(slug)
    .bind(format!("sub_{slug}"))
    .bind(interval)
    .bind(seats)
    .execute(&app.state.db)
    .await
    .unwrap();
}

/// Fill `slug` up to `total` members, so the seat sync agrees with `pay`.
async fn members(app: &TestApp, slug: &str, total: i32) {
    sqlx::query(
        "WITH t AS (SELECT id FROM teams WHERE slug = $1), \
              u AS (INSERT INTO users (email, created_at) \
                    SELECT 'm' || gen_random_uuid() || '-' || $1 || '@example.com', now() \
                    FROM generate_series(1, $2 - (SELECT count(*) FROM memberships m, t \
                                                  WHERE m.team_id = t.id)::int) g \
                    RETURNING id) \
         INSERT INTO memberships (team_id, user_id, role, created_at) \
         SELECT t.id, u.id, 'member', now() FROM t, u",
    )
    .bind(slug)
    .bind(total)
    .execute(&app.state.db)
    .await
    .unwrap();
}

async fn grant(app: &TestApp, slug: &str, until: Option<DateTime<Utc>>) {
    sqlx::query("UPDATE teams SET override_plan = 'team', override_until = $2 WHERE slug = $1")
        .bind(slug)
        .bind(until)
        .execute(&app.state.db)
        .await
        .unwrap();
}

#[sqlx::test]
async fn overview_counts(pool: PgPool) {
    let app = plain(pool).await;
    let root = app.sign_in(ROOT).await;
    let ada = app.sign_in(ADA).await;
    app.create_team(&ada, "acme").await;
    let team = team_id(&app, "acme").await;
    // A collection with two versions, one of them 3 days old.
    let col: Uuid = sqlx::query_scalar(
        "INSERT INTO collections (team_id, slug, name, access) VALUES ($1, 'api', 'api', 'public') \
         RETURNING id",
    )
    .bind(team)
    .fetch_one(&app.state.db)
    .await
    .unwrap();
    for (n, age) in [(1, 3), (2, 0)] {
        sqlx::query(
            "INSERT INTO versions (collection_id, number, hash, published_by, created_at) \
             VALUES ($1, $2, 'h', 'ci', $3)",
        )
        .bind(col)
        .bind(n)
        .bind(now() - Duration::days(age))
        .execute(&app.state.db)
        .await
        .unwrap();
    }
    // Ada signed up 5 days ago.
    sqlx::query("UPDATE users SET created_at = $2 WHERE email = $1")
        .bind(ADA)
        .bind(now() - Duration::days(5))
        .execute(&app.state.db)
        .await
        .unwrap();
    sqlx::query(
        "INSERT INTO admin_events (admin_email, action, target_kind, target_label, at) \
         VALUES ('root@example.com', 'user.suspend', 'user', 'ada@example.com', $1)",
    )
    .bind(now() - Duration::minutes(8))
    .execute(&app.state.db)
    .await
    .unwrap();

    let body = ok(&app, "/v1/admin/overview", &root).await;
    assert_eq!(body["counts"]["users"], 2);
    assert_eq!(body["counts"]["teams"], 1);
    assert_eq!(body["counts"]["collections"], 1);
    assert_eq!(body["counts"]["collections_public"], 1);
    // Billing is off: no revenue figures, and nothing is paying.
    assert_eq!(body["counts"]["paying_teams"], Value::Null);
    assert_eq!(body["counts"]["mrr_cents"], Value::Null);
    assert_eq!(body["billing"], false);
    assert_eq!(body["counts"]["granted_teams"], 0);

    let signups = body["signups_30d"].as_array().unwrap();
    assert_eq!(signups.len(), 30);
    assert_eq!(signups[29]["day"], "2026-11-04");
    assert_eq!(signups[0]["day"], "2026-10-06");
    assert_eq!(signups[29]["n"], 1, "root signed in today");
    assert_eq!(signups[24]["day"], "2026-10-30");
    assert_eq!(signups[24]["n"], 1, "ada, 5 days ago");
    assert_eq!(
        signups
            .iter()
            .map(|d| d["n"].as_i64().unwrap())
            .sum::<i64>(),
        2
    );
    let teams = body["teams_30d"].as_array().unwrap();
    assert_eq!(teams.len(), 30);
    assert_eq!(teams[29]["n"], 1);

    let publishes = body["publishes_14d"].as_array().unwrap();
    assert_eq!(publishes.len(), 14);
    assert_eq!(publishes[0]["day"], "2026-10-22");
    assert_eq!(publishes[13]["n"], 1);
    assert_eq!(publishes[10]["n"], 1);
    assert_eq!(publishes[12]["n"], 0);

    let events = body["recent_events"].as_array().unwrap();
    assert_eq!(events.len(), 1);
    assert_eq!(events[0]["action"], "user.suspend");
    assert_eq!(events[0]["admin_email"], "root@example.com");

    // The status strip.
    assert_eq!(body["signup_mode"], "open");
    assert!(body["outbox"]["waiting"].as_i64().unwrap() >= 2, "{body}");
    assert_eq!(body["outbox"]["failed_today"], 0);
    assert_eq!(body["last_webhook_at"], Value::Null);
}

#[sqlx::test]
async fn p95_from_recent_requests(pool: PgPool) {
    let app = plain(pool).await;
    let root = app.sign_in(ROOT).await;
    // The tower layer has seen the sign-in requests already.
    assert!(app.state.metrics.requests_5m(now()) > 0);
    // 1000 slow requests drown the few quick ones from the sign-in.
    for _ in 0..1000 {
        app.state
            .metrics
            .record_request(now(), std::time::Duration::from_millis(200), 200);
    }
    let body = ok(&app, "/v1/admin/overview", &root).await;
    assert_eq!(body["p95_ms"], 200);
    let health = ok(&app, "/v1/admin/health", &root).await;
    assert_eq!(health["stats"]["p95_ms"], 200);
    // Past 5 minutes they no longer count.
    app.clock.advance(Duration::minutes(6));
    let body = ok(&app, "/v1/admin/overview", &root).await;
    assert_eq!(body["p95_ms"], Value::Null);
}

#[sqlx::test]
async fn mrr_annual_counts_twelfth(pool: PgPool) {
    let (app, _) = billing(pool).await;
    let root = app.sign_in(ROOT).await;
    let ada = app.sign_in(ADA).await;
    app.create_team(&ada, "acme").await;
    app.create_team(&ada, "globex").await;
    pay(&app, "acme", "annual", 3).await;
    pay(&app, "globex", "monthly", 2).await;

    let body = ok(&app, "/v1/admin/stripe", &root).await;
    // 3 x 9000 / 12 + 2 x 900.
    assert_eq!(body["mrr_cents"], 2250 + 1800);
    assert_eq!(body["arr_cents"], (2250 + 1800) * 12);
    assert_eq!(body["paying_teams"], 2);
    assert_eq!(body["seats"], 5);
    assert_eq!(body["arpa"], 2025);
    assert_eq!(body["currency"], "usd");
    let sources = body["by_source"].as_array().unwrap();
    let find = |s: &str| sources.iter().find(|x| x["source"] == s).unwrap().clone();
    assert_eq!(find("stripe_annual")["teams"], 1);
    assert_eq!(find("stripe_annual")["seats"], 3);
    assert_eq!(find("stripe_monthly")["teams"], 1);
    assert_eq!(find("granted")["teams"], 0);

    let overview = ok(&app, "/v1/admin/overview", &root).await;
    assert_eq!(overview["counts"]["mrr_cents"], 4050);
    assert_eq!(overview["counts"]["paying_teams"], 2);
}

#[sqlx::test]
async fn granted_not_revenue(pool: PgPool) {
    let (app, _) = billing(pool).await;
    let root = app.sign_in(ROOT).await;
    let ada = app.sign_in(ADA).await;
    for slug in ["acme", "globex", "initech"] {
        app.create_team(&ada, slug).await;
    }
    pay(&app, "acme", "monthly", 2).await;
    // Granted, with no Stripe subscription: not revenue.
    grant(&app, "globex", None).await;
    // Its override ended yesterday: back on its Stripe plan, which is revenue.
    pay(&app, "initech", "monthly", 1).await;
    grant(&app, "initech", Some(now() - Duration::days(1))).await;

    let body = ok(&app, "/v1/admin/stripe", &root).await;
    assert_eq!(body["mrr_cents"], 1800 + 900);
    assert_eq!(body["paying_teams"], 2);
    let granted = body["by_source"]
        .as_array()
        .unwrap()
        .iter()
        .find(|s| s["source"] == "granted")
        .unwrap()
        .clone();
    assert_eq!(granted["teams"], 1);
    assert_eq!(granted["seats"], 1);

    // An active override on a paying team moves it to "granted" too.
    grant(&app, "acme", None).await;
    let body = ok(&app, "/v1/admin/stripe", &root).await;
    assert_eq!(body["mrr_cents"], 900);
    let overview = ok(&app, "/v1/admin/overview", &root).await;
    assert_eq!(overview["counts"]["granted_teams"], 2);
    assert_eq!(overview["counts"]["mrr_cents"], 900);
}

#[sqlx::test]
async fn daily_snapshot_once_per_day(pool: PgPool) {
    let (app, _) = billing(pool).await;
    let root = app.sign_in(ROOT).await;
    let ada = app.sign_in(ADA).await;
    app.create_team(&ada, "acme").await;
    members(&app, "acme", 2).await;
    pay(&app, "acme", "monthly", 2).await;
    tick(&app.state, now()).await;
    tick(&app.state, now() + Duration::minutes(1)).await;
    let rows = || async {
        sqlx::query_as::<_, (String, i64, i32, i32)>(
            "SELECT day::text, mrr_cents, teams, seats FROM mrr_snapshots ORDER BY day",
        )
        .fetch_all(&app.state.db)
        .await
        .unwrap()
    };
    assert_eq!(rows().await, [("2026-11-04".to_string(), 1800, 1, 2)]);
    // A change later the same day does not rewrite the day.
    members(&app, "acme", 5).await;
    pay(&app, "acme", "monthly", 5).await;
    tick(&app.state, now() + Duration::hours(5)).await;
    assert_eq!(rows().await.len(), 1);
    assert_eq!(rows().await[0].1, 1800);
    // The next UTC day gets its own row.
    tick(&app.state, now() + Duration::hours(15)).await;
    let all = rows().await;
    assert_eq!(all.len(), 2);
    assert_eq!(all[1], ("2026-11-05".to_string(), 4500, 1, 5));

    app.clock.advance(Duration::hours(15));
    let body = ok(&app, "/v1/admin/stripe", &root).await;
    let series = body["mrr_90d"].as_array().unwrap();
    assert_eq!(series.len(), 2);
    assert_eq!(series[0], json!({"day": "2026-11-04", "mrr_cents": 1800}));
    assert_eq!(series[1], json!({"day": "2026-11-05", "mrr_cents": 4500}));
}

#[sqlx::test]
async fn health_job_times(pool: PgPool) {
    let app = plain(pool).await;
    let root = app.sign_in(ROOT).await;
    let jobs = |body: &Value| -> Vec<(String, Value, String)> {
        body["jobs"]
            .as_array()
            .unwrap()
            .iter()
            .map(|j| {
                (
                    j["name"].as_str().unwrap().to_owned(),
                    j["last_run"].clone(),
                    j["status"].as_str().unwrap().to_owned(),
                )
            })
            .collect()
    };
    let before = jobs(&ok(&app, "/v1/admin/health", &root).await);
    let names: Vec<&str> = before.iter().map(|j| j.0.as_str()).collect();
    assert_eq!(
        names,
        [
            "mail_worker",
            "billing_sync",
            "override_expiry",
            "broadcast_fanout",
            "mrr_snapshot",
            "audit_retention"
        ]
    );
    assert!(before.iter().all(|j| j.1.is_null() && j.2 == "never_run"));

    tick(&app.state, now()).await;
    fan_out_due(&app.state, now()).await.unwrap();
    let after = jobs(&ok(&app, "/v1/admin/health", &root).await);
    for (name, last, status) in &after {
        if name == "mail_worker" {
            assert!(last.is_null(), "the mail worker did not run");
        } else {
            assert_eq!(last.as_str().unwrap(), "2026-11-04T10:00:00Z", "{name}");
            assert_eq!(status, "ok", "{name}");
        }
    }
    // A job that stopped ticking is stale.
    app.clock.advance(Duration::minutes(10));
    let later = jobs(&ok(&app, "/v1/admin/health", &root).await);
    assert!(
        later
            .iter()
            .any(|j| j.0 == "billing_sync" && j.2 == "stale")
    );

    // Rate-limit hits are counted per key prefix, never with the key itself.
    let w = Duration::minutes(15);
    app.state
        .limiter
        .check("email-start:email:x@y.test", 1, w)
        .unwrap();
    for _ in 0..3 {
        assert!(
            app.state
                .limiter
                .check("email-start:email:x@y.test", 1, w)
                .is_err()
        );
    }
    assert!(
        app.state
            .limiter
            .check("shared:ip:2001:db8::1", 0, w)
            .is_err()
    );
    let health = ok(&app, "/v1/admin/health", &root).await;
    let limits = health["rate_limits"].as_array().unwrap();
    let hit = limits
        .iter()
        .find(|l| l["key"] == "email-start:email")
        .unwrap();
    assert_eq!(hit["n"], 3);
    assert!(hit["what"].as_str().unwrap().contains("email"));
    assert!(
        limits
            .iter()
            .any(|l| l["key"] == "shared:ip" && l["n"] == 1)
    );
    let text = health.to_string();
    assert!(
        !text.contains("x@y.test") && !text.contains("2001:db8"),
        "{text}"
    );
    // Hits fall out of the hour.
    app.clock.advance(Duration::hours(2));
    let health = ok(&app, "/v1/admin/health", &root).await;
    assert!(health["rate_limits"].as_array().unwrap().is_empty());

    let stats = &health["stats"];
    assert_eq!(stats["version"], env!("CARGO_PKG_VERSION"));
    assert!(stats["db"]["size_bytes"].as_i64().unwrap() > 0);
    assert!(stats["db"]["connections"].as_i64().unwrap() >= 1);
    assert_eq!(stats["db"]["migration"], 4);
}

fn sign(ts: i64, body: &str) -> String {
    let mut mac = Hmac::<Sha256>::new_from_slice(b"whsec_fake").unwrap();
    mac.update(format!("{ts}.{body}").as_bytes());
    let hex: String = mac
        .finalize()
        .into_bytes()
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect();
    format!("t={ts},v1={hex}")
}

async fn deliver(app: &TestApp, id: &str, kind: &str) -> StatusCode {
    let object = if kind == "checkout.session.completed" {
        json!({"id": "cs_1", "subscription": "sub_1"})
    } else {
        json!({"id": "sub_1", "object": "subscription"})
    };
    let body = json!({"id": id, "type": kind, "data": {"object": object}}).to_string();
    let sig = sign(app.clock.now().timestamp(), &body);
    app.post_raw("/v1/billing/webhook", &body, &[("stripe-signature", &sig)])
        .await
        .0
}

fn subscription(fake: &FakeBilling, team: Uuid, status: &str, price: &str, quantity: u32) {
    let mut sub = fake.subscription.lock().unwrap();
    sub.team_id = Some(team);
    sub.status = status.into();
    sub.price = price.into();
    sub.quantity = quantity;
}

type EventRow = (String, Option<Uuid>, Option<String>, Option<i64>);

#[sqlx::test]
async fn webhook_fills_event_summary_and_mrr_delta(pool: PgPool) {
    let (app, fake) = billing(pool).await;
    let root = app.sign_in(ROOT).await;
    let ada = app.sign_in(ADA).await;
    app.create_team(&ada, "acme").await;
    let team = team_id(&app, "acme").await;

    subscription(&fake, team, "active", "price_annual", 3);
    assert_eq!(
        deliver(&app, "evt_1", "checkout.session.completed").await,
        StatusCode::OK
    );
    subscription(&fake, team, "active", "price_annual", 5);
    app.clock.advance(Duration::minutes(1));
    assert_eq!(
        deliver(&app, "evt_2", "customer.subscription.updated").await,
        StatusCode::OK
    );
    subscription(&fake, team, "canceled", "price_annual", 5);
    app.clock.advance(Duration::minutes(1));
    assert_eq!(
        deliver(&app, "evt_3", "customer.subscription.deleted").await,
        StatusCode::OK
    );

    let rows: Vec<EventRow> = sqlx::query_as(
        "SELECT id, team_id, summary, mrr_delta_cents FROM stripe_events ORDER BY received_at",
    )
    .fetch_all(&app.state.db)
    .await
    .unwrap();
    assert_eq!(
        rows,
        [
            (
                "evt_1".into(),
                Some(team),
                Some("Team annual, 3 seats".into()),
                Some(2250)
            ),
            (
                "evt_2".into(),
                Some(team),
                Some("Seats 3 to 5".into()),
                Some(1500)
            ),
            (
                "evt_3".into(),
                Some(team),
                Some("Cancelled".into()),
                Some(-3750)
            ),
        ]
    );

    let body = ok(&app, "/v1/admin/stripe", &root).await;
    assert_eq!(body["mrr_cents"], 0);
    assert_eq!(body["net_new"]["new_cents"], 2250);
    assert_eq!(body["net_new"]["expansion_cents"], 1500);
    assert_eq!(body["net_new"]["churned_cents"], -3750);
    assert_eq!(body["net_new"]["total_cents"], 0);
    assert_eq!(body["net_new"]["cancelled_teams"], 1);
    let events = body["events"].as_array().unwrap();
    assert_eq!(events.len(), 3);
    assert_eq!(events[0]["id"], "evt_3", "newest first");
    assert_eq!(events[0]["type"], "customer.subscription.deleted");
    assert_eq!(events[0]["team"]["slug"], "acme");
    assert_eq!(events[0]["summary"], "Cancelled");
    assert_eq!(events[0]["mrr_delta_cents"], -3750);
}

#[sqlx::test]
async fn webhook_counts_feed_health(pool: PgPool) {
    let (app, fake) = billing(pool).await;
    let root = app.sign_in(ROOT).await;
    let ada = app.sign_in(ADA).await;
    app.create_team(&ada, "acme").await;
    let team = team_id(&app, "acme").await;
    subscription(&fake, team, "active", "price_monthly", 1);
    assert_eq!(
        deliver(&app, "evt_1", "checkout.session.completed").await,
        StatusCode::OK
    );
    assert_eq!(
        deliver(&app, "evt_1", "checkout.session.completed").await,
        StatusCode::OK
    );
    let (status, _) = app
        .post_raw(
            "/v1/billing/webhook",
            "{}",
            &[("stripe-signature", "t=1,v1=00")],
        )
        .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);

    let health = ok(&app, "/v1/admin/health", &root).await;
    assert_eq!(
        health["webhooks_24h"],
        json!([{"type": "checkout.session.completed", "n": 1}])
    );
    assert_eq!(health["duplicates_24h"], 1);
    assert_eq!(health["bad_signatures_24h"], 1);
    let overview = ok(&app, "/v1/admin/overview", &root).await;
    assert_eq!(overview["last_webhook_at"], "2026-11-04T10:00:00Z");
    let stripe = ok(&app, "/v1/admin/stripe", &root).await;
    assert_eq!(
        stripe["webhooks"],
        json!({"events_24h": 1, "bad_signatures_24h": 1, "duplicates_24h": 1})
    );
}

#[sqlx::test]
async fn stripe_stats_404_billing_off(pool: PgPool) {
    let app = plain(pool).await;
    let root = app.sign_in(ROOT).await;
    let (status, body) = get(&app, "/v1/admin/stripe", &root).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    assert_eq!(body["error"], "not_found");
    // A granted team still shows on the overview.
    let ada = app.sign_in(ADA).await;
    app.create_team(&ada, "acme").await;
    grant(&app, "acme", None).await;
    let overview = ok(&app, "/v1/admin/overview", &root).await;
    assert_eq!(overview["counts"]["granted_teams"], 1);
    assert_eq!(overview["counts"]["granted_seats"], 1);
    ok(&app, "/v1/admin/health", &root).await;
}
