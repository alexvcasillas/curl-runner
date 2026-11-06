//! Keeping the `teams` table in step with Stripe: applying a re-read
//! subscription, and the background tick for seat counts, the past-due
//! downgrade and plan overrides (the 14-day reminder and expiry).

use std::future::Future;
use std::time::Duration;

use chrono::{DateTime, Utc};
use curl_runner_cloud_proto::Plan;
use serde_json::json;
use sqlx::{FromRow, PgConnection};
use uuid::Uuid;

use super::effective::{PlanChange, PlanRow, effective_plan, parse_plan};
use super::mrr::Billed;
use super::routes::current_prices;
use super::{BillingError, Prices, Subscription, stripe_down_log};
use crate::error::ApiError;
use crate::mail::Mail;
use crate::state::AppState;
use curl_runner_cloud_proto::ErrorCode;

/// Time between ticks.
const TICK: Duration = Duration::from_secs(60);
/// How long before an override ends its owners are reminded.
pub const OVERRIDE_REMINDER: chrono::Duration = chrono::Duration::days(14);

/// Statuses after which a subscription is gone for good.
fn is_terminal(status: &str) -> bool {
    matches!(status, "canceled" | "incomplete_expired")
}

/// What a subscription status does to a team's plan and past-due clock.
#[derive(Debug, PartialEq, Eq)]
struct Outcome {
    plan: &'static str,
    past_due_since: Option<DateTime<Utc>>,
}

/// `plan` and `past_due_since` are the team's current values.
fn outcome(
    status: &str,
    plan: &'static str,
    past_due_since: Option<DateTime<Utc>>,
    now: DateTime<Utc>,
) -> Outcome {
    match status {
        "active" | "trialing" => Outcome {
            plan: "team",
            past_due_since: None,
        },
        "past_due" | "unpaid" => Outcome {
            plan,
            past_due_since: Some(past_due_since.map_or(now, |since| since.min(now))),
        },
        s if is_terminal(s) => Outcome {
            plan: "free",
            past_due_since: None,
        },
        // `incomplete`, `paused`: nothing to grant or take away yet.
        _ => Outcome {
            plan,
            past_due_since,
        },
    }
}

#[derive(FromRow)]
struct TeamRow {
    id: Uuid,
    name: String,
    slug: String,
    plan: String,
    past_due_since: Option<DateTime<Utc>>,
    stripe_subscription_id: Option<String>,
    billing_status: Option<String>,
    override_plan: Option<String>,
    override_until: Option<DateTime<Utc>>,
    billing_interval: Option<String>,
    seats_synced: Option<i32>,
}

impl TeamRow {
    /// The effective plan the team would have with `plan` and `subscription`.
    fn effective(&self, plan: &str, subscription: Option<&str>, now: DateTime<Utc>) -> Plan {
        let row = PlanRow {
            plan: plan.to_owned(),
            stripe_subscription_id: subscription.map(str::to_owned),
            override_plan: self.override_plan.clone(),
            override_until: self.override_until,
        };
        effective_plan(&row, now).0
    }
}

const TEAM_COLUMNS: &str = "id, name, slug::text AS slug, plan, past_due_since, \
                            stripe_subscription_id, billing_status, override_plan, override_until, \
                            billing_interval, seats_synced";

fn plan_str(plan: &str) -> &'static str {
    if plan == "team" { "team" } else { "free" }
}

async fn lock_team_by_id(tx: &mut PgConnection, id: Uuid) -> Result<Option<TeamRow>, ApiError> {
    Ok(sqlx::query_as(&format!(
        "SELECT {TEAM_COLUMNS} FROM teams WHERE id = $1 FOR UPDATE"
    ))
    .bind(id)
    .fetch_optional(tx)
    .await?)
}

async fn lock_team_by_subscription(
    tx: &mut PgConnection,
    subscription: &str,
) -> Result<Option<TeamRow>, ApiError> {
    Ok(sqlx::query_as(&format!(
        "SELECT {TEAM_COLUMNS} FROM teams WHERE stripe_subscription_id = $1 FOR UPDATE"
    ))
    .bind(subscription)
    .fetch_optional(tx)
    .await?)
}

