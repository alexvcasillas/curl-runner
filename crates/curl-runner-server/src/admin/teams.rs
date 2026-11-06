//! Teams: list, search, CSV, detail and the actions on one team (plan
//! override, suspension, delete, member roles, Stripe sync, invoice resend,
//! email the owners). Any admin reads. Writes need `billing` (override,
//! Stripe), `teams` (suspension, delete, roles) or `emails`, and log one
//! event each in their own transaction.
//!
//! An override beats Stripe while `override_until` is null or in the future
//! (see `billing::effective`); Stripe keeps billing meanwhile, which the
//! grant answers as `stripe_conflict`. A suspended team cannot publish and its
//! share links answer 404; nothing is deleted and members keep reading their
//! own team.

use axum::Json;
use axum::extract::State;
use axum::http::StatusCode;
use axum::response::Response;
use chrono::{DateTime, Datelike, NaiveDate, SecondsFormat, TimeZone, Utc};
use curl_runner_cloud_proto::{Access, ErrorCode, Plan, Role, Source};
use futures_util::{StreamExt, TryStreamExt, stream};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use sqlx::{FromRow, PgConnection, PgPool};
use uuid::Uuid;

use super::auth::{AdminAuth, Perm};
use super::csv;
use super::events::{AdminTx, Event};
use super::page::{Cursor, EXPORT_PAGE, invalid, non_empty, page_limit, search_pattern};
use super::users::{EmailReq, clean_reason};
use crate::auth::session::column_enum;
use crate::billing::effective::{
    PlanChange, PlanRow, PlanSource, effective_plan, effective_plan_sql, override_active_sql,
    parse_plan, plan_columns, plan_str,
};
use crate::billing::mrr::{Billed, granted_sql, is_live_subscription, live_subscription_sql};
use crate::billing::routes::{current_prices, stripe_down};
use crate::billing::sync::{OVERRIDE_REMINDER, apply_missing, apply_subscription, mail_owners};
use crate::billing::{BillingError, Prices};
use crate::collections::share_blocked;
use crate::error::ApiError;
use crate::extract::{ApiJson, ApiPath, ApiQuery};
use crate::mail::Mail;
use crate::plans::limits;
use crate::state::AppState;
use crate::teams::{delete_team_row, enum_str, is_last_owner, last_owner};

const HISTORY_ROWS: i64 = 100;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Filter {
    All,
    Stripe,
    Granted,
    PastDue,
    Free,
    Suspended,
}

impl Filter {
    fn parse(s: &str) -> Option<Self> {
        Some(match s {
            "all" => Self::All,
            "stripe" => Self::Stripe,
            "granted" => Self::Granted,
            "past_due" => Self::PastDue,
            "free" => Self::Free,
            "suspended" => Self::Suspended,
            _ => return None,
        })
    }

    fn as_str(self) -> &'static str {
        match self {
            Self::All => "all",
            Self::Stripe => "stripe",
            Self::Granted => "granted",
            Self::PastDue => "past_due",
            Self::Free => "free",
            Self::Suspended => "suspended",
        }
    }
}

#[derive(Deserialize)]
pub struct ListQuery {
    q: Option<String>,
    filter: Option<String>,
    cursor: Option<String>,
    limit: Option<String>,
}

#[derive(Debug, Clone)]
struct Filters {
    /// `ILIKE` pattern for `q`, already escaped.
    pattern: Option<String>,
    filter: Filter,
}

impl ListQuery {
    fn filters(&self) -> Result<Filters, ApiError> {
        let pattern = search_pattern(self.q.as_ref())?;
        let filter = match non_empty(self.filter.as_ref()) {
            None => Filter::All,
            Some(f) => Filter::parse(f).ok_or_else(|| {
                invalid("`filter` must be all, stripe, granted, past_due, free or suspended.")
            })?,
        };
        Ok(Filters { pattern, filter })
    }

    fn cursor(&self) -> Result<Option<Cursor>, ApiError> {
        Cursor::parse(self.cursor.as_ref())
    }
}

/// `$1` the pattern, `$2` the filter, `$3` now. `stripe` is a live
/// subscription, override or not; `granted` is [`granted_sql`]; `free` is on
/// Free with neither an override nor a suspension.
fn where_sql() -> String {
    let active = override_active_sql("t", "$3");
    let effective = effective_plan_sql("t", "$3");
    let live = live_subscription_sql("t");
    let granted = granted_sql("t", "$3");
    format!(
        "($1::text IS NULL OR t.name ILIKE $1 OR t.slug::text ILIKE $1 \
           OR EXISTS (SELECT 1 FROM memberships m JOIN users u ON u.id = m.user_id \
                      WHERE m.team_id = t.id AND m.role = 'owner' AND u.email::text ILIKE $1)) \
         AND (($2 = 'all') \
           OR ($2 = 'stripe' AND {live}) \
           OR ($2 = 'granted' AND {granted}) \
           OR ($2 = 'past_due' AND t.past_due_since IS NOT NULL) \
           OR ($2 = 'free' AND {effective} = 'free' AND NOT {active} AND t.suspended_at IS NULL) \
           OR ($2 = 'suspended' AND t.suspended_at IS NOT NULL))"
    )
}

#[derive(FromRow)]
struct TeamSql {
    id: Uuid,
    slug: String,
    name: String,
    created_at: DateTime<Utc>,
    #[sqlx(flatten)]
    plan: PlanRow,
    billing_status: Option<String>,
    billing_interval: Option<String>,
    past_due_since: Option<DateTime<Utc>>,
    cancel_at: Option<DateTime<Utc>>,
    seats_synced: Option<i32>,
    suspended_at: Option<DateTime<Utc>>,
    owner_email: Option<String>,
    owner_name: Option<String>,
    members: i64,
    collections: i64,
}

#[derive(Serialize)]
pub struct OwnerRef {
    email: String,
    name: Option<String>,
}

