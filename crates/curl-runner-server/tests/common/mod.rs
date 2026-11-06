//! Shared harness for the DB-backed integration tests. Each test file uses a
//! subset of it, hence `dead_code`.
#![allow(dead_code)]

use std::collections::HashMap;
use std::sync::Arc;
use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};

use axum::Router;
use axum::body::Body;
use axum::http::{Method, Request, StatusCode, header};
use chrono::{DateTime, Utc};
use curl_runner_server::auth::providers::FakeProviders;
use curl_runner_server::billing::{Billing, BillingError, CheckoutRequest, Price, Subscription};
use curl_runner_server::clock::FixedClock;
use curl_runner_server::config::Config;
use curl_runner_server::{AppState, app};
use http_body_util::BodyExt;
use serde_json::Value;
use sqlx::PgPool;
use tower::ServiceExt;

/// The fixed "now" every test starts at.
pub const NOW: &str = "2026-11-04T10:00:00Z";

pub struct TestApp {
    pub state: AppState,
    pub clock: Arc<FixedClock>,
    /// What the OAuth providers answer; set per test.
    pub providers: Arc<FakeProviders>,
    router: Router,
}

/// A config with only the required values, plus `overrides`.
pub fn test_config(overrides: &[(&str, &str)]) -> Config {
    let mut env: HashMap<&str, &str> = HashMap::from([
        ("DATABASE_URL", "postgres://test@localhost/test"),
        ("SECRET_KEY", "test-secret-key-test-secret-key-0123"),
        // Open by default so any signed-in test user can create a team.
        ("SIGNUP", "open"),
        // Free limits by default, since most tests exercise them.
        ("DEFAULT_PLAN", "free"),
    ]);
    env.extend(overrides.iter().copied());
    Config::from_env(|k| env.get(k).map(|v| (*v).to_owned())).expect("test config")
}

/// The app with [`test_config`] defaults. `pool` comes from `#[sqlx::test]`,
/// which has already run the migrations.
pub async fn test_app(pool: PgPool) -> TestApp {
    test_app_with(pool, test_config(&[])).await
}

/// The app with a custom config. The clock starts at [`NOW`]; the mailer only
/// writes to the outbox table, so tests read sent mail from there.
pub async fn test_app_with(pool: PgPool, config: Config) -> TestApp {
    let start: DateTime<Utc> = NOW.parse().unwrap();
    let clock = Arc::new(FixedClock::new(start));
    let mut state = AppState::new(pool, config, clock.clone());
    let providers = Arc::new(FakeProviders::new());
    state.oauth = providers.clone();
    TestApp {
        providers,
        router: app(state.clone()),
        state,
        clock,
    }
}

impl TestApp {
    /// Send one request. The body is parsed as JSON; an empty body is `Null`.
    pub async fn request(
        &self,
        method: Method,
        path: &str,
        json: Option<Value>,
        bearer: Option<&str>,
    ) -> (StatusCode, Value) {
        let mut req = Request::builder().method(method).uri(path);
        if let Some(token) = bearer {
            req = req.header(header::AUTHORIZATION, format!("Bearer {token}"));
        }
        let body = match json {
            Some(value) => {
                req = req.header(header::CONTENT_TYPE, "application/json");
                Body::from(value.to_string())
            }
            None => Body::empty(),
        };
        let res = self
            .router
            .clone()
            .oneshot(req.body(body).unwrap())
            .await
            .unwrap();
        let status = res.status();
        let bytes = res.into_body().collect().await.unwrap().to_bytes();
        let value = if bytes.is_empty() {
            Value::Null
        } else {
            serde_json::from_slice(&bytes)
                .unwrap_or_else(|_| Value::String(String::from_utf8_lossy(&bytes).into_owned()))
        };
        (status, value)
    }

    /// Ask for a sign-in code and read it, and the link token (web only), from
    /// the newest queued mail to `email`. Mail bodies are intact until the worker runs.
    pub async fn start_email(&self, email: &str, client: &str) -> (String, Option<String>) {
        let (status, _) = self
            .request(
                Method::POST,
                "/v1/auth/email/start",
                Some(serde_json::json!({"email": email, "client": client})),
                None,
            )
            .await;
        assert_eq!(status, StatusCode::ACCEPTED);
        let body: String = sqlx::query_scalar(
            "SELECT text_body FROM outbox_emails WHERE to_addr = $1 ORDER BY ctid DESC LIMIT 1",
        )
        .bind(email.to_lowercase())
        .fetch_one(&self.state.db)
        .await
        .unwrap();
        let code = body
            .split("sign-in code is ")
            .nth(1)
            .and_then(|r| r.get(..6))
            .expect("code in mail")
            .to_owned();
        let token = body
            .split("token=")
            .nth(1)
            .and_then(|r| r.split('&').next())
            .map(str::to_owned);
        (code, token)
    }