/// Mail `make(email)` to every owner of `team_id`.
pub(crate) async fn mail_owners(
    state: &AppState,
    tx: &mut PgConnection,
    team_id: Uuid,
    make: impl Fn(String) -> Mail,
) -> Result<(), ApiError> {
    let owners: Vec<String> = sqlx::query_scalar(
        "SELECT u.email::text FROM memberships m JOIN users u ON u.id = m.user_id \
         WHERE m.team_id = $1 AND m.role = 'owner' ORDER BY u.email",
    )
    .bind(team_id)
    .fetch_all(&mut *tx)
    .await?;
    for to in owners {
        state.mailer.enqueue(tx, make(to)).await?;
    }
    Ok(())
}

async fn mail_plan_changed(
    state: &AppState,
    tx: &mut PgConnection,
    team: &TeamRow,
) -> Result<(), ApiError> {
    let settings = state.settings().await?;
    mail_owners(state, tx, team.id, |to| Mail::PlanChanged {
        to,
        team_name: team.name.clone(),
        slug: team.slug.clone(),
        app_url: state.config.app_url.clone(),
        free_member_limit: settings.free_member_limit,
        free_restricted: settings.free_restricted,
    })
    .await
}

fn interval_of(state: &AppState, price: &str) -> Option<&'static str> {
    let config = state.config.billing.as_ref()?;
    if price == config.price_monthly {
        Some("monthly")
    } else if price == config.price_annual {
        Some("annual")
    } else {
        None
    }
}

fn at(secs: Option<i64>) -> Option<DateTime<Utc>> {
    secs.and_then(|s| DateTime::from_timestamp(s, 0))
}

/// Cancel a subscription that must not exist. A failure fails the caller, so
/// Stripe redelivers the webhook; one already gone counts as done.
async fn cancel_extra(state: &AppState, id: &str) -> Result<(), ApiError> {
    let Some(billing) = &state.billing else {
        return Ok(());
    };
    match billing.cancel_subscription(id).await {
        Ok(()) | Err(BillingError::Api { status: 404, .. }) => Ok(()),
        Err(err) => {
            stripe_down_log(&err);
            Err(ApiError::new(
                ErrorCode::Unavailable,
                "Billing is not reachable right now.",
            ))
        }
    }
}

/// What applying a Stripe event did to a team, for the Stripe page's event list.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Applied {
    pub team_id: Uuid,
    /// A short human line such as "Team annual, 3 seats" or "Cancelled".
    pub summary: String,
    /// The team's MRR after minus before, in cents. `None` when the prices
    /// could not be read.
    pub mrr_delta_cents: Option<i64>,
}

fn seats_word(n: i64) -> String {
    format!("{n} seat{}", if n == 1 { "" } else { "s" })
}

/// The line for an event that moved a team from `was_paying` at `seats_before`
/// on `interval_before` to `status` at `seats` on `interval`.
fn summary_line(
    status: &str,
    was_paying: bool,
    (interval_before, seats_before): (Option<&str>, i64),
    (interval, seats): (Option<&str>, i64),
) -> String {
    if is_terminal(status) {
        return "Cancelled".to_owned();
    }
    if matches!(status, "past_due" | "unpaid") {
        return format!("Payment failed, {}", seats_word(seats));
    }
    let plan = |interval: Option<&str>| match interval {
        Some(i) => format!("Team {i}"),
        None => "Team".to_owned(),
    };
    if was_paying && interval_before != interval && interval.is_some() {
        format!(
            "Switched to {}, {}",
            interval.unwrap_or_default(),
            seats_word(seats)
        )
    } else if was_paying && seats_before != seats {
        format!("Seats {seats_before} to {seats}")
    } else {
        format!("{}, {}", plan(interval), seats_word(seats))
    }
}

/// The team's billed seats: what Stripe last had, else its members.
async fn billed_seats(
    tx: &mut PgConnection,
    team_id: Uuid,
    synced: Option<i32>,
) -> Result<i64, ApiError> {
    let seats = match synced {
        Some(n) => i64::from(n),
        None => {
            sqlx::query_scalar("SELECT count(*) FROM memberships WHERE team_id = $1")
                .bind(team_id)
                .fetch_one(tx)
                .await?
        }
    };
    Ok(seats.max(1))
}

/// The prices, for a delta. A failure costs the delta, never the event.
async fn prices_or_log(state: &AppState) -> Option<Prices> {
    match current_prices(state).await {
        Ok(prices) => prices,
        Err(err) => {
            stripe_down_log(&err);
            None
        }
    }
}

