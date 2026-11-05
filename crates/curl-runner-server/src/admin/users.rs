//! Users: list, search, CSV, detail and the actions on one account (suspend,
//! sign out, unlink, delete, email). Any admin reads; writes need `users`
//! (`emails` for the email) and log one event each in their own transaction.
//!
//! Suspension blocks every way of signing in (see `auth::session`) and deletes
//! the user's sessions. Publish tokens they made keep working, so CI does not
//! break; so does everything else the teams own. Deleting a user removes the
//! account, its identities, sessions and memberships, and the access-list
//! entries that are exactly its email (domain entries stay). Tokens the user
//! made stay with the team: `tokens.created_by` is `ON DELETE SET NULL`, so
//! they lose their creator and nothing else. Admin events keep the
//! `admin_email` snapshot, so the log still names a deleted admin.

use axum::Json;
use axum::extract::State;
use axum::http::StatusCode;
use axum::response::Response;
use chrono::{DateTime, SecondsFormat, Utc};
use curl_runner_cloud_proto::{ErrorCode, Plan, Role};
use futures_util::{StreamExt, TryStreamExt, stream};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use sqlx::types::Json as Jsonb;
use sqlx::{FromRow, PgConnection, PgPool};
use uuid::Uuid;

use super::activity::{Cursor, EXPORT_PAGE, invalid, non_empty, page_limit};
use super::auth::{AdminAuth, Perm};
use super::csv;
use super::events::{AdminTx, Event};
use crate::auth::session::column_enum;
use crate::billing::effective::{PlanRow, PlanSource, effective_plan, plan_columns};
use crate::collections::{SharedWithMe, shared_for, shared_from};
use crate::error::ApiError;
use crate::extract::{ApiJson, ApiPath, ApiQuery};
use crate::mail::Mail;
use crate::state::AppState;
use crate::teams::{first_operator, mark_seats_dirty};

const MAX_QUERY: usize = 200;
const MAX_REASON: usize = 500;
const MAX_SUBJECT: usize = 200;
const MAX_BODY: usize = 10_000;
const MAX_BULK: usize = 200;
const DETAIL_EVENTS: i64 = 20;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Filter {
    All,
    Members,
    Customers,
    Suspended,
    Admins,
}

impl Filter {
    fn parse(s: &str) -> Option<Self> {
        Some(match s {
            "all" => Self::All,
            "members" => Self::Members,
            "customers" => Self::Customers,
            "suspended" => Self::Suspended,
            "admins" => Self::Admins,
            _ => return None,
        })
    }

    fn as_str(self) -> &'static str {
        match self {
            Self::All => "all",
            Self::Members => "members",
            Self::Customers => "customers",
            Self::Suspended => "suspended",
            Self::Admins => "admins",
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
    /// Lowercased owner emails, for the `admins` filter.
    owners: Vec<String>,
}

/// `s` as an `ILIKE` contains-pattern, with `%`, `_` and `\` taken literally.
pub(super) fn like_pattern(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 2);
    out.push('%');
    for c in s.chars() {
        if matches!(c, '%' | '_' | '\\') {
            out.push('\\');
        }
        out.push(c);
    }
    out.push('%');
    out
}

/// The owners' emails, lowercased: `OPERATOR_EMAILS` and the first operator.
async fn owner_emails(state: &AppState) -> Result<Vec<String>, ApiError> {
    let mut owners = state.config.operators.clone();
    if let Some(first) = first_operator(&state.db).await? {
        owners.push(first);
    }
    for o in &mut owners {
        *o = o.to_lowercase();
    }
    Ok(owners)
}

impl ListQuery {
    async fn filters(&self, state: &AppState) -> Result<Filters, ApiError> {
        let pattern = match non_empty(self.q.as_ref()) {
            Some(q) if q.chars().count() > MAX_QUERY => {
                return Err(invalid("`q` is too long."));
            }
            Some(q) => Some(like_pattern(q)),
            None => None,
        };
        let filter = match non_empty(self.filter.as_ref()) {
            None => Filter::All,
            Some(f) => Filter::parse(f).ok_or_else(|| {
                invalid("`filter` must be all, members, customers, suspended or admins.")
            })?,
        };
        let owners = if filter == Filter::Admins {
            owner_emails(state).await?
        } else {
            Vec::new()
        };
        Ok(Filters {
            pattern,
            filter,
            owners,
        })
    }

