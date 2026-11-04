//! Teams, members, invites and the operator endpoints.

use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::routing::{delete, get, post, put};
use axum::{Json, Router};
use chrono::{DateTime, Duration, Utc};
use curl_runner_cloud_proto::{ErrorCode, Plan, Role, SignupMode};
use serde::{Deserialize, Serialize};
use sqlx::{PgConnection, PgPool};
use uuid::Uuid;

use crate::auth::UserAuth;
use crate::auth::session::column_enum;
use crate::config::Config;
use crate::error::ApiError;
use crate::extract::ApiJson;
use crate::hash::{new_token, secret_hash};
use crate::mail::Mail;
use crate::plans::limits;
use crate::state::AppState;

/// Member invites last this long.
pub const INVITE_TTL: Duration = Duration::days(7);

const FIRST_OPERATOR: &str = "first_operator";

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/v1/teams", post(create_team))
        .route("/v1/teams/{team}", get(get_team))
        .route("/v1/teams/{team}/invites", post(invite))
        .route("/v1/invites/{token}/accept", post(accept_invite))
        .route(
            "/v1/teams/{team}/members/{user_id}",
            delete(remove_member).patch(set_role),
        )
        .route("/v1/admin/teams/{team}/plan", put(set_plan))
}

/// A slug: `^[a-z0-9][a-z0-9-]{0,62}$`. Shared by teams and collections.
pub fn valid_slug(slug: &str) -> bool {
    let b = slug.as_bytes();
    !b.is_empty()
        && b.len() <= 63
        && (b[0].is_ascii_lowercase() || b[0].is_ascii_digit())
        && b.iter()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || *c == b'-')
}

/// A caller's standing in one team.
#[derive(Debug, Clone, Copy)]
pub struct TeamAccess {
    pub team_id: Uuid,
    pub role: Role,
}

impl TeamAccess {
    /// `403 forbidden` unless the caller is an Owner.
    pub fn require_owner(&self) -> Result<(), ApiError> {
        if self.role == Role::Owner {
            Ok(())
        } else {
            Err(ApiError::new(
                ErrorCode::Forbidden,
                "Only team owners can do this.",
            ))
        }
    }
}

/// The caller's access to the team `slug`. A non-member (or a missing team) is `404`.
pub async fn team_access(db: &PgPool, user_id: Uuid, slug: &str) -> Result<TeamAccess, ApiError> {
    let row: Option<(Uuid, String)> = sqlx::query_as(
        "SELECT t.id, m.role FROM teams t JOIN memberships m ON m.team_id = t.id \
         WHERE t.slug = $1 AND m.user_id = $2",
    )
    .bind(slug)
    .bind(user_id)
    .fetch_optional(db)
    .await?;
    let (team_id, role) = row.ok_or_else(ApiError::not_found)?;
    Ok(TeamAccess {
        team_id,
        role: column_enum(role)?,
    })
}

async fn first_operator(db: &PgPool) -> Result<Option<String>, ApiError> {
    Ok(
        sqlx::query_scalar("SELECT value FROM settings WHERE key = $1")
            .bind(FIRST_OPERATOR)
            .fetch_optional(db)
            .await?,
    )
}

/// An email in `OPERATOR_EMAILS`, or the recorded first operator.
pub async fn is_operator(db: &PgPool, config: &Config, email: &str) -> Result<bool, ApiError> {
    let email = email.to_lowercase();
    if config.operators.contains(&email) {
        return Ok(true);
    }
    Ok(first_operator(db)
        .await?
        .is_some_and(|op| op.to_lowercase() == email))
}