/// Write what Stripe says about `sub` onto its team, in the caller's
/// transaction. A subscription with no team here is ignored.
pub async fn apply_subscription(
    state: &AppState,
    tx: &mut PgConnection,
    sub: &Subscription,
    now: DateTime<Utc>,
) -> Result<Option<Applied>, ApiError> {
    let mut team = match sub.team_id {
        Some(id) => lock_team_by_id(tx, id).await?,
        None => None,
    };
    if team.is_none() {
        team = lock_team_by_subscription(tx, &sub.id).await?;
    }
    let terminal = is_terminal(&sub.status);
    let Some(team) = team else {
        if !terminal && sub.team_id.is_some() {
            // The team was deleted after the owner paid: stop the charge.
            tracing::error!(
                "stripe subscription {} is for a deleted team {:?}: cancelling it",
                sub.id,
                sub.team_id
            );
            cancel_extra(state, &sub.id).await?;
        } else {
            tracing::warn!("stripe subscription {} matches no team", sub.id);
        }
        return Ok(None);
    };
    // A late event for a subscription the team has since replaced.
    if terminal
        && team
            .stripe_subscription_id
            .as_deref()
            .is_some_and(|current| current != sub.id)
    {
        return Ok(None);
    }
    // A second live subscription for the team: keep the first, cancel this one.
    if !terminal
        && let Some(current) = team.stripe_subscription_id.as_deref()
        && current != sub.id
        && matches!(
            team.billing_status.as_deref(),
            Some("active" | "trialing" | "past_due" | "unpaid")
        )
    {
        tracing::error!(
            "team {} already has subscription {current}: cancelling duplicate {}",
            team.id,
            sub.id
        );
        cancel_extra(state, &sub.id).await?;
        return Ok(None);
    }

    let prices = prices_or_log(state).await;
    let seats_before = billed_seats(tx, team.id, team.seats_synced).await?;
    let was = plan_str(&team.plan);
    let next = outcome(&sub.status, was, team.past_due_since, now);
    sqlx::query(
        "UPDATE teams SET plan = $2, stripe_customer_id = $3, stripe_subscription_id = $4, \
         billing_interval = COALESCE($5, billing_interval), billing_status = $6, \
         past_due_since = $7, seats_synced = $8, cancel_at = $9, current_period_end = $10 \
         WHERE id = $1",
    )
    .bind(team.id)
    .bind(next.plan)
    .bind(&sub.customer)
    .bind((!terminal).then_some(&sub.id))
    .bind(interval_of(state, &sub.price))
    .bind(&sub.status)
    .bind(next.past_due_since)
    .bind((!terminal).then(|| i32::try_from(sub.quantity).unwrap_or(i32::MAX)))
    .bind(if terminal { None } else { at(sub.cancel_at) })
    .bind(if terminal {
        None
    } else {
        at(sub.current_period_end)
    })
    .execute(&mut *tx)
    .await?;

    if was != next.plan {
        PlanChange {
            team_id: team.id,
            at: now,
            source: "stripe",
            plan: parse_plan(next.plan),
            admin: None,
            reason: None,
            detail: json!({"from": was, "status": sub.status, "subscription": sub.id}),
        }
        .record(tx)
        .await?;
    }
    let before = team.effective(was, team.stripe_subscription_id.as_deref(), now);
    let after = team.effective(next.plan, (!terminal).then_some(sub.id.as_str()), now);

    // The copy says "keep the Team plan": only for a team that has it through
    // Stripe and is on it now (a Free override in force: nothing to keep).
    if was == "team"
        && before == Plan::Team
        && team.past_due_since.is_none()
        && let Some(since) = next.past_due_since
    {
        let deadline = since + state.settings().await?.grace();
        mail_owners(state, tx, team.id, |to| Mail::PaymentFailed {
            to,
            team_name: team.name.clone(),
            slug: team.slug.clone(),
            deadline,
            app_url: state.config.app_url.clone(),
        })
        .await?;
    }
    // An active override hides the change: nothing to tell the owners.
    if before == Plan::Team && after == Plan::Free {
        mail_plan_changed(state, tx, &team).await?;
    }

    let overridden = PlanRow {
        plan: String::new(),
        stripe_subscription_id: None,
        override_plan: team.override_plan.clone(),
        override_until: team.override_until,
    }
    .override_active(now);
    let interval_after = interval_of(state, &sub.price).or(team.billing_interval.as_deref());
    let seats_after = i64::from(sub.quantity).max(1);
    let billed_before = Billed {
        subscribed: team.stripe_subscription_id.is_some(),
        status: team.billing_status.as_deref(),
        interval: team.billing_interval.as_deref(),
        seats: seats_before,
        overridden,
    };
    let billed_after = Billed {
        subscribed: !terminal,
        status: Some(&sub.status),
        interval: interval_after,
        seats: seats_after,
        overridden,
    };
    Ok(Some(Applied {
        team_id: team.id,
        summary: summary_line(
            &sub.status,
            billed_before.is_revenue(),
            (team.billing_interval.as_deref(), seats_before),
            (interval_after, seats_after),
        ),
        mrr_delta_cents: prices
            .as_ref()
            .map(|p| billed_after.mrr(p) - billed_before.mrr(p)),
    }))
}

