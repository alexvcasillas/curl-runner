#![cfg(feature = "db-tests")]

mod common;

use std::sync::atomic::Ordering;

use axum::http::{Method, StatusCode};
use chrono::{DateTime, Duration, Utc};
use common::{NOW, TestApp, test_app, test_app_billing};
use curl_runner_server::billing::sync::tick;
use curl_runner_server::clock::Clock;
use hmac::{Hmac, Mac};
use serde_json::{Value, json};
use sha2::Sha256;
use sqlx::PgPool;
use uuid::Uuid;

const SECRET: &str = "whsec_fake";

fn sign(secret: &str, ts: i64, body: &str) -> String {
    let mut mac = Hmac::<Sha256>::new_from_slice(secret.as_bytes()).unwrap();
    mac.update(format!("{ts}.{body}").as_bytes());
    let hex: String = mac
        .finalize()
        .into_bytes()
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect();
    format!("t={ts},v1={hex}")
}

fn event(id: &str, kind: &str, object: Value) -> Value {
    json!({"id": id, "type": kind, "data": {"object": object}})
}

fn checkout(id: &str) -> Value {
    event(
        id,
        "checkout.session.completed",
        json!({"id": "cs_1", "subscription": "sub_1"}),
    )
}

fn invoice(id: &str, kind: &str) -> Value {
    event(
        id,
        kind,
        json!({"id": "in_1", "parent": {"subscription_details": {"subscription": "sub_1"}}}),
    )
}

/// POST `event`, signed with the clock's current time.
async fn deliver(app: &TestApp, event: &Value) -> (StatusCode, Value) {
    let body = event.to_string();
    let sig = sign(SECRET, app.clock.now().timestamp(), &body);
    app.post_raw("/v1/billing/webhook", &body, &[("stripe-signature", &sig)])
        .await
}

struct Team {
    id: Uuid,
    ada: String,
}

/// Team `acme` owned by ada, with the fake Stripe subscription pointing at it.
async fn setup(pool: PgPool) -> (TestApp, std::sync::Arc<common::FakeBilling>, Team) {
    let (app, fake) = test_app_billing(pool).await;
    let ada = app.sign_in("ada@example.com").await;
    app.create_team(&ada, "acme").await;
    let id: Uuid = sqlx::query_scalar("SELECT id FROM teams WHERE slug = 'acme'")
        .fetch_one(&app.state.db)
        .await
        .unwrap();
    fake.subscription.lock().unwrap().team_id = Some(id);
    (app, fake, Team { id, ada })
}

#[derive(sqlx::FromRow, Debug)]
struct Row {
    plan: String,
    stripe_customer_id: Option<String>,
    stripe_subscription_id: Option<String>,
    billing_interval: Option<String>,
    billing_status: Option<String>,
    past_due_since: Option<DateTime<Utc>>,
    seats_synced: Option<i32>,
    current_period_end: Option<DateTime<Utc>>,
}

async fn row(app: &TestApp) -> Row {
    sqlx::query_as(
        "SELECT plan, stripe_customer_id, stripe_subscription_id, billing_interval, \
         billing_status, past_due_since, seats_synced, current_period_end \
         FROM teams WHERE slug = 'acme'",
    )
    .fetch_one(&app.state.db)
    .await
    .unwrap()
}

async fn mails(app: &TestApp, subject_like: &str) -> Vec<String> {
    sqlx::query_scalar("SELECT to_addr FROM outbox_emails WHERE subject LIKE $1 ORDER BY to_addr")
        .bind(subject_like)
        .fetch_all(&app.state.db)
        .await
        .unwrap()
}

fn set_status(fake: &common::FakeBilling, status: &str) {
    fake.subscription.lock().unwrap().status = status.into();
}

#[sqlx::test]
async fn webhook_404_when_billing_off(pool: PgPool) {
    let app = test_app(pool).await;
    let (status, body) = app.post_raw("/v1/billing/webhook", "{}", &[]).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    assert_eq!(body["error"], "not_found");
}

