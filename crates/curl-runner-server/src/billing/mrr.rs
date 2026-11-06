//! Monthly recurring revenue, from the `teams` table and the cached Stripe
//! prices: seats times the price of the team's interval, an annual price
//! counting as a twelfth. Only a team that pays through Stripe is revenue
//! (`active` or `past_due`, with a known interval). An override does not
//! change that: Stripe keeps billing the team, so it stays revenue. A team an
//! admin granted Team that is not revenue is counted apart ("granted").
//!
//! The plan predicates every admin page shares live here, so the overview,
//! the Stripe page, the teams list and the broadcast audiences agree:
//!
//! - [`revenue_sql`]: the team pays (`paying`);
//! - [`live_subscription_sql`]: a Stripe subscription exists that is not gone
//!   for good;
//! - [`granted_sql`]: an admin granted Team and the team does not pay.
//!
//! "Live" is wider than "revenue" on purpose. A subscription that is
//! `trialing`, `incomplete` or `unpaid` has not paid (yet), so it is no
//! revenue. It is still there, though: Stripe may bill it, the customer
//! portal manages it, and a new checkout would make a second one. So
//! `has_subscription`, the override's `stripe_conflict` and the teams list's
//! `stripe` filter use "live"; MRR and the `paying_teams` audience use
//! "revenue". Every team is exactly one of paying, granted or neither.

use chrono::{DateTime, Utc};
use sqlx::{FromRow, PgPool};

use super::Prices;
use super::effective::override_active_sql;
use super::routes::{current_prices, stripe_down};
use crate::error::ApiError;
use crate::state::AppState;

/// The MRR of `seats` on `interval` (`monthly` or `annual`), in cents. An
/// unknown interval is no revenue.
pub fn team_mrr(prices: &Prices, interval: Option<&str>, seats: i64) -> i64 {
    match interval {
        Some("monthly") => seats * prices.monthly.unit_amount,
        Some("annual") => seats * prices.annual.unit_amount / 12,
        _ => 0,
    }
}

/// What decides whether a team is revenue, and how much.
#[derive(Debug, Clone, Copy)]
pub struct Billed<'a> {
    pub subscribed: bool,
    pub status: Option<&'a str>,
    pub interval: Option<&'a str>,
    pub seats: i64,
}

impl Billed<'_> {
    /// The rule of [`revenue_sql`], for one team.
    pub fn is_revenue(&self) -> bool {
        self.subscribed
            && matches!(self.status, Some("active" | "past_due"))
            && self.interval.is_some()
    }

    pub fn mrr(&self, prices: &Prices) -> i64 {
        if self.is_revenue() {
            team_mrr(prices, self.interval, self.seats)
        } else {
            0
        }
    }
}

/// Whether the `teams` row `alias` counts as revenue: a subscription that is
/// `active` or `past_due`, with a known interval. SQL twin of
/// [`Billed::is_revenue`]. An override in force is no reason to leave one
/// out: Stripe still bills it. Never `NULL`, even with no status yet, so
/// `NOT` of it is safe.
pub fn revenue_sql(alias: &str) -> String {
    format!(
        "COALESCE({alias}.stripe_subscription_id IS NOT NULL \
          AND {alias}.billing_status IN ('active', 'past_due') \
          AND {alias}.billing_interval IS NOT NULL, false)"
    )
}

/// Statuses after which a Stripe subscription is gone for good.
const ENDED: [&str; 2] = ["canceled", "incomplete_expired"];

/// Whether a team with `subscription_id` and `status` has a live Stripe
/// subscription: set, and not gone for good. Rust twin of
/// [`live_subscription_sql`]. Wider than revenue: see the module docs.
pub fn is_live_subscription(subscription_id: Option<&str>, status: Option<&str>) -> bool {
    subscription_id.is_some() && !status.is_some_and(|s| ENDED.contains(&s))
}

/// Whether the `teams` row `alias` has a live Stripe subscription. A status
/// not synced yet (`NULL`) is live, as in [`is_live_subscription`].
pub fn live_subscription_sql(alias: &str) -> String {
    format!(
        "({alias}.stripe_subscription_id IS NOT NULL \
          AND {alias}.billing_status IS DISTINCT FROM '{}' \
          AND {alias}.billing_status IS DISTINCT FROM '{}')",
        ENDED[0], ENDED[1]
    )
}

/// Whether an admin granted the `teams` row `alias` Team at `now` (a
/// placeholder or expression) and it is not revenue. A Free override is no
/// grant, and a paying team with an override on top stays paying.
pub fn granted_sql(alias: &str, now: &str) -> String {
    format!(
        "({} AND {alias}.override_plan = 'team' AND NOT {})",
        override_active_sql(alias, now),
        revenue_sql(alias)
    )
}

