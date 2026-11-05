#![cfg(feature = "db-tests")]

mod common;

use std::sync::atomic::Ordering;

use axum::http::{Method, StatusCode};
use chrono::Duration;
use common::{FakeBilling, test_app, test_app_billing};
use curl_runner_server::billing::Billing;
use serde_json::json;
use sqlx::PgPool;

/// Sign `email` in and make them a member of `slug` directly.
async fn join(app: &common::TestApp, email: &str, slug: &str) -> String {
    let session = app.sign_in(email).await;
    sqlx::query(
        "INSERT INTO memberships (team_id, user_id, role, created_at) \
         SELECT t.id, u.id, 'member', now() FROM teams t, users u \
         WHERE t.slug = $1 AND u.email = $2",
    )
    .bind(slug)
    .bind(email)
    .execute(&app.state.db)
    .await
    .unwrap();
    session
}

#[sqlx::test]
async fn billing_off_hides_endpoints_and_info_false(pool: PgPool) {
    let app = test_app(pool).await;
    let s = app.sign_in("ada@example.com").await;
    app.create_team(&s, "acme").await;
    let (_, info) = app.request(Method::GET, "/v1/info", None, None).await;
    assert_eq!(info["billing"], false);
    for (method, path) in [
        (Method::GET, "/v1/billing/prices"),
        (Method::GET, "/v1/teams/acme/billing"),
        (Method::POST, "/v1/teams/acme/billing/checkout"),
        (Method::POST, "/v1/teams/acme/billing/portal"),
    ] {
        let body = (method == Method::POST).then(|| json!({"interval": "monthly"}));
        let (status, res) = app.request(method, path, body, Some(&s)).await;
        assert_eq!(status, StatusCode::NOT_FOUND, "{path}");
        assert_eq!(res["error"], "not_found");
    }
}

#[sqlx::test]
async fn info_billing_true_when_configured(pool: PgPool) {
    let (app, _) = test_app_billing(pool).await;
    let (_, info) = app.request(Method::GET, "/v1/info", None, None).await;
    assert_eq!(info["billing"], true);
}

#[sqlx::test]
async fn prices_cached_one_hour(pool: PgPool) {
    let (app, fake) = test_app_billing(pool).await;
    let (status, body) = app
        .request(Method::GET, "/v1/billing/prices", None, None)
        .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(
        body,
        json!({
            "monthly": {"unit_amount": 900, "currency": "usd"},
            "annual": {"unit_amount": 9000, "currency": "usd"},
        })
    );
    app.request(Method::GET, "/v1/billing/prices", None, None)
        .await;
    // One fetch per price, once.
    assert_eq!(fake.price_calls.load(Ordering::SeqCst), 2);
    app.clock.advance(Duration::minutes(61));
    app.request(Method::GET, "/v1/billing/prices", None, None)
        .await;
    assert_eq!(fake.price_calls.load(Ordering::SeqCst), 4);
}

#[sqlx::test]
async fn checkout_owner_only_sends_member_count_and_team_id(pool: PgPool) {
    let (app, fake) = test_app_billing(pool).await;
    let ada = app.sign_in("ada@example.com").await;
    app.create_team(&ada, "acme").await;
    let bob = join(&app, "bob@example.com", "acme").await;
    let outsider = app.sign_in("eve@example.com").await;
    let path = "/v1/teams/acme/billing/checkout";
    let body = || Some(json!({"interval": "annual"}));

    let (status, res) = app.request(Method::POST, path, body(), Some(&bob)).await;
    assert_eq!(status, StatusCode::FORBIDDEN, "{res}");
    let (status, _) = app
        .request(Method::POST, path, body(), Some(&outsider))
        .await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    assert!(fake.checkouts.lock().unwrap().is_empty());

    let (status, res) = app.request(Method::POST, path, body(), Some(&ada)).await;
    assert_eq!(status, StatusCode::OK, "{res}");
    assert_eq!(res["url"], "https://checkout.stripe.test/c/1");
    let calls = fake.checkouts.lock().unwrap();
    let req = &calls[0];
    let team_id: uuid::Uuid = sqlx::query_scalar("SELECT id FROM teams WHERE slug = 'acme'")
        .fetch_one(&app.state.db)
        .await
        .unwrap();
    assert_eq!(req.team_id, team_id);
    assert_eq!(req.team_slug, "acme");
    assert_eq!(req.quantity, 2);
    assert_eq!(req.price, "price_annual");
    assert_eq!(req.email, "ada@example.com");
    assert_eq!(req.customer, None);
    assert_eq!(
        req.success_url,
        "http://localhost:3000/t/acme/billing?checkout=done"
    );
    assert_eq!(
        req.cancel_url,
        "http://localhost:3000/t/acme/billing?checkout=cancelled"
    );
}