#[derive(Serialize)]
pub struct TeamRow {
    slug: String,
    name: String,
    created_at: DateTime<Utc>,
    /// The longest-standing owner.
    owner: Option<OwnerRef>,
    members: i64,
    collections: i64,
    plan: Plan,
    plan_source: PlanSource,
    billing_status: Option<String>,
    billing_interval: Option<String>,
    past_due_since: Option<DateTime<Utc>>,
    cancel_at: Option<DateTime<Utc>>,
    /// When an active override ends; `null` with no override or no end date.
    override_until: Option<DateTime<Utc>>,
    suspended: bool,
    /// What its Stripe subscription brings in a month: seats times the cached
    /// price, annual counted as a twelfth, as on the Stripe page. `0` when the
    /// team is not revenue (`billing::mrr`), `null` when the price can't be
    /// read.
    mrr_cents: Option<i64>,
}

/// The unit price for `interval` (`monthly` or `annual`), and its currency.
fn unit_price<'a>(prices: &'a Prices, interval: Option<&str>) -> Option<(i64, &'a str)> {
    let price = match interval {
        Some("monthly") => &prices.monthly,
        Some("annual") => &prices.annual,
        _ => return None,
    };
    Some((price.unit_amount, price.currency.as_str()))
}

/// The seats Stripe bills: the last count pushed, else the member count.
fn seats(seats_synced: Option<i32>, members: i64) -> i64 {
    seats_synced.map_or(members, i64::from).max(1)
}

impl TeamSql {
    fn mrr_cents(&self, prices: Option<&Prices>) -> Option<i64> {
        let billed = Billed {
            subscribed: self.plan.stripe_subscription_id.is_some(),
            status: self.billing_status.as_deref(),
            interval: self.billing_interval.as_deref(),
            seats: seats(self.seats_synced, self.members),
        };
        if !billed.is_revenue() {
            return Some(0);
        }
        Some(billed.mrr(prices?))
    }

    fn into_row(self, now: DateTime<Utc>, prices: Option<&Prices>) -> TeamRow {
        let (plan, plan_source) = effective_plan(&self.plan, now);
        let mrr_cents = self.mrr_cents(prices);
        TeamRow {
            owner: self.owner_email.map(|email| OwnerRef {
                email,
                name: self.owner_name,
            }),
            slug: self.slug,
            name: self.name,
            created_at: self.created_at,
            members: self.members,
            collections: self.collections,
            plan,
            plan_source,
            billing_status: self.billing_status,
            billing_interval: self.billing_interval,
            past_due_since: self.past_due_since,
            cancel_at: self.cancel_at,
            override_until: (plan_source == PlanSource::Override)
                .then_some(self.plan.override_until)
                .flatten(),
            suspended: self.suspended_at.is_some(),
            mrr_cents,
        }
    }
}

/// Up to `limit` teams after `cursor` (newest first), and the next cursor.
async fn page(
    db: &PgPool,
    f: &Filters,
    now: DateTime<Utc>,
    cursor: Option<Cursor>,
    limit: i64,
) -> Result<(Vec<TeamSql>, Option<Cursor>), ApiError> {
    let mut rows: Vec<TeamSql> = sqlx::query_as(&format!(
        "SELECT t.id, t.slug::text AS slug, t.name, t.created_at, {}, \
                t.billing_status, t.billing_interval, t.past_due_since, t.cancel_at, \
                t.seats_synced, t.suspended_at, o.email AS owner_email, o.name AS owner_name, \
                (SELECT count(*) FROM memberships m WHERE m.team_id = t.id) AS members, \
                (SELECT count(*) FROM collections c WHERE c.team_id = t.id) AS collections \
         FROM teams t \
         LEFT JOIN LATERAL (SELECT u.email::text AS email, u.name FROM memberships m \
                            JOIN users u ON u.id = m.user_id \
                            WHERE m.team_id = t.id AND m.role = 'owner' \
                            ORDER BY m.created_at, u.email LIMIT 1) o ON true \
         WHERE {} AND ($4::timestamptz IS NULL OR (t.created_at, t.id) < ($4, $5::uuid)) \
         ORDER BY t.created_at DESC, t.id DESC LIMIT $6",
        plan_columns("t"),
        where_sql()
    ))
    .bind(&f.pattern)
    .bind(f.filter.as_str())
    .bind(now)
    .bind(cursor.map(|c| c.at))
    .bind(cursor.map(|c| c.id))
    .bind(limit + 1)
    .fetch_all(db)
    .await?;
    let more = rows.len() as i64 > limit;
    rows.truncate(usize::try_from(limit).unwrap_or(0));
    let next = if more {
        rows.last().map(|r| Cursor {
            at: r.created_at,
            id: r.id,
        })
    } else {
        None
    };
    Ok((rows, next))
}

/// The cached price list, or `None` (logged) when billing is off or Stripe
/// can't be read. At most one Stripe read an hour, never one per row.
async fn prices_or_none(state: &AppState) -> Option<Prices> {
    match current_prices(state).await {
        Ok(prices) => prices,
        Err(err) => {
            tracing::warn!("stripe prices unavailable: {err}");
            None
        }
    }
}

#[derive(Serialize)]
pub struct TeamsPage {
    items: Vec<TeamRow>,
    next_cursor: Option<String>,
    /// Teams matching the filters, over every page.
    total: i64,
}

/// `GET /v1/admin/teams?q=&filter=all|stripe|granted|past_due|free|suspended&cursor=&limit=`.
/// `q` matches the name, the slug or an owner's email.
pub async fn list(
    _admin: AdminAuth,
    State(state): State<AppState>,
    ApiQuery(q): ApiQuery<ListQuery>,
) -> Result<Json<TeamsPage>, ApiError> {
    let filters = q.filters()?;
    let cursor = q.cursor()?;
    let limit = page_limit(q.limit.as_ref())?;
    let now = state.clock.now();
    let (rows, next) = page(&state.db, &filters, now, cursor, limit).await?;
    let total: i64 = sqlx::query_scalar(&format!(
        "SELECT count(*) FROM teams t WHERE {}",
        where_sql()
    ))
    .bind(&filters.pattern)
    .bind(filters.filter.as_str())
    .bind(now)
    .fetch_one(&state.db)
    .await?;
    let prices = prices_or_none(&state).await;
    Ok(Json(TeamsPage {
        items: rows
            .into_iter()
            .map(|r| r.into_row(now, prices.as_ref()))
            .collect(),
        next_cursor: next.map(Cursor::encode),
        total,
    }))
}