/// Whether `email` may create a team under the signup mode.
///
/// With `domains` and no domains listed, the earliest account on the server
/// is recorded as the first operator and may create teams.
pub async fn can_create_team(db: &PgPool, config: &Config, email: &str) -> Result<bool, ApiError> {
    if is_operator(db, config, email).await? {
        return Ok(true);
    }
    let email = email.to_lowercase();
    match config.signup {
        SignupMode::Open => Ok(true),
        SignupMode::Closed => Ok(false),
        SignupMode::Domains => {
            if let Some((_, domain)) = email.rsplit_once('@')
                && config.signup_domains.iter().any(|d| d == domain)
            {
                return Ok(true);
            }
            if !config.signup_domains.is_empty() {
                return Ok(false);
            }
            let earliest: Option<String> =
                sqlx::query_scalar("SELECT email::text FROM users ORDER BY created_at, id LIMIT 1")
                    .fetch_optional(db)
                    .await?;
            if earliest.is_none_or(|e| e.to_lowercase() != email) {
                return Ok(false);
            }
            sqlx::query(
                "INSERT INTO settings (key, value) VALUES ($1, $2) ON CONFLICT (key) DO NOTHING",
            )
            .bind(FIRST_OPERATOR)
            .bind(&email)
            .execute(db)
            .await?;
            is_operator(db, config, &email).await
        }
    }
}

fn invalid(message: &str) -> ApiError {
    ApiError::new(ErrorCode::InvalidRequest, message)
}

#[derive(Deserialize)]
pub struct CreateTeam {
    slug: String,
    name: String,
}

#[derive(Serialize)]
pub struct CreatedTeam {
    slug: String,
    name: String,
    plan: Plan,
}

async fn create_team(
    State(state): State<AppState>,
    auth: UserAuth,
    ApiJson(req): ApiJson<CreateTeam>,
) -> Result<(StatusCode, Json<CreatedTeam>), ApiError> {
    let name = req.name.trim().to_owned();
    if !valid_slug(&req.slug) {
        return Err(invalid(
            "The slug must be lowercase letters, digits and dashes, up to 63 characters.",
        ));
    }
    if name.is_empty() {
        return Err(invalid("The team needs a name."));
    }
    if !can_create_team(&state.db, &state.config, &auth.email).await? {
        return Err(ApiError::new(
            ErrorCode::SignupClosed,
            "Creating teams is closed on this server.",
        ));
    }
    let plan = if state.config.signup == SignupMode::Domains {
        Plan::Team
    } else {
        Plan::Free
    };
    let now = state.clock.now();
    let mut tx = state.db.begin().await?;
    let team_id: Option<Uuid> = sqlx::query_scalar(
        "INSERT INTO teams (slug, name, plan, created_at) VALUES ($1, $2, $3, $4) \
         ON CONFLICT (slug) DO NOTHING RETURNING id",
    )
    .bind(&req.slug)
    .bind(&name)
    .bind(plan_str(plan))
    .bind(now)
    .fetch_optional(&mut *tx)
    .await?;
    let team_id = team_id
        .ok_or_else(|| ApiError::new(ErrorCode::Conflict, "That team slug is already taken."))?;
    sqlx::query(
        "INSERT INTO memberships (team_id, user_id, role, created_at) VALUES ($1, $2, 'owner', $3)",
    )
    .bind(team_id)
    .bind(auth.user_id)
    .bind(now)
    .execute(&mut *tx)
    .await?;
    tx.commit().await?;
    Ok((
        StatusCode::CREATED,
        Json(CreatedTeam {
            slug: req.slug,
            name,
            plan,
        }),
    ))
}

fn plan_str(plan: Plan) -> &'static str {
    match plan {
        Plan::Free => "free",
        Plan::Team => "team",
    }
}

#[derive(Serialize)]
struct MemberBody {
    user_id: Uuid,
    email: String,
    name: Option<String>,
    role: Role,
}

#[derive(Serialize)]
struct PendingInviteBody {
    id: Uuid,
    email: String,
    role: Role,
    expires_at: DateTime<Utc>,
}

#[derive(Serialize)]
pub struct TeamDetail {
    slug: String,
    name: String,
    plan: Plan,
    role: Role,
    members: Vec<MemberBody>,
    pending_invites: Vec<PendingInviteBody>,
}

