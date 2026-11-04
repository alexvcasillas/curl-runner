//! Team publish tokens and the `PublishAuth` extractor.

use std::collections::HashMap;

use axum::Json;
use axum::Router;
use axum::extract::{FromRequestParts, Path, State};
use axum::http::StatusCode;
use axum::http::request::Parts;
use axum::routing::{delete, get};
use chrono::{DateTime, Duration, Utc};
use curl_runner_cloud_proto::ErrorCode;
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

use crate::auth::UserAuth;
use crate::auth::session::{bearer_token, validate_session};
use crate::error::ApiError;
use crate::extract::ApiJson;
use crate::hash::{new_token, secret_hash};
use crate::state::AppState;
use crate::teams::team_access;

/// `last_used_at` is written at most this often.
const TOUCH_EVERY: Duration = Duration::minutes(1);
const MAX_NAME_LEN: usize = 100;
const MAX_EXPIRY_DAYS: u32 = 3650;

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/v1/teams/{team}/tokens", get(list).post(create))
        .route("/v1/teams/{team}/tokens/{id}", delete(revoke))
}

/// Who is publishing.
#[derive(Debug, Clone)]
pub enum Publisher {
    Token {
        team_id: Uuid,
        token_id: Uuid,
        name: String,
    },
    User {
        user_id: Uuid,
        email: String,
    },
}

impl Publisher {
    /// `"token {name}"` or the user's email; stored as `versions.published_by`.
    pub fn label(&self) -> String {
        match self {
            Self::Token { name, .. } => format!("token {name}"),
            Self::User { email, .. } => email.clone(),
        }
    }
}

/// A caller allowed to publish to the team in the URL's `{team}` segment:
/// a `crt_` token of that team, or a `crs_` session of one of its members.
#[derive(Debug, Clone)]
pub struct PublishAuth {
    pub team_id: Uuid,
    pub publisher: Publisher,
}

fn unauthorized() -> ApiError {
    ApiError::new(ErrorCode::Unauthorized, "Sign in or use a valid token.")
}

impl FromRequestParts<AppState> for PublishAuth {
    type Rejection = ApiError;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        let token = bearer_token(&parts.headers)
            .ok_or_else(unauthorized)?
            .to_owned();
        let token = token.as_str();
        let Path(params) = Path::<HashMap<String, String>>::from_request_parts(parts, state)
            .await
            .map_err(|_| ApiError::not_found())?;
        let slug = params.get("team").ok_or_else(ApiError::not_found)?;
        if token.starts_with("crt_") {
            token_auth(state, token, slug).await
        } else if token.starts_with("crs_") {
            let user = validate_session(state, token).await?;
            let access = team_access(&state.db, user.user_id, slug).await?;
            Ok(Self {
                team_id: access.team_id,
                publisher: Publisher::User {
                    user_id: user.user_id,
                    email: user.email,
                },
            })
        } else {
            Err(unauthorized())
        }
    }
}

#[derive(FromRow)]
struct TokenRow {
    id: Uuid,
    name: String,
    team_id: Uuid,
    last_used_at: Option<DateTime<Utc>>,
    same_team: bool,
}

async fn token_auth(state: &AppState, token: &str, slug: &str) -> Result<PublishAuth, ApiError> {
    let hash = secret_hash(state.config.secret_key.as_bytes(), token);
    let now = state.clock.now();
    let row: Option<TokenRow> = sqlx::query_as(
        "SELECT k.id, k.name, k.team_id, k.last_used_at, (t.slug = $3) AS same_team \
         FROM tokens k JOIN teams t ON t.id = k.team_id \
         WHERE k.token_hash = $1 AND k.revoked_at IS NULL \
         AND (k.expires_at IS NULL OR k.expires_at > $2)",
    )
    .bind(&hash)
    .bind(now)
    .bind(slug)
    .fetch_optional(&state.db)
    .await?;
    let TokenRow {
        id: token_id,
        name,
        team_id,
        last_used_at,
        same_team,
    } = row.ok_or_else(unauthorized)?;
    if !same_team {
        return Err(ApiError::not_found());
    }
    if last_used_at.is_none_or(|t| now - t >= TOUCH_EVERY) {
        sqlx::query("UPDATE tokens SET last_used_at = $2 WHERE id = $1")
            .bind(token_id)
            .bind(now)
            .execute(&state.db)
            .await?;
    }
    Ok(PublishAuth {
        team_id,
        publisher: Publisher::Token {
            team_id,
            token_id,
            name,
        },
    })
}