const CSV_HEADER: [&str; 12] = [
    "slug",
    "name",
    "owner",
    "members",
    "collections",
    "plan",
    "plan_source",
    "billing_status",
    "override_until",
    "suspended",
    "mrr_cents",
    "created_at",
];

fn csv_row(r: TeamRow) -> Vec<String> {
    let at = |d: DateTime<Utc>| d.to_rfc3339_opts(SecondsFormat::Secs, true);
    vec![
        r.slug,
        r.name,
        r.owner.map(|o| o.email).unwrap_or_default(),
        r.members.to_string(),
        r.collections.to_string(),
        plan_str(r.plan).to_owned(),
        r.plan_source.as_str().to_owned(),
        r.billing_status.unwrap_or_default(),
        r.override_until.map(at).unwrap_or_default(),
        r.suspended.to_string(),
        r.mrr_cents.map(|m| m.to_string()).unwrap_or_default(),
        at(r.created_at),
    ]
}

/// `GET /v1/admin/teams.csv` with the same `q` and `filter`, every matching
/// team, read in pages of 500 as the download goes.
pub async fn export(
    _admin: AdminAuth,
    State(state): State<AppState>,
    ApiQuery(q): ApiQuery<ListQuery>,
) -> Result<Response, ApiError> {
    let filters = q.filters()?;
    let now = state.clock.now();
    let prices = prices_or_none(&state).await;
    let db = state.db.clone();
    // State: `None` once the last page is out, else the cursor to read after.
    let pages = stream::try_unfold(Some(None), move |next: Option<Option<Cursor>>| {
        let db = db.clone();
        let filters = filters.clone();
        let prices = prices.clone();
        async move {
            let Some(cursor) = next else {
                return Ok(None);
            };
            let (rows, next) = page(&db, &filters, now, cursor, EXPORT_PAGE).await?;
            let rows: Vec<Vec<String>> = rows
                .into_iter()
                .map(|r| csv_row(r.into_row(now, prices.as_ref())))
                .collect();
            Ok::<_, ApiError>(Some((rows, next.map(Some))))
        }
    });
    let rows = pages
        .map_ok(|rows| stream::iter(rows.into_iter().map(Ok)))
        .try_flatten()
        .boxed();
    Ok(csv::stream("teams", now.date_naive(), &CSV_HEADER, rows))
}

// ---- detail ----

#[derive(FromRow)]
struct DetailSql {
    id: Uuid,
    slug: String,
    name: String,
    created_at: DateTime<Utc>,
    #[sqlx(flatten)]
    plan: PlanRow,
    stripe_customer_id: Option<String>,
    billing_status: Option<String>,
    billing_interval: Option<String>,
    past_due_since: Option<DateTime<Utc>>,
    cancel_at: Option<DateTime<Utc>>,
    current_period_end: Option<DateTime<Utc>>,
    seats_synced: Option<i32>,
    override_reason: Option<String>,
    override_at: Option<DateTime<Utc>>,
    override_reminded_at: Option<DateTime<Utc>>,
    override_by_email: Option<String>,
    suspended_at: Option<DateTime<Utc>>,
    suspended_reason: Option<String>,
    suspended_by_email: Option<String>,
    members: i64,
}

const DETAIL_SQL: &str = "SELECT t.id, t.slug::text AS slug, t.name, t.created_at, \
     t.plan, t.stripe_subscription_id, t.override_plan, t.override_until, \
     t.stripe_customer_id, t.billing_status, t.billing_interval, t.past_due_since, \
     t.cancel_at, t.current_period_end, t.seats_synced, t.override_reason, t.override_at, \
     t.override_reminded_at, ob.email::text AS override_by_email, t.suspended_at, \
     t.suspended_reason, sb.email::text AS suspended_by_email, \
     (SELECT count(*) FROM memberships m WHERE m.team_id = t.id) AS members \
     FROM teams t LEFT JOIN users ob ON ob.id = t.override_by \
     LEFT JOIN users sb ON sb.id = t.suspended_by";

async fn detail_row(conn: &mut PgConnection, slug: &str) -> Result<DetailSql, ApiError> {
    sqlx::query_as(&format!("{DETAIL_SQL} WHERE t.slug = $1"))
        .bind(slug)
        .fetch_optional(conn)
        .await?
        .ok_or_else(ApiError::not_found)
}

#[derive(Serialize)]
pub struct LimitsBody {
    /// Accepted members plus pending invites; `null` is unlimited.
    members: Option<u32>,
    restricted: bool,
}

#[derive(Serialize)]
pub struct TeamInfo {
    id: Uuid,
    slug: String,
    name: String,
    created_at: DateTime<Utc>,
    plan: Plan,
    plan_source: PlanSource,
    limits: LimitsBody,
}

/// The Stripe card. There is no card brand or last four: the subscription
/// read does not carry them, and the page does not add a Stripe call.
#[derive(Serialize)]
pub struct StripeBlock {
    status: Option<String>,
    interval: Option<String>,
    seats: i64,
    /// Seats times the cached price, per interval; `null` when the price
    /// can't be read.
    amount_cents: Option<i64>,
    currency: Option<String>,
    /// The current period's end: when it renews, unless `cancel_at` is set.
    renews: Option<DateTime<Utc>>,
    cancel_at: Option<DateTime<Utc>>,
    past_due_since: Option<DateTime<Utc>>,
    subscription_id: String,
    customer_id: Option<String>,
}

#[derive(Serialize)]
pub struct OverrideBody {
    plan: Plan,
    /// `null` for no end date.
    until: Option<DateTime<Utc>>,
    reason: Option<String>,
    /// The granting admin's current email; `null` once their account is gone.
    by: Option<String>,
    at: Option<DateTime<Utc>>,
    /// When the 14-day reminder went out.
    reminded_at: Option<DateTime<Utc>>,
    /// False once `until` has passed and before the worker clears it.
    active: bool,
}

