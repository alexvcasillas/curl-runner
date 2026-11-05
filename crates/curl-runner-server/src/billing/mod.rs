//! Stripe billing: the [`Billing`] seam, the REST client behind it and the
//! billing endpoints. Billing is off unless all four `STRIPE_*` variables are set.

pub mod effective;
pub mod routes;
pub mod stripe;
pub mod sync;
pub mod webhook;

use std::fmt;
use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use uuid::Uuid;

/// What Checkout needs to start a subscription for a team.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CheckoutRequest {
    pub team_id: Uuid,
    pub team_slug: String,
    /// The team's Stripe customer, when it has one.
    pub customer: Option<String>,
    /// The paying owner's email, used when there is no customer yet.
    pub email: String,
    pub price: String,
    pub quantity: u32,
    pub success_url: String,
    pub cancel_url: String,
}

/// A Stripe subscription, reduced to what this server reads.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Subscription {
    pub id: String,
    pub customer: String,
    pub status: String,
    pub price: String,
    pub quantity: u32,
    /// From the subscription's `team_id` metadata.
    pub team_id: Option<Uuid>,
    /// Unix seconds.
    pub cancel_at: Option<i64>,
    /// Unix seconds.
    pub current_period_end: Option<i64>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Price {
    /// In the currency's smallest unit.
    pub unit_amount: i64,
    pub currency: String,
    /// Stripe's `month` or `year`.
    pub interval: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BillingError {
    /// Stripe answered with an error. `body` has the key removed.
    Api { status: u16, body: String },
    /// No usable answer: connection, timeout or an unreadable body.
    Unreachable(String),
}

impl fmt::Display for BillingError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Api { status, body } => write!(f, "stripe returned {status}: {body}"),
            Self::Unreachable(why) => write!(f, "stripe unreachable: {why}"),
        }
    }
}

impl std::error::Error for BillingError {}

#[async_trait]
pub trait Billing: Send + Sync {
    /// A Checkout Session URL for a new subscription.
    async fn checkout_url(&self, req: CheckoutRequest) -> Result<String, BillingError>;
    /// A Customer Portal session URL.
    async fn portal_url(&self, customer: &str, return_url: &str) -> Result<String, BillingError>;
    async fn subscription(&self, id: &str) -> Result<Subscription, BillingError>;
    /// Set the seat count, prorating the change.
    async fn set_quantity(&self, subscription: &str, quantity: u32) -> Result<(), BillingError>;
    /// Cancel immediately (`DELETE /v1/subscriptions/{id}`).
    async fn cancel_subscription(&self, id: &str) -> Result<(), BillingError>;
    async fn price(&self, id: &str) -> Result<Price, BillingError>;
    /// Email the customer's newest open invoice again
    /// (`GET /v1/invoices?customer=&status=open&limit=1`, then
    /// `POST /v1/invoices/{id}/send`). Its id, or `None` when there is none.
    async fn send_latest_invoice(&self, customer: &str) -> Result<Option<String>, BillingError>;
}

/// Log a Stripe failure from background work.
pub(crate) fn stripe_down_log(err: &BillingError) {
    tracing::warn!("stripe request failed: {err}");
}

/// The public price list, as `GET /v1/billing/prices` answers it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Prices {
    pub monthly: Price,
    pub annual: Price,
}

/// How long fetched prices are served from memory.
pub const PRICE_TTL: chrono::Duration = chrono::Duration::hours(1);

/// In-memory price list. Single-instance by design, like the rate limiter.
#[derive(Clone, Default)]
pub struct PriceCache(Arc<Mutex<Option<CachedPrices>>>);

type CachedPrices = (DateTime<Utc>, Prices);

impl PriceCache {
    /// The cached prices, unless older than [`PRICE_TTL`] at `now`.
    pub fn get(&self, now: DateTime<Utc>) -> Option<Prices> {
        let guard = self.0.lock().unwrap_or_else(|e| e.into_inner());
        guard
            .as_ref()
            .filter(|(at, _)| now < *at + PRICE_TTL)
            .map(|(_, prices)| prices.clone())
    }

    pub fn put(&self, now: DateTime<Utc>, prices: Prices) {
        *self.0.lock().unwrap_or_else(|e| e.into_inner()) = Some((now, prices));
    }
}