#[sqlx::test]
async fn bad_signature_400(pool: PgPool) {
    let (app, fake, _) = setup(pool).await;
    let body = checkout("evt_1").to_string();
    let now = app.clock.now().timestamp();
    for sig in [
        None,
        Some("garbage".to_owned()),
        Some(sign("whsec_other", now, &body)),
        Some(sign(SECRET, now, "{}")),
    ] {
        let headers: Vec<(&str, &str)> = sig
            .iter()
            .map(|s| ("stripe-signature", s.as_str()))
            .collect();
        let (status, res) = app.post_raw("/v1/billing/webhook", &body, &headers).await;
        assert_eq!(status, StatusCode::BAD_REQUEST, "{sig:?}");
        assert_eq!(res["error"], "invalid_request");
    }
    assert_eq!(fake.subscription_calls.load(Ordering::SeqCst), 0);
    assert_eq!(row(&app).await.plan, "free");
}

#[sqlx::test]
async fn stale_timestamp_400(pool: PgPool) {
    let (app, _, _) = setup(pool).await;
    let body = checkout("evt_1").to_string();
    let old = app.clock.now().timestamp() - 301;
    let sig = sign(SECRET, old, &body);
    let (status, res) = app
        .post_raw("/v1/billing/webhook", &body, &[("stripe-signature", &sig)])
        .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(res["error"], "invalid_request");
    // Inside the tolerance it is accepted.
    let fresh = sign(SECRET, app.clock.now().timestamp() - 299, &body);
    let (status, _) = app
        .post_raw(
            "/v1/billing/webhook",
            &body,
            &[("stripe-signature", &fresh)],
        )
        .await;
    assert_eq!(status, StatusCode::OK);
}

#[sqlx::test]
async fn duplicate_event_is_noop(pool: PgPool) {
    let (app, fake, _) = setup(pool).await;
    let evt = checkout("evt_1");
    assert_eq!(deliver(&app, &evt).await, (StatusCode::OK, json!({})));
    assert_eq!(deliver(&app, &evt).await, (StatusCode::OK, json!({})));
    assert_eq!(fake.subscription_calls.load(Ordering::SeqCst), 1);
}

#[sqlx::test]
async fn unhandled_event_is_200_and_not_read(pool: PgPool) {
    let (app, fake, _) = setup(pool).await;
    let evt = event("evt_x", "customer.created", json!({"id": "cus_1"}));
    assert_eq!(deliver(&app, &evt).await, (StatusCode::OK, json!({})));
    assert_eq!(fake.subscription_calls.load(Ordering::SeqCst), 0);
}

#[sqlx::test]
async fn stripe_failure_is_503_and_the_event_is_retried(pool: PgPool) {
    let (app, fake, _) = setup(pool).await;
    fake.fail.store(true, Ordering::SeqCst);
    let (status, _) = deliver(&app, &checkout("evt_1")).await;
    assert_eq!(status, StatusCode::SERVICE_UNAVAILABLE);
    fake.fail.store(false, Ordering::SeqCst);
    let (status, _) = deliver(&app, &checkout("evt_1")).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(row(&app).await.plan, "team");
}

#[sqlx::test]
async fn checkout_completed_sets_team_plan(pool: PgPool) {
    let (app, _, _) = setup(pool).await;
    let (status, body) = deliver(&app, &checkout("evt_1")).await;
    assert_eq!((status, body), (StatusCode::OK, json!({})));
    let r = row(&app).await;
    assert_eq!(r.plan, "team");
    assert_eq!(r.stripe_customer_id.as_deref(), Some("cus_1"));
    assert_eq!(r.stripe_subscription_id.as_deref(), Some("sub_1"));
    assert_eq!(r.billing_interval.as_deref(), Some("monthly"));
    assert_eq!(r.billing_status.as_deref(), Some("active"));
    assert_eq!(r.past_due_since, None);
    assert_eq!(r.seats_synced, Some(1));
    assert_eq!(
        r.current_period_end.map(|t| t.timestamp()),
        Some(1_793_000_000)
    );
}

#[sqlx::test]
async fn annual_price_sets_annual_interval(pool: PgPool) {
    let (app, fake, _) = setup(pool).await;
    fake.subscription.lock().unwrap().price = "price_annual".into();
    deliver(&app, &checkout("evt_1")).await;
    assert_eq!(row(&app).await.billing_interval.as_deref(), Some("annual"));
}