/// Stripe no longer knows `subscription`: its team is on Free.
pub async fn apply_missing(
    state: &AppState,
    tx: &mut PgConnection,
    subscription: &str,
    now: DateTime<Utc>,
) -> Result<Option<Applied>, ApiError> {
    let Some(team) = lock_team_by_subscription(tx, subscription).await? else {
        return Ok(None);
    };
    let delta = match prices_or_log(state).await {
        Some(prices) => {
            let overridden = PlanRow {
                plan: String::new(),
                stripe_subscription_id: None,
                override_plan: team.override_plan.clone(),
                override_until: team.override_until,
            }
            .override_active(now);
            let seats = billed_seats(tx, team.id, team.seats_synced).await?;
            let before = Billed {
                subscribed: true,
                status: team.billing_status.as_deref(),
                interval: team.billing_interval.as_deref(),
                seats,
                overridden,
            };
            Some(-before.mrr(&prices))
        }
        None => None,
    };
    sqlx::query(
        "UPDATE teams SET plan = 'free', stripe_subscription_id = NULL, \
         billing_status = 'canceled', past_due_since = NULL, seats_synced = NULL, \
         cancel_at = NULL, current_period_end = NULL WHERE id = $1",
    )
    .bind(team.id)
    .execute(&mut *tx)
    .await?;
    if team.plan == "team" {
        PlanChange {
            team_id: team.id,
            at: now,
            source: "stripe",
            plan: Plan::Free,
            admin: None,
            reason: None,
            detail: json!({"from": "team", "status": "missing", "subscription": subscription}),
        }
        .record(tx)
        .await?;
        // An active override hides the change: nothing to tell the owners.
        if team.effective("team", Some(subscription), now) == Plan::Team
            && team.effective("free", None, now) == Plan::Free
        {
            mail_plan_changed(state, tx, &team).await?;
        }
    }
    Ok(Some(Applied {
        team_id: team.id,
        summary: "Cancelled".to_owned(),
        mrr_delta_cents: delta,
    }))
}

/// One pass of background work: move teams that stayed past due for the grace period
/// to Free, end plan overrides that ran out and remind owners 14 days before
/// one does, push changed seat counts to Stripe, write the day's MRR
/// snapshot (once per UTC day) and delete admin events older than 2 years (an
/// indexed delete that almost always finds nothing). A failure is logged and
/// the next tick tries again. Each job's end is recorded for the health page.
pub async fn tick(state: &AppState, now: DateTime<Utc>) {
    let m = &state.metrics;
    let mut billing_ok = true;
    if let Err(err) = downgrade_past_due(state, now).await {
        tracing::error!("past-due downgrade: {err}");
        billing_ok = false;
    }
    let mut overrides_ok = true;
    if let Err(err) = expire_overrides(state, now).await {
        tracing::error!("override expiry: {err}");
        overrides_ok = false;
    }
    if let Err(err) = remind_overrides(state, now).await {
        tracing::error!("override reminder: {err}");
        overrides_ok = false;
    }
    if let Err(err) = sync_seats(state, now).await {
        tracing::error!("seat sync: {err}");
        billing_ok = false;
    }
    m.job_ran("billing_sync", now, billing_ok);
    m.job_ran("override_expiry", now, overrides_ok);
    let snapshot = super::mrr::snapshot(state, now).await;
    if let Err(err) = &snapshot {
        tracing::error!("mrr snapshot: {err}");
    }
    m.job_ran("mrr_snapshot", now, snapshot.is_ok());
    let retention = crate::admin::events::prune(&state.db, now).await;
    if let Err(err) = &retention {
        tracing::error!("audit log retention: {err}");
    }
    m.job_ran("audit_retention", now, retention.is_ok());
}

/// `tick` every minute until `shutdown` resolves.
pub async fn run(state: AppState, shutdown: impl Future<Output = ()>) {
    tokio::pin!(shutdown);
    loop {
        tokio::select! {
            () = &mut shutdown => return,
            () = tokio::time::sleep(TICK) => {},
        }
        tick(&state, state.clock.now()).await;
    }
}

