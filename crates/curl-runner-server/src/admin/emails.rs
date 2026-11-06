//! Broadcasts and the delivery log. Any admin reads; writing, testing and
//! sending a broadcast needs `emails` and logs one event in its own
//! transaction.
//!
//! A broadcast is a draft until it is sent or scheduled. Sending only marks
//! it `sending`; the broadcast worker (`mail::worker::fan_out_due`) expands
//! the audience into one outbox row per recipient, 500 at a time, so no
//! request ever loops over the audience.
//!
//! Audiences, after skipping anyone who turned product emails off or is
//! suspended:
//! - `everyone`: every account;
//! - `team_owners`: owners of any team;
//! - `paying_teams`: owners of a team with a live Stripe subscription;
//! - `free_teams`: owners of a team whose effective plan is Free;
//! - `granted_teams`: owners of a team with an active plan override.
//!
//! A person gets one email. `{{team_name}}` and `{{plan}}` come from the
//! first team they joined among those that put them in the audience (any
//! team for `everyone`).

use axum::Json;
use axum::extract::State;
use axum::http::StatusCode;
use chrono::{DateTime, Duration, Utc};
use curl_runner_cloud_proto::{ErrorCode, Plan};
use serde::{Deserialize, Serialize};
use serde_json::json;
use sqlx::{FromRow, PgConnection, PgPool};
use uuid::Uuid;

use super::activity::{Cursor, invalid, non_empty, page_limit};
use super::auth::{AdminAuth, Perm};
use super::events::{AdminTx, Event};
use super::users::like_pattern;
use crate::billing::effective::{PlanRow, effective_plan, effective_plan_sql, override_active_sql};
use crate::error::ApiError;
use crate::extract::{ApiJson, ApiPath, ApiQuery};
use crate::mail::markdown::Vars;
use crate::mail::worker::MAX_ATTEMPTS;
use crate::mail::{KINDS, Mail, templates};
use crate::state::AppState;

const MAX_SUBJECT: usize = 200;
const MAX_BODY: usize = 20_000;
const MAX_QUERY: usize = 200;
/// Broadcasts listed, newest first.
const LIST_LIMIT: i64 = 100;
/// Test sends per admin per hour.
const TESTS_PER_HOUR: u32 = 10;

/// Who a broadcast goes to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Audience {
    Everyone,
    TeamOwners,
    PayingTeams,
    FreeTeams,
    GrantedTeams,
}

impl Audience {
    pub fn parse(s: &str) -> Option<Self> {
        Some(match s {
            "everyone" => Self::Everyone,
            "team_owners" => Self::TeamOwners,
            "paying_teams" => Self::PayingTeams,
            "free_teams" => Self::FreeTeams,
            "granted_teams" => Self::GrantedTeams,
            _ => return None,
        })
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Everyone => "everyone",
            Self::TeamOwners => "team_owners",
            Self::PayingTeams => "paying_teams",
            Self::FreeTeams => "free_teams",
            Self::GrantedTeams => "granted_teams",
        }
    }

    fn from_column(s: &str) -> Self {
        Self::parse(s).unwrap_or(Self::Everyone)
    }
}

fn bad_audience() -> ApiError {
    invalid("`audience` must be everyone, team_owners, paying_teams, free_teams or granted_teams.")
}

/// One recipient: the user and the team their variables come from.
#[derive(Debug, FromRow)]
pub(crate) struct Recipient {
    pub user_id: Uuid,
    /// Lowercased.
    pub email: String,
    pub name: Option<String>,
    pub product_emails: bool,
    pub team_name: Option<String>,
    pub plan: Option<String>,
    pub stripe_subscription_id: Option<String>,
    pub override_plan: Option<String>,
    pub override_until: Option<DateTime<Utc>>,
}

