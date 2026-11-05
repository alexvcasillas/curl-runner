//! Keeping the `teams` table in step with Stripe: applying a re-read
//! subscription, and the background tick for seat counts and the past-due
//! downgrade.

use std::future::Future;
use std::time::Duration;

use chrono::{DateTime, Utc};
use sqlx::{FromRow, PgConnection};
use uuid::Uuid;

use super::{BillingError, Subscription, stripe_down_log};
use crate::error::ApiError;
use crate::mail::Mail;
use crate::state::AppState;
use curl_runner_cloud_proto::ErrorCode;

/// How long a team may stay past due before it moves to Free.
pub const PAST_DUE_GRACE: chrono::Duration = chrono::Duration::days(14);
/// Time between ticks.
const TICK: Duration = Duration::from_secs(60);

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
}

const TEAM_COLUMNS: &str = "id, name, slug::text AS slug, plan, past_due_since, \
                            stripe_subscription_id, billing_status";

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
async fn mail_owners(
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
    mail_owners(state, tx, team.id, |to| Mail::PlanChanged {
        to,
        team_name: team.name.clone(),
        slug: team.slug.clone(),
        app_url: state.config.app_url.clone(),
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

/// Write what Stripe says about `sub` onto its team, in the caller's
/// transaction. A subscription with no team here is ignored.
pub async fn apply_subscription(
    state: &AppState,
    tx: &mut PgConnection,
    sub: &Subscription,
    now: DateTime<Utc>,
) -> Result<(), ApiError> {
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
        return Ok(());
    };
    // A late event for a subscription the team has since replaced.
    if terminal
        && team
            .stripe_subscription_id
            .as_deref()
            .is_some_and(|current| current != sub.id)
    {
        return Ok(());
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
        return Ok(());
    }

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

    // The copy says "keep the Team plan": only for a team that has it.
    if was == "team"
        && team.past_due_since.is_none()
        && let Some(since) = next.past_due_since
    {
        let deadline = since + PAST_DUE_GRACE;
        mail_owners(state, tx, team.id, |to| Mail::PaymentFailed {
            to,
            team_name: team.name.clone(),
            slug: team.slug.clone(),
            deadline,
            app_url: state.config.app_url.clone(),
        })
        .await?;
    }
    if was == "team" && next.plan == "free" {
        mail_plan_changed(state, tx, &team).await?;
    }
    Ok(())
}

/// Stripe no longer knows `subscription`: its team is on Free.
pub async fn apply_missing(
    state: &AppState,
    tx: &mut PgConnection,
    subscription: &str,
) -> Result<(), ApiError> {
    let Some(team) = lock_team_by_subscription(tx, subscription).await? else {
        return Ok(());
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
        mail_plan_changed(state, tx, &team).await?;
    }
    Ok(())
}

/// One pass of background work: move teams that stayed past due for 14 days to
/// Free, push changed seat counts to Stripe, and delete admin events older
/// than 2 years (an indexed delete that almost always finds nothing). A
/// failure is logged and the next tick tries again.
pub async fn tick(state: &AppState, now: DateTime<Utc>) {
    if let Err(err) = downgrade_past_due(state, now).await {
        tracing::error!("past-due downgrade: {err}");
    }
    if let Err(err) = sync_seats(state).await {
        tracing::error!("seat sync: {err}");
    }
    if let Err(err) = crate::admin::events::prune(&state.db, now).await {
        tracing::error!("audit log retention: {err}");
    }
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
    let cutoff = now - PAST_DUE_GRACE;
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
        mail_plan_changed(state, &mut tx, &team).await?;
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

async fn sync_seats(state: &AppState) -> Result<(), ApiError> {
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
                apply_missing(state, &mut tx, &team.stripe_subscription_id).await?;
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