#[derive(Serialize)]
pub struct Suspension {
    at: DateTime<Utc>,
    reason: Option<String>,
    /// The admin's current email; `null` once their account is gone.
    by: Option<String>,
}

impl DetailSql {
    fn stripe(&self, prices: Option<&Prices>) -> Option<StripeBlock> {
        let subscription_id = self.plan.stripe_subscription_id.clone()?;
        let seats = seats(self.seats_synced, self.members);
        let unit = prices.and_then(|p| unit_price(p, self.billing_interval.as_deref()));
        Some(StripeBlock {
            status: self.billing_status.clone(),
            interval: self.billing_interval.clone(),
            seats,
            amount_cents: unit.map(|(amount, _)| amount * seats),
            currency: unit.map(|(_, currency)| currency.to_owned()),
            renews: self.current_period_end,
            cancel_at: self.cancel_at,
            past_due_since: self.past_due_since,
            subscription_id,
            customer_id: self.stripe_customer_id.clone(),
        })
    }

    fn override_body(&self, now: DateTime<Utc>) -> Option<OverrideBody> {
        let plan = self.plan.override_plan.as_deref()?;
        Some(OverrideBody {
            plan: parse_plan(plan),
            until: self.plan.override_until,
            reason: self.override_reason.clone(),
            by: self.override_by_email.clone(),
            at: self.override_at,
            reminded_at: self.override_reminded_at,
            active: self.plan.override_active(now),
        })
    }

    fn suspension(&self) -> Option<Suspension> {
        self.suspended_at.map(|at| Suspension {
            at,
            reason: self.suspended_reason.clone(),
            by: self.suspended_by_email.clone(),
        })
    }
}

#[derive(Serialize, FromRow)]
pub struct HistoryRow {
    at: DateTime<Utc>,
    /// `default`, `admin`, `stripe`, `past_due`, `override`,
    /// `override_removed` or `expired`.
    source: String,
    plan: String,
    /// The admin's email for admin changes; `null` otherwise.
    admin: Option<String>,
    reason: Option<String>,
    detail: Value,
}

#[derive(FromRow)]
struct MemberSql {
    user_id: Uuid,
    email: String,
    name: Option<String>,
    role: String,
    joined_at: DateTime<Utc>,
}

#[derive(Serialize)]
pub struct MemberRow {
    user_id: Uuid,
    email: String,
    name: Option<String>,
    role: Role,
    joined_at: DateTime<Utc>,
}

#[derive(Serialize)]
pub struct PendingRow {
    id: Uuid,
    email: String,
    role: Role,
    expires_at: DateTime<Utc>,
}

#[derive(FromRow)]
struct CollectionSql {
    slug: String,
    name: String,
    source: String,
    access: String,
    current_version: Option<i32>,
    updated_at: Option<DateTime<Utc>>,
    link_blocked: bool,
}

#[derive(Serialize)]
pub struct CollectionRow {
    slug: String,
    name: String,
    source: Source,
    access: Access,
    current_version: Option<i32>,
    updated_at: Option<DateTime<Utc>>,
    /// The share link answers 404 (the team is suspended).
    link_blocked: bool,
}

#[derive(Serialize)]
pub struct Usage {
    /// Publish tokens used since the start of the month (UTC).
    tokens_used_this_month: i64,
    /// Versions published since the start of the month (UTC).
    publishes_this_month: i64,
}

#[derive(Serialize)]
pub struct TeamDetail {
    team: TeamInfo,
    stripe: Option<StripeBlock>,
    #[serde(rename = "override")]
    override_: Option<OverrideBody>,
    /// Newest first, at most 100.
    plan_history: Vec<HistoryRow>,
    members: Vec<MemberRow>,
    pending: Vec<PendingRow>,
    collections: Vec<CollectionRow>,
    usage: Usage,
    suspension: Option<Suspension>,
}

fn month_start(now: DateTime<Utc>) -> DateTime<Utc> {
    let first = NaiveDate::from_ymd_opt(now.year(), now.month(), 1)
        .and_then(|d| d.and_hms_opt(0, 0, 0))
        .map(|d| Utc.from_utc_datetime(&d));
    first.unwrap_or(now)
}