impl Recipient {
    /// `{{first_name}}`: the name's first word, else "there". `{{team_name}}`
    /// and `{{plan}}`: the chosen team, else "your team" on Free.
    pub fn vars(&self, now: DateTime<Utc>) -> Vars {
        let plan = match &self.plan {
            Some(plan) => {
                let row = PlanRow {
                    plan: plan.clone(),
                    stripe_subscription_id: self.stripe_subscription_id.clone(),
                    override_plan: self.override_plan.clone(),
                    override_until: self.override_until,
                };
                effective_plan(&row, now).0
            }
            None => Plan::Free,
        };
        Vars {
            first_name: self
                .name
                .as_deref()
                .and_then(|n| n.split_whitespace().next())
                .unwrap_or("there")
                .to_owned(),
            team_name: self
                .team_name
                .clone()
                .unwrap_or_else(|| "your team".to_owned()),
            plan: match plan {
                Plan::Free => "Free",
                Plan::Team => "Team",
            }
            .to_owned(),
        }
    }
}

/// One row per recipient of `audience`, ordered by user id. `$1` is now;
/// `extra` adds conditions on `u` with later placeholders. Without
/// `include_opted_out`, people who turned product emails off and suspended
/// accounts are left out.
pub(crate) fn recipients_sql(audience: Audience, include_opted_out: bool, extra: &str) -> String {
    let (joins, cond) = match audience {
        Audience::Everyone => (
            "LEFT JOIN memberships m ON m.user_id = u.id LEFT JOIN teams t ON t.id = m.team_id",
            "true".to_owned(),
        ),
        other => {
            let cond = match other {
                Audience::TeamOwners | Audience::Everyone => "true".to_owned(),
                Audience::PayingTeams => "t.stripe_subscription_id IS NOT NULL \
                     AND t.billing_status IN ('active', 'trialing', 'past_due', 'unpaid')"
                    .to_owned(),
                Audience::FreeTeams => format!("{} = 'free'", effective_plan_sql("t", "$1")),
                Audience::GrantedTeams => override_active_sql("t", "$1"),
            };
            (
                "JOIN memberships m ON m.user_id = u.id AND m.role = 'owner' \
                 JOIN teams t ON t.id = m.team_id",
                cond,
            )
        }
    };
    let reachable = if include_opted_out {
        "true"
    } else {
        "u.product_emails AND u.suspended_at IS NULL"
    };
    format!(
        "SELECT DISTINCT ON (u.id) u.id AS user_id, lower(u.email::text) AS email, u.name, \
                u.product_emails, t.name AS team_name, t.plan, t.stripe_subscription_id, \
                t.override_plan, t.override_until \
         FROM users u {joins} \
         WHERE {reachable} AND ({cond}) AND $1::timestamptz IS NOT NULL {extra} \
         ORDER BY u.id, m.created_at, m.team_id"
    )
}

/// How many people `audience` reaches now.
pub(crate) async fn audience_size(
    db: &PgPool,
    audience: Audience,
    now: DateTime<Utc>,
) -> Result<i64, ApiError> {
    Ok(sqlx::query_scalar(&format!(
        "SELECT count(*) FROM ({}) r",
        recipients_sql(audience, false, "")
    ))
    .bind(now)
    .fetch_one(db)
    .await?)
}

/// `user_id` as a recipient of `audience` (opted out or not), and whether
/// the audience includes them. Someone outside it is shown with their first
/// team.
async fn recipient_for(
    conn: &mut PgConnection,
    user_id: Uuid,
    audience: Audience,
    now: DateTime<Utc>,
) -> Result<(Recipient, bool), ApiError> {
    for (aud, inside) in [(audience, true), (Audience::Everyone, false)] {
        let row: Option<Recipient> = sqlx::query_as(&recipients_sql(aud, true, "AND u.id = $2"))
            .bind(now)
            .bind(user_id)
            .fetch_optional(&mut *conn)
            .await?;
        if let Some(row) = row {
            return Ok((row, inside));
        }
    }
    Err(ApiError::not_found())
}