#[derive(Deserialize)]
struct CreateBody {
    name: String,
    expires_in_days: Option<u32>,
}

#[derive(Serialize)]
struct CreatedBody {
    id: Uuid,
    name: String,
    /// The clear token. Shown only here.
    token: String,
    expires_at: Option<DateTime<Utc>>,
}

async fn create(
    State(state): State<AppState>,
    auth: UserAuth,
    Path(team): Path<String>,
    ApiJson(body): ApiJson<CreateBody>,
) -> Result<(StatusCode, Json<CreatedBody>), ApiError> {
    let access = team_access(&state.db, auth.user_id, &team).await?;
    let name = body.name.trim();
    if name.is_empty() || name.chars().count() > MAX_NAME_LEN {
        return Err(ApiError::new(
            ErrorCode::InvalidRequest,
            "Give the token a name of up to 100 characters.",
        ));
    }
    if body
        .expires_in_days
        .is_some_and(|d| d == 0 || d > MAX_EXPIRY_DAYS)
    {
        return Err(ApiError::new(
            ErrorCode::InvalidRequest,
            "Expiry must be between 1 and 3650 days.",
        ));
    }
    let now = state.clock.now();
    let expires_at = body
        .expires_in_days
        .map(|d| now + Duration::days(i64::from(d)));
    let token = new_token("crt_");
    let id: Uuid = sqlx::query_scalar(
        "INSERT INTO tokens (team_id, name, token_hash, created_by, created_at, expires_at) \
         VALUES ($1, $2, $3, $4, $5, $6) RETURNING id",
    )
    .bind(access.team_id)
    .bind(name)
    .bind(secret_hash(state.config.secret_key.as_bytes(), &token))
    .bind(auth.user_id)
    .bind(now)
    .bind(expires_at)
    .fetch_one(&state.db)
    .await?;
    Ok((
        StatusCode::CREATED,
        Json(CreatedBody {
            id,
            name: name.to_owned(),
            token,
            expires_at,
        }),
    ))
}

#[derive(Serialize, FromRow)]
struct TokenBody {
    id: Uuid,
    name: String,
    created_by: Option<String>,
    created_at: DateTime<Utc>,
    expires_at: Option<DateTime<Utc>>,
    last_used_at: Option<DateTime<Utc>>,
}

/// Live (not revoked, not expired) tokens, newest first. No secrets.
async fn list(
    State(state): State<AppState>,
    auth: UserAuth,
    Path(team): Path<String>,
) -> Result<Json<Vec<TokenBody>>, ApiError> {
    let access = team_access(&state.db, auth.user_id, &team).await?;
    let tokens: Vec<TokenBody> = sqlx::query_as(
        "SELECT k.id, k.name, u.email::text AS created_by, k.created_at, k.expires_at, k.last_used_at \
         FROM tokens k LEFT JOIN users u ON u.id = k.created_by \
         WHERE k.team_id = $1 AND k.revoked_at IS NULL \
         AND (k.expires_at IS NULL OR k.expires_at > $2) \
         ORDER BY k.created_at DESC, k.id",
    )
    .bind(access.team_id)
    .bind(state.clock.now())
    .fetch_all(&state.db)
    .await?;
    Ok(Json(tokens))
}

async fn revoke(
    State(state): State<AppState>,
    auth: UserAuth,
    Path((team, id)): Path<(String, Uuid)>,
) -> Result<StatusCode, ApiError> {
    let access = team_access(&state.db, auth.user_id, &team).await?;
    let done = sqlx::query(
        "UPDATE tokens SET revoked_at = $3 WHERE id = $1 AND team_id = $2 AND revoked_at IS NULL",
    )
    .bind(id)
    .bind(access.team_id)
    .bind(state.clock.now())
    .execute(&state.db)
    .await?;
    if done.rows_affected() == 0 {
        return Err(ApiError::not_found());
    }
    Ok(StatusCode::NO_CONTENT)
}