/// `GET /v1/admin/teams/{slug}`
pub async fn detail(
    _admin: AdminAuth,
    State(state): State<AppState>,
    ApiPath(slug): ApiPath<String>,
) -> Result<Json<TeamDetail>, ApiError> {
    let now = state.clock.now();
    let mut conn = state.db.acquire().await?;
    let t = detail_row(&mut conn, &slug).await?;
    let (plan, plan_source) = effective_plan(&t.plan, now);
    let settings = state.settings().await?;
    let lim = limits(plan, &settings);
    let history: Vec<HistoryRow> = sqlx::query_as(
        "SELECT h.at, h.source, h.plan, a.email::text AS admin, h.reason, h.detail \
         FROM plan_history h LEFT JOIN users a ON a.id = h.admin_user_id \
         WHERE h.team_id = $1 ORDER BY h.at DESC, h.id DESC LIMIT $2",
    )
    .bind(t.id)
    .bind(HISTORY_ROWS)
    .fetch_all(&mut *conn)
    .await?;
    let members: Vec<MemberSql> = sqlx::query_as(
        "SELECT u.id AS user_id, u.email::text AS email, u.name, m.role, m.created_at AS joined_at \
         FROM memberships m \
         JOIN users u ON u.id = m.user_id WHERE m.team_id = $1 \
         ORDER BY m.role = 'owner' DESC, m.created_at, u.email",
    )
    .bind(t.id)
    .fetch_all(&mut *conn)
    .await?;
    let pending: Vec<(Uuid, String, String, DateTime<Utc>)> = sqlx::query_as(
        "SELECT id, email::text, role, expires_at FROM member_invites \
         WHERE team_id = $1 AND accepted_at IS NULL AND expires_at > $2 \
         ORDER BY expires_at, email",
    )
    .bind(t.id)
    .bind(now)
    .fetch_all(&mut *conn)
    .await?;
    let collections: Vec<CollectionSql> = sqlx::query_as(&format!(
        "SELECT c.slug::text AS slug, c.name, c.source, c.access, c.current_version, \
                (SELECT v.created_at FROM versions v \
                 WHERE v.collection_id = c.id AND v.number = c.current_version) AS updated_at, \
                {} AS link_blocked \
         FROM collections c JOIN teams t ON t.id = c.team_id \
         WHERE c.team_id = $1 ORDER BY c.slug",
        share_blocked("t", "c")
    ))
    .bind(t.id)
    .fetch_all(&mut *conn)
    .await?;
    let since = month_start(now);
    let (tokens_used_this_month, publishes_this_month): (i64, i64) = sqlx::query_as(
        "SELECT (SELECT count(*) FROM tokens k WHERE k.team_id = $1 AND k.last_used_at >= $2), \
                (SELECT count(*) FROM versions v JOIN collections c ON c.id = v.collection_id \
                 WHERE c.team_id = $1 AND v.created_at >= $2)",
    )
    .bind(t.id)
    .bind(since)
    .fetch_one(&mut *conn)
    .await?;
    drop(conn);
    let prices = if t.plan.stripe_subscription_id.is_some() {
        prices_or_none(&state).await
    } else {
        None
    };
    Ok(Json(TeamDetail {
        stripe: t.stripe(prices.as_ref()),
        override_: t.override_body(now),
        suspension: t.suspension(),
        team: TeamInfo {
            id: t.id,
            slug: t.slug,
            name: t.name,
            created_at: t.created_at,
            plan,
            plan_source,
            limits: LimitsBody {
                members: lim.members,
                restricted: lim.restricted,
            },
        },
        plan_history: history,
        members: members
            .into_iter()
            .map(|m| {
                Ok(MemberRow {
                    user_id: m.user_id,
                    email: m.email,
                    name: m.name,
                    role: column_enum(m.role)?,
                    joined_at: m.joined_at,
                })
            })
            .collect::<Result<_, ApiError>>()?,
        pending: pending
            .into_iter()
            .map(|(id, email, role, expires_at)| {
                Ok(PendingRow {
                    id,
                    email,
                    role: column_enum(role)?,
                    expires_at,
                })
            })
            .collect::<Result<_, ApiError>>()?,
        collections: collections
            .into_iter()
            .map(|c| {
                Ok(CollectionRow {
                    slug: c.slug,
                    name: c.name,
                    source: column_enum(c.source)?,
                    access: column_enum(c.access)?,
                    current_version: c.current_version,
                    updated_at: c.updated_at,
                    link_blocked: c.link_blocked,
                })
            })
            .collect::<Result<_, ApiError>>()?,
        usage: Usage {
            tokens_used_this_month,
            publishes_this_month,
        },
    }))
}

// ---- writes ----

/// A team row held `FOR UPDATE` for one admin write.
#[derive(FromRow)]
struct Locked {
    id: Uuid,
    slug: String,
    name: String,
    #[sqlx(flatten)]
    plan: PlanRow,
    billing_status: Option<String>,
    override_reminded_at: Option<DateTime<Utc>>,
    suspended_at: Option<DateTime<Utc>>,
}

impl Locked {
    /// Stripe is still billing the team, whatever an override says.
    fn stripe_conflict(&self) -> bool {
        is_live_subscription(
            self.plan.stripe_subscription_id.as_deref(),
            self.billing_status.as_deref(),
        )
    }
}

/// The team `slug`, row-locked. Unknown: `404`.
async fn lock_team(conn: &mut PgConnection, slug: &str) -> Result<Locked, ApiError> {
    sqlx::query_as(&format!(
        "SELECT t.id, t.slug::text AS slug, t.name, {}, t.billing_status, \
                t.override_reminded_at, t.suspended_at \
         FROM teams t WHERE t.slug = $1 FOR UPDATE",
        plan_columns("t")
    ))
    .bind(slug)
    .fetch_optional(conn)
    .await?
    .ok_or_else(ApiError::not_found)
}

fn conflict(message: &str) -> ApiError {
    ApiError::new(ErrorCode::Conflict, message)
}

#[derive(Deserialize)]
pub struct OverrideReq {
    plan: Plan,
    /// `null` or absent for no end date.
    #[serde(default)]
    until: Option<DateTime<Utc>>,
    /// Internal only: never sent to the team.
    reason: Option<String>,
    /// Email the owners about the grant.
    #[serde(default)]
    notify: bool,
}

#[derive(Serialize)]
pub struct OverrideResult {
    /// The effective plan now.
    plan: Plan,
    plan_source: PlanSource,
    #[serde(rename = "override")]
    override_: Option<OverrideBody>,
    /// A live Stripe subscription keeps billing the team.
    stripe_conflict: bool,
}