#[derive(FromRow)]
struct BroadcastSql {
    id: Uuid,
    subject: String,
    body_md: String,
    audience: String,
    status: String,
    scheduled_at: Option<DateTime<Utc>>,
    sent_at: Option<DateTime<Utc>>,
    sent_count: i32,
    created_by: Option<Uuid>,
    created_by_email: Option<String>,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
    delivered: i64,
    failed: i64,
}

#[derive(Serialize)]
pub struct CreatedBy {
    user_id: Uuid,
    email: String,
}

#[derive(Serialize)]
pub struct BroadcastBody {
    id: Uuid,
    subject: String,
    body_md: String,
    audience: String,
    /// `draft`, `scheduled`, `sending` or `sent`.
    status: String,
    scheduled_at: Option<DateTime<Utc>>,
    sent_at: Option<DateTime<Utc>>,
    /// Outbox rows queued for it so far.
    sent_count: i32,
    created_by: Option<CreatedBy>,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
    /// Copies the mail server accepted.
    delivered: i64,
    /// Copies given up on.
    failed: i64,
}

impl From<BroadcastSql> for BroadcastBody {
    fn from(b: BroadcastSql) -> Self {
        Self {
            id: b.id,
            subject: b.subject,
            body_md: b.body_md,
            audience: b.audience,
            status: b.status,
            scheduled_at: b.scheduled_at,
            sent_at: b.sent_at,
            sent_count: b.sent_count,
            created_by: b
                .created_by
                .zip(b.created_by_email)
                .map(|(user_id, email)| CreatedBy { user_id, email }),
            created_at: b.created_at,
            updated_at: b.updated_at,
            delivered: b.delivered,
            failed: b.failed,
        }
    }
}

const SELECT_SQL: &str = "SELECT b.id, b.subject, b.body_md, b.audience, b.status, b.scheduled_at, \
            b.sent_at, b.sent_count, b.created_by, u.email::text AS created_by_email, \
            b.created_at, b.updated_at, \
            (SELECT count(*) FROM outbox_emails o \
             WHERE o.broadcast_id = b.id AND o.sent_at IS NOT NULL) AS delivered, \
            (SELECT count(*) FROM outbox_emails o \
             WHERE o.broadcast_id = b.id AND o.failed_at IS NOT NULL) AS failed \
     FROM broadcasts b LEFT JOIN users u ON u.id = b.created_by";

async fn load(conn: &mut PgConnection, id: Uuid) -> Result<BroadcastBody, ApiError> {
    let row: Option<BroadcastSql> = sqlx::query_as(&format!("{SELECT_SQL} WHERE b.id = $1"))
        .bind(id)
        .fetch_optional(conn)
        .await?;
    row.map(Into::into).ok_or_else(ApiError::not_found)
}

#[derive(FromRow)]
struct Locked {
    subject: String,
    body_md: String,
    audience: String,
    status: String,
}

async fn lock(conn: &mut PgConnection, id: Uuid) -> Result<Locked, ApiError> {
    sqlx::query_as(
        "SELECT subject, body_md, audience, status FROM broadcasts WHERE id = $1 FOR UPDATE",
    )
    .bind(id)
    .fetch_optional(conn)
    .await?
    .ok_or_else(ApiError::not_found)
}

/// `409` once a broadcast is going out: it can no longer change.
fn ensure_editable(status: &str) -> Result<(), ApiError> {
    match status {
        "sending" => Err(ApiError::new(
            ErrorCode::Conflict,
            "This broadcast is already sending.",
        )),
        "sent" => Err(ApiError::new(
            ErrorCode::Conflict,
            "This broadcast was already sent.",
        )),
        _ => Ok(()),
    }
}

/// `400` unless there is something to send.
fn ensure_complete(b: &Locked) -> Result<(), ApiError> {
    if b.subject.trim().is_empty() || b.body_md.trim().is_empty() {
        return Err(invalid("Write a subject and a message first."));
    }
    Ok(())
}

#[derive(Deserialize)]
pub struct DraftReq {
    subject: Option<String>,
    body_md: Option<String>,
    audience: Option<String>,
}