    /// Sign `email` in (start, read the code, verify) and return the session token.
    pub async fn sign_in(&self, email: &str) -> String {
        let (code, _) = self.start_email(email, "web").await;
        let (status, body) = self
            .request(
                Method::POST,
                "/v1/auth/email/verify",
                Some(serde_json::json!({"email": email, "code": code})),
                None,
            )
            .await;
        assert_eq!(status, StatusCode::OK, "{body}");
        body["session"].as_str().expect("session token").to_owned()
    }

    /// POST a JSON body with an `X-Forwarded-For` header.
    pub async fn request_with_ip(
        &self,
        path: &str,
        json: Value,
        forwarded: &str,
    ) -> (StatusCode, Value) {
        let req = Request::builder()
            .method(Method::POST)
            .uri(path)
            .header(header::CONTENT_TYPE, "application/json")
            .header("x-forwarded-for", forwarded)
            .body(Body::from(json.to_string()))
            .unwrap();
        self.send(req).await
    }

    /// POST a raw (possibly malformed) body labelled as JSON.
    pub async fn request_raw(&self, method: Method, path: &str, body: &str) -> (StatusCode, Value) {
        let req = Request::builder()
            .method(method)
            .uri(path)
            .header(header::CONTENT_TYPE, "application/json")
            .body(Body::from(body.to_owned()))
            .unwrap();
        self.send(req).await
    }

    async fn send(&self, req: Request<Body>) -> (StatusCode, Value) {
        let res = self.router.clone().oneshot(req).await.unwrap();
        let status = res.status();
        let bytes = res.into_body().collect().await.unwrap().to_bytes();
        let value = if bytes.is_empty() {
            Value::Null
        } else {
            serde_json::from_slice(&bytes)
                .unwrap_or_else(|_| Value::String(String::from_utf8_lossy(&bytes).into_owned()))
        };
        (status, value)
    }

    /// POST a raw body with the given headers.
    pub async fn post_raw(
        &self,
        path: &str,
        body: &str,
        headers: &[(&str, &str)],
    ) -> (StatusCode, Value) {
        let mut req = Request::builder().method(Method::POST).uri(path);
        for (name, value) in headers {
            req = req.header(*name, *value);
        }
        self.send(req.body(Body::from(body.to_owned())).unwrap())
            .await
    }

    /// GET with a raw `Authorization` header value.
    pub async fn request_with_auth(&self, path: &str, authorization: &str) -> (StatusCode, Value) {
        let req = Request::builder()
            .uri(path)
            .header(header::AUTHORIZATION, authorization)
            .body(Body::empty())
            .unwrap();
        self.send(req).await
    }

    /// Create team `slug` (named after it) as the user behind `session`.
    /// Needs a config under which that user may create teams (the default).
    pub async fn create_team(&self, session: &str, slug: &str) {
        let (status, body) = self
            .request(
                Method::POST,
                "/v1/teams",
                Some(serde_json::json!({"slug": slug, "name": slug})),
                Some(session),
            )
            .await;
        assert_eq!(status, StatusCode::CREATED, "{body}");
    }
}

/// The four `STRIPE_*` settings that turn billing on.
pub const BILLING_ENV: [(&str, &str); 4] = [
    ("STRIPE_SECRET_KEY", "sk_test_fake"),
    ("STRIPE_WEBHOOK_SECRET", "whsec_fake"),
    ("STRIPE_PRICE_MONTHLY", "price_monthly"),
    ("STRIPE_PRICE_ANNUAL", "price_annual"),
];

/// Test double for Stripe: records every call, answers canned values.
pub struct FakeBilling {
    pub checkouts: Mutex<Vec<CheckoutRequest>>,
    /// `(customer, return_url)` of each portal call.
    pub portals: Mutex<Vec<(String, String)>>,
    pub quantities: Mutex<Vec<(String, u32)>>,
    pub cancelled: Mutex<Vec<String>>,
    /// The customer of each `send_latest_invoice` call.
    pub invoices_sent: Mutex<Vec<String>>,
    /// `send_latest_invoice` finds no invoice.
    pub no_invoice: AtomicBool,
    pub price_calls: AtomicU32,
    pub subscription_calls: AtomicU32,
    /// Make every call fail like an unreachable Stripe.
    pub fail: AtomicBool,
    /// Answer `subscription` like Stripe's 404 for an unknown id.
    pub missing: AtomicBool,
    /// Answer `set_quantity` like Stripe's 404 for an unknown id.
    pub not_found_on_quantity: AtomicBool,
    /// When set, the next `subscription` call answers its current state, then
    /// waits (after signalling `entered`) until `release` fires: a slow read.
    pub gate: Mutex<Option<(Arc<tokio::sync::Notify>, Arc<tokio::sync::Notify>)>>,
    pub subscription: Mutex<Subscription>,
}

