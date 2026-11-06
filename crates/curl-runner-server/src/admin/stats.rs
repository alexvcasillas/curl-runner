//! The admin overview, health and Stripe pages. Read only: any admin may call
//! them. The overview and health work with billing off (revenue figures are
//! then `null`); the Stripe page answers `404`.
//!
//! Granted teams are counted apart from revenue. The overview always carries
//! `granted_teams` and `granted_seats`, so they show with billing off too;
//! the Stripe page lists them as the `granted` row of `by_source`.
//!
//! Health figures that come from memory (latency, job runs, bad signatures,
//! duplicates, rate-limit hits) start over on every restart.

use axum::Json;
use axum::extract::State;
use chrono::{DateTime, Datelike, Duration, NaiveDate, Utc};
use curl_runner_cloud_proto::SignupMode;
use serde::Serialize;
use sqlx::{FromRow, PgPool};

use super::auth::AdminAuth;
use crate::billing::mrr::{self, Group, Totals};
use crate::billing::routes::{current_prices, stripe_down};
use crate::billing::{BillingError, Prices};
use crate::error::ApiError;
use crate::mail::worker::MAX_ATTEMPTS;
use crate::metrics::{JOBS, STALE_AFTER};
use crate::state::AppState;

/// How many days each overview chart covers.
const SIGNUP_DAYS: i32 = 30;
const PUBLISH_DAYS: i32 = 14;
const MRR_DAYS: i64 = 90;
const RECENT_EVENTS: i64 = 6;
const STRIPE_EVENTS: i64 = 20;
/// An override "ends soon" when it ends within this many days.
const ENDING_DAYS: i64 = 30;

/// The summary of an event that starts or keeps a plan (see
/// `billing::sync::summary_line`): a rise in MRR with it is "new", else "seats".
const NEW_SUMMARY: &str = "Team %";

#[derive(Serialize, FromRow)]
pub struct Day {
    day: String,
    n: i64,
}

/// `days` UTC days ending today, oldest first, zero-filled, counting the rows
/// of `table` by their `created_at`.
async fn per_day(
    db: &PgPool,
    table: &'static str,
    today: NaiveDate,
    days: i32,
) -> Result<Vec<Day>, ApiError> {
    Ok(sqlx::query_as(&format!(
        "SELECT s.d::text AS day, count(t.created_at)::bigint AS n \
         FROM (SELECT ($1::date - g) AS d FROM generate_series(0, $2 - 1) g) s \
         LEFT JOIN {table} t ON (t.created_at AT TIME ZONE 'UTC')::date = s.d \
         GROUP BY s.d ORDER BY s.d"
    ))
    .bind(today)
    .bind(days)
    .fetch_all(db)
    .await?)
}

/// Prices for a figure the page can do without: logged, then `None`.
async fn prices_or_log(state: &AppState) -> Option<Prices> {
    match current_prices(state).await {
        Ok(prices) => prices,
        Err(err) => {
            tracing::warn!("admin stats without prices: {err}");
            None
        }
    }
}

#[derive(Serialize)]
pub struct Counts {
    users: i64,
    teams: i64,
    collections: i64,
    collections_public: i64,
    collections_restricted: i64,
    /// Distinct people on access lists, members or not.
    access_list_people: i64,
    /// Teams with a plan override in force.
    plan_overrides: i64,
    /// Of those, how many end within 30 days.
    plan_overrides_ending: i64,
    /// Teams granted Team by an admin, and their members. Not revenue.
    granted_teams: i64,
    granted_seats: i64,
    /// `null` when billing is off or Stripe could not be read.
    paying_teams: Option<i64>,
    paying_annual: Option<i64>,
    paying_monthly: Option<i64>,
    mrr_cents: Option<i64>,
}

#[derive(Serialize)]
pub struct PastDue {
    teams: i64,
    /// The one whose grace period ends first.
    next: Option<PastDueNext>,
}

#[derive(Serialize)]
pub struct PastDueNext {
    team: String,
    name: String,
    deadline: DateTime<Utc>,
}

#[derive(Serialize, FromRow)]
pub struct RecentEvent {
    id: uuid::Uuid,
    admin_email: String,
    action: String,
    target_kind: Option<String>,
    target_id: Option<String>,
    target_label: Option<String>,
    at: DateTime<Utc>,
}

#[derive(Serialize)]
pub struct Outbox {
    waiting: i64,
    failed_today: i64,
}