struct Draft {
    subject: Option<String>,
    body_md: Option<String>,
    audience: Option<Audience>,
}

impl DraftReq {
    fn checked(self) -> Result<Draft, ApiError> {
        let subject = self.subject.map(|s| s.trim().to_owned());
        if subject
            .as_ref()
            .is_some_and(|s| s.chars().count() > MAX_SUBJECT)
        {
            return Err(invalid("The subject can be at most 200 characters."));
        }
        let body_md = self.body_md.map(|b| b.trim_end().to_owned());
        if body_md
            .as_ref()
            .is_some_and(|b| b.chars().count() > MAX_BODY)
        {
            return Err(invalid("The message can be at most 20000 characters."));
        }
        let audience = self
            .audience
            .map(|a| Audience::parse(&a).ok_or_else(bad_audience))
            .transpose()?;
        Ok(Draft {
            subject,
            body_md,
            audience,
        })
    }
}

#[derive(Serialize)]
pub struct BroadcastList {
    items: Vec<BroadcastBody>,
}

/// `GET /v1/admin/broadcasts`: the latest 100, newest first.
pub async fn list(
    _admin: AdminAuth,
    State(state): State<AppState>,
) -> Result<Json<BroadcastList>, ApiError> {
    let rows: Vec<BroadcastSql> = sqlx::query_as(&format!(
        "{SELECT_SQL} ORDER BY b.created_at DESC, b.id DESC LIMIT $1"
    ))
    .bind(LIST_LIMIT)
    .fetch_all(&state.db)
    .await?;
    Ok(Json(BroadcastList {
        items: rows.into_iter().map(Into::into).collect(),
    }))
}

/// `GET /v1/admin/broadcasts/{id}`.
pub async fn detail(
    _admin: AdminAuth,
    State(state): State<AppState>,
    ApiPath(id): ApiPath<Uuid>,
) -> Result<Json<BroadcastBody>, ApiError> {
    let mut conn = state.db.acquire().await?;
    Ok(Json(load(&mut conn, id).await?))
}

/// `POST /v1/admin/broadcasts {subject?, body_md?, audience?}`, perm
/// `emails`: a new draft. Every field may start empty.
pub async fn create(
    admin: AdminAuth,
    State(state): State<AppState>,
    ApiJson(req): ApiJson<DraftReq>,
) -> Result<(StatusCode, Json<BroadcastBody>), ApiError> {
    admin.require(Perm::Emails)?;
    let d = req.checked()?;
    let mut tx = AdminTx::begin(&state, &admin).await?;
    let id: Uuid = sqlx::query_scalar(
        "INSERT INTO broadcasts (subject, body_md, audience, created_by, created_at, updated_at) \
         VALUES ($1, $2, $3, $4, $5, $5) RETURNING id",
    )
    .bind(d.subject.unwrap_or_default())
    .bind(d.body_md.unwrap_or_default())
    .bind(d.audience.unwrap_or(Audience::Everyone).as_str())
    .bind(admin.user_id)
    .bind(tx.at())
    .fetch_one(&mut *tx)
    .await?;
    let body = load(&mut tx, id).await?;
    tx.commit(
        Event::new("broadcast.create")
            .target("broadcast", id, body.subject.clone())
            .detail(json!({"audience": body.audience})),
    )
    .await?;
    Ok((StatusCode::CREATED, Json(body)))
}