    fn cursor(&self) -> Result<Option<Cursor>, ApiError> {
        non_empty(self.cursor.as_ref())
            .map(|c| Cursor::decode(c).ok_or_else(|| invalid("`cursor` is not valid.")))
            .transpose()
    }
}

const WHERE: &str = "($1::text IS NULL OR u.email::text ILIKE $1 OR u.name ILIKE $1) \
     AND (($2 = 'all') \
       OR ($2 = 'members' AND EXISTS (SELECT 1 FROM memberships m WHERE m.user_id = u.id)) \
       OR ($2 = 'customers' AND NOT EXISTS (SELECT 1 FROM memberships m WHERE m.user_id = u.id)) \
       OR ($2 = 'suspended' AND u.suspended_at IS NOT NULL) \
       OR ($2 = 'admins' AND (lower(u.email::text) = ANY($3) \
            OR EXISTS (SELECT 1 FROM admins a WHERE a.user_id = u.id))))";

#[derive(Serialize, Deserialize)]
pub struct TeamRef {
    slug: String,
    name: String,
    /// `owner` or `member`.
    role: Role,
}

#[derive(FromRow)]
struct UserSql {
    id: Uuid,
    email: String,
    name: Option<String>,
    created_at: DateTime<Utc>,
    suspended_at: Option<DateTime<Utc>>,
    methods: Vec<String>,
    teams: Jsonb<Vec<TeamRef>>,
    shared_in: i64,
    last_active: Option<DateTime<Utc>>,
}

#[derive(Serialize)]
pub struct UserRow {
    id: Uuid,
    email: String,
    name: Option<String>,
    /// How they can sign in: `email`, then each linked provider.
    methods: Vec<String>,
    teams: Vec<TeamRef>,
    /// Collections other teams share with them.
    shared_in: i64,
    /// The newest session use, if any session is left.
    last_active: Option<DateTime<Utc>>,
    /// `active` or `suspended`.
    status: &'static str,
    created_at: DateTime<Utc>,
}

fn status(suspended_at: Option<DateTime<Utc>>) -> &'static str {
    if suspended_at.is_some() {
        "suspended"
    } else {
        "active"
    }
}

impl From<UserSql> for UserRow {
    fn from(r: UserSql) -> Self {
        Self {
            id: r.id,
            email: r.email,
            name: r.name,
            methods: r.methods,
            teams: r.teams.0,
            shared_in: r.shared_in,
            last_active: r.last_active,
            status: status(r.suspended_at),
            created_at: r.created_at,
        }
    }
}