async fn get_team(
    State(state): State<AppState>,
    auth: UserAuth,
    Path(slug): Path<String>,
) -> Result<Json<TeamDetail>, ApiError> {
    let access = team_access(&state.db, auth.user_id, &slug).await?;
    let (slug, name, plan): (String, String, String) =
        sqlx::query_as("SELECT slug::text, name, plan FROM teams WHERE id = $1")
            .bind(access.team_id)
            .fetch_one(&state.db)
            .await?;
    let members: Vec<(Uuid, String, Option<String>, String)> = sqlx::query_as(
        "SELECT u.id, u.email::text, u.name, m.role FROM memberships m \
         JOIN users u ON u.id = m.user_id WHERE m.team_id = $1 \
         ORDER BY m.created_at, u.email",
    )
    .bind(access.team_id)
    .fetch_all(&state.db)
    .await?;
    let invites: Vec<(Uuid, String, String, DateTime<Utc>)> = sqlx::query_as(
        "SELECT id, email::text, role, expires_at FROM member_invites \
         WHERE team_id = $1 AND accepted_at IS NULL AND expires_at > $2 \
         ORDER BY expires_at, email",
    )
    .bind(access.team_id)
    .bind(state.clock.now())
    .fetch_all(&state.db)
    .await?;
    Ok(Json(TeamDetail {
        slug,
        name,
        plan: column_enum(plan)?,
        role: access.role,
        members: members
            .into_iter()
            .map(|(user_id, email, name, role)| {
                Ok(MemberBody {
                    user_id,
                    email,
                    name,
                    role: column_enum(role)?,
                })
            })
            .collect::<Result<_, ApiError>>()?,
        pending_invites: invites
            .into_iter()
            .map(|(id, email, role, expires_at)| {
                Ok(PendingInviteBody {
                    id,
                    email,
                    role: column_enum(role)?,
                    expires_at,
                })
            })
            .collect::<Result<_, ApiError>>()?,
    }))
}

fn plan_limit() -> ApiError {
    ApiError::new(
        ErrorCode::PlanLimit,
        "The Free plan allows 3 members. Upgrade to add more.",
    )
}

/// Lock the team row (serialising member changes) and return its plan.
async fn lock_team(conn: &mut PgConnection, team_id: Uuid) -> Result<Plan, ApiError> {
    let plan: String = sqlx::query_scalar("SELECT plan FROM teams WHERE id = $1 FOR UPDATE")
        .bind(team_id)
        .fetch_one(conn)
        .await?;
    column_enum(plan)
}

async fn member_count(conn: &mut PgConnection, team_id: Uuid) -> Result<i64, ApiError> {
    Ok(
        sqlx::query_scalar("SELECT count(*) FROM memberships WHERE team_id = $1")
            .bind(team_id)
            .fetch_one(conn)
            .await?,
    )
}

#[derive(Deserialize)]
pub struct InviteReq {
    email: String,
    role: Role,
}

#[derive(Serialize)]
pub struct InviteCreated {
    id: Uuid,
    email: String,
    role: Role,
    expires_at: DateTime<Utc>,
}

async fn invite(
    State(state): State<AppState>,
    auth: UserAuth,
    Path(slug): Path<String>,
    ApiJson(req): ApiJson<InviteReq>,
) -> Result<(StatusCode, Json<InviteCreated>), ApiError> {
    let access = team_access(&state.db, auth.user_id, &slug).await?;
    access.require_owner()?;
    let email = req.email.trim().to_lowercase();
    if !email.contains('@') || email.len() > 254 {
        return Err(invalid("Enter a valid email address."));
    }
    let now = state.clock.now();
    let expires_at = now + INVITE_TTL;
    let token = new_token("cri_");
    let mut tx = state.db.begin().await?;
    let plan = lock_team(&mut tx, access.team_id).await?;
    if let Some(max) = limits(plan).members {
        let pending: i64 = sqlx::query_scalar(
            "SELECT count(*) FROM member_invites \
             WHERE team_id = $1 AND accepted_at IS NULL AND expires_at > $2",
        )
        .bind(access.team_id)
        .bind(now)
        .fetch_one(&mut *tx)
        .await?;
        if member_count(&mut tx, access.team_id).await? + pending >= i64::from(max) {
            return Err(plan_limit());
        }
    }
    let id: Uuid = sqlx::query_scalar(
        "INSERT INTO member_invites (team_id, email, role, token_hash, invited_by, expires_at) \
         VALUES ($1, $2, $3, $4, $5, $6) RETURNING id",
    )
    .bind(access.team_id)
    .bind(&email)
    .bind(role_str(req.role))
    .bind(secret_hash(state.config.secret_key.as_bytes(), &token))
    .bind(auth.user_id)
    .bind(expires_at)
    .fetch_one(&mut *tx)
    .await?;
    let (team_name, inviter): (String, String) = sqlx::query_as(
        "SELECT t.name, COALESCE(u.name, u.email::text) FROM teams t, users u \
         WHERE t.id = $1 AND u.id = $2",
    )
    .bind(access.team_id)
    .bind(auth.user_id)
    .fetch_one(&mut *tx)
    .await?;
    state
        .mailer
        .enqueue(
            &mut tx,
            Mail::MemberInvite {
                to: email.clone(),
                team_name,
                inviter,
                link: format!("{}/invites/{token}", state.config.app_url),
            },
        )
        .await?;
    tx.commit().await?;
    Ok((
        StatusCode::CREATED,
        Json(InviteCreated {
            id,
            email,
            role: req.role,
            expires_at,
        }),
    ))
}