/// `PATCH /v1/admin/broadcasts/{id}`, perm `emails`. A draft or a scheduled
/// broadcast; `409` once it is sending or sent. A scheduled one cannot be
/// left without a subject or message (`400`).
pub async fn update(
    admin: AdminAuth,
    State(state): State<AppState>,
    ApiPath(id): ApiPath<Uuid>,
    ApiJson(req): ApiJson<DraftReq>,
) -> Result<Json<BroadcastBody>, ApiError> {
    admin.require(Perm::Emails)?;
    let d = req.checked()?;
    let mut tx = AdminTx::begin(&state, &admin).await?;
    let current = lock(&mut tx, id).await?;
    ensure_editable(&current.status)?;
    sqlx::query(
        "UPDATE broadcasts SET subject = COALESCE($2, subject), body_md = COALESCE($3, body_md), \
                audience = COALESCE($4, audience), updated_at = $5 WHERE id = $1",
    )
    .bind(id)
    .bind(&d.subject)
    .bind(&d.body_md)
    .bind(d.audience.map(Audience::as_str))
    .bind(tx.at())
    .execute(&mut *tx)
    .await?;
    if current.status == "scheduled" {
        // A scheduled broadcast must stay sendable.
        ensure_complete(&lock(&mut tx, id).await?)?;
    }
    let body = load(&mut tx, id).await?;
    let mut changed = Vec::new();
    if d.subject.is_some() {
        changed.push("subject");
    }
    if d.body_md.is_some() {
        changed.push("body_md");
    }
    if d.audience.is_some() {
        changed.push("audience");
    }
    tx.commit(
        Event::new("broadcast.update")
            .target("broadcast", id, body.subject.clone())
            .detail(json!({"changed": changed, "audience": body.audience})),
    )
    .await?;
    Ok(Json(body))
}

/// Recipient `r`'s copy of the broadcast.
pub(crate) fn copy_for(
    r: &Recipient,
    subject: &str,
    body_md: &str,
    unsubscribe: String,
    now: DateTime<Utc>,
    test: bool,
) -> Mail {
    Mail::Broadcast {
        to: r.email.clone(),
        subject: subject.to_owned(),
        body_md: body_md.to_owned(),
        vars: r.vars(now),
        unsubscribe,
        test,
    }
}

/// `POST /v1/admin/broadcasts/{id}/test`, perm `emails`: one copy to the
/// caller, with their own variables, marked `[Test]`. It is not one of the
/// broadcast's rows and ignores the opt-out.
pub async fn test(
    admin: AdminAuth,
    State(state): State<AppState>,
    ApiPath(id): ApiPath<Uuid>,
) -> Result<(StatusCode, Json<serde_json::Value>), ApiError> {
    admin.require(Perm::Emails)?;
    let mut tx = AdminTx::begin(&state, &admin).await?;
    let b = lock(&mut tx, id).await?;
    ensure_complete(&b)?;
    state.limiter.check(
        &format!("admin:broadcast-test:{}", admin.user_id),
        TESTS_PER_HOUR,
        Duration::hours(1),
    )?;
    let now = tx.at();
    let (me, _) = recipient_for(
        &mut tx,
        admin.user_id,
        Audience::from_column(&b.audience),
        now,
    )
    .await?;
    let link = crate::mail::unsubscribe::link(&state.config, admin.user_id);
    let mail = copy_for(&me, &b.subject, &b.body_md, link, now, true);
    state
        .mailer
        .enqueue_for(&mut tx, mail, Some(admin.user_id))
        .await?;
    tx.commit(
        Event::new("broadcast.test")
            .target("broadcast", id, b.subject.clone())
            .detail(json!({"to": me.email})),
    )
    .await?;
    Ok((StatusCode::ACCEPTED, Json(json!({"to": me.email}))))
}

#[derive(Deserialize)]
pub struct SendReq {
    /// When to send; absent for now.
    at: Option<DateTime<Utc>>,
}