#[sqlx::test]
async fn team_found_by_subscription_id_without_metadata(pool: PgPool) {
    let (app, fake, _) = setup(pool).await;
    deliver(&app, &checkout("evt_1")).await;
    fake.subscription.lock().unwrap().team_id = None;
    set_status(&fake, "past_due");
    let evt = event(
        "evt_2",
        "customer.subscription.updated",
        json!({"id": "sub_1"}),
    );
    deliver(&app, &evt).await;
    assert_eq!(row(&app).await.billing_status.as_deref(), Some("past_due"));
}

#[sqlx::test]
async fn deleted_before_retried_checkout_ends_free(pool: PgPool) {
    let (app, fake, _) = setup(pool).await;
    deliver(&app, &checkout("evt_1")).await;
    assert_eq!(row(&app).await.plan, "team");
    set_status(&fake, "canceled");
    let deleted = event(
        "evt_2",
        "customer.subscription.deleted",
        json!({"id": "sub_1"}),
    );
    deliver(&app, &deleted).await;
    let r = row(&app).await;
    assert_eq!(r.plan, "free");
    assert_eq!(r.stripe_subscription_id, None);
    // The retried checkout event re-reads the subscription: canceled.
    deliver(&app, &checkout("evt_3")).await;
    let r = row(&app).await;
    assert_eq!(r.plan, "free");
    assert_eq!(r.stripe_subscription_id, None);
    // One mail to the one owner, not one per event.
    assert_eq!(
        mails(&app, "% moved to the Free plan").await,
        ["ada@example.com"]
    );
}

#[sqlx::test]
async fn missing_subscription_ends_free(pool: PgPool) {
    let (app, fake, _) = setup(pool).await;
    deliver(&app, &checkout("evt_1")).await;
    // A subscription Stripe no longer knows is the same as a canceled one.
    fake.missing.store(true, Ordering::SeqCst);
    let evt = event(
        "evt_2",
        "customer.subscription.deleted",
        json!({"id": "sub_1"}),
    );
    let (status, _) = deliver(&app, &evt).await;
    assert_eq!(status, StatusCode::OK);
    let r = row(&app).await;
    assert_eq!((r.plan.as_str(), r.stripe_subscription_id), ("free", None));
}

#[sqlx::test]
async fn old_subscription_cancel_does_not_end_a_newer_one(pool: PgPool) {
    let (app, fake, _) = setup(pool).await;
    deliver(&app, &checkout("evt_1")).await;
    // The team resubscribed: the old one's late `deleted` must not downgrade it.
    sqlx::query("UPDATE teams SET stripe_subscription_id = 'sub_2'")
        .execute(&app.state.db)
        .await
        .unwrap();
    set_status(&fake, "canceled");
    let evt = event(
        "evt_2",
        "customer.subscription.deleted",
        json!({"id": "sub_1"}),
    );
    deliver(&app, &evt).await;
    let r = row(&app).await;
    assert_eq!(r.plan, "team");
    assert_eq!(r.stripe_subscription_id.as_deref(), Some("sub_2"));
}

#[sqlx::test]
async fn grace_period_comes_from_the_settings(pool: PgPool) {
    let (app, fake, _team) = setup(pool).await;
    // A 7 day grace, set the way the admin page does.
    app.state.settings().await.unwrap();
    sqlx::query("UPDATE server_settings SET grace_days = 7")
        .execute(&app.state.db)
        .await
        .unwrap();
    app.state.settings_cache.invalidate();
    deliver(&app, &checkout("evt_1")).await;
    set_status(&fake, "past_due");
    deliver(&app, &invoice("evt_2", "invoice.payment_failed")).await;

    app.clock.advance(Duration::days(6));
    tick(&app.state, app.clock.now()).await;
    assert_eq!(row(&app).await.plan, "team", "+6 days");
    app.clock.advance(Duration::days(1));
    tick(&app.state, app.clock.now()).await;
    assert_eq!(row(&app).await.plan, "free", "+7 days");
}

