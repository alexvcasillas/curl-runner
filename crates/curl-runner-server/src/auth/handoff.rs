//! Device hand-off: a signed-in dashboard session mints a one-time code that
//! the studio or CLI redeems for its own device session.

use axum::Json;
use axum::extract::State;
use chrono::Duration;
use curl_runner_cloud_proto::ErrorCode;
use serde::{Deserialize, Serialize};

use crate::auth::session::{UserAuth, UserBody, create_session};
use crate::error::ApiError;
use crate::extract::{ApiJson, ClientIp};
use crate::hash::{new_token, secret_hash};
use crate::state::AppState;

const CODE_TTL: Duration = Duration::minutes(2);
const MAX_LABEL: usize = 100;

#[derive(Deserialize)]
pub struct CreateRequest {
    session_label: Option<String>,
}

#[derive(Serialize)]
pub struct CreateBody {
    code: String,
}

/// Mint a hand-off code. Only a web session may, so a device session cannot
/// spawn more of itself. The device session is created at redeem time, since
/// its token is shown once and the server keeps only its hash.
pub async fn create(
    State(state): State<AppState>,
    auth: UserAuth,
    ApiJson(req): ApiJson<CreateRequest>,
) -> Result<Json<CreateBody>, ApiError> {
    let kind: String = sqlx::query_scalar("SELECT kind FROM sessions WHERE id = $1")
        .bind(auth.session_id)
        .fetch_one(&state.db)
        .await?;
    if kind != "web" {
        return Err(ApiError::new(
            ErrorCode::Forbidden,
            "Hand off from a web session.",
        ));
    }
    let label = req
        .session_label
        .map(|l| l.trim().to_owned())
        .filter(|l| !l.is_empty());
    if label
        .as_ref()
        .is_some_and(|l| l.chars().count() > MAX_LABEL)
    {
        return Err(ApiError::new(
            ErrorCode::InvalidRequest,
            "The label is too long.",
        ));
    }
    let now = state.clock.now();
    sqlx::query("DELETE FROM auth_handoffs WHERE expires_at <= $1")
        .bind(now)
        .execute(&state.db)
        .await?;
    let code = new_token("crh_");
    sqlx::query(
        "INSERT INTO auth_handoffs (code_hash, session_id, expires_at, label) \
         VALUES ($1, $2, $3, $4)",
    )
    .bind(secret_hash(state.config.secret_key.as_bytes(), &code))
    .bind(auth.session_id)
    .bind(now + CODE_TTL)
    .bind(&label)
    .execute(&state.db)
    .await?;
    Ok(Json(CreateBody { code }))
}

#[derive(Deserialize)]
pub struct RedeemRequest {
    code: String,
}

#[derive(Serialize)]
pub struct RedeemBody {
    session: String,
    user: UserBody,
}

pub async fn redeem(
    State(state): State<AppState>,
    ClientIp(ip): ClientIp,
    ApiJson(req): ApiJson<RedeemRequest>,
) -> Result<Json<RedeemBody>, ApiError> {
    state.limiter.check(
        &format!("handoff-redeem:ip:{ip}"),
        20,
        Duration::minutes(15),
    )?;
    let now = state.clock.now();
    let mut tx = state.db.begin().await?;
    // Deleting is the single-use guarantee: only one caller gets the row.
    let row: Option<(uuid::Uuid, chrono::DateTime<chrono::Utc>, Option<String>)> = sqlx::query_as(
        "DELETE FROM auth_handoffs WHERE code_hash = $1 \
             RETURNING session_id, expires_at, label",
    )
    .bind(secret_hash(
        state.config.secret_key.as_bytes(),
        req.code.trim(),
    ))
    .fetch_optional(&mut *tx)
    .await?;
    let (session_id, expires_at, label) =
        row.ok_or_else(|| ApiError::new(ErrorCode::CodeInvalid, "That code is not right."))?;
    if expires_at <= now {
        tx.commit().await?;
        return Err(ApiError::new(
            ErrorCode::CodeExpired,
            "That code has expired. Ask for a new one.",
        ));
    }
    // The issuing web session must still be live.
    let user: Option<UserBody> = sqlx::query_as(
        "SELECT u.id, u.email::text AS email, u.name FROM sessions s \
         JOIN users u ON u.id = s.user_id \
         WHERE s.id = $1 AND s.revoked_at IS NULL AND s.expires_at > $2",
    )
    .bind(session_id)
    .bind(now)
    .fetch_optional(&mut *tx)
    .await?;
    let user =
        user.ok_or_else(|| ApiError::new(ErrorCode::CodeInvalid, "That code is not right."))?;
    let session = create_session(&mut tx, &state, user.id, "device", label.as_deref()).await?;
    tx.commit().await?;
    Ok(Json(RedeemBody { session, user }))
}