/// `POST /v1/admin/broadcasts/{id}/send {at?}`, perm `emails`. With a
/// future `at` it is scheduled (and may be rescheduled or edited until
/// then); without one it is marked `sending` and the broadcast worker queues
/// it within seconds. `409` once it is sending or sent.
pub async fn send(
    admin: AdminAuth,
    State(state): State<AppState>,
    ApiPath(id): ApiPath<Uuid>,
    ApiJson(req): ApiJson<SendReq>,
) -> Result<(StatusCode, Json<BroadcastBody>), ApiError> {
    admin.require(Perm::Emails)?;
    let mut tx = AdminTx::begin(&state, &admin).await?;
    let b = lock(&mut tx, id).await?;
    ensure_editable(&b.status)?;
    ensure_complete(&b)?;
    let now = tx.at();
    let audience = Audience::from_column(&b.audience);
    let event = match req.at {
        Some(at) if at <= now => return Err(invalid("`at` must be in the future.")),
        Some(at) => {
            sqlx::query(
                "UPDATE broadcasts SET status = 'scheduled', scheduled_at = $2, updated_at = $3 \
                 WHERE id = $1",
            )
            .bind(id)
            .bind(at)
            .bind(now)
            .execute(&mut *tx)
            .await?;
            Event::new("broadcast.schedule").detail(json!({"at": at, "audience": b.audience}))
        }
        None => {
            sqlx::query(
                "UPDATE broadcasts SET status = 'sending', scheduled_at = $2, updated_at = $2 \
                 WHERE id = $1",
            )
            .bind(id)
            .bind(now)
            .execute(&mut *tx)
            .await?;
            let recipients = audience_size(&state.db, audience, now).await?;
            Event::new("broadcast.send")
                .detail(json!({"audience": b.audience, "recipients": recipients}))
        }
    };
    let body = load(&mut tx, id).await?;
    tx.commit(event.target("broadcast", id, b.subject.clone()))
        .await?;
    Ok((StatusCode::ACCEPTED, Json(body)))
}

#[derive(Deserialize)]
pub struct PreviewQuery {
    /// A user id or email; the caller when absent.
    as_user: Option<String>,
}

#[derive(Serialize)]
pub struct PreviewUser {
    id: Uuid,
    email: String,
    name: Option<String>,
}

#[derive(Serialize)]
pub struct PreviewBody {
    subject: String,
    html: String,
    text: String,
    as_user: PreviewUser,
    /// Whether the broadcast's audience includes them.
    in_audience: bool,
    /// They turned product emails off, so they would be skipped.
    opted_out: bool,
}

/// `GET /v1/admin/broadcasts/{id}/preview?as_user=`: the subject, HTML and
/// text one person would get. The unsubscribe link in it does not work.
pub async fn preview(
    admin: AdminAuth,
    State(state): State<AppState>,
    ApiPath(id): ApiPath<Uuid>,
    ApiQuery(q): ApiQuery<PreviewQuery>,
) -> Result<Json<PreviewBody>, ApiError> {
    let mut conn = state.db.acquire().await?;
    let b: Option<(String, String, String)> =
        sqlx::query_as("SELECT subject, body_md, audience FROM broadcasts WHERE id = $1")
            .bind(id)
            .fetch_optional(&mut *conn)
            .await?;
    let (subject, body_md, audience) = b.ok_or_else(ApiError::not_found)?;
    let user_id = match non_empty(q.as_user.as_ref()) {
        None => admin.user_id,
        Some(who) => {
            let found: Option<Uuid> = match who.parse::<Uuid>() {
                Ok(uid) => sqlx::query_scalar("SELECT id FROM users WHERE id = $1").bind(uid),
                Err(_) => sqlx::query_scalar("SELECT id FROM users WHERE email = $1").bind(who),
            }
            .fetch_optional(&mut *conn)
            .await?;
            found.ok_or_else(|| {
                ApiError::new(ErrorCode::NotFound, "No user with that id or email.")
            })?
        }
    };
    let now = state.clock.now();
    let (r, in_audience) =
        recipient_for(&mut conn, user_id, Audience::from_column(&audience), now).await?;
    let link = format!("{}/v1/unsubscribe?token=preview", state.config.api_url);
    let mail = copy_for(&r, &subject, &body_md, link, now, false);
    let (subject, text, html) = templates::render(&mail);
    Ok(Json(PreviewBody {
        subject,
        html,
        text,
        as_user: PreviewUser {
            id: r.user_id,
            email: r.email,
            name: r.name,
        },
        in_audience,
        opted_out: !r.product_emails,
    }))
}

