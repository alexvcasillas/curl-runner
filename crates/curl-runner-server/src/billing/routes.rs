//! The billing endpoints. Each answers `404 not_found` when billing is off.

use std::sync::Arc;

use axum::extract::{Path, State};
use axum::routing::{get, post};
use axum::{Json, Router};
use chrono::{DateTime, Utc};
use curl_runner_cloud_proto::{ErrorCode, Plan};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::effective::{PlanRow, PlanSource, effective_plan, plan_columns};
use super::mrr::is_live_subscription;
use super::{Billing, BillingError, CheckoutRequest, Price, Prices};
use crate::auth::UserAuth;
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
pub(crate) fn stripe_down(err: BillingError) -> ApiError {
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

/// The price list, from memory when fetched within the last hour, so at
/// most one pair of Stripe reads an hour. `None` when billing is off.
pub(crate) async fn current_prices(state: &AppState) -> Result<Option<Prices>, BillingError> {
    let (Some(billing), Some(config)) = (&state.billing, &state.config.billing) else {
        return Ok(None);
    };
    let now = state.clock.now();
    if let Some(cached) = state.price_cache.get(now) {
        return Ok(Some(cached));
    }
    let fresh = Prices {
        monthly: billing.price(&config.price_monthly).await?,
        annual: billing.price(&config.price_annual).await?,
    };
    state.price_cache.put(now, fresh.clone());
    Ok(Some(fresh))
}

/// Public. Served from memory for an hour.
async fn prices(State(state): State<AppState>) -> Result<Json<PricesBody>, ApiError> {
    enabled(&state)?;
    let prices = current_prices(&state)
        .await
        .map_err(stripe_down)?
        .ok_or_else(ApiError::not_found)?;
    Ok(Json(PricesBody {
        monthly: prices.monthly.into(),
        annual: prices.annual.into(),
    }))
}

#[derive(sqlx::FromRow)]
struct BillingRow {
    #[sqlx(flatten)]
    plan: PlanRow,
    billing_interval: Option<String>,
    billing_status: Option<String>,
    past_due_since: Option<DateTime<Utc>>,
    cancel_at: Option<DateTime<Utc>>,
    current_period_end: Option<DateTime<Utc>>,
    stripe_customer_id: Option<String>,
}

async fn billing_row(state: &AppState, team_id: Uuid) -> Result<BillingRow, ApiError> {
    Ok(sqlx::query_as(&format!(
        "SELECT {}, t.billing_interval, t.billing_status, t.past_due_since, t.cancel_at, \
         t.current_period_end, t.stripe_customer_id \
         FROM teams t WHERE t.id = $1",
        plan_columns("t")
    ))
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
    /// `override` while an admin's override is in force, else `stripe` when
    /// a Stripe subscription backs the plan, else `none`.
    plan_source: PlanSource,
    /// When the override ends; `null` with no override or no end date.
    override_until: Option<DateTime<Utc>>,
    /// A live Stripe subscription exists, whatever `plan_source` says: an
    /// override can sit on top of one that keeps billing.
    has_subscription: bool,
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
    let now = state.clock.now();
    let (plan, plan_source) = effective_plan(&row.plan, now);
    let has_subscription = is_live_subscription(
        row.plan.stripe_subscription_id.as_deref(),
        row.billing_status.as_deref(),
    );
    Ok(Json(SummaryBody {
        plan,
        interval: row.billing_interval,
        status: row.billing_status,
        seats: member_count(&state, access.team_id).await?,
        past_due_since: row.past_due_since,
        can_manage: access.role == curl_runner_cloud_proto::Role::Owner,
        cancel_at: row.cancel_at,
        current_period_end: row.current_period_end,
        plan_source,
        override_until: if plan_source == PlanSource::Override {
            row.plan.override_until
        } else {
            None
        },
        has_subscription,
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
    if is_live_subscription(
        row.plan.stripe_subscription_id.as_deref(),
        row.billing_status.as_deref(),
    ) {
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