/// Up to `limit` users after `cursor` (newest account first), and the cursor
/// for the next page.
async fn page(
    state: &AppState,
    f: &Filters,
    cursor: Option<Cursor>,
    limit: i64,
) -> Result<(Vec<UserSql>, Option<Cursor>), ApiError> {
    let email = "lower(u.email::text)::citext";
    let domain = "('*@' || substring(lower(u.email::text) from '[^@]*$'))::citext";
    let mut rows: Vec<UserSql> = sqlx::query_as(&format!(
        "SELECT u.id, u.email::text AS email, u.name, u.created_at, u.suspended_at, \
                ARRAY['email']::text[] || COALESCE((SELECT array_agg(DISTINCT i.provider ORDER BY i.provider) \
                    FROM identities i WHERE i.user_id = u.id), '{{}}') AS methods, \
                COALESCE((SELECT jsonb_agg(jsonb_build_object('slug', t.slug::text, 'name', t.name, 'role', m.role) \
                    ORDER BY t.name, t.slug) \
                    FROM memberships m JOIN teams t ON t.id = m.team_id \
                    WHERE m.user_id = u.id), '[]'::jsonb) AS teams, \
                (SELECT count(*) {}) AS shared_in, \
                (SELECT max(s.last_used_at) FROM sessions s WHERE s.user_id = u.id) AS last_active \
         FROM users u \
         WHERE {WHERE} AND ($4::timestamptz IS NULL OR (u.created_at, u.id) < ($4, $5::uuid)) \
         ORDER BY u.created_at DESC, u.id DESC LIMIT $6",
        shared_from("u.id", email, domain, "$7"),
    ))
    .bind(&f.pattern)
    .bind(f.filter.as_str())
    .bind(&f.owners)
    .bind(cursor.map(|c| c.at))
    .bind(cursor.map(|c| c.id))
    .bind(limit + 1)
    .bind(state.clock.now())
    .fetch_all(&state.db)
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

async fn count(db: &PgPool, f: &Filters) -> Result<i64, ApiError> {
    Ok(
        sqlx::query_scalar(&format!("SELECT count(*) FROM users u WHERE {WHERE}"))
            .bind(&f.pattern)
            .bind(f.filter.as_str())
            .bind(&f.owners)
            .fetch_one(db)
            .await?,
    )
}

#[derive(Serialize)]
pub struct UsersPage {
    items: Vec<UserRow>,
    next_cursor: Option<String>,
    /// Users matching the filters, over every page.
    total: i64,
}

/// `GET /v1/admin/users?q=&filter=all|members|customers|suspended|admins&cursor=&limit=`
pub async fn list(
    _admin: AdminAuth,
    State(state): State<AppState>,
    ApiQuery(q): ApiQuery<ListQuery>,
) -> Result<Json<UsersPage>, ApiError> {
    let filters = q.filters(&state).await?;
    let cursor = q.cursor()?;
    let limit = page_limit(q.limit.as_ref())?;
    let (rows, next) = page(&state, &filters, cursor, limit).await?;
    Ok(Json(UsersPage {
        items: rows.into_iter().map(Into::into).collect(),
        next_cursor: next.map(Cursor::encode),
        total: count(&state.db, &filters).await?,
    }))
}

const CSV_HEADER: [&str; 8] = [
    "email",
    "name",
    "sign_in",
    "teams",
    "shared_in",
    "last_active",
    "status",
    "created_at",
];

fn csv_row(r: UserSql) -> Vec<String> {
    let at = |d: DateTime<Utc>| d.to_rfc3339_opts(SecondsFormat::Secs, true);
    vec![
        r.email,
        r.name.unwrap_or_default(),
        r.methods.join(" "),
        r.teams
            .0
            .iter()
            .map(|t| t.name.as_str())
            .collect::<Vec<_>>()
            .join("; "),
        r.shared_in.to_string(),
        r.last_active.map(at).unwrap_or_default(),
        status(r.suspended_at).to_owned(),
        at(r.created_at),
    ]
}

/// `GET /v1/admin/users.csv` with the same `q` and `filter`, every matching
/// user, read in pages of 500 as the download goes.
pub async fn export(
    _admin: AdminAuth,
    State(state): State<AppState>,
    ApiQuery(q): ApiQuery<ListQuery>,
) -> Result<Response, ApiError> {
    let filters = q.filters(&state).await?;
    let today = state.clock.now().date_naive();
    // State: `None` once the last page is out, else the cursor to read after.
    let pages = stream::try_unfold(Some(None), move |next: Option<Option<Cursor>>| {
        let state = state.clone();
        let filters = filters.clone();
        async move {
            let Some(cursor) = next else {
                return Ok(None);
            };
            let (rows, next) = page(&state, &filters, cursor, EXPORT_PAGE).await?;
            let rows: Vec<Vec<String>> = rows.into_iter().map(csv_row).collect();
            Ok::<_, ApiError>(Some((rows, next.map(Some))))
        }
    });
    let rows = pages
        .map_ok(|rows| stream::iter(rows.into_iter().map(Ok)))
        .try_flatten()
        .boxed();
    Ok(csv::stream("users", today, &CSV_HEADER, rows))
}

// ---- detail ----

#[derive(FromRow)]
struct ProfileSql {
    id: Uuid,
    email: String,
    name: Option<String>,
    created_at: DateTime<Utc>,
    product_emails: bool,
    suspended_at: Option<DateTime<Utc>>,
    suspended_reason: Option<String>,
    suspended_by_email: Option<String>,
    last_active: Option<DateTime<Utc>>,
}

#[derive(Serialize)]
pub struct Suspension {
    at: DateTime<Utc>,
    reason: Option<String>,
    /// The admin's current email; `None` once their account is gone.
    by: Option<String>,
}

#[derive(Serialize)]
pub struct Profile {
    id: Uuid,
    email: String,
    name: Option<String>,
    status: &'static str,
    created_at: DateTime<Utc>,
    last_active: Option<DateTime<Utc>>,
    product_emails: bool,
    suspension: Option<Suspension>,
}

#[derive(FromRow)]
struct TeamSql {
    slug: String,
    name: String,
    role: String,
    joined_at: DateTime<Utc>,
    #[sqlx(flatten)]
    plan: PlanRow,
}

#[derive(Serialize)]
pub struct TeamEntry {
    slug: String,
    name: String,
    role: Role,
    joined_at: DateTime<Utc>,
    plan: Plan,
    /// What backs the plan: `override` (an admin grant), `stripe` or `none`.
    plan_source: PlanSource,
}

#[derive(Serialize, FromRow)]
pub struct IdentityEntry {
    provider: String,
    /// The provider's id for the person; `None` for the `email` pseudo-identity.
    subject: Option<String>,
    email: Option<String>,
    created_at: DateTime<Utc>,
}

#[derive(Serialize, FromRow)]
pub struct SessionEntry {
    id: Uuid,
    kind: String,
    label: Option<String>,
    created_at: DateTime<Utc>,
    last_used_at: DateTime<Utc>,
    expires_at: DateTime<Utc>,
}

#[derive(Serialize, FromRow)]
pub struct EventEntry {
    id: Uuid,
    at: DateTime<Utc>,
    admin: String,
    action: String,
    detail: Value,
    ip: Option<String>,
}

#[derive(Serialize)]
pub struct UserDetail {
    user: Profile,
    teams: Vec<TeamEntry>,
    identities: Vec<IdentityEntry>,
    sessions: Vec<SessionEntry>,
    /// Collections other teams share with them. Never carries a share link.
    shared: Vec<SharedWithMe>,
    /// The last 20 admin events on them, including bulk actions that named them.
    events: Vec<EventEntry>,
}

/// `GET /v1/admin/users/{id}`
pub async fn detail(
    _admin: AdminAuth,
    State(state): State<AppState>,
    ApiPath(id): ApiPath<Uuid>,
) -> Result<Json<UserDetail>, ApiError> {
    let now = state.clock.now();
    let profile: ProfileSql = sqlx::query_as(
        "SELECT u.id, u.email::text AS email, u.name, u.created_at, u.product_emails, \
                u.suspended_at, u.suspended_reason, b.email::text AS suspended_by_email, \
                (SELECT max(s.last_used_at) FROM sessions s WHERE s.user_id = u.id) AS last_active \
         FROM users u LEFT JOIN users b ON b.id = u.suspended_by WHERE u.id = $1",
    )
    .bind(id)
    .fetch_optional(&state.db)
    .await?
    .ok_or_else(ApiError::not_found)?;
    let teams: Vec<TeamSql> = sqlx::query_as(&format!(
        "SELECT t.slug::text AS slug, t.name, m.role, m.created_at AS joined_at, {} \
         FROM memberships m JOIN teams t ON t.id = m.team_id \
         WHERE m.user_id = $1 ORDER BY t.name, t.slug",
        plan_columns("t")
    ))
    .bind(id)
    .fetch_all(&state.db)
    .await?;
    let linked: Vec<IdentityEntry> = sqlx::query_as(
        "SELECT provider, subject, email::text AS email, created_at FROM identities \
         WHERE user_id = $1 ORDER BY created_at, provider",
    )
    .bind(id)
    .fetch_all(&state.db)
    .await?;
    let mut identities = vec![IdentityEntry {
        provider: "email".to_owned(),
        subject: None,
        email: Some(profile.email.clone()),
        created_at: profile.created_at,
    }];
    identities.extend(linked);
    let sessions: Vec<SessionEntry> = sqlx::query_as(
        "SELECT id, kind, label, created_at, last_used_at, expires_at FROM sessions \
         WHERE user_id = $1 AND revoked_at IS NULL AND expires_at > $2 \
         ORDER BY last_used_at DESC, id",
    )
    .bind(id)
    .bind(now)
    .fetch_all(&state.db)
    .await?;
    let events: Vec<EventEntry> = sqlx::query_as(
        "SELECT e.id, e.at, COALESCE(a.email::text, e.admin_email) AS admin, e.action, \
                e.detail, e.ip \
         FROM admin_events e LEFT JOIN users a ON a.id = e.admin_user_id \
         WHERE (e.target_kind = 'user' AND e.target_id = $1) \
            OR (e.action LIKE 'users.bulk.%' AND e.detail->'ids' @> to_jsonb($1::text)) \
         ORDER BY e.at DESC, e.id DESC LIMIT $2",
    )
    .bind(id.to_string())
    .bind(DETAIL_EVENTS)
    .fetch_all(&state.db)
    .await?;
    let shared = shared_for(&state, id, &profile.email, false).await?;
    Ok(Json(UserDetail {
        user: Profile {
            id: profile.id,
            status: status(profile.suspended_at),
            suspension: profile.suspended_at.map(|at| Suspension {
                at,
                reason: profile.suspended_reason,
                by: profile.suspended_by_email,
            }),
            email: profile.email,
            name: profile.name,
            created_at: profile.created_at,
            last_active: profile.last_active,
            product_emails: profile.product_emails,
        },
        teams: teams
            .into_iter()
            .map(|t| {
                let (plan, plan_source) = effective_plan(&t.plan, now);
                Ok(TeamEntry {
                    slug: t.slug,
                    name: t.name,
                    role: column_enum(t.role)?,
                    joined_at: t.joined_at,
                    plan,
                    plan_source,
                })
            })
            .collect::<Result<_, ApiError>>()?,
        identities,
        sessions,
        shared,
        events,
    }))
}

// ---- writes ----

struct Target {
    id: Uuid,
    email: String,
    name: Option<String>,
}

impl Target {
    /// How the board's copy refers to them: a first name, else the email.
    fn short_name(&self) -> &str {
        self.name
            .as_deref()
            .and_then(|n| n.split_whitespace().next())
            .unwrap_or(&self.email)
    }
}

/// The user, row-locked. Unknown: `404`.
async fn lock_target(conn: &mut PgConnection, id: Uuid) -> Result<Target, ApiError> {
    let row: Option<(String, Option<String>)> =
        sqlx::query_as("SELECT email::text, name FROM users WHERE id = $1 FOR UPDATE")
            .bind(id)
            .fetch_optional(conn)
            .await?;
    let (email, name) = row.ok_or_else(ApiError::not_found)?;
    Ok(Target { id, email, name })
}

fn conflict(message: &str) -> ApiError {
    ApiError::new(ErrorCode::Conflict, message)
}

/// Suspending or deleting: not an owner, not yourself.
fn guard(admin: &AdminAuth, owners: &[String], target: &Target) -> Result<(), ApiError> {
    if owners.contains(&target.email.to_lowercase()) {
        return Err(conflict(
            "Owners come from OPERATOR_EMAILS and can't be suspended or deleted here.",
        ));
    }
    if target.id == admin.user_id {
        return Err(conflict("You can't do that to your own account."));
    }
    Ok(())
}

/// `reason` trimmed; empty is `None`.
pub(super) fn clean_reason(reason: Option<String>) -> Result<Option<String>, ApiError> {
    let reason = reason
        .map(|r| r.trim().to_owned())
        .filter(|r| !r.is_empty());
    if reason
        .as_ref()
        .is_some_and(|r| r.chars().count() > MAX_REASON)
    {
        return Err(invalid("The reason is too long."));
    }
    Ok(reason)
}

/// Mark suspended and delete every session. `false` if already suspended.
async fn suspend_row(
    conn: &mut PgConnection,
    admin: &AdminAuth,
    at: DateTime<Utc>,
    id: Uuid,
    reason: Option<&str>,
) -> Result<bool, ApiError> {
    let done = sqlx::query(
        "UPDATE users SET suspended_at = $2, suspended_by = $3, suspended_reason = $4 \
         WHERE id = $1 AND suspended_at IS NULL",
    )
    .bind(id)
    .bind(at)
    .bind(admin.user_id)
    .bind(reason)
    .execute(&mut *conn)
    .await?;
    if done.rows_affected() == 0 {
        return Ok(false);
    }
    sqlx::query("DELETE FROM sessions WHERE user_id = $1")
        .bind(id)
        .execute(conn)
        .await?;
    Ok(true)
}

/// Revoke every live session. Returns how many.
async fn revoke_all(
    conn: &mut PgConnection,
    at: DateTime<Utc>,
    user_id: Uuid,
) -> Result<u64, ApiError> {
    Ok(sqlx::query(
        "UPDATE sessions SET revoked_at = $2 WHERE user_id = $1 AND revoked_at IS NULL \
         AND expires_at > $2",
    )
    .bind(user_id)
    .bind(at)
    .execute(conn)
    .await?
    .rows_affected())
}

#[derive(Serialize)]
pub struct StatusBody {
    id: Uuid,
    status: &'static str,
}

#[derive(Deserialize)]
pub struct SuspendReq {
    reason: Option<String>,
    /// Also email them that the account is suspended.
    #[serde(default)]
    notify: bool,
}

/// `POST /v1/admin/users/{id}/suspend`: `409` if already suspended, an owner or the caller.
pub async fn suspend(
    admin: AdminAuth,
    State(state): State<AppState>,
    ApiPath(id): ApiPath<Uuid>,
    ApiJson(req): ApiJson<SuspendReq>,
) -> Result<Json<StatusBody>, ApiError> {
    admin.require(Perm::Users)?;
    let reason = clean_reason(req.reason)?;
    let owners = owner_emails(&state).await?;
    let mut tx = AdminTx::begin(&state, &admin).await?;
    let target = lock_target(&mut tx, id).await?;
    guard(&admin, &owners, &target)?;
    let at = tx.at();
    if !suspend_row(&mut tx, &admin, at, id, reason.as_deref()).await? {
        return Err(conflict("That account is already suspended."));
    }
    if req.notify {
        state
            .mailer
            .enqueue_for(
                &mut tx,
                Mail::AccountSuspended {
                    to: target.email.clone(),
                    reason: reason.clone(),
                },
                Some(id),
            )
            .await?;
    }
    tx.commit(
        Event::new("user.suspend")
            .target("user", id, &target.email)
            .detail(json!({"reason": reason, "notify": req.notify})),
    )
    .await?;
    Ok(Json(StatusBody {
        id,
        status: "suspended",
    }))
}

/// `POST /v1/admin/users/{id}/unsuspend`: `409` if not suspended.
pub async fn unsuspend(
    admin: AdminAuth,
    State(state): State<AppState>,
    ApiPath(id): ApiPath<Uuid>,
) -> Result<Json<StatusBody>, ApiError> {
    admin.require(Perm::Users)?;
    let mut tx = AdminTx::begin(&state, &admin).await?;
    let target = lock_target(&mut tx, id).await?;
    let done = sqlx::query(
        "UPDATE users SET suspended_at = NULL, suspended_by = NULL, suspended_reason = NULL \
         WHERE id = $1 AND suspended_at IS NOT NULL",
    )
    .bind(id)
    .execute(&mut *tx)
    .await?;
    if done.rows_affected() == 0 {
        return Err(conflict("That account is not suspended."));
    }
    tx.commit(Event::new("user.unsuspend").target("user", id, &target.email))
        .await?;
    Ok(Json(StatusBody {
        id,
        status: "active",
    }))
}

/// `POST /v1/admin/users/{id}/sign-out-everywhere`
pub async fn sign_out_everywhere(
    admin: AdminAuth,
    State(state): State<AppState>,
    ApiPath(id): ApiPath<Uuid>,
) -> Result<StatusCode, ApiError> {
    admin.require(Perm::Users)?;
    let mut tx = AdminTx::begin(&state, &admin).await?;
    let target = lock_target(&mut tx, id).await?;
    let at = tx.at();
    let sessions = revoke_all(&mut tx, at, id).await?;
    tx.commit(
        Event::new("user.sign_out_everywhere")
            .target("user", id, &target.email)
            .detail(json!({"sessions": sessions})),
    )
    .await?;
    Ok(StatusCode::NO_CONTENT)
}

/// `DELETE /v1/admin/users/{id}/sessions/{sid}`: one of that user's live sessions.
pub async fn revoke_session(
    admin: AdminAuth,
    State(state): State<AppState>,
    ApiPath((id, sid)): ApiPath<(Uuid, Uuid)>,
) -> Result<StatusCode, ApiError> {
    admin.require(Perm::Users)?;
    let mut tx = AdminTx::begin(&state, &admin).await?;
    let target = lock_target(&mut tx, id).await?;
    let at = tx.at();
    let label: Option<Option<String>> = sqlx::query_scalar(
        "UPDATE sessions SET revoked_at = $3 \
         WHERE id = $1 AND user_id = $2 AND revoked_at IS NULL RETURNING label",
    )
    .bind(sid)
    .bind(id)
    .bind(at)
    .fetch_optional(&mut *tx)
    .await?;
    let label = label.ok_or_else(ApiError::not_found)?;
    tx.commit(
        Event::new("user.session.revoke")
            .target("user", id, &target.email)
            .detail(json!({"session_id": sid, "label": label})),
    )
    .await?;
    Ok(StatusCode::NO_CONTENT)
}

/// `DELETE /v1/admin/users/{id}/identities/{provider}`. The `email` pseudo
/// identity is the account itself: `400`. The next sign-in with that provider
/// links again by email.
pub async fn unlink_identity(
    admin: AdminAuth,
    State(state): State<AppState>,
    ApiPath((id, provider)): ApiPath<(Uuid, String)>,
) -> Result<StatusCode, ApiError> {
    admin.require(Perm::Users)?;
    if provider == "email" {
        return Err(invalid(
            "Email is the account itself and can't be unlinked.",
        ));
    }
    let mut tx = AdminTx::begin(&state, &admin).await?;
    let target = lock_target(&mut tx, id).await?;
    let done = sqlx::query("DELETE FROM identities WHERE user_id = $1 AND provider = $2")
        .bind(id)
        .bind(&provider)
        .execute(&mut *tx)
        .await?;
    if done.rows_affected() == 0 {
        return Err(ApiError::not_found());
    }
    tx.commit(
        Event::new("user.identity.unlink")
            .target("user", id, &target.email)
            .detail(json!({"provider": provider})),
    )
    .await?;
    Ok(StatusCode::NO_CONTENT)
}

/// `DELETE /v1/admin/users/{id}`. The sole owner of a team: `409` naming it.
pub async fn delete(
    admin: AdminAuth,
    State(state): State<AppState>,
    ApiPath(id): ApiPath<Uuid>,
) -> Result<StatusCode, ApiError> {
    admin.require(Perm::Users)?;
    let owners = owner_emails(&state).await?;
    let mut tx = AdminTx::begin(&state, &admin).await?;
    let target = lock_target(&mut tx, id).await?;
    guard(&admin, &owners, &target)?;
    // Lock their teams (serialising member changes), then look for one they
    // alone own.
    let team_ids: Vec<Uuid> = sqlx::query_scalar(
        "SELECT t.id FROM teams t JOIN memberships m ON m.team_id = t.id \
         WHERE m.user_id = $1 ORDER BY t.id FOR UPDATE OF t",
    )
    .bind(id)
    .fetch_all(&mut *tx)
    .await?;
    let sole: Option<String> = sqlx::query_scalar(
        "SELECT t.name FROM memberships m JOIN teams t ON t.id = m.team_id \
         WHERE m.user_id = $1 AND m.role = 'owner' \
           AND NOT EXISTS (SELECT 1 FROM memberships o \
                           WHERE o.team_id = m.team_id AND o.role = 'owner' AND o.user_id <> $1) \
         ORDER BY t.name LIMIT 1",
    )
    .bind(id)
    .fetch_optional(&mut *tx)
    .await?;
    if let Some(team) = sole {
        return Err(conflict(&format!(
            "{} is the only owner of {team}. Make someone else an owner, or delete {team} first.",
            target.short_name()
        )));
    }
    // Opens recorded under their exact email would read as "Opened" for a
    // re-added address. `entry` is nullable, so the open itself (and the count)
    // stays and only the email is dropped; domain entries are not theirs.
    sqlx::query("UPDATE share_opens SET entry = NULL WHERE user_id = $1 AND entry = $2")
        .bind(id)
        .bind(&target.email)
        .execute(&mut *tx)
        .await?;
    // Access entries that are exactly their email; domain entries are not theirs.
    let entries = sqlx::query("DELETE FROM access_entries WHERE kind = 'email' AND entry = $1")
        .bind(&target.email)
        .execute(&mut *tx)
        .await?
        .rows_affected();
    sqlx::query("DELETE FROM email_codes WHERE email = $1")
        .bind(&target.email)
        .execute(&mut *tx)
        .await?;
    // Identities, sessions, memberships and an admin row go with the account;
    // tokens they made stay, with no creator.
    sqlx::query("DELETE FROM users WHERE id = $1")
        .bind(id)
        .execute(&mut *tx)
        .await?;
    for team_id in &team_ids {
        mark_seats_dirty(&mut tx, *team_id).await?;
    }
    tx.commit(
        Event::new("user.delete")
            .target("user", id, &target.email)
            .detail(json!({"teams": team_ids.len(), "access_entries": entries})),
    )
    .await?;
    Ok(StatusCode::NO_CONTENT)
}

#[derive(Deserialize)]
pub struct EmailReq {
    subject: String,
    body_md: String,
}

impl EmailReq {
    /// The subject and body, trimmed and checked: `(subject, body)`.
    pub(super) fn checked(self) -> Result<(String, String), ApiError> {
        let subject = self.subject.trim().to_owned();
        let body = self.body_md.trim().to_owned();
        if subject.is_empty() || subject.chars().count() > MAX_SUBJECT {
            return Err(invalid("The subject must be 1 to 200 characters."));
        }
        if body.is_empty() || body.chars().count() > MAX_BODY {
            return Err(invalid("The message must be 1 to 10000 characters."));
        }
        Ok((subject, body))
    }
}

/// `POST /v1/admin/users/{id}/email`, perm `emails`. The body is plain text
/// with blank-line paragraphs for now.
pub async fn email(
    admin: AdminAuth,
    State(state): State<AppState>,
    ApiPath(id): ApiPath<Uuid>,
    ApiJson(req): ApiJson<EmailReq>,
) -> Result<(StatusCode, Json<Value>), ApiError> {
    admin.require(Perm::Emails)?;
    let (subject, body) = req.checked()?;
    let mut tx = AdminTx::begin(&state, &admin).await?;
    let target = lock_target(&mut tx, id).await?;
    state
        .mailer
        .enqueue_for(
            &mut tx,
            Mail::AdminMessage {
                to: target.email.clone(),
                subject: subject.clone(),
                body,
            },
            Some(id),
        )
        .await?;
    tx.commit(
        Event::new("user.email")
            .target("user", id, &target.email)
            .detail(json!({"subject": subject})),
    )
    .await?;
    Ok((StatusCode::ACCEPTED, Json(json!({}))))
}

#[derive(Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BulkAction {
    SignOut,
    Suspend,
}