fn role_str(role: Role) -> &'static str {
    match role {
        Role::Owner => "owner",
        Role::Member => "member",
    }
}

#[derive(Serialize)]
pub struct Joined {
    slug: String,
    name: String,
    role: Role,
}

async fn accept_invite(
    State(state): State<AppState>,
    auth: UserAuth,
    Path(token): Path<String>,
) -> Result<Json<Joined>, ApiError> {
    let hash = secret_hash(state.config.secret_key.as_bytes(), &token);
    let now = state.clock.now();
    let row: Option<(Uuid, Uuid, String)> = sqlx::query_as(
        "SELECT id, team_id, role FROM member_invites \
         WHERE token_hash = $1 AND accepted_at IS NULL AND expires_at > $2",
    )
    .bind(&hash)
    .bind(now)
    .fetch_optional(&state.db)
    .await?;
    let (invite_id, team_id, role) = row.ok_or_else(ApiError::not_found)?;
    let mut tx = state.db.begin().await?;
    let plan = lock_team(&mut tx, team_id).await?;
    // Re-read under the lock: a concurrent accept may have used it.
    let still_open: Option<Uuid> = sqlx::query_scalar(
        "SELECT id FROM member_invites WHERE id = $1 AND accepted_at IS NULL AND expires_at > $2",
    )
    .bind(invite_id)
    .bind(now)
    .fetch_optional(&mut *tx)
    .await?;
    if still_open.is_none() {
        return Err(ApiError::not_found());
    }
    let already: Option<Uuid> =
        sqlx::query_scalar("SELECT user_id FROM memberships WHERE team_id = $1 AND user_id = $2")
            .bind(team_id)
            .bind(auth.user_id)
            .fetch_optional(&mut *tx)
            .await?;
    if already.is_some() {
        return Err(ApiError::new(
            ErrorCode::Conflict,
            "You are already a member of this team.",
        ));
    }
    if let Some(max) = limits(plan).members
        && member_count(&mut tx, team_id).await? >= i64::from(max)
    {
        return Err(plan_limit());
    }
    sqlx::query(
        "INSERT INTO memberships (team_id, user_id, role, created_at) VALUES ($1, $2, $3, $4)",
    )
    .bind(team_id)
    .bind(auth.user_id)
    .bind(&role)
    .bind(now)
    .execute(&mut *tx)
    .await?;
    sqlx::query("UPDATE member_invites SET accepted_at = $2 WHERE id = $1")
        .bind(invite_id)
        .bind(now)
        .execute(&mut *tx)
        .await?;
    let (slug, name): (String, String) =
        sqlx::query_as("SELECT slug::text, name FROM teams WHERE id = $1")
            .bind(team_id)
            .fetch_one(&mut *tx)
            .await?;
    tx.commit().await?;
    Ok(Json(Joined {
        slug,
        name,
        role: column_enum(role)?,
    }))
}

