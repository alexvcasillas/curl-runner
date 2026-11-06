//! A team's effective plan, and its history.
//!
//! `teams.plan` holds what Stripe (or the default, or an admin's direct set)
//! says. An admin's override beats it while `override_until` is null or in
//! the future; once it ends the team is back on `teams.plan`, never forced to
//! Free. Every plan read goes through [`effective_plan`]. List filters that
//! must run in SQL use [`effective_plan_sql`], its twin.

use chrono::{DateTime, Utc};
use curl_runner_cloud_proto::Plan;
use serde::Serialize;
use serde_json::Value;
use sqlx::{FromRow, PgConnection};
use uuid::Uuid;

use crate::error::ApiError;

/// What backs a team's effective plan.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum PlanSource {
    /// An admin's override.
    Override,
    /// A Stripe subscription.
    Stripe,
    /// Neither: the default plan, or one an admin set directly.
    None,
}

impl PlanSource {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Override => "override",
            Self::Stripe => "stripe",
            Self::None => "none",
        }
    }
}

/// The `teams` columns the effective plan depends on.
#[derive(Debug, Clone, PartialEq, Eq, FromRow)]
pub struct PlanRow {
    pub plan: String,
    pub stripe_subscription_id: Option<String>,
    pub override_plan: Option<String>,
    pub override_until: Option<DateTime<Utc>>,
}

/// SQL select list for a [`PlanRow`] of the `teams` row aliased `alias`.
pub fn plan_columns(alias: &str) -> String {
    format!(
        "{alias}.plan, {alias}.stripe_subscription_id, {alias}.override_plan, {alias}.override_until"
    )
}

/// A `plan` column value (checked by the schema) as the proto enum.
pub fn parse_plan(value: &str) -> Plan {
    if value == "team" {
        Plan::Team
    } else {
        Plan::Free
    }
}

pub fn plan_str(plan: Plan) -> &'static str {
    match plan {
        Plan::Free => "free",
        Plan::Team => "team",
    }
}

impl PlanRow {
    /// Whether the override counts at `now`.
    pub fn override_active(&self, now: DateTime<Utc>) -> bool {
        self.override_plan.is_some() && self.override_until.is_none_or(|until| until > now)
    }

    /// The plan without any override: Stripe's, else the stored one.
    pub fn underlying(&self) -> (Plan, PlanSource) {
        let source = if self.stripe_subscription_id.is_some() {
            PlanSource::Stripe
        } else {
            PlanSource::None
        };
        (parse_plan(&self.plan), source)
    }
}

/// The plan `team` has at `now`, and what backs it.
pub fn effective_plan(team: &PlanRow, now: DateTime<Utc>) -> (Plan, PlanSource) {
    match &team.override_plan {
        Some(plan) if team.override_active(now) => (parse_plan(plan), PlanSource::Override),
        _ => team.underlying(),
    }
}

/// SQL twin of [`PlanRow::override_active`] for the `teams` row `alias`;
/// `now` is a placeholder or expression.
pub fn override_active_sql(alias: &str, now: &str) -> String {
    format!(
        "({alias}.override_plan IS NOT NULL \
          AND ({alias}.override_until IS NULL OR {alias}.override_until > {now}))"
    )
}

/// SQL twin of [`effective_plan`]'s plan: `'free'` or `'team'`.
pub fn effective_plan_sql(alias: &str, now: &str) -> String {
    format!(
        "(CASE WHEN {} THEN {alias}.override_plan ELSE {alias}.plan END)",
        override_active_sql(alias, now)
    )
}

/// The effective plan of `team_id`, reading the row; `lock` takes it `FOR UPDATE`.
pub async fn team_plan(
    conn: &mut PgConnection,
    team_id: Uuid,
    now: DateTime<Utc>,
    lock: bool,
) -> Result<(Plan, PlanSource), ApiError> {
    let row: PlanRow = sqlx::query_as(&format!(
        "SELECT {} FROM teams t WHERE t.id = $1{}",
        plan_columns("t"),
        if lock { " FOR UPDATE" } else { "" }
    ))
    .bind(team_id)
    .fetch_one(conn)
    .await?;
    Ok(effective_plan(&row, now))
}