#[derive(Deserialize)]
pub struct BulkReq {
    ids: Vec<Uuid>,
    action: BulkAction,
    reason: Option<String>,
}

#[derive(Serialize)]
pub struct BulkBody {
    /// How many were changed.
    done: usize,
    /// Ids left alone: unknown, an owner, the caller, or already suspended.
    skipped: Vec<Uuid>,
}

/// `POST /v1/admin/users/bulk`: one transaction and one `users.bulk` event.
pub async fn bulk(
    admin: AdminAuth,
    State(state): State<AppState>,
    ApiJson(req): ApiJson<BulkReq>,
) -> Result<Json<BulkBody>, ApiError> {
    admin.require(Perm::Users)?;
    let mut ids = req.ids;
    ids.sort();
    ids.dedup();
    if ids.is_empty() || ids.len() > MAX_BULK {
        return Err(invalid("Pick between 1 and 200 users."));
    }
    let reason = clean_reason(req.reason)?;
    let owners = owner_emails(&state).await?;
    let mut tx = AdminTx::begin(&state, &admin).await?;
    // Sorted by id, so two batches cannot lock in opposite orders.
    let rows: Vec<(Uuid, String, Option<String>)> = sqlx::query_as(
        "SELECT id, email::text, name FROM users WHERE id = ANY($1) ORDER BY id FOR UPDATE",
    )
    .bind(&ids)
    .fetch_all(&mut *tx)
    .await?;
    let at = tx.at();
    let mut done = Vec::new();
    for (id, email, name) in rows {
        let target = Target { id, email, name };
        let changed = match req.action {
            // Not the caller: they would end their own session mid-request.
            BulkAction::SignOut if id == admin.user_id => false,
            BulkAction::SignOut => {
                revoke_all(&mut tx, at, id).await?;
                true
            }
            BulkAction::Suspend => {
                guard(&admin, &owners, &target).is_ok()
                    && suspend_row(&mut tx, &admin, at, id, reason.as_deref()).await?
            }
        };
        if changed {
            done.push(id);
        }
    }
    let skipped: Vec<Uuid> = ids.iter().copied().filter(|i| !done.contains(i)).collect();
    let action = match req.action {
        BulkAction::SignOut => "sign_out",
        BulkAction::Suspend => "suspend",
    };
    let mut event = Event::new(match req.action {
        BulkAction::SignOut => "users.bulk.sign_out",
        BulkAction::Suspend => "users.bulk.suspend",
    })
    .detail(json!({
        "action": action,
        "ids": done,
        "skipped": skipped,
        "reason": reason,
    }));
    event.target_kind = Some("users");
    event.target_label = Some(format!("{} users", done.len()));
    tx.commit(event).await?;
    Ok(Json(BulkBody {
        done: done.len(),
        skipped,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn like_patterns_take_wildcards_literally() {
        assert_eq!(like_pattern("ada"), "%ada%");
        assert_eq!(like_pattern("100%_\\"), "%100\\%\\_\\\\%");
    }

    #[test]
    fn filters_parse() {
        for f in ["all", "members", "customers", "suspended", "admins"] {
            assert_eq!(Filter::parse(f).unwrap().as_str(), f);
        }
        assert!(Filter::parse("nope").is_none());
    }

    #[test]
    fn short_name_is_a_first_name_or_the_email() {
        let t = |name: Option<&str>| Target {
            id: Uuid::nil(),
            email: "ada@example.com".into(),
            name: name.map(str::to_owned),
        };
        assert_eq!(t(Some("Ada Lovelace")).short_name(), "Ada");
        assert_eq!(t(None).short_name(), "ada@example.com");
        assert_eq!(t(Some("  ")).short_name(), "ada@example.com");
    }
}