async fn downgrade_past_due(state: &AppState, now: DateTime<Utc>) -> Result<(), ApiError> {
    // How long a team may stay past due is the grace period setting.
    let cutoff = now - state.settings().await?.grace();
    let due: Vec<Uuid> =
        sqlx::query_scalar("SELECT id FROM teams WHERE plan = 'team' AND past_due_since <= $1")
            .bind(cutoff)
            .fetch_all(&state.db)
            .await?;
    for id in due {
        let mut tx = state.db.begin().await?;
        // Re-check under the lock: a payment may have just cleared it.
        let team: Option<TeamRow> = sqlx::query_as(&format!(
            "SELECT {TEAM_COLUMNS} FROM teams \
             WHERE id = $1 AND plan = 'team' AND past_due_since <= $2 FOR UPDATE"
        ))
        .bind(id)
        .bind(cutoff)
        .fetch_optional(&mut *tx)
        .await?;
        let Some(team) = team else {
            continue;
        };
        // The Stripe subscription is left to Stripe's own dunning.
        sqlx::query("UPDATE teams SET plan = 'free' WHERE id = $1")
            .bind(id)
            .execute(&mut *tx)
            .await?;
        PlanChange {
            team_id: id,
            at: now,
            source: "past_due",
            plan: Plan::Free,
            admin: None,
            reason: None,
            detail: json!({"from": "team", "past_due_since": team.past_due_since}),
        }
        .record(&mut tx)
        .await?;
        // An active override hides the change: nothing to tell the owners.
        let sub = team.stripe_subscription_id.as_deref();
        if team.effective("team", sub, now) == Plan::Team
            && team.effective("free", sub, now) == Plan::Free
        {
            mail_plan_changed(state, &mut tx, &team).await?;
        }
        tx.commit().await?;
    }
    Ok(())
}

#[derive(FromRow)]
struct SeatRow {
    id: Uuid,
    stripe_subscription_id: String,
    seats: i32,
}

/// Clear overrides whose end date has passed. The team is back on its stored
/// (Stripe or default) plan, which `plan_history` records as `expired`.
async fn expire_overrides(state: &AppState, now: DateTime<Utc>) -> Result<(), ApiError> {
    let due: Vec<Uuid> = sqlx::query_scalar(
        "SELECT id FROM teams WHERE override_plan IS NOT NULL AND override_until <= $1",
    )
    .bind(now)
    .fetch_all(&state.db)
    .await?;
    for id in due {
        let mut tx = state.db.begin().await?;
        // Re-check under the lock: an admin may have just extended it.
        let team: Option<TeamRow> = sqlx::query_as(&format!(
            "SELECT {TEAM_COLUMNS} FROM teams \
             WHERE id = $1 AND override_plan IS NOT NULL AND override_until <= $2 FOR UPDATE"
        ))
        .bind(id)
        .bind(now)
        .fetch_optional(&mut *tx)
        .await?;
        let Some(team) = team else {
            continue;
        };
        sqlx::query(
            "UPDATE teams SET override_plan = NULL, override_until = NULL, \
             override_reason = NULL, override_by = NULL, override_at = NULL, \
             override_reminded_at = NULL WHERE id = $1",
        )
        .bind(id)
        .execute(&mut *tx)
        .await?;
        let granted = team.override_plan.as_deref().map(parse_plan);
        let fallback = parse_plan(&team.plan);
        PlanChange {
            team_id: id,
            at: now,
            source: "expired",
            plan: fallback,
            admin: None,
            reason: None,
            detail: json!({"override": team.override_plan, "until": team.override_until}),
        }
        .record(&mut tx)
        .await?;
        if granted == Some(Plan::Team) && fallback == Plan::Free {
            mail_plan_changed(state, &mut tx, &team).await?;
        }
        tx.commit().await?;
    }
    Ok(())
}