#[derive(Serialize)]
pub struct Overview {
    /// Whether Stripe billing is configured.
    billing: bool,
    counts: Counts,
    past_due: PastDue,
    signups_30d: Vec<Day>,
    teams_30d: Vec<Day>,
    publishes_14d: Vec<Day>,
    recent_events: Vec<RecentEvent>,
    // The status strip.
    p95_ms: Option<u32>,
    outbox: Outbox,
    last_webhook_at: Option<DateTime<Utc>>,
    signup_mode: SignupMode,
}

#[derive(FromRow)]
struct CountRow {
    users: i64,
    teams: i64,
    collections: i64,
    collections_public: i64,
    collections_restricted: i64,
    access_list_people: i64,
    plan_overrides: i64,
    plan_overrides_ending: i64,
}

fn day_start(day: NaiveDate) -> DateTime<Utc> {
    day.and_time(chrono::NaiveTime::MIN).and_utc()
}

async fn outbox(db: &PgPool, now: DateTime<Utc>) -> Result<(i64, i64, Option<i64>), ApiError> {
    let (waiting, oldest): (i64, Option<DateTime<Utc>>) = sqlx::query_as(
        "SELECT count(*)::bigint, min(COALESCE(created_at, next_attempt_at)) \
         FROM outbox_emails WHERE sent_at IS NULL AND failed_at IS NULL AND attempts < $1",
    )
    .bind(MAX_ATTEMPTS)
    .fetch_one(db)
    .await?;
    let failed_today: i64 =
        sqlx::query_scalar("SELECT count(*)::bigint FROM outbox_emails WHERE failed_at >= $1")
            .bind(day_start(now.date_naive()))
            .fetch_one(db)
            .await?;
    Ok((
        waiting,
        failed_today,
        oldest.map(|at| (now - at).num_seconds().max(0)),
    ))
}

async fn last_webhook(db: &PgPool) -> Result<Option<DateTime<Utc>>, ApiError> {
    Ok(
        sqlx::query_scalar("SELECT max(received_at) FROM stripe_events")
            .fetch_one(db)
            .await?,
    )
}

pub async fn overview(
    _admin: AdminAuth,
    State(state): State<AppState>,
) -> Result<Json<Overview>, ApiError> {
    let now = state.clock.now();
    let today = now.date_naive();
    let db = &state.db;
    let settings = state.settings().await?;

    let c: CountRow = sqlx::query_as(
        "SELECT (SELECT count(*) FROM users) AS users, \
                (SELECT count(*) FROM teams) AS teams, \
                (SELECT count(*) FROM collections) AS collections, \
                (SELECT count(*) FROM collections WHERE access = 'public') AS collections_public, \
                (SELECT count(*) FROM collections WHERE access = 'restricted') \
                    AS collections_restricted, \
                (SELECT count(DISTINCT entry) FROM access_entries) AS access_list_people, \
                (SELECT count(*) FROM teams t WHERE t.override_plan IS NOT NULL \
                   AND (t.override_until IS NULL OR t.override_until > $1)) AS plan_overrides, \
                (SELECT count(*) FROM teams t WHERE t.override_plan IS NOT NULL \
                   AND t.override_until > $1 AND t.override_until <= $2) AS plan_overrides_ending",
    )
    .bind(now)
    .bind(now + Duration::days(ENDING_DAYS))
    .fetch_one(db)
    .await?;
    let granted = mrr::granted(db, now).await?;
    let totals = match prices_or_log(&state).await {
        Some(prices) => Some(mrr::totals(db, &prices, now).await?),
        None => None,
    };

    let past_due_teams: i64 = sqlx::query_scalar(
        "SELECT count(*)::bigint FROM teams WHERE plan = 'team' AND past_due_since IS NOT NULL",
    )
    .fetch_one(db)
    .await?;
    let next: Option<(String, String, DateTime<Utc>)> = sqlx::query_as(
        "SELECT slug::text, name, past_due_since FROM teams \
         WHERE plan = 'team' AND past_due_since IS NOT NULL \
         ORDER BY past_due_since, slug LIMIT 1",
    )
    .fetch_optional(db)
    .await?;

    let (waiting, failed_today, _) = outbox(db, now).await?;
    let recent_events = sqlx::query_as(
        "SELECT id, admin_email, action, target_kind, target_id, target_label, at \
         FROM admin_events ORDER BY at DESC, id DESC LIMIT $1",
    )
    .bind(RECENT_EVENTS)
    .fetch_all(db)
    .await?;

    Ok(Json(Overview {
        billing: state.billing.is_some(),
        counts: Counts {
            users: c.users,
            teams: c.teams,
            collections: c.collections,
            collections_public: c.collections_public,
            collections_restricted: c.collections_restricted,
            access_list_people: c.access_list_people,
            plan_overrides: c.plan_overrides,
            plan_overrides_ending: c.plan_overrides_ending,
            granted_teams: granted.teams,
            granted_seats: granted.seats,
            paying_teams: totals.map(|t| t.paying_teams),
            paying_annual: totals.map(|t| t.annual.teams),
            paying_monthly: totals.map(|t| t.monthly.teams),
            mrr_cents: totals.map(|t| t.mrr_cents),
        },
        past_due: PastDue {
            teams: past_due_teams,
            next: next.map(|(team, name, since)| PastDueNext {
                team,
                name,
                deadline: since + settings.grace(),
            }),
        },
        signups_30d: per_day(db, "users", today, SIGNUP_DAYS).await?,
        teams_30d: per_day(db, "teams", today, SIGNUP_DAYS).await?,
        publishes_14d: per_day(db, "versions", today, PUBLISH_DAYS).await?,
        recent_events,
        p95_ms: state.metrics.p95_ms(now),
        outbox: Outbox {
            waiting,
            failed_today,
        },
        last_webhook_at: last_webhook(db).await?,
        signup_mode: settings.signup_mode,
    }))
}

