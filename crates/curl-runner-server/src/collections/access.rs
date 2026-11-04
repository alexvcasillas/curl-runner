//! The customer side: fetching a shared collection by its link token.
//!
//! Access is decided per request from the collection's `access` mode and, for
//! `restricted`, the signed-in user's email against the access list (members
//! of the owning team are always let in). A
//! restricted collection keeps serving after its team drops to Free; only
//! publishing and list changes are plan-gated.

use axum::Json;
use axum::extract::{Path, State};
use chrono::{DateTime, Duration, Utc};
use curl_runner_cloud_proto::{Access, BundleFile, ErrorCode, Input, SharedInfo, Snapshot};
use serde::Serialize;
use sqlx::types::Json as Jsonb;
use uuid::Uuid;

use crate::auth::session::column_enum;
use crate::auth::{OptionalUserAuth, UserAuth};
use crate::error::ApiError;
use crate::extract::ClientIp;
use crate::state::AppState;

/// Requests per IP per minute across the three `/v1/shared/{token}` routes.
const SHARED_PER_MINUTE: u32 = 120;

/// Collection config files at the version root. They are not requests. Keep
/// in step with `curl_runner_core::config::CONFIG_FILE_PATHS`, the source of truth.
const CONFIG_FILES: [&str; 4] = [
    "curl-runner.yaml",
    "curl-runner.yml",
    ".curl-runner.yaml",
    ".curl-runner.yml",
];

/// A collection reached through a live link the caller may read.
struct Shared {
    collection_id: Uuid,
    team_name: String,
    slug: String,
    name: String,
    description: Option<String>,
    access: Access,
    version: i32,
    version_id: Uuid,
    hash: String,
    published_at: DateTime<Utc>,
    /// The access-list entry that let the caller in (restricted only).
    matched_entry: Option<String>,
}

fn unauthorized() -> ApiError {
    ApiError::new(ErrorCode::Unauthorized, "Sign in to open this collection.")
}

type Row = (
    Uuid,
    Uuid,
    String,
    String,
    String,
    Option<String>,
    String,
    i32,
    Uuid,
    String,
    DateTime<Utc>,
);

/// Find the collection behind `token` and decide whether `auth` may read it.
async fn resolve(
    state: &AppState,
    ip: &str,
    auth: Option<&UserAuth>,
    token: &str,
) -> Result<Shared, ApiError> {
    state.limiter.check(
        &format!("shared:ip:{ip}"),
        SHARED_PER_MINUTE,
        Duration::minutes(1),
    )?;
    if !token.starts_with("crsl_") {
        return Err(ApiError::not_found());
    }
    let hash = crate::hash::secret_hash(state.config.secret_key.as_bytes(), token);
    let row: Option<Row> = sqlx::query_as(
        "SELECT c.id, c.team_id, t.name, c.slug::text, c.name, c.description, c.access, \
                v.number, v.id, v.hash, v.created_at \
         FROM collections c \
         JOIN teams t ON t.id = c.team_id \
         JOIN versions v ON v.collection_id = c.id AND v.number = c.current_version \
         WHERE c.link_hash = $1 AND c.access <> 'off' \
           AND (c.link_expires_at IS NULL OR c.link_expires_at > $2)",
    )
    .bind(&hash)
    .bind(state.clock.now())
    .fetch_optional(&state.db)
    .await?;
    let (
        collection_id,
        team_id,
        team_name,
        slug,
        name,
        description,
        access,
        version,
        version_id,
        hash,
        published_at,
    ) = row.ok_or_else(ApiError::not_found)?;
    let access: Access = column_enum(access)?;
    let matched_entry = if access == Access::Restricted {
        let user = auth.ok_or_else(unauthorized)?;
        if is_member(state, team_id, user.user_id).await? {
            None
        } else {
            let allowed = matching_entry(state, collection_id, &user.email).await?;
            Some(allowed.ok_or_else(|| {
            ApiError::new(
                ErrorCode::Forbidden,
                format!(
                    "Your account isn't on the list for this collection. Ask {team_name} for access."
                ),
            )
        })?)
        }
    } else {
        None
    };
    Ok(Shared {
        collection_id,
        team_name,
        slug,
        name,
        description,
        access,
        version,
        version_id,
        hash,
        published_at,
        matched_entry,
    })
}

async fn is_member(state: &AppState, team_id: Uuid, user_id: Uuid) -> Result<bool, ApiError> {
    Ok(sqlx::query_scalar(
        "SELECT EXISTS (SELECT 1 FROM memberships WHERE team_id = $1 AND user_id = $2)",
    )
    .bind(team_id)
    .bind(user_id)
    .fetch_one(&state.db)
    .await?)
}