/// `PUT /v1/admin/teams/{slug}/override`, perm `billing`: grant an override,
/// or edit the active one. A new end date re-arms the 14-day reminder,
/// unless the end is within 14 days and the owners were just told.
pub async fn grant(
    admin: AdminAuth,
    State(state): State<AppState>,
    ApiPath(slug): ApiPath<String>,
    ApiJson(req): ApiJson<OverrideReq>,
) -> Result<Json<OverrideResult>, ApiError> {
    admin.require(Perm::Billing)?;
    let reason = clean_reason(req.reason)?;
    let mut tx = AdminTx::begin(&state, &admin).await?;
    let at = tx.at();
    if req.until.is_some_and(|until| until <= at) {
        return Err(invalid("The end date must be in the future."));
    }
    let team = lock_team(&mut tx, &slug).await?;
    let updating = team.plan.override_active(at);
    // An override that ended but the worker has not cleared yet: record its
    // expiry first, so the history reads as it happened.
    if let Some(ended) = team.plan.override_plan.as_deref()
        && !updating
    {
        PlanChange {
            team_id: team.id,
            at,
            source: "expired",
            plan: team.plan.underlying().0,
            admin: None,
            reason: None,
            detail: json!({"override": ended, "until": team.plan.override_until}),
        }
        .record(&mut tx)
        .await?;
    }
    // Owners told now about an end inside the window need no reminder on top;
    // an edit that keeps the end date keeps its reminder state; any other new
    // end date re-arms it.
    let reminded_at = if req.notify
        && req
            .until
            .is_some_and(|until| until <= at + OVERRIDE_REMINDER)
    {
        Some(at)
    } else if updating && team.plan.override_until == req.until {
        team.override_reminded_at
    } else {
        None
    };
    let plan = enum_str(req.plan);
    sqlx::query(
        "UPDATE teams SET override_plan = $2, override_until = $3, override_reason = $4, \
         override_by = $5, override_at = $6, override_reminded_at = $7 WHERE id = $1",
    )
    .bind(team.id)
    .bind(&plan)
    .bind(req.until)
    .bind(&reason)
    .bind(admin.user_id)
    .bind(at)
    .bind(reminded_at)
    .execute(&mut *tx)
    .await?;
    let action = if updating {
        "team.override.update"
    } else {
        "team.override.grant"
    };
    PlanChange {
        team_id: team.id,
        at,
        source: "override",
        plan: req.plan,
        admin: Some(admin.user_id),
        reason: reason.as_deref(),
        detail: json!({"until": req.until, "action": action}),
    }
    .record(&mut tx)
    .await?;
    if req.notify {
        mail_owners(&state, &mut tx, team.id, |to| Mail::PlanGranted {
            to,
            team_name: team.name.clone(),
            slug: team.slug.clone(),
            plan: req.plan,
            until: req.until,
            app_url: state.config.app_url.clone(),
        })
        .await?;
    }
    let stripe_conflict = team.stripe_conflict();
    tx.commit(
        Event::new(action)
            .target("team", team.id, &team.slug)
            .detail(json!({
                "plan": plan,
                "until": req.until,
                "reason": reason,
                "notify": req.notify,
                "stripe_conflict": stripe_conflict,
            })),
    )
    .await?;
    Ok(Json(OverrideResult {
        plan: req.plan,
        plan_source: PlanSource::Override,
        override_: Some(OverrideBody {
            plan: req.plan,
            until: req.until,
            reason,
            by: Some(admin.email.clone()),
            at: Some(at),
            reminded_at,
            active: true,
        }),
        stripe_conflict,
    }))
}

/// `DELETE /v1/admin/teams/{slug}/override`, perm `billing`: back to the
/// Stripe or stored plan. `409` with no override.
pub async fn remove_override(
    admin: AdminAuth,
    State(state): State<AppState>,
    ApiPath(slug): ApiPath<String>,
) -> Result<Json<OverrideResult>, ApiError> {
    admin.require(Perm::Billing)?;
    let mut tx = AdminTx::begin(&state, &admin).await?;
    let at = tx.at();
    let team = lock_team(&mut tx, &slug).await?;
    let Some(was) = team.plan.override_plan.clone() else {
        return Err(conflict("This team has no plan override."));
    };
    sqlx::query(
        "UPDATE teams SET override_plan = NULL, override_until = NULL, override_reason = NULL, \
         override_by = NULL, override_at = NULL, override_reminded_at = NULL WHERE id = $1",
    )
    .bind(team.id)
    .execute(&mut *tx)
    .await?;
    let (plan, plan_source) = team.plan.underlying();
    PlanChange {
        team_id: team.id,
        at,
        source: "override_removed",
        plan,
        admin: Some(admin.user_id),
        reason: None,
        detail: json!({"override": was, "until": team.plan.override_until}),
    }
    .record(&mut tx)
    .await?;
    let stripe_conflict = team.stripe_conflict();
    tx.commit(
        Event::new("team.override.remove")
            .target("team", team.id, &team.slug)
            .detail(json!({"plan": was, "until": team.plan.override_until})),
    )
    .await?;
    Ok(Json(OverrideResult {
        plan,
        plan_source,
        override_: None,
        stripe_conflict,
    }))
}

#[derive(Serialize)]
pub struct StatusBody {
    slug: String,
    /// `active` or `suspended`.
    status: &'static str,
}

#[derive(Deserialize)]
pub struct SuspendReq {
    reason: Option<String>,
    /// Email the owners that the team is suspended.
    #[serde(default)]
    notify: bool,
}

/// `POST /v1/admin/teams/{slug}/suspend`, perm `teams`. `409` if already.
pub async fn suspend(
    admin: AdminAuth,
    State(state): State<AppState>,
    ApiPath(slug): ApiPath<String>,
    ApiJson(req): ApiJson<SuspendReq>,
) -> Result<Json<StatusBody>, ApiError> {
    admin.require(Perm::Teams)?;
    let reason = clean_reason(req.reason)?;
    let mut tx = AdminTx::begin(&state, &admin).await?;
    let team = lock_team(&mut tx, &slug).await?;
    if team.suspended_at.is_some() {
        return Err(conflict("That team is already suspended."));
    }
    sqlx::query(
        "UPDATE teams SET suspended_at = $2, suspended_by = $3, suspended_reason = $4 \
         WHERE id = $1",
    )
    .bind(team.id)
    .bind(tx.at())
    .bind(admin.user_id)
    .bind(&reason)
    .execute(&mut *tx)
    .await?;
    if req.notify {
        mail_owners(&state, &mut tx, team.id, |to| Mail::TeamSuspended {
            to,
            team_name: team.name.clone(),
            reason: reason.clone(),
        })
        .await?;
    }
    tx.commit(
        Event::new("team.suspend")
            .target("team", team.id, &team.slug)
            .detail(json!({"reason": reason, "notify": req.notify})),
    )
    .await?;
    Ok(Json(StatusBody {
        slug: team.slug,
        status: "suspended",
    }))
}