#[derive(Deserialize)]
pub struct AudienceQuery {
    audience: Option<String>,
}

/// `GET /v1/admin/audience-count?audience=`: people it would reach now,
/// after opt-outs.
pub async fn audience_count(
    _admin: AdminAuth,
    State(state): State<AppState>,
    ApiQuery(q): ApiQuery<AudienceQuery>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let audience = non_empty(q.audience.as_ref())
        .and_then(Audience::parse)
        .ok_or_else(bad_audience)?;
    let count = audience_size(&state.db, audience, state.clock.now()).await?;
    Ok(Json(json!({"audience": audience.as_str(), "count": count})))
}

#[derive(Deserialize)]
pub struct LogQuery {
    kind: Option<String>,
    status: Option<String>,
    q: Option<String>,
    cursor: Option<String>,
    limit: Option<String>,
}

#[derive(FromRow)]
struct LogSql {
    id: Uuid,
    to_addr: String,
    subject: String,
    kind: String,
    status: String,
    broadcast_id: Option<Uuid>,
    created_at: Option<DateTime<Utc>>,
    sent_at: Option<DateTime<Utc>>,
    failed_at: Option<DateTime<Utc>>,
    next_attempt_at: DateTime<Utc>,
    attempts: i32,
    last_error: Option<String>,
    sort_at: DateTime<Utc>,
}

#[derive(Serialize)]
pub struct LogRow {
    id: Uuid,
    to: String,
    /// A sign-in code's subject is replaced: it holds the code.
    subject: String,
    kind: String,
    /// `queued`, `sent` or `failed`.
    status: String,
    broadcast_id: Option<Uuid>,
    /// `null` on rows queued before 0004.
    created_at: Option<DateTime<Utc>>,
    sent_at: Option<DateTime<Utc>>,
    failed_at: Option<DateTime<Utc>>,
    /// The next try, for a queued row.
    next_attempt_at: Option<DateTime<Utc>>,
    attempts: i32,
    /// The mail server's answer to the last failed try.
    last_error: Option<String>,
}

#[derive(Serialize)]
pub struct LogPage {
    items: Vec<LogRow>,
    next_cursor: Option<String>,
}

const SIGN_IN_SUBJECT: &str = "Your curl-runner sign-in code";