#[derive(Serialize)]
pub struct DbStats {
    size_bytes: i64,
    connections: i64,
    max_connections: i64,
    /// The newest applied migration.
    migration: Option<i64>,
}

#[derive(Serialize)]
pub struct EmailStats {
    waiting: i64,
    oldest_waiting_secs: Option<i64>,
    failed_today: i64,
}

#[derive(Serialize)]
pub struct HealthStats {
    version: &'static str,
    uptime_secs: i64,
    p95_ms: Option<u32>,
    requests_5m: usize,
    errors_5xx_5m: usize,
    db: DbStats,
    email: EmailStats,
}

#[derive(Serialize)]
pub struct Job {
    name: &'static str,
    last_run: Option<DateTime<Utc>>,
    /// `ok`, `failed` (the last run hit an error), `stale` (no run for 5
    /// minutes) or `never_run` (since the server started).
    status: &'static str,
}

#[derive(Serialize, FromRow)]
pub struct WebhookCount {
    #[sqlx(rename = "type")]
    #[serde(rename = "type")]
    kind: String,
    n: i64,
}

#[derive(Serialize)]
pub struct RateLimit {
    /// The key prefix, such as `email-start:email`. Never the full key.
    key: String,
    what: String,
    n: usize,
}

#[derive(Serialize)]
pub struct Health {
    stats: HealthStats,
    jobs: Vec<Job>,
    webhooks_24h: Vec<WebhookCount>,
    bad_signatures_24h: usize,
    duplicates_24h: usize,
    last_webhook_at: Option<DateTime<Utc>>,
    rate_limits: Vec<RateLimit>,
}

/// What a rate-limit key prefix guards.
fn limit_label(prefix: &str) -> &str {
    match prefix {
        "email-start:ip" => "Email sign-in starts, per IP",
        "email-start:email" => "Sign-in codes, per email",
        "email-start:email-day" => "Sign-in codes, per email per day",
        "email-verify:ip" => "Code checks, per IP",
        "email-verify:email" => "Code checks, per email",
        "handoff-redeem:ip" => "Sign-in handoffs, per IP",
        "invite-preview:ip" => "Invite previews, per IP",
        "shared:ip" => "Share link opens, per IP",
        "publish:team" => "Publishes, per team",
        "admin:broadcast-test" => "Broadcast test emails, per admin",
        "admin:test-email" => "Settings test emails, per admin",
        other => other,
    }
}

fn job_status(last: Option<crate::metrics::JobRun>, now: DateTime<Utc>) -> &'static str {
    match last {
        None => "never_run",
        Some(run) if !run.ok => "failed",
        Some(run) if now - run.at > STALE_AFTER => "stale",
        Some(_) => "ok",
    }
}