#[sqlx::test]
async fn past_due_14_days_downgrades_and_mails_owners(pool: PgPool) {
    let (app, fake, team) = setup(pool).await;
    let bob = app.sign_in("bob@example.com").await;
    sqlx::query(
        "INSERT INTO memberships (team_id, user_id, role, created_at) \
         SELECT $1, id, 'owner', now() FROM users WHERE email = 'bob@example.com'",
    )
    .bind(team.id)
    .execute(&app.state.db)
    .await
    .unwrap();
    let _ = (bob, &team.ada);
    deliver(&app, &checkout("evt_1")).await;

    set_status(&fake, "past_due");
    deliver(&app, &invoice("evt_2", "invoice.payment_failed")).await;
    let r = row(&app).await;
    assert_eq!(r.plan, "team");
    assert_eq!(r.past_due_since, Some(NOW.parse().unwrap()));
    assert_eq!(
        mails(&app, "We couldn't take%").await,
        ["ada@example.com", "bob@example.com"]
    );
    // Stripe retries and fails again: the clock keeps its first value, no second mail.
    app.clock.advance(Duration::days(1));
    deliver(&app, &invoice("evt_3", "invoice.payment_failed")).await;
    assert_eq!(row(&app).await.past_due_since, Some(NOW.parse().unwrap()));
    assert_eq!(mails(&app, "We couldn't take%").await.len(), 2);

    app.clock.advance(Duration::days(12));
    tick(&app.state, app.clock.now()).await;
    assert_eq!(row(&app).await.plan, "team", "+13 days");
    assert!(mails(&app, "% moved to the Free plan").await.is_empty());

    app.clock.advance(Duration::days(1));
    tick(&app.state, app.clock.now()).await;
    assert_eq!(row(&app).await.plan, "free", "+14 days");
    assert_eq!(
        mails(&app, "% moved to the Free plan").await,
        ["ada@example.com", "bob@example.com"]
    );
    // Stripe's own dunning keeps the subscription; the tick never cancels it.
    assert!(fake.cancelled.lock().unwrap().is_empty());
    assert_eq!(
        row(&app).await.stripe_subscription_id.as_deref(),
        Some("sub_1")
    );
    tick(&app.state, app.clock.now()).await;
    assert_eq!(mails(&app, "% moved to the Free plan").await.len(), 2);

    // Paying restores the plan.
    set_status(&fake, "active");
    deliver(&app, &invoice("evt_4", "invoice.paid")).await;
    let r = row(&app).await;
    assert_eq!((r.plan.as_str(), r.past_due_since), ("team", None));
}

#[sqlx::test]
async fn invoice_paid_clears_past_due(pool: PgPool) {
    let (app, fake, _) = setup(pool).await;
    deliver(&app, &checkout("evt_1")).await;
    set_status(&fake, "past_due");
    deliver(&app, &invoice("evt_2", "invoice.payment_failed")).await;
    assert!(row(&app).await.past_due_since.is_some());
    set_status(&fake, "active");
    deliver(&app, &invoice("evt_3", "invoice.paid")).await;
    let r = row(&app).await;
    assert_eq!(r.past_due_since, None);
    assert_eq!(r.plan, "team");
    assert_eq!(r.billing_status.as_deref(), Some("active"));
}

#[sqlx::test]
async fn invoice_without_a_subscription_is_ignored(pool: PgPool) {
    let (app, fake, _) = setup(pool).await;
    let evt = event(
        "evt_1",
        "invoice.paid",
        json!({"id": "in_1", "parent": null}),
    );
    assert_eq!(deliver(&app, &evt).await, (StatusCode::OK, json!({})));
    assert_eq!(fake.subscription_calls.load(Ordering::SeqCst), 0);
}

/// Invite `email` to acme as ada and accept as them.
async fn add_member(app: &TestApp, ada: &str, email: &str) {
    let (status, body) = app
        .request(
            Method::POST,
            "/v1/teams/acme/invites",
            Some(json!({"email": email, "role": "member"})),
            Some(ada),
        )
        .await;
    assert_eq!(status, StatusCode::CREATED, "{body}");
    let text: String = sqlx::query_scalar(
        "SELECT text_body FROM outbox_emails WHERE to_addr = $1 ORDER BY ctid DESC LIMIT 1",
    )
    .bind(email)
    .fetch_one(&app.state.db)
    .await
    .unwrap();
    let token = text
        .split("/invites/")
        .nth(1)
        .unwrap()
        .split(|c: char| !(c.is_ascii_alphanumeric() || c == '_' || c == '-'))
        .next()
        .unwrap()
        .to_owned();
    let session = app.sign_in(email).await;
    let (status, body) = app
        .request(
            Method::POST,
            &format!("/v1/invites/{token}/accept"),
            None,
            Some(&session),
        )
        .await;
    assert_eq!(status, StatusCode::OK, "{body}");
}

