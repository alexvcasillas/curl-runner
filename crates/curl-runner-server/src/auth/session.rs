//! Session tokens, the `UserAuth` extractor and the session endpoints.

use axum::Json;
use axum::extract::{FromRequestParts, Path, State};
use axum::http::StatusCode;
use axum::http::header::AUTHORIZATION;
use axum::http::request::Parts;
use chrono::{DateTime, Duration, Utc};
use curl_runner_cloud_proto::ErrorCode;
use serde::Serialize;
use sqlx::{FromRow, PgConnection};
use uuid::Uuid;

use crate::error::ApiError;
use crate::hash::{new_token, secret_hash};
use crate::state::AppState;

/// Sessions slide: each use pushes expiry to this far from now.
pub const SESSION_TTL: Duration = Duration::days(90);
/// `last_used_at` (and the slide) are written at most this often.
const TOUCH_EVERY: Duration = Duration::minutes(1);

/// The signed-in user behind a valid `Authorization: Bearer crs_...` header.
#[derive(Debug, Clone)]
pub struct UserAuth {
    pub user_id: Uuid,
    pub session_id: Uuid,
    pub email: String,
}

fn unauthorized() -> ApiError {
    ApiError::new(ErrorCode::Unauthorized, "Sign in to continue.")
}

impl FromRequestParts<AppState> for UserAuth {
    type Rejection = ApiError;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        let token = parts
            .headers
            .get(AUTHORIZATION)
            .and_then(|v| v.to_str().ok())
            .and_then(|v| v.strip_prefix("Bearer "))
            .map(str::trim)
            .filter(|t| t.starts_with("crs_"))
            .ok_or_else(unauthorized)?;
        let hash = secret_hash(state.config.secret_key.as_bytes(), token);
        let now = state.clock.now();
        let row: Option<(Uuid, Uuid, String, DateTime<Utc>)> = sqlx::query_as(
            "SELECT s.id, s.user_id, u.email::text, s.last_used_at \
             FROM sessions s JOIN users u ON u.id = s.user_id \
             WHERE s.token_hash = $1 AND s.revoked_at IS NULL AND s.expires_at > $2",
        )
        .bind(&hash)
        .bind(now)
        .fetch_optional(&state.db)
        .await?;
        let (session_id, user_id, email, last_used_at) = row.ok_or_else(unauthorized)?;
        if now - last_used_at >= TOUCH_EVERY {
            sqlx::query("UPDATE sessions SET last_used_at = $2, expires_at = $3 WHERE id = $1")
                .bind(session_id)
                .bind(now)
                .bind(now + SESSION_TTL)
                .execute(&state.db)
                .await?;
        }
        Ok(Self {
            user_id,
            session_id,
            email,
        })
    }
}

/// Insert a session for `user_id` and return the clear token (shown once).
pub async fn create_session(
    conn: &mut PgConnection,
    state: &AppState,
    user_id: Uuid,
    kind: &str,
    label: Option<&str>,
) -> Result<String, ApiError> {
    let token = new_token("crs_");
    let now = state.clock.now();
    sqlx::query(
        "INSERT INTO sessions (user_id, kind, token_hash, label, created_at, last_used_at, expires_at) \
         VALUES ($1, $2, $3, $4, $5, $5, $6)",
    )
    .bind(user_id)
    .bind(kind)
    .bind(secret_hash(state.config.secret_key.as_bytes(), &token))
    .bind(label)
    .bind(now)
    .bind(now + SESSION_TTL)
    .execute(conn)
    .await?;
    Ok(token)
}

#[derive(Serialize, FromRow)]
pub struct UserBody {
    pub id: Uuid,
    pub email: String,
    pub name: Option<String>,
}

#[derive(Serialize, FromRow)]
struct TeamBody {
    slug: String,
    name: String,
    role: String,
    plan: String,
}

#[derive(Serialize)]
pub struct MeBody {
    user: UserBody,
    teams: Vec<TeamBody>,
}

pub async fn me(State(state): State<AppState>, auth: UserAuth) -> Result<Json<MeBody>, ApiError> {
    let user: UserBody =
        sqlx::query_as("SELECT id, email::text AS email, name FROM users WHERE id = $1")
            .bind(auth.user_id)
            .fetch_one(&state.db)
            .await?;
    let teams: Vec<TeamBody> = sqlx::query_as(
        "SELECT t.slug::text AS slug, t.name, m.role, t.plan \
         FROM memberships m JOIN teams t ON t.id = m.team_id \
         WHERE m.user_id = $1 ORDER BY t.name, t.slug",
    )
    .bind(auth.user_id)
    .fetch_all(&state.db)
    .await?;
    Ok(Json(MeBody { user, teams }))
}

pub async fn sign_out(
    State(state): State<AppState>,
    auth: UserAuth,
) -> Result<Json<serde_json::Value>, ApiError> {
    revoke_session(&state, auth.user_id, auth.session_id).await?;
    Ok(Json(serde_json::json!({})))
}

#[derive(Serialize, FromRow)]
pub struct SessionBody {
    id: Uuid,
    kind: String,
    label: Option<String>,
    created_at: DateTime<Utc>,
    last_used_at: DateTime<Utc>,
    expires_at: DateTime<Utc>,
    current: bool,
}

pub async fn list(
    State(state): State<AppState>,
    auth: UserAuth,
) -> Result<Json<Vec<SessionBody>>, ApiError> {
    let sessions: Vec<SessionBody> = sqlx::query_as(
        "SELECT id, kind, label, created_at, last_used_at, expires_at, (id = $2) AS current \
         FROM sessions WHERE user_id = $1 AND revoked_at IS NULL AND expires_at > $3 \
         ORDER BY created_at DESC, id",
    )
    .bind(auth.user_id)
    .bind(auth.session_id)
    .bind(state.clock.now())
    .fetch_all(&state.db)
    .await?;
    Ok(Json(sessions))
}

pub async fn revoke(
    State(state): State<AppState>,
    auth: UserAuth,
    Path(id): Path<Uuid>,
) -> Result<StatusCode, ApiError> {
    revoke_session(&state, auth.user_id, id).await?;
    Ok(StatusCode::NO_CONTENT)
}

/// Revoke one of `user_id`'s live sessions; anyone else's is `404`.
async fn revoke_session(state: &AppState, user_id: Uuid, id: Uuid) -> Result<(), ApiError> {
    let done = sqlx::query(
        "UPDATE sessions SET revoked_at = $3 WHERE id = $1 AND user_id = $2 AND revoked_at IS NULL",
    )
    .bind(id)
    .bind(user_id)
    .bind(state.clock.now())
    .execute(&state.db)
    .await?;
    if done.rows_affected() == 0 {
        return Err(ApiError::not_found());
    }
    Ok(())
}