pub async fn health(
    _admin: AdminAuth,
    State(state): State<AppState>,
) -> Result<Json<Health>, ApiError> {
    let now = state.clock.now();
    let db = &state.db;
    let m = &state.metrics;

    let (size_bytes, connections, max_connections): (i64, i64, i64) = sqlx::query_as(
        "SELECT pg_database_size(current_database()), \
                (SELECT count(*) FROM pg_stat_activity WHERE datname = current_database()), \
                current_setting('max_connections')::bigint",
    )
    .fetch_one(db)
    .await?;
    let migration: Option<i64> =
        sqlx::query_scalar("SELECT max(version) FROM _sqlx_migrations WHERE success")
            .fetch_one(db)
            .await?;
    let (waiting, failed_today, oldest) = outbox(db, now).await?;

    let webhooks_24h = sqlx::query_as(
        "SELECT type, count(*)::bigint AS n FROM stripe_events WHERE received_at > $1 \
         GROUP BY type ORDER BY n DESC, type",
    )
    .bind(now - Duration::hours(24))
    .fetch_all(db)
    .await?;

    Ok(Json(Health {
        stats: HealthStats {
            version: env!("CARGO_PKG_VERSION"),
            uptime_secs: m.started().map_or(0, |at| (now - at).num_seconds().max(0)),
            p95_ms: m.p95_ms(now),
            requests_5m: m.requests_5m(now),
            errors_5xx_5m: m.errors_5xx_5m(now),
            db: DbStats {
                size_bytes,
                connections,
                max_connections,
                migration,
            },
            email: EmailStats {
                waiting,
                oldest_waiting_secs: oldest,
                failed_today,
            },
        },
        jobs: JOBS
            .iter()
            .map(|name| {
                let last = m.job(name);
                Job {
                    name,
                    last_run: last.map(|run| run.at),
                    status: job_status(last, now),
                }
            })
            .collect(),
        webhooks_24h,
        bad_signatures_24h: m.bad_signatures_24h(now),
        duplicates_24h: m.duplicates_24h(now),
        last_webhook_at: last_webhook(db).await?,
        rate_limits: m
            .rate_limit_hits(now)
            .into_iter()
            .map(|(key, n)| RateLimit {
                what: limit_label(&key).to_owned(),
                key,
                n,
            })
            .collect(),
    }))
}

#[derive(Serialize)]
pub struct MrrPoint {
    day: String,
    mrr_cents: i64,
}

#[derive(Serialize)]
pub struct NetNew {
    /// From the first of the month (UTC) to now.
    since: NaiveDate,
    total_cents: i64,
    new_cents: i64,
    expansion_cents: i64,
    /// Cancellations and seats removed, a negative number.
    churned_cents: i64,
    cancelled_teams: i64,
}

#[derive(Serialize)]
pub struct Source {
    /// `stripe_annual`, `stripe_monthly` or `granted`.
    source: &'static str,
    teams: i64,
    seats: i64,
}

#[derive(Serialize)]
pub struct StripeTeam {
    slug: String,
    name: String,
}

#[derive(Serialize, FromRow)]
struct EventRow {
    id: String,
    #[sqlx(rename = "type")]
    kind: String,
    team_slug: Option<String>,
    team_name: Option<String>,
    summary: Option<String>,
    mrr_delta_cents: Option<i64>,
    at: DateTime<Utc>,
}

#[derive(Serialize)]
pub struct StripeEvent {
    id: String,
    #[serde(rename = "type")]
    kind: String,
    team: Option<StripeTeam>,
    summary: Option<String>,
    mrr_delta_cents: Option<i64>,
    at: DateTime<Utc>,
}

#[derive(Serialize)]
pub struct StripePastDue {
    teams: i64,
    at_risk_cents: i64,
}

#[derive(Serialize)]
pub struct WebhookStats {
    events_24h: i64,
    bad_signatures_24h: usize,
    duplicates_24h: usize,
}

#[derive(Serialize)]
pub struct StripeStats {
    currency: String,
    mrr_cents: i64,
    arr_cents: i64,
    paying_teams: i64,
    seats: i64,
    /// MRR per paying team, rounded down; 0 with none.
    arpa: i64,
    past_due: StripePastDue,
    /// One point per day that has a snapshot, plus today's live figure.
    mrr_90d: Vec<MrrPoint>,
    net_new: NetNew,
    by_source: Vec<Source>,
    webhooks: WebhookStats,
    events: Vec<StripeEvent>,
}

fn source(source: &'static str, g: Group) -> Source {
    Source {
        source,
        teams: g.teams,
        seats: g.seats,
    }
}