#[sqlx::test]
async fn accepting_invite_triggers_quantity_sync(pool: PgPool) {
    let (app, fake, team) = setup(pool).await;
    deliver(&app, &checkout("evt_1")).await;
    assert_eq!(row(&app).await.seats_synced, Some(1));
    add_member(&app, &team.ada, "jane@example.com").await;
    assert_eq!(row(&app).await.seats_synced, None);

    tick(&app.state, app.clock.now()).await;
    assert_eq!(*fake.quantities.lock().unwrap(), [("sub_1".to_owned(), 2)]);
    assert_eq!(row(&app).await.seats_synced, Some(2));
    // Nothing changed: the next tick makes no call.
    tick(&app.state, app.clock.now()).await;
    assert_eq!(fake.quantities.lock().unwrap().len(), 1);

    // Removing a member syncs too.
    let (_, members) = app
        .request(Method::GET, "/v1/teams/acme", None, Some(&team.ada))
        .await;
    let jane = members["members"]
        .as_array()
        .unwrap()
        .iter()
        .find(|m| m["email"] == "jane@example.com")
        .unwrap()["user_id"]
        .as_str()
        .unwrap()
        .to_owned();
    let (status, _) = app
        .request(
            Method::DELETE,
            &format!("/v1/teams/acme/members/{jane}"),
            None,
            Some(&team.ada),
        )
        .await;
    assert_eq!(status, StatusCode::NO_CONTENT);
    assert_eq!(row(&app).await.seats_synced, None);
    tick(&app.state, app.clock.now()).await;
    assert_eq!(fake.quantities.lock().unwrap().last().unwrap().1, 1);
}

#[sqlx::test]
async fn seat_sync_failure_retries_next_tick(pool: PgPool) {
    let (app, fake, team) = setup(pool).await;
    deliver(&app, &checkout("evt_1")).await;
    add_member(&app, &team.ada, "jane@example.com").await;

    fake.fail.store(true, Ordering::SeqCst);
    tick(&app.state, app.clock.now()).await;
    assert!(fake.quantities.lock().unwrap().is_empty());
    assert_eq!(row(&app).await.seats_synced, None);

    fake.fail.store(false, Ordering::SeqCst);
    tick(&app.state, app.clock.now()).await;
    assert_eq!(*fake.quantities.lock().unwrap(), [("sub_1".to_owned(), 2)]);
    assert_eq!(row(&app).await.seats_synced, Some(2));
}

#[sqlx::test]
async fn team_without_subscription_is_never_synced(pool: PgPool) {
    let (app, fake, team) = setup(pool).await;
    add_member(&app, &team.ada, "jane@example.com").await;
    tick(&app.state, app.clock.now()).await;
    assert!(fake.quantities.lock().unwrap().is_empty());
}

#[sqlx::test]
async fn team_delete_cancels_subscription(pool: PgPool) {
    let (app, fake, team) = setup(pool).await;
    deliver(&app, &checkout("evt_1")).await;

    // A Stripe failure keeps the team.
    fake.fail.store(true, Ordering::SeqCst);
    let (status, body) = app
        .request(Method::DELETE, "/v1/teams/acme", None, Some(&team.ada))
        .await;
    assert_eq!(status, StatusCode::SERVICE_UNAVAILABLE, "{body}");
    assert_eq!(body["error"], "unavailable");
    assert_eq!(row(&app).await.plan, "team");

    fake.fail.store(false, Ordering::SeqCst);
    let (status, _) = app
        .request(Method::DELETE, "/v1/teams/acme", None, Some(&team.ada))
        .await;
    assert_eq!(status, StatusCode::NO_CONTENT);
    assert_eq!(*fake.cancelled.lock().unwrap(), ["sub_1"]);
    let left: i64 = sqlx::query_scalar("SELECT count(*) FROM teams WHERE slug = 'acme'")
        .fetch_one(&app.state.db)
        .await
        .unwrap();
    assert_eq!(left, 0);
}

