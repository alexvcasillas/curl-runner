//! Collections: listing, source switching, deletion. Publishing lives in
//! [`publish`], version reads in [`versions`].

mod publish;
mod versions;

use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::routing::{get, put};
use axum::{Json, Router};
use chrono::{DateTime, Utc};
use curl_runner_cloud_proto::{Access, Source};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

use crate::auth::UserAuth;
use crate::auth::session::column_enum;
use crate::error::ApiError;
use crate::extract::ApiJson;
use crate::state::AppState;
use crate::teams::team_access;

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/v1/teams/{team}/collections", get(list))
        .route(
            "/v1/teams/{team}/collections/{slug}",
            axum::routing::patch(patch).delete(remove),
        )
        .route(
            "/v1/teams/{team}/collections/{slug}/versions",
            // The publish handler reads its own body, capped in `publish`.
            put(publish::publish).get(versions::list),
        )
        .route(
            "/v1/teams/{team}/collections/{slug}/versions/{n}/files",
            get(versions::files),
        )
}

/// The wire form of a `source` column value.
fn source_str(source: Source) -> &'static str {
    match source {
        Source::Git => "git",
        Source::Cloud => "cloud",
    }
}

#[derive(Serialize)]
struct CollectionBody {
    slug: String,
    name: String,
    source: Source,
    access: Access,
    current_version: Option<i32>,
    updated_at: Option<DateTime<Utc>>,
}

#[derive(FromRow)]
struct CollectionRow {
    slug: String,
    name: String,
    source: String,
    access: String,
    current_version: Option<i32>,
    updated_at: Option<DateTime<Utc>>,
}

impl CollectionRow {
    fn into_body(self) -> Result<CollectionBody, ApiError> {
        Ok(CollectionBody {
            slug: self.slug,
            name: self.name,
            source: column_enum(self.source)?,
            access: column_enum(self.access)?,
            current_version: self.current_version,
            updated_at: self.updated_at,
        })
    }
}

/// `updated_at` is the current version's publish time (`null` before the first).
const COLLECTION_COLUMNS: &str = "c.slug::text AS slug, c.name, c.source, c.access, c.current_version, \
     (SELECT v.created_at FROM versions v \
      WHERE v.collection_id = c.id AND v.number = c.current_version) AS updated_at";

async fn list(
    State(state): State<AppState>,
    auth: UserAuth,
    Path(team): Path<String>,
) -> Result<Json<Vec<CollectionBody>>, ApiError> {
    let access = team_access(&state.db, auth.user_id, &team).await?;
    let rows: Vec<CollectionRow> = sqlx::query_as(&format!(
        "SELECT {COLLECTION_COLUMNS} FROM collections c WHERE c.team_id = $1 ORDER BY c.slug"
    ))
    .bind(access.team_id)
    .fetch_all(&state.db)
    .await?;
    Ok(Json(
        rows.into_iter()
            .map(CollectionRow::into_body)
            .collect::<Result<_, _>>()?,
    ))
}

/// The id of `slug` in the team, or `404`.
async fn collection_id(db: &sqlx::PgPool, team_id: Uuid, slug: &str) -> Result<Uuid, ApiError> {
    sqlx::query_scalar("SELECT id FROM collections WHERE team_id = $1 AND slug = $2")
        .bind(team_id)
        .bind(slug)
        .fetch_optional(db)
        .await?
        .ok_or_else(ApiError::not_found)
}

#[derive(Deserialize)]
struct PatchBody {
    source: Option<Source>,
}

/// Switch a collection's source. The current version is kept either way.
async fn patch(
    State(state): State<AppState>,
    auth: UserAuth,
    Path((team, slug)): Path<(String, String)>,
    ApiJson(body): ApiJson<PatchBody>,
) -> Result<Json<CollectionBody>, ApiError> {
    let access = team_access(&state.db, auth.user_id, &team).await?;
    let source = body.source.map(source_str);
    let row: Option<CollectionRow> = sqlx::query_as(&format!(
        "WITH c AS (UPDATE collections SET source = COALESCE($3, source) \
                    WHERE team_id = $1 AND slug = $2 RETURNING *) \
         SELECT {COLLECTION_COLUMNS} FROM c"
    ))
    .bind(access.team_id)
    .bind(&slug)
    .bind(source)
    .fetch_optional(&state.db)
    .await?;
    Ok(Json(row.ok_or_else(ApiError::not_found)?.into_body()?))
}

/// Owner only. Versions and access entries go with it; blobs stay.
async fn remove(
    State(state): State<AppState>,
    auth: UserAuth,
    Path((team, slug)): Path<(String, String)>,
) -> Result<StatusCode, ApiError> {
    let access = team_access(&state.db, auth.user_id, &team).await?;
    access.require_owner()?;
    let done = sqlx::query("DELETE FROM collections WHERE team_id = $1 AND slug = $2")
        .bind(access.team_id)
        .bind(&slug)
        .execute(&state.db)
        .await?;
    if done.rows_affected() == 0 {
        return Err(ApiError::not_found());
    }
    Ok(StatusCode::NO_CONTENT)
}