pub async fn stripe(
    _admin: AdminAuth,
    State(state): State<AppState>,
) -> Result<Json<StripeStats>, ApiError> {
    let prices = current_prices(&state)
        .await
        .map_err(|err: BillingError| stripe_down(err))?
        .ok_or_else(ApiError::not_found)?;
    let now = state.clock.now();
    let today = now.date_naive();
    let db = &state.db;
    let totals: Totals = mrr::totals(db, &prices, now).await?;
    let granted = mrr::granted(db, now).await?;

    let mut mrr_90d: Vec<(NaiveDate, i64)> = sqlx::query_as(
        "SELECT day, mrr_cents FROM mrr_snapshots WHERE day > $1 AND day < $2 ORDER BY day",
    )
    .bind(today - Duration::days(MRR_DAYS))
    .bind(today)
    .fetch_all(db)
    .await?;
    // Today's point is always the live figure, so the line ends on the headline.
    mrr_90d.push((today, totals.mrr_cents));

    let month = today.with_day(1).unwrap_or(today);
    let (new_cents, expansion_cents, churned_cents, cancelled_teams): (i64, i64, i64, i64) =
        sqlx::query_as(&format!(
            "SELECT COALESCE(sum(mrr_delta_cents) FILTER \
                        (WHERE mrr_delta_cents > 0 AND COALESCE(summary, '') LIKE '{NEW_SUMMARY}'), 0)::bigint, \
                    COALESCE(sum(mrr_delta_cents) FILTER \
                        (WHERE mrr_delta_cents > 0 AND COALESCE(summary, '') NOT LIKE '{NEW_SUMMARY}'), 0)::bigint, \
                    COALESCE(sum(mrr_delta_cents) FILTER (WHERE mrr_delta_cents < 0), 0)::bigint, \
                    count(DISTINCT team_id) FILTER (WHERE summary = 'Cancelled')::bigint \
             FROM stripe_events WHERE received_at >= $1"
        ))
        .bind(day_start(month))
        .fetch_one(db)
        .await?;

    let events: Vec<EventRow> = sqlx::query_as(
        "SELECT e.id, e.type, t.slug::text AS team_slug, t.name AS team_name, e.summary, \
                e.mrr_delta_cents, e.received_at AS at \
         FROM stripe_events e LEFT JOIN teams t ON t.id = e.team_id \
         ORDER BY e.received_at DESC, e.id DESC LIMIT $1",
    )
    .bind(STRIPE_EVENTS)
    .fetch_all(db)
    .await?;
    let events_24h: i64 =
        sqlx::query_scalar("SELECT count(*)::bigint FROM stripe_events WHERE received_at > $1")
            .bind(now - Duration::hours(24))
            .fetch_one(db)
            .await?;

    Ok(Json(StripeStats {
        currency: prices.monthly.currency.clone(),
        mrr_cents: totals.mrr_cents,
        arr_cents: totals.mrr_cents * 12,
        paying_teams: totals.paying_teams,
        seats: totals.seats,
        arpa: if totals.paying_teams == 0 {
            0
        } else {
            totals.mrr_cents / totals.paying_teams
        },
        past_due: StripePastDue {
            teams: totals.past_due_teams,
            at_risk_cents: totals.past_due_cents,
        },
        mrr_90d: mrr_90d
            .into_iter()
            .map(|(day, mrr_cents)| MrrPoint {
                day: day.to_string(),
                mrr_cents,
            })
            .collect(),
        net_new: NetNew {
            since: month,
            total_cents: new_cents + expansion_cents + churned_cents,
            new_cents,
            expansion_cents,
            churned_cents,
            cancelled_teams,
        },
        by_source: vec![
            source("stripe_annual", totals.annual),
            source("stripe_monthly", totals.monthly),
            source("granted", granted),
        ],
        webhooks: WebhookStats {
            events_24h,
            bad_signatures_24h: state.metrics.bad_signatures_24h(now),
            duplicates_24h: state.metrics.duplicates_24h(now),
        },
        events: events
            .into_iter()
            .map(|e| StripeEvent {
                id: e.id,
                kind: e.kind,
                team: e
                    .team_slug
                    .zip(e.team_name)
                    .map(|(slug, name)| StripeTeam { slug, name }),
                summary: e.summary,
                mrr_delta_cents: e.mrr_delta_cents,
                at: e.at,
            })
            .collect(),
    }))
}