/// `POST /v1/admin/teams/{slug}/unsuspend`, perm `teams`. `409` if not suspended.
pub async fn unsuspend(
    admin: AdminAuth,
    State(state): State<AppState>,
    ApiPath(slug): ApiPath<String>,
) -> Result<Json<StatusBody>, ApiError> {
    admin.require(Perm::Teams)?;
    let mut tx = AdminTx::begin(&state, &admin).await?;
    let team = lock_team(&mut tx, &slug).await?;
    if team.suspended_at.is_none() {
        return Err(conflict("That team is not suspended."));
    }
    sqlx::query(
        "UPDATE teams SET suspended_at = NULL, suspended_by = NULL, suspended_reason = NULL \
         WHERE id = $1",
    )
    .bind(team.id)
    .execute(&mut *tx)
    .await?;
    tx.commit(Event::new("team.unsuspend").target("team", team.id, &team.slug))
        .await?;
    Ok(Json(StatusBody {
        slug: team.slug,
        status: "active",
    }))
}

/// `DELETE /v1/admin/teams/{slug}`, perm `teams`: the 2a team delete, which
/// cancels the Stripe subscription first (`503` and nothing deleted when
/// Stripe can't be reached).
pub async fn delete(
    admin: AdminAuth,
    State(state): State<AppState>,
    ApiPath(slug): ApiPath<String>,
) -> Result<StatusCode, ApiError> {
    admin.require(Perm::Teams)?;
    let mut tx = AdminTx::begin(&state, &admin).await?;
    let team = lock_team(&mut tx, &slug).await?;
    let cancelled = delete_team_row(&state, &mut tx, team.id, &team.slug).await?;
    tx.commit(
        Event::new("team.delete")
            .target("team", team.id, &team.slug)
            .detail(json!({"name": team.name, "subscription_cancelled": cancelled})),
    )
    .await?;
    Ok(StatusCode::NO_CONTENT)
}

#[derive(Deserialize)]
pub struct RoleReq {
    role: Role,
}

#[derive(Serialize)]
pub struct RoleBody {
    user_id: Uuid,
    role: Role,
}

/// `PATCH /v1/admin/teams/{slug}/members/{user_id}`, perm `teams`. The team
/// keeps at least one owner (`409`). Not a member: `404`.
pub async fn set_member_role(
    admin: AdminAuth,
    State(state): State<AppState>,
    ApiPath((slug, user_id)): ApiPath<(String, Uuid)>,
    ApiJson(req): ApiJson<RoleReq>,
) -> Result<Json<RoleBody>, ApiError> {
    admin.require(Perm::Teams)?;
    let mut tx = AdminTx::begin(&state, &admin).await?;
    let team = lock_team(&mut tx, &slug).await?;
    let member: Option<(String, String)> = sqlx::query_as(
        "SELECT m.role, u.email::text FROM memberships m JOIN users u ON u.id = m.user_id \
         WHERE m.team_id = $1 AND m.user_id = $2",
    )
    .bind(team.id)
    .bind(user_id)
    .fetch_optional(&mut *tx)
    .await?;
    let (from, email) = member.ok_or_else(ApiError::not_found)?;
    let to = enum_str(req.role);
    if from == to {
        return Ok(Json(RoleBody {
            user_id,
            role: req.role,
        }));
    }
    if req.role != Role::Owner && is_last_owner(&mut tx, team.id, user_id).await? {
        return Err(last_owner());
    }
    sqlx::query("UPDATE memberships SET role = $3 WHERE team_id = $1 AND user_id = $2")
        .bind(team.id)
        .bind(user_id)
        .bind(&to)
        .execute(&mut *tx)
        .await?;
    tx.commit(
        Event::new("team.member.role")
            .target("team", team.id, &team.slug)
            .detail(json!({"user_id": user_id, "email": email, "from": from, "to": to})),
    )
    .await?;
    Ok(Json(RoleBody {
        user_id,
        role: req.role,
    }))
}

fn billing_off() -> ApiError {
    conflict("Billing is off on this server.")
}

#[derive(Serialize)]
pub struct SyncBody {
    plan: Plan,
    plan_source: PlanSource,
    /// `null` once the team has no subscription.
    stripe: Option<StripeBlock>,
}

/// `POST /v1/admin/teams/{slug}/stripe-sync`, perm `billing`: read the
/// subscription from Stripe and apply it, as a webhook would. One Stripe
/// no longer knows moves the team to Free.
pub async fn stripe_sync(
    admin: AdminAuth,
    State(state): State<AppState>,
    ApiPath(slug): ApiPath<String>,
) -> Result<Json<SyncBody>, ApiError> {
    admin.require(Perm::Billing)?;
    let billing = state.billing.clone().ok_or_else(billing_off)?;
    let found: Option<(Uuid, Option<String>)> =
        sqlx::query_as("SELECT id, stripe_subscription_id FROM teams WHERE slug = $1")
            .bind(&slug)
            .fetch_optional(&state.db)
            .await?;
    let (team_id, subscription) = found.ok_or_else(ApiError::not_found)?;
    let subscription =
        subscription.ok_or_else(|| conflict("This team has no Stripe subscription."))?;
    let mut tx = AdminTx::begin(&state, &admin).await?;
    let at = tx.at();
    // The webhook's per-subscription lock, taken before the read: a webhook
    // that applied an older read meanwhile has committed, so this newer read
    // is the one that lands, and the two never interleave.
    sqlx::query("SELECT pg_advisory_xact_lock(hashtext($1))")
        .bind(&subscription)
        .execute(&mut *tx)
        .await?;
    let read = match billing.subscription(&subscription).await {
        Ok(sub) => Some(sub),
        Err(BillingError::Api { status: 404, .. }) => None,
        Err(err) => return Err(stripe_down(err)),
    };
    let status = match &read {
        Some(sub) => {
            apply_subscription(&state, &mut tx, sub, at).await?;
            sub.status.clone()
        }
        None => {
            apply_missing(&state, &mut tx, &subscription, at).await?;
            "missing".to_owned()
        }
    };
    let t = sqlx::query_as::<_, DetailSql>(&format!("{DETAIL_SQL} WHERE t.id = $1"))
        .bind(team_id)
        .fetch_one(&mut *tx)
        .await?;
    tx.commit(
        Event::new("team.stripe_sync")
            .target("team", team_id, &t.slug)
            .detail(json!({"subscription": subscription, "status": status})),
    )
    .await?;
    let (plan, plan_source) = effective_plan(&t.plan, at);
    let prices = if t.plan.stripe_subscription_id.is_some() {
        prices_or_none(&state).await
    } else {
        None
    };
    Ok(Json(SyncBody {
        plan,
        plan_source,
        stripe: t.stripe(prices.as_ref()),
    }))
}