/// `GET /v1/admin/emails/log?kind=&status=queued|sent|failed&q=&cursor=&limit=`:
/// `outbox_emails`, newest first; rows from before 0004 (no `created_at`)
/// come last. `q` matches the recipient only.
pub async fn log(
    _admin: AdminAuth,
    State(state): State<AppState>,
    ApiQuery(q): ApiQuery<LogQuery>,
) -> Result<Json<LogPage>, ApiError> {
    let kind = match non_empty(q.kind.as_ref()) {
        None => None,
        Some(k) if KINDS.contains(&k) => Some(k.to_owned()),
        Some(_) => return Err(invalid("`kind` is not a known email kind.")),
    };
    let status = match non_empty(q.status.as_ref()) {
        None => None,
        Some(s @ ("queued" | "sent" | "failed")) => Some(s.to_owned()),
        Some(_) => return Err(invalid("`status` must be queued, sent or failed.")),
    };
    let pattern = match non_empty(q.q.as_ref()) {
        Some(q) if q.chars().count() > MAX_QUERY => return Err(invalid("`q` is too long.")),
        Some(q) => Some(like_pattern(q)),
        None => None,
    };
    let cursor = non_empty(q.cursor.as_ref())
        .map(|c| Cursor::decode(c).ok_or_else(|| invalid("`cursor` is not valid.")))
        .transpose()?;
    let limit = page_limit(q.limit.as_ref())?;
    let status_sql = "(CASE WHEN o.sent_at IS NOT NULL THEN 'sent' \
                       WHEN o.failed_at IS NOT NULL OR o.attempts >= $1 THEN 'failed' \
                       ELSE 'queued' END)";
    let sort_sql = "COALESCE(o.created_at, 'epoch'::timestamptz)";
    let rows: Vec<LogSql> = sqlx::query_as(&format!(
        "SELECT o.id, o.to_addr, \
                CASE WHEN o.kind = 'sign_in_code' OR o.subject LIKE '{SIGN_IN_SUBJECT}%' \
                     THEN '{SIGN_IN_SUBJECT}' ELSE o.subject END AS subject, \
                o.kind, {status_sql} AS status, o.broadcast_id, o.created_at, o.sent_at, \
                o.failed_at, o.next_attempt_at, o.attempts, o.last_error, {sort_sql} AS sort_at \
         FROM outbox_emails o \
         WHERE ($2::text IS NULL OR o.kind = $2) \
           AND ($3::text IS NULL OR {status_sql} = $3) \
           AND ($4::text IS NULL OR o.to_addr ILIKE $4) \
           AND ($5::timestamptz IS NULL OR ({sort_sql}, o.id) < ($5, $6::uuid)) \
         ORDER BY {sort_sql} DESC, o.id DESC LIMIT $7"
    ))
    .bind(MAX_ATTEMPTS)
    .bind(&kind)
    .bind(&status)
    .bind(&pattern)
    .bind(cursor.map(|c| c.at))
    .bind(cursor.map(|c| c.id))
    .bind(limit + 1)
    .fetch_all(&state.db)
    .await?;
    let more = rows.len() as i64 > limit;
    let mut items = Vec::with_capacity(rows.len());
    let mut last = None;
    for r in rows.into_iter().take(usize::try_from(limit).unwrap_or(0)) {
        last = Some(Cursor {
            at: r.sort_at,
            id: r.id,
        });
        let queued = r.status == "queued";
        items.push(LogRow {
            id: r.id,
            to: r.to_addr,
            subject: r.subject,
            kind: r.kind,
            status: r.status,
            broadcast_id: r.broadcast_id,
            created_at: r.created_at,
            sent_at: r.sent_at,
            failed_at: r.failed_at,
            next_attempt_at: queued.then_some(r.next_attempt_at),
            attempts: r.attempts,
            last_error: r.last_error,
        });
    }
    Ok(Json(LogPage {
        items,
        next_cursor: if more { last.map(Cursor::encode) } else { None },
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn audiences_round_trip() {
        for a in [
            Audience::Everyone,
            Audience::TeamOwners,
            Audience::PayingTeams,
            Audience::FreeTeams,
            Audience::GrantedTeams,
        ] {
            assert_eq!(Audience::parse(a.as_str()), Some(a));
        }
        assert_eq!(Audience::parse("all"), None);
    }

    fn recipient(name: Option<&str>, team: Option<(&str, &str)>) -> Recipient {
        Recipient {
            user_id: Uuid::nil(),
            email: "a@example.com".into(),
            name: name.map(str::to_owned),
            product_emails: true,
            team_name: team.map(|t| t.0.to_owned()),
            plan: team.map(|t| t.1.to_owned()),
            stripe_subscription_id: None,
            override_plan: None,
            override_until: None,
        }
    }

    #[test]
    fn vars_fall_back() {
        let now = Utc::now();
        let v = recipient(Some("  Ada  Lovelace "), Some(("Acme", "team"))).vars(now);
        assert_eq!(
            (v.first_name.as_str(), v.team_name.as_str(), v.plan.as_str()),
            ("Ada", "Acme", "Team")
        );
        let v = recipient(Some("   "), None).vars(now);
        assert_eq!(
            (v.first_name.as_str(), v.team_name.as_str(), v.plan.as_str()),
            ("there", "your team", "Free")
        );
        let mut r = recipient(None, Some(("Acme", "free")));
        r.override_plan = Some("team".into());
        assert_eq!(r.vars(now).plan, "Team");
        r.override_until = Some(now - Duration::seconds(1));
        assert_eq!(r.vars(now).plan, "Free");
    }
}