/// One `plan_history` row. `source` is where the change came from:
///
/// - `default`: the team was created on the default plan;
/// - `admin`: an admin set the stored plan directly;
/// - `stripe`: Stripe changed the stored plan;
/// - `past_due`: the grace period ran out;
/// - `override`: an admin granted or edited an override;
/// - `override_removed`: an admin removed it;
/// - `expired`: the override's end date passed.
///
/// `plan` is the plan the change leaves in force from that source: the
/// override's for `override`, the fallback for `override_removed` and
/// `expired`, the stored plan otherwise.
#[derive(Debug, Clone)]
pub struct PlanChange<'a> {
    pub team_id: Uuid,
    pub at: DateTime<Utc>,
    pub source: &'static str,
    pub plan: Plan,
    pub admin: Option<Uuid>,
    pub reason: Option<&'a str>,
    pub detail: Value,
}

impl PlanChange<'_> {
    pub async fn record(self, conn: &mut PgConnection) -> Result<(), ApiError> {
        sqlx::query(
            "INSERT INTO plan_history (team_id, at, source, plan, admin_user_id, reason, detail) \
             VALUES ($1, $2, $3, $4, $5, $6, $7)",
        )
        .bind(self.team_id)
        .bind(self.at)
        .bind(self.source)
        .bind(plan_str(self.plan))
        .bind(self.admin)
        .bind(self.reason)
        .bind(self.detail)
        .execute(conn)
        .await?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Duration;

    fn now() -> DateTime<Utc> {
        "2026-11-04T10:00:00Z".parse().unwrap()
    }

    fn row(plan: &str, stripe: bool, over: Option<&str>, until: Option<i64>) -> PlanRow {
        PlanRow {
            plan: plan.into(),
            stripe_subscription_id: stripe.then(|| "sub_1".into()),
            override_plan: over.map(Into::into),
            override_until: until.map(|d| now() + Duration::days(d)),
        }
    }

    #[test]
    fn no_override_is_the_stored_plan() {
        assert_eq!(
            effective_plan(&row("team", true, None, None), now()),
            (Plan::Team, PlanSource::Stripe)
        );
        assert_eq!(
            effective_plan(&row("free", false, None, None), now()),
            (Plan::Free, PlanSource::None)
        );
        // An end date without a plan is no override.
        assert_eq!(
            effective_plan(&row("free", false, None, Some(3)), now()),
            (Plan::Free, PlanSource::None)
        );
    }

    #[test]
    fn a_live_override_beats_stripe_either_way() {
        assert_eq!(
            effective_plan(&row("free", true, Some("team"), Some(30)), now()),
            (Plan::Team, PlanSource::Override)
        );
        assert_eq!(
            effective_plan(&row("team", true, Some("free"), None), now()),
            (Plan::Free, PlanSource::Override)
        );
    }

    #[test]
    fn an_ended_override_falls_back_to_stripe_not_free() {
        assert_eq!(
            effective_plan(&row("team", true, Some("free"), Some(-1)), now()),
            (Plan::Team, PlanSource::Stripe)
        );
        // Exactly at the end it has ended.
        assert_eq!(
            effective_plan(&row("team", true, Some("free"), Some(0)), now()),
            (Plan::Team, PlanSource::Stripe)
        );
        assert_eq!(
            effective_plan(&row("free", false, Some("team"), Some(-1)), now()),
            (Plan::Free, PlanSource::None)
        );
    }

    #[test]
    fn sources_serialise_as_the_wire_names() {
        for s in [PlanSource::Override, PlanSource::Stripe, PlanSource::None] {
            assert_eq!(serde_json::to_value(s).unwrap(), s.as_str());
        }
    }
}