impl FakeBilling {
    pub fn new() -> Self {
        Self {
            checkouts: Mutex::default(),
            portals: Mutex::default(),
            quantities: Mutex::default(),
            cancelled: Mutex::default(),
            invoices_sent: Mutex::default(),
            no_invoice: AtomicBool::new(false),
            price_calls: AtomicU32::new(0),
            subscription_calls: AtomicU32::new(0),
            fail: AtomicBool::new(false),
            missing: AtomicBool::new(false),
            not_found_on_quantity: AtomicBool::new(false),
            gate: Mutex::new(None),
            subscription: Mutex::new(Subscription {
                id: "sub_1".into(),
                customer: "cus_1".into(),
                status: "active".into(),
                price: "price_monthly".into(),
                quantity: 1,
                team_id: None,
                cancel_at: None,
                current_period_end: Some(1_793_000_000),
            }),
        }
    }

    fn check(&self) -> Result<(), BillingError> {
        if self.fail.load(Ordering::SeqCst) {
            Err(BillingError::Unreachable("fake failure".into()))
        } else {
            Ok(())
        }
    }
}

#[async_trait::async_trait]
impl Billing for FakeBilling {
    async fn checkout_url(&self, req: CheckoutRequest) -> Result<String, BillingError> {
        self.check()?;
        self.checkouts.lock().unwrap().push(req);
        Ok("https://checkout.stripe.test/c/1".into())
    }

    async fn portal_url(&self, customer: &str, return_url: &str) -> Result<String, BillingError> {
        self.check()?;
        self.portals
            .lock()
            .unwrap()
            .push((customer.to_owned(), return_url.to_owned()));
        Ok("https://billing.stripe.test/p/1".into())
    }

    async fn subscription(&self, _id: &str) -> Result<Subscription, BillingError> {
        self.check()?;
        self.subscription_calls.fetch_add(1, Ordering::SeqCst);
        if self.missing.load(Ordering::SeqCst) {
            return Err(BillingError::Api {
                status: 404,
                body: "resource_missing".into(),
            });
        }
        let answer = self.subscription.lock().unwrap().clone();
        let gate = self.gate.lock().unwrap().take();
        if let Some((entered, release)) = gate {
            entered.notify_one();
            release.notified().await;
        }
        Ok(answer)
    }

    async fn set_quantity(&self, subscription: &str, quantity: u32) -> Result<(), BillingError> {
        self.check()?;
        if self.not_found_on_quantity.load(Ordering::SeqCst) {
            return Err(BillingError::Api {
                status: 404,
                body: "resource_missing".into(),
            });
        }
        self.quantities
            .lock()
            .unwrap()
            .push((subscription.to_owned(), quantity));
        Ok(())
    }

    async fn cancel_subscription(&self, id: &str) -> Result<(), BillingError> {
        self.check()?;
        self.cancelled.lock().unwrap().push(id.to_owned());
        Ok(())
    }

    async fn send_latest_invoice(&self, customer: &str) -> Result<Option<String>, BillingError> {
        self.check()?;
        if self.no_invoice.load(Ordering::SeqCst) {
            return Ok(None);
        }
        self.invoices_sent.lock().unwrap().push(customer.to_owned());
        Ok(Some("in_1".into()))
    }

    async fn price(&self, id: &str) -> Result<Price, BillingError> {
        self.check()?;
        self.price_calls.fetch_add(1, Ordering::SeqCst);
        let annual = id == "price_annual";
        Ok(Price {
            unit_amount: if annual { 9_000 } else { 900 },
            currency: "usd".into(),
            interval: if annual { "year" } else { "month" }.into(),
        })
    }
}

/// The app with billing configured and a [`FakeBilling`] behind it.
pub async fn test_app_billing(pool: PgPool) -> (TestApp, Arc<FakeBilling>) {
    test_app_billing_with(pool, test_config(&BILLING_ENV)).await
}

/// Like [`test_app_billing`] with a custom config, which must turn billing on.
pub async fn test_app_billing_with(pool: PgPool, config: Config) -> (TestApp, Arc<FakeBilling>) {
    let app = test_app_with(pool, config).await;
    let fake = Arc::new(FakeBilling::new());
    let mut state = app.state.clone();
    state.billing = Some(fake.clone());
    let app = TestApp {
        router: curl_runner_server::app(state.clone()),
        state,
        ..app
    };
    (app, fake)
}
