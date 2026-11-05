//! The billing endpoints. Each answers `404 not_found` when billing is off.

use std::sync::Arc;

use axum::extract::{Path, State};
use axum::routing::{get, post};
use axum::{Json, Router};
use chrono::{DateTime, Utc};
use curl_runner_cloud_proto::{ErrorCode, Plan};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::{Billing, BillingError, CheckoutRequest, Price, Prices};
use crate::auth::UserAuth;
use crate::auth::session::column_enum;
use crate::config::BillingConfig;
use crate::error::ApiError;
use crate::extract::ApiJson;
use crate::state::AppState;
use crate::teams::team_access;

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/v1/billing/prices", get(prices))
        .route("/v1/teams/{team}/billing", get(summary))
        .route("/v1/teams/{team}/billing/checkout", post(checkout))
        .route("/v1/teams/{team}/billing/portal", post(portal))
}

/// The billing client and its config, or `404` when billing is off.
fn enabled(state: &AppState) -> Result<(&Arc<dyn Billing>, &BillingConfig), ApiError> {
    match (&state.billing, &state.config.billing) {
        (Some(billing), Some(config)) => Ok((billing, config)),
        _ => Err(ApiError::not_found()),
    }
}

/// `503` for any Stripe failure. The body goes to the log (the key is already
/// scrubbed from it); the caller sees a fixed sentence.
fn stripe_down(err: BillingError) -> ApiError {
    tracing::warn!("stripe request failed: {err}");
    ApiError::new(
        ErrorCode::Unavailable,
        "Billing is not reachable right now. Try again in a minute.",
    )
}

#[derive(Serialize)]
struct PriceBody {
    unit_amount: i64,
    currency: String,
}

#[derive(Serialize)]
struct PricesBody {
    monthly: PriceBody,
    annual: PriceBody,
}

impl From<Price> for PriceBody {
    fn from(p: Price) -> Self {
        Self {
            unit_amount: p.unit_amount,
            currency: p.currency,
        }
    }
}

/// Public. Served from memory for an hour.
async fn prices(State(state): State<AppState>) -> Result<Json<PricesBody>, ApiError> {
    let (billing, config) = enabled(&state)?;
    let now = state.clock.now();
    let prices = match state.price_cache.get(now) {
        Some(cached) => cached,
        None => {
            let fresh = Prices {
                monthly: billing
                    .price(&config.price_monthly)
                    .await
                    .map_err(stripe_down)?,
                annual: billing
                    .price(&config.price_annual)
                    .await
                    .map_err(stripe_down)?,
            };
            state.price_cache.put(now, fresh.clone());
            fresh
        }
    };
    Ok(Json(PricesBody {
        monthly: prices.monthly.into(),
        annual: prices.annual.into(),
    }))
}

#[derive(sqlx::FromRow)]
struct BillingRow {
    plan: String,
    billing_interval: Option<String>,
    billing_status: Option<String>,
    past_due_since: Option<DateTime<Utc>>,
    cancel_at: Option<DateTime<Utc>>,
    current_period_end: Option<DateTime<Utc>>,
    stripe_customer_id: Option<String>,
    stripe_subscription_id: Option<String>,
}

async fn billing_row(state: &AppState, team_id: Uuid) -> Result<BillingRow, ApiError> {
    Ok(sqlx::query_as(
        "SELECT plan, billing_interval, billing_status, past_due_since, cancel_at, \
         current_period_end, stripe_customer_id, stripe_subscription_id \
         FROM teams WHERE id = $1",
    )
    .bind(team_id)
    .fetch_one(&state.db)
    .await?)
}

async fn member_count(state: &AppState, team_id: Uuid) -> Result<u32, ApiError> {
    let n: i64 = sqlx::query_scalar("SELECT count(*) FROM memberships WHERE team_id = $1")
        .bind(team_id)
        .fetch_one(&state.db)
        .await?;
    Ok(u32::try_from(n).unwrap_or(u32::MAX))
}