/// The access-list entry covering `email`: the email itself, else its domain.
async fn matching_entry(
    state: &AppState,
    collection_id: Uuid,
    email: &str,
) -> Result<Option<String>, ApiError> {
    let email = email.to_lowercase();
    let domain = email.rsplit_once('@').map_or("", |(_, d)| d);
    Ok(sqlx::query_scalar(
        "SELECT entry::text FROM access_entries WHERE collection_id = $1 \
           AND ((kind = 'email' AND entry = $2) OR (kind = 'domain' AND entry = $3)) \
         ORDER BY kind = 'email' DESC LIMIT 1",
    )
    .bind(collection_id)
    .bind(&email)
    .bind(format!("*@{domain}"))
    .fetch_optional(&state.db)
    .await?)
}

/// Record that `auth` opened the collection, at most once per user per UTC
/// day. Anonymous opens of a public collection are not recorded at all, so
/// public traffic cannot grow the table.
async fn record_open(state: &AppState, shared: &Shared, auth: Option<&UserAuth>) {
    let Some(user) = auth else { return };
    let now = state.clock.now();
    let day_start = now.date_naive().and_time(chrono::NaiveTime::MIN).and_utc();
    let result = sqlx::query(
        "INSERT INTO share_opens (collection_id, user_id, entry, opened_at) \
         SELECT $1, $2, $3, $4 WHERE NOT EXISTS \
           (SELECT 1 FROM share_opens WHERE collection_id = $1 AND user_id = $2 AND opened_at >= $5)",
    )
    .bind(shared.collection_id)
    .bind(user.user_id)
    .bind(&shared.matched_entry)
    .bind(now)
    .bind(day_start)
    .execute(&state.db)
    .await;
    // A failed bookkeeping write must not fail the customer's read.
    if let Err(err) = result {
        tracing::warn!("could not record a share open: {err}");
    }
}

async fn request_count(state: &AppState, version_id: Uuid) -> Result<u32, ApiError> {
    let n: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM version_files WHERE version_id = $1 AND path <> ALL($2)",
    )
    .bind(version_id)
    .bind(&CONFIG_FILES[..])
    .fetch_one(&state.db)
    .await?;
    Ok(u32::try_from(n).unwrap_or(u32::MAX))
}

async fn info_of(state: &AppState, shared: &Shared) -> Result<SharedInfo, ApiError> {
    Ok(SharedInfo {
        team: shared.team_name.clone(),
        collection: shared.slug.clone(),
        name: shared.name.clone(),
        description: shared.description.clone(),
        version: u32::try_from(shared.version).unwrap_or_default(),
        updated_at: shared.published_at,
        request_count: request_count(state, shared.version_id).await?,
        access: shared.access,
    })
}

pub(super) async fn info(
    State(state): State<AppState>,
    ClientIp(ip): ClientIp,
    OptionalUserAuth(auth): OptionalUserAuth,
    Path(token): Path<String>,
) -> Result<Json<SharedInfo>, ApiError> {
    let shared = resolve(&state, &ip, auth.as_ref(), &token).await?;
    record_open(&state, &shared, auth.as_ref()).await;
    Ok(Json(info_of(&state, &shared).await?))
}

pub(super) async fn snapshot(
    State(state): State<AppState>,
    ClientIp(ip): ClientIp,
    OptionalUserAuth(auth): OptionalUserAuth,
    Path(token): Path<String>,
) -> Result<Json<Snapshot>, ApiError> {
    let shared = resolve(&state, &ip, auth.as_ref(), &token).await?;
    record_open(&state, &shared, auth.as_ref()).await;
    let files: Vec<(String, String)> = sqlx::query_as(
        "SELECT f.path, b.text FROM version_files f JOIN blobs b ON b.hash = f.hash \
         WHERE f.version_id = $1 ORDER BY f.path",
    )
    .bind(shared.version_id)
    .fetch_all(&state.db)
    .await?;
    let inputs: Jsonb<Vec<Input>> = sqlx::query_scalar("SELECT inputs FROM versions WHERE id = $1")
        .bind(shared.version_id)
        .fetch_one(&state.db)
        .await?;
    Ok(Json(Snapshot {
        info: info_of(&state, &shared).await?,
        files: files
            .into_iter()
            .map(|(path, text)| BundleFile { path, text })
            .collect(),
        inputs: inputs.0,
    }))
}

#[derive(Serialize)]
pub(super) struct Latest {
    version: u32,
    hash: String,
}

/// The cheap update check: no files, no open recorded.
pub(super) async fn latest(
    State(state): State<AppState>,
    ClientIp(ip): ClientIp,
    OptionalUserAuth(auth): OptionalUserAuth,
    Path(token): Path<String>,
) -> Result<Json<Latest>, ApiError> {
    let shared = resolve(&state, &ip, auth.as_ref(), &token).await?;
    Ok(Json(Latest {
        version: u32::try_from(shared.version).unwrap_or_default(),
        hash: shared.hash,
    }))
}