fn last_owner() -> ApiError {
    ApiError::new(
        ErrorCode::Conflict,
        "A team needs at least one owner. Make someone else an owner first.",
    )
}

/// Under the team lock: whether `user_id` is the team's only owner.
async fn is_last_owner(
    conn: &mut PgConnection,
    team_id: Uuid,
    user_id: Uuid,
) -> Result<bool, ApiError> {
    let owners: Vec<Uuid> =
        sqlx::query_scalar("SELECT user_id FROM memberships WHERE team_id = $1 AND role = 'owner'")
            .bind(team_id)
            .fetch_all(conn)
            .await?;
    Ok(owners == [user_id])
}

async fn remove_member(
    State(state): State<AppState>,
    auth: UserAuth,
    Path((slug, user_id)): Path<(String, Uuid)>,
) -> Result<StatusCode, ApiError> {
    let access = team_access(&state.db, auth.user_id, &slug).await?;
    if user_id != auth.user_id {
        access.require_owner()?;
    }
    let mut tx = state.db.begin().await?;
    lock_team(&mut tx, access.team_id).await?;
    if is_last_owner(&mut tx, access.team_id, user_id).await? {
        return Err(last_owner());
    }
    let done = sqlx::query("DELETE FROM memberships WHERE team_id = $1 AND user_id = $2")
        .bind(access.team_id)
        .bind(user_id)
        .execute(&mut *tx)
        .await?;
    if done.rows_affected() == 0 {
        return Err(ApiError::not_found());
    }
    tx.commit().await?;
    Ok(StatusCode::NO_CONTENT)
}

#[derive(Deserialize)]
pub struct SetRole {
    role: Role,
}

#[derive(Serialize)]
pub struct MemberRole {
    user_id: Uuid,
    role: Role,
}

async fn set_role(
    State(state): State<AppState>,
    auth: UserAuth,
    Path((slug, user_id)): Path<(String, Uuid)>,
    ApiJson(req): ApiJson<SetRole>,
) -> Result<Json<MemberRole>, ApiError> {
    let access = team_access(&state.db, auth.user_id, &slug).await?;
    access.require_owner()?;
    let mut tx = state.db.begin().await?;
    lock_team(&mut tx, access.team_id).await?;
    if req.role != Role::Owner && is_last_owner(&mut tx, access.team_id, user_id).await? {
        return Err(last_owner());
    }
    let done = sqlx::query("UPDATE memberships SET role = $3 WHERE team_id = $1 AND user_id = $2")
        .bind(access.team_id)
        .bind(user_id)
        .bind(role_str(req.role))
        .execute(&mut *tx)
        .await?;
    if done.rows_affected() == 0 {
        return Err(ApiError::not_found());
    }
    tx.commit().await?;
    Ok(Json(MemberRole {
        user_id,
        role: req.role,
    }))
}

#[derive(Deserialize)]
pub struct SetPlan {
    plan: Plan,
}

#[derive(Serialize)]
pub struct TeamPlan {
    slug: String,
    plan: Plan,
}

async fn set_plan(
    State(state): State<AppState>,
    auth: UserAuth,
    Path(slug): Path<String>,
    ApiJson(req): ApiJson<SetPlan>,
) -> Result<Json<TeamPlan>, ApiError> {
    if !is_operator(&state.db, &state.config, &auth.email).await? {
        return Err(ApiError::not_found());
    }
    let done = sqlx::query("UPDATE teams SET plan = $2 WHERE slug = $1")
        .bind(&slug)
        .bind(plan_str(req.plan))
        .execute(&state.db)
        .await?;
    if done.rows_affected() == 0 {
        return Err(ApiError::not_found());
    }
    Ok(Json(TeamPlan {
        slug,
        plan: req.plan,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn slug_rule() {
        for ok in ["a", "0", "acme", "payments-api", "a-", &"a".repeat(63)] {
            assert!(valid_slug(ok), "{ok}");
        }
        for bad in ["", "-a", "A", "a_b", "a b", "a.b", &"a".repeat(64)] {
            assert!(!valid_slug(bad), "{bad}");
        }
    }
}