#[sqlx::test]
async fn team_delete_without_subscription_calls_nothing(pool: PgPool) {
    let (app, fake, team) = setup(pool).await;
    let (status, _) = app
        .request(Method::DELETE, "/v1/teams/acme", None, Some(&team.ada))
        .await;
    assert_eq!(status, StatusCode::NO_CONTENT);
    assert!(fake.cancelled.lock().unwrap().is_empty());
}

#[sqlx::test]
async fn slow_read_cannot_overwrite_a_newer_event(pool: PgPool) {
    use std::sync::Arc;
    use tokio::sync::Notify;
    let (app, fake, _) = setup(pool).await;
    let (entered, release) = (Arc::new(Notify::new()), Arc::new(Notify::new()));
    *fake.gate.lock().unwrap() = Some((entered.clone(), release.clone()));

    // A reads `active`, then stalls before applying it.
    let evt_a = checkout("evt_a");
    let a = deliver(&app, &evt_a);
    // Stripe moves on to `canceled` while A is stalled; B arrives meanwhile and
    // A is released only after B has had time to run (or to queue behind A).
    let rest = async {
        entered.notified().await;
        set_status(&fake, "canceled");
        let deleted = event(
            "evt_b",
            "customer.subscription.deleted",
            json!({"id": "sub_1"}),
        );
        let unblock = async {
            tokio::time::sleep(std::time::Duration::from_millis(500)).await;
            release.notify_one();
        };
        tokio::join!(deliver(&app, &deleted), unblock).0
    };
    let ((sa, _), (sb, _)) = tokio::join!(a, rest);
    assert_eq!((sa, sb), (StatusCode::OK, StatusCode::OK));
    let r = row(&app).await;
    assert_eq!(r.plan, "free");
    assert_eq!(r.stripe_subscription_id, None);
}

#[sqlx::test]
async fn payment_failed_mail_only_for_a_team_on_the_team_plan(pool: PgPool) {
    let (app, fake, _) = setup(pool).await;
    // Never completed checkout: the team is still Free.
    set_status(&fake, "past_due");
    deliver(&app, &invoice("evt_1", "invoice.payment_failed")).await;
    assert!(row(&app).await.past_due_since.is_some());
    assert!(mails(&app, "We couldn't take%").await.is_empty());
}

#[sqlx::test]
async fn subscription_for_deleted_team_is_cancelled(pool: PgPool) {
    let (app, fake, team) = setup(pool).await;
    let (status, _) = app
        .request(Method::DELETE, "/v1/teams/acme", None, Some(&team.ada))
        .await;
    assert_eq!(status, StatusCode::NO_CONTENT);
    let (status, _) = deliver(&app, &checkout("evt_1")).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(*fake.cancelled.lock().unwrap(), ["sub_1"]);
}

#[sqlx::test]
async fn second_subscription_for_a_team_is_cancelled(pool: PgPool) {
    let (app, fake, _team) = setup(pool).await;
    deliver(&app, &checkout("evt_1")).await;
    fake.subscription.lock().unwrap().id = "sub_2".into();
    let second = event(
        "evt_2",
        "checkout.session.completed",
        json!({"id": "cs_2", "subscription": "sub_2"}),
    );
    let (status, _) = deliver(&app, &second).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(
        row(&app).await.stripe_subscription_id.as_deref(),
        Some("sub_1")
    );
    assert_eq!(*fake.cancelled.lock().unwrap(), ["sub_2"]);
}

#[sqlx::test]
async fn seat_sync_on_missing_subscription_applies_missing(pool: PgPool) {
    let (app, fake, team) = setup(pool).await;
    deliver(&app, &checkout("evt_1")).await;
    add_member(&app, &team.ada, "jane@example.com").await;
    fake.not_found_on_quantity.store(true, Ordering::SeqCst);
    tick(&app.state, app.clock.now()).await;
    let r = row(&app).await;
    assert_eq!(r.plan, "free");
    assert_eq!(r.stripe_subscription_id, None);
    assert_eq!(r.billing_status.as_deref(), Some("canceled"));
}