#[sqlx::test]
async fn checkout_conflict_when_subscribed(pool: PgPool) {
    let (app, fake) = test_app_billing(pool).await;
    let ada = app.sign_in("ada@example.com").await;
    app.create_team(&ada, "acme").await;
    sqlx::query(
        "UPDATE teams SET plan = 'team', stripe_customer_id = 'cus_1', \
         stripe_subscription_id = 'sub_1', billing_status = 'active'",
    )
    .execute(&app.state.db)
    .await
    .unwrap();
    let (status, res) = app
        .request(
            Method::POST,
            "/v1/teams/acme/billing/checkout",
            Some(json!({"interval": "monthly"})),
            Some(&ada),
        )
        .await;
    assert_eq!(status, StatusCode::CONFLICT);
    assert_eq!(res["error"], "conflict");
    assert_eq!(
        res["message"],
        "This team already has a subscription. Manage it from the portal."
    );
    assert!(fake.checkouts.lock().unwrap().is_empty());
}

#[sqlx::test]
async fn portal_needs_customer(pool: PgPool) {
    let (app, fake) = test_app_billing(pool).await;
    let ada = app.sign_in("ada@example.com").await;
    app.create_team(&ada, "acme").await;
    let bob = join(&app, "bob@example.com", "acme").await;
    let path = "/v1/teams/acme/billing/portal";

    let (status, _) = app.request(Method::POST, path, None, Some(&ada)).await;
    assert_eq!(status, StatusCode::CONFLICT);
    sqlx::query("UPDATE teams SET stripe_customer_id = 'cus_1'")
        .execute(&app.state.db)
        .await
        .unwrap();
    let (status, _) = app.request(Method::POST, path, None, Some(&bob)).await;
    assert_eq!(status, StatusCode::FORBIDDEN);
    let (status, res) = app.request(Method::POST, path, None, Some(&ada)).await;
    assert_eq!(status, StatusCode::OK, "{res}");
    assert_eq!(res["url"], "https://billing.stripe.test/p/1");
    assert_eq!(
        fake.portals.lock().unwrap()[0],
        (
            "cus_1".to_owned(),
            "http://localhost:3000/t/acme/billing".to_owned()
        )
    );
}

#[sqlx::test]
async fn billing_summary_non_member_404(pool: PgPool) {
    let (app, _) = test_app_billing(pool).await;
    let ada = app.sign_in("ada@example.com").await;
    app.create_team(&ada, "acme").await;
    let eve = app.sign_in("eve@example.com").await;
    let (status, _) = app
        .request(Method::GET, "/v1/teams/acme/billing", None, Some(&eve))
        .await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    let bob = join(&app, "bob@example.com", "acme").await;
    let (status, res) = app
        .request(Method::GET, "/v1/teams/acme/billing", None, Some(&bob))
        .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(res["can_manage"], false);
    assert_eq!(res["seats"], 2);
    assert_eq!(res["plan"], "free");
}

#[sqlx::test]
async fn summary_has_plan_source_and_period_fields(pool: PgPool) {
    let (app, _) = test_app_billing(pool).await;
    let ada = app.sign_in("ada@example.com").await;
    app.create_team(&ada, "acme").await;
    let (_, res) = app
        .request(Method::GET, "/v1/teams/acme/billing", None, Some(&ada))
        .await;
    assert_eq!(
        res,
        json!({
            "plan": "free", "interval": null, "status": null, "seats": 1,
            "past_due_since": null, "can_manage": true, "cancel_at": null,
            "current_period_end": null, "plan_source": "none",
        })
    );
    sqlx::query(
        "UPDATE teams SET plan = 'team', stripe_subscription_id = 'sub_1', \
         billing_interval = 'annual', billing_status = 'past_due', \
         past_due_since = '2026-11-01T00:00:00Z', cancel_at = '2026-12-01T00:00:00Z', \
         current_period_end = '2026-12-01T00:00:00Z'",
    )
    .execute(&app.state.db)
    .await
    .unwrap();
    let (_, res) = app
        .request(Method::GET, "/v1/teams/acme/billing", None, Some(&ada))
        .await;
    assert_eq!(
        res,
        json!({
            "plan": "team", "interval": "annual", "status": "past_due", "seats": 1,
            "past_due_since": "2026-11-01T00:00:00Z", "can_manage": true,
            "cancel_at": "2026-12-01T00:00:00Z",
            "current_period_end": "2026-12-01T00:00:00Z", "plan_source": "stripe",
        })
    );
}

#[sqlx::test]
async fn stripe_error_is_503(pool: PgPool) {
    let (app, fake) = test_app_billing(pool).await;
    let ada = app.sign_in("ada@example.com").await;
    app.create_team(&ada, "acme").await;
    fake.fail.store(true, Ordering::SeqCst);
    let (status, res) = app
        .request(
            Method::POST,
            "/v1/teams/acme/billing/checkout",
            Some(json!({"interval": "monthly"})),
            Some(&ada),
        )
        .await;
    assert_eq!(status, StatusCode::SERVICE_UNAVAILABLE);
    assert_eq!(res["error"], "unavailable");
    assert_eq!(
        res["message"],
        "Billing is not reachable right now. Try again in a minute."
    );
    let (status, _) = app
        .request(Method::GET, "/v1/billing/prices", None, None)
        .await;
    assert_eq!(status, StatusCode::SERVICE_UNAVAILABLE);
}

#[tokio::test]
async fn cancel_subscription_is_recorded_by_fake() {
    let fake = FakeBilling::new();
    fake.cancel_subscription("sub_9").await.unwrap();
    assert_eq!(*fake.cancelled.lock().unwrap(), ["sub_9"]);
}