/// Stripe refusing the send (`400`, such as an invoice that is not finalized)
/// is a state the admin can see and wait out, not an outage.
fn resend_error(err: BillingError) -> ApiError {
    match err {
        BillingError::Api { status: 400, body } => {
            tracing::info!("stripe refused to send the invoice: {body}");
            conflict("There's no finished invoice to resend yet.")
        }
        err => stripe_down(err),
    }
}

/// `POST /v1/admin/teams/{slug}/resend-invoice`, perm `billing`: Stripe
/// emails the customer's newest invoice again.
pub async fn resend_invoice(
    admin: AdminAuth,
    State(state): State<AppState>,
    ApiPath(slug): ApiPath<String>,
) -> Result<Json<Value>, ApiError> {
    admin.require(Perm::Billing)?;
    let billing = state.billing.clone().ok_or_else(billing_off)?;
    let found: Option<(Uuid, String, Option<String>)> =
        sqlx::query_as("SELECT id, slug::text, stripe_customer_id FROM teams WHERE slug = $1")
            .bind(&slug)
            .fetch_optional(&state.db)
            .await?;
    let (team_id, slug, customer) = found.ok_or_else(ApiError::not_found)?;
    let customer = customer.ok_or_else(|| conflict("This team has no Stripe customer yet."))?;
    let invoice = billing
        .send_latest_invoice(&customer)
        .await
        .map_err(resend_error)?
        .ok_or_else(|| conflict("Stripe has no invoice for this team yet."))?;
    let tx = AdminTx::begin(&state, &admin).await?;
    tx.commit(
        Event::new("team.invoice.resend")
            .target("team", team_id, &slug)
            .detail(json!({"invoice_id": invoice, "customer": customer})),
    )
    .await?;
    Ok(Json(json!({ "invoice_id": invoice })))
}

/// `POST /v1/admin/teams/{slug}/email-owners {subject, body_md}`, perm
/// `emails`: one message to each owner.
pub async fn email_owners(
    admin: AdminAuth,
    State(state): State<AppState>,
    ApiPath(slug): ApiPath<String>,
    ApiJson(req): ApiJson<EmailReq>,
) -> Result<(StatusCode, Json<Value>), ApiError> {
    admin.require(Perm::Emails)?;
    let (subject, body) = req.checked()?;
    let mut tx = AdminTx::begin(&state, &admin).await?;
    let team = lock_team(&mut tx, &slug).await?;
    let owners: Vec<(Uuid, String)> = sqlx::query_as(
        "SELECT u.id, u.email::text FROM memberships m JOIN users u ON u.id = m.user_id \
         WHERE m.team_id = $1 AND m.role = 'owner' ORDER BY u.email",
    )
    .bind(team.id)
    .fetch_all(&mut *tx)
    .await?;
    for (user_id, to) in &owners {
        state
            .mailer
            .enqueue_for(
                &mut tx,
                Mail::AdminMessage {
                    to: to.clone(),
                    subject: subject.clone(),
                    body: body.clone(),
                },
                Some(*user_id),
            )
            .await?;
    }
    tx.commit(
        Event::new("team.email_owners")
            .target("team", team.id, &team.slug)
            .detail(json!({"subject": subject, "recipients": owners.len()})),
    )
    .await?;
    Ok((StatusCode::ACCEPTED, Json(json!({}))))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::billing::Price;

    #[test]
    fn filters_parse() {
        for f in ["all", "stripe", "granted", "past_due", "free", "suspended"] {
            assert_eq!(Filter::parse(f).unwrap().as_str(), f);
        }
        assert!(Filter::parse("override").is_none());
    }

    fn prices() -> Prices {
        let p = |unit_amount| Price {
            unit_amount,
            currency: "usd".into(),
            interval: String::new(),
        };
        Prices {
            monthly: p(900),
            annual: p(9_000),
        }
    }

    #[test]
    fn unit_price_follows_the_interval() {
        let prices = prices();
        assert_eq!(unit_price(&prices, Some("monthly")), Some((900, "usd")));
        assert_eq!(unit_price(&prices, Some("annual")), Some((9_000, "usd")));
        assert_eq!(unit_price(&prices, None), None);
    }

    #[test]
    fn seats_never_drop_below_one() {
        assert_eq!(seats(Some(3), 1), 3);
        assert_eq!(seats(None, 4), 4);
        assert_eq!(seats(Some(0), 0), 1);
    }

    #[test]
    fn resend_refusal_by_stripe_is_a_conflict_not_an_outage() {
        let err = resend_error(BillingError::Api {
            status: 400,
            body: "invoice is not finalized".into(),
        });
        assert_eq!(err.code, ErrorCode::Conflict);
        assert_eq!(err.message, "There's no finished invoice to resend yet.");
        let err = resend_error(BillingError::Unreachable("timeout".into()));
        assert_eq!(err.code, ErrorCode::Unavailable);
        let err = resend_error(BillingError::Api {
            status: 500,
            body: String::new(),
        });
        assert_eq!(err.code, ErrorCode::Unavailable);
    }

    #[test]
    fn month_start_is_the_first_at_midnight_utc() {
        let now: DateTime<Utc> = "2026-11-04T10:00:00Z".parse().unwrap();
        assert_eq!(
            month_start(now),
            "2026-11-01T00:00:00Z".parse::<DateTime<Utc>>().unwrap()
        );
    }
}