/// Email the owners of each team whose override ends within
/// [`OVERRIDE_REMINDER`], once per end date: `override_reminded_at` is set
/// with the mail and cleared whenever an admin sets a new end date.
async fn remind_overrides(state: &AppState, now: DateTime<Utc>) -> Result<(), ApiError> {
    const DUE: &str = "override_plan IS NOT NULL AND override_reminded_at IS NULL \
                       AND override_until > $1 AND override_until <= $2";
    let soon = now + OVERRIDE_REMINDER;
    let due: Vec<Uuid> = sqlx::query_scalar(&format!("SELECT id FROM teams WHERE {DUE}"))
        .bind(now)
        .bind(soon)
        .fetch_all(&state.db)
        .await?;
    for id in due {
        let mut tx = state.db.begin().await?;
        let team: Option<TeamRow> = sqlx::query_as(&format!(
            "SELECT {TEAM_COLUMNS} FROM teams WHERE id = $3 AND {DUE} FOR UPDATE"
        ))
        .bind(now)
        .bind(soon)
        .bind(id)
        .fetch_optional(&mut *tx)
        .await?;
        let Some(team) = team else {
            continue;
        };
        let (Some(plan), Some(until)) = (team.override_plan.as_deref(), team.override_until) else {
            continue;
        };
        let plan = parse_plan(plan);
        mail_owners(state, &mut tx, team.id, |to| Mail::OverrideEnding {
            to,
            team_name: team.name.clone(),
            slug: team.slug.clone(),
            plan,
            until,
            app_url: state.config.app_url.clone(),
        })
        .await?;
        sqlx::query("UPDATE teams SET override_reminded_at = $2 WHERE id = $1")
            .bind(id)
            .bind(now)
            .execute(&mut *tx)
            .await?;
        tx.commit().await?;
    }
    Ok(())
}

async fn sync_seats(state: &AppState, now: DateTime<Utc>) -> Result<(), ApiError> {
    let Some(billing) = &state.billing else {
        return Ok(());
    };
    let dirty: Vec<SeatRow> = sqlx::query_as(
        "SELECT t.id, t.stripe_subscription_id, c.seats FROM teams t \
         CROSS JOIN LATERAL (SELECT count(*)::int AS seats FROM memberships m \
                             WHERE m.team_id = t.id) c \
         WHERE t.stripe_subscription_id IS NOT NULL AND t.seats_synced IS DISTINCT FROM c.seats",
    )
    .fetch_all(&state.db)
    .await?;
    for team in dirty {
        let seats = u32::try_from(team.seats.max(1)).unwrap_or(1);
        if let Err(err) = billing
            .set_quantity(&team.stripe_subscription_id, seats)
            .await
        {
            if matches!(err, BillingError::Api { status: 404, .. }) {
                // The subscription is gone: the team moves to Free.
                let mut tx = state.db.begin().await?;
                apply_missing(state, &mut tx, &team.stripe_subscription_id, now).await?;
                tx.commit().await?;
            } else {
                stripe_down_log(&err);
            }
            continue;
        }
        // Only if the count is still what was sent: a member added meanwhile
        // keeps the team dirty for the next tick.
        sqlx::query(
            "UPDATE teams SET seats_synced = $2 WHERE id = $1 \
             AND (SELECT count(*) FROM memberships WHERE team_id = $1) = $2",
        )
        .bind(team.id)
        .bind(team.seats)
        .execute(&state.db)
        .await?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn now() -> DateTime<Utc> {
        "2026-11-04T10:00:00Z".parse().unwrap()
    }

    #[test]
    fn active_and_trialing_grant_team_and_clear_past_due() {
        for status in ["active", "trialing"] {
            let o = outcome(status, "free", Some(now()), now());
            assert_eq!(
                o,
                Outcome {
                    plan: "team",
                    past_due_since: None
                }
            );
        }
    }

    #[test]
    fn past_due_keeps_plan_and_the_earlier_clock() {
        let earlier = now() - chrono::Duration::days(3);
        for status in ["past_due", "unpaid"] {
            assert_eq!(
                outcome(status, "team", None, now()),
                Outcome {
                    plan: "team",
                    past_due_since: Some(now())
                }
            );
            assert_eq!(
                outcome(status, "team", Some(earlier), now()).past_due_since,
                Some(earlier)
            );
            // A clock in the future (skewed) never pushes the deadline out.
            let later = now() + chrono::Duration::days(1);
            assert_eq!(
                outcome(status, "team", Some(later), now()).past_due_since,
                Some(now())
            );
        }
    }

    #[test]
    fn terminal_statuses_end_on_free() {
        for status in ["canceled", "incomplete_expired"] {
            assert_eq!(
                outcome(status, "team", Some(now()), now()),
                Outcome {
                    plan: "free",
                    past_due_since: None
                }
            );
        }
    }

    #[test]
    fn other_statuses_change_nothing() {
        for status in ["incomplete", "paused"] {
            assert_eq!(
                outcome(status, "free", None, now()),
                Outcome {
                    plan: "free",
                    past_due_since: None
                }
            );
        }
    }
}