/// What the team is billed for: the seats Stripe last had, else its members.
const SEATS: &str = "GREATEST(COALESCE(t.seats_synced, \
                     (SELECT count(*)::int FROM memberships m WHERE m.team_id = t.id)), 1)::bigint";

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Group {
    pub teams: i64,
    pub seats: i64,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Totals {
    pub mrr_cents: i64,
    pub paying_teams: i64,
    pub seats: i64,
    pub annual: Group,
    pub monthly: Group,
    pub past_due_teams: i64,
    /// The MRR of the past-due teams, which is part of `mrr_cents`.
    pub past_due_cents: i64,
}

#[derive(FromRow)]
struct Line {
    interval: String,
    past_due: bool,
    teams: i64,
    seats: i64,
    mrr: i64,
}

pub async fn totals(db: &PgPool, prices: &Prices) -> Result<Totals, ApiError> {
    let lines: Vec<Line> = sqlx::query_as(&format!(
        "SELECT x.interval, x.past_due, count(*)::bigint AS teams, sum(x.seats)::bigint AS seats, \
                sum(x.seats * CASE x.interval WHEN 'annual' THEN $2 ELSE $1 END \
                    / CASE x.interval WHEN 'annual' THEN 12 ELSE 1 END)::bigint AS mrr \
         FROM (SELECT t.billing_interval AS interval, t.billing_status = 'past_due' AS past_due, \
                      {SEATS} AS seats FROM teams t WHERE {}) x \
         GROUP BY 1, 2",
        revenue_sql("t")
    ))
    .bind(prices.monthly.unit_amount)
    .bind(prices.annual.unit_amount)
    .fetch_all(db)
    .await?;
    let mut out = Totals::default();
    for l in lines {
        out.mrr_cents += l.mrr;
        out.paying_teams += l.teams;
        out.seats += l.seats;
        let group = if l.interval == "annual" {
            &mut out.annual
        } else {
            &mut out.monthly
        };
        group.teams += l.teams;
        group.seats += l.seats;
        if l.past_due {
            out.past_due_teams += l.teams;
            out.past_due_cents += l.mrr;
        }
    }
    Ok(out)
}

/// Teams an admin granted Team ([`granted_sql`]), and their members. A
/// paying team with an override on top counts as revenue instead.
pub async fn granted(db: &PgPool, now: DateTime<Utc>) -> Result<Group, ApiError> {
    let (teams, seats): (i64, i64) = sqlx::query_as(&format!(
        "SELECT count(*)::bigint, COALESCE(sum((SELECT count(*) FROM memberships m \
                WHERE m.team_id = t.id)), 0)::bigint \
         FROM teams t WHERE {}",
        granted_sql("t", "$1")
    ))
    .bind(now)
    .fetch_one(db)
    .await?;
    Ok(Group { teams, seats })
}

/// Write today's row (UTC) once: the first writer of the day wins. Does
/// nothing when billing is off or today already has a row.
pub async fn snapshot(state: &AppState, now: DateTime<Utc>) -> Result<(), ApiError> {
    if state.billing.is_none() {
        return Ok(());
    }
    let day = now.date_naive();
    let done: bool =
        sqlx::query_scalar("SELECT EXISTS (SELECT 1 FROM mrr_snapshots WHERE day = $1)")
            .bind(day)
            .fetch_one(&state.db)
            .await?;
    if done {
        return Ok(());
    }
    let Some(prices) = current_prices(state).await.map_err(stripe_down)? else {
        return Ok(());
    };
    let totals = totals(&state.db, &prices).await?;
    sqlx::query(
        "INSERT INTO mrr_snapshots (day, mrr_cents, teams, seats) VALUES ($1, $2, $3, $4) \
         ON CONFLICT (day) DO NOTHING",
    )
    .bind(day)
    .bind(totals.mrr_cents)
    .bind(i32::try_from(totals.paying_teams).unwrap_or(i32::MAX))
    .bind(i32::try_from(totals.seats).unwrap_or(i32::MAX))
    .execute(&state.db)
    .await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::billing::Price;

    fn prices() -> Prices {
        let price = |unit_amount, interval: &str| Price {
            unit_amount,
            currency: "usd".into(),
            interval: interval.into(),
        };
        Prices {
            monthly: price(900, "month"),
            annual: price(9000, "year"),
        }
    }

    #[test]
    fn annual_is_a_twelfth_of_the_year() {
        assert_eq!(team_mrr(&prices(), Some("monthly"), 3), 2700);
        assert_eq!(team_mrr(&prices(), Some("annual"), 3), 2250);
        // Multiplied before it is divided: no cent is lost per seat.
        assert_eq!(team_mrr(&prices(), Some("annual"), 1), 750);
        assert_eq!(team_mrr(&prices(), None, 3), 0);
    }

    #[test]
    fn live_is_set_and_not_ended() {
        assert!(!is_live_subscription(None, Some("active")));
        assert!(is_live_subscription(Some("sub_1"), None));
        for s in ["active", "trialing", "past_due", "unpaid", "incomplete"] {
            assert!(is_live_subscription(Some("sub_1"), Some(s)), "{s}");
        }
        for s in ["canceled", "incomplete_expired"] {
            assert!(!is_live_subscription(Some("sub_1"), Some(s)), "{s}");
        }
    }
}