#[derive(Serialize)]
struct SummaryBody {
    plan: Plan,
    interval: Option<String>,
    status: Option<String>,
    seats: u32,
    past_due_since: Option<DateTime<Utc>>,
    can_manage: bool,
    cancel_at: Option<DateTime<Utc>>,
    current_period_end: Option<DateTime<Utc>>,
    /// `stripe` when a Stripe subscription backs the plan, else `none`.
    plan_source: &'static str,
}

/// Members only.
async fn summary(
    State(state): State<AppState>,
    auth: UserAuth,
    Path(slug): Path<String>,
) -> Result<Json<SummaryBody>, ApiError> {
    enabled(&state)?;
    let access = team_access(&state.db, auth.user_id, &slug).await?;
    let row = billing_row(&state, access.team_id).await?;
    Ok(Json(SummaryBody {
        plan: column_enum(row.plan)?,
        interval: row.billing_interval,
        status: row.billing_status,
        seats: member_count(&state, access.team_id).await?,
        past_due_since: row.past_due_since,
        can_manage: access.role == curl_runner_cloud_proto::Role::Owner,
        cancel_at: row.cancel_at,
        current_period_end: row.current_period_end,
        plan_source: if row.stripe_subscription_id.is_some() {
            "stripe"
        } else {
            "none"
        },
    }))
}

#[derive(Deserialize)]
#[serde(rename_all = "snake_case")]
enum Interval {
    Monthly,
    Annual,
}

#[derive(Deserialize)]
struct CheckoutBody {
    interval: Interval,
}

#[derive(Serialize)]
struct UrlBody {
    url: String,
}

/// Statuses after which a subscription is gone for good.
fn is_live(status: Option<&str>) -> bool {
    !matches!(status, Some("canceled" | "incomplete_expired"))
}

/// Owner only.
async fn checkout(
    State(state): State<AppState>,
    auth: UserAuth,
    Path(slug): Path<String>,
    ApiJson(body): ApiJson<CheckoutBody>,
) -> Result<Json<UrlBody>, ApiError> {
    let (billing, config) = enabled(&state)?;
    let access = team_access(&state.db, auth.user_id, &slug).await?;
    access.require_owner()?;
    let row = billing_row(&state, access.team_id).await?;
    if row.stripe_subscription_id.is_some() && is_live(row.billing_status.as_deref()) {
        return Err(ApiError::new(
            ErrorCode::Conflict,
            "This team already has a subscription. Manage it from the portal.",
        ));
    }
    let price = match body.interval {
        Interval::Monthly => &config.price_monthly,
        Interval::Annual => &config.price_annual,
    };
    let app_url = &state.config.app_url;
    let url = billing
        .checkout_url(CheckoutRequest {
            team_id: access.team_id,
            team_slug: slug.clone(),
            customer: row.stripe_customer_id,
            email: auth.email,
            price: price.clone(),
            quantity: member_count(&state, access.team_id).await?.max(1),
            success_url: format!("{app_url}/t/{slug}/billing?checkout=done"),
            cancel_url: format!("{app_url}/t/{slug}/billing?checkout=cancelled"),
        })
        .await
        .map_err(stripe_down)?;
    Ok(Json(UrlBody { url }))
}

/// Owner only. Needs a Stripe customer, which the first checkout creates.
async fn portal(
    State(state): State<AppState>,
    auth: UserAuth,
    Path(slug): Path<String>,
) -> Result<Json<UrlBody>, ApiError> {
    let (billing, _) = enabled(&state)?;
    let access = team_access(&state.db, auth.user_id, &slug).await?;
    access.require_owner()?;
    let row = billing_row(&state, access.team_id).await?;
    let customer = row.stripe_customer_id.ok_or_else(|| {
        ApiError::new(
            ErrorCode::Conflict,
            "This team has no billing account yet. Subscribe first.",
        )
    })?;
    let return_url = format!("{}/t/{slug}/billing", state.config.app_url);
    let url = billing
        .portal_url(&customer, &return_url)
        .await
        .map_err(stripe_down)?;
    Ok(Json(UrlBody { url }))
}
