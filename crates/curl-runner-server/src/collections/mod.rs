//! Collections: listing, source switching, deletion. Publishing lives in
//! [`publish`], version reads in [`versions`].

mod access;
pub(crate) mod gallery;
mod publish;
mod share;
mod versions;

pub(crate) use access::{SharedWithMe, share_blocked, shared_for, shared_from};
pub(crate) use share::parse_email;

use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::routing::{get, put};
use axum::{Json, Router};
use chrono::{DateTime, Utc};
use curl_runner_cloud_proto::{Access, GitMeta, Source};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

use crate::auth::UserAuth;
use crate::auth::session::column_enum;
use crate::error::ApiError;
use crate::extract::ApiJson;
use crate::state::AppState;
use crate::teams::team_access;
use crate::tokens::PublishAuth;

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
            "/v1/teams/{team}/collections/{slug}/versions/{n}",
            get(versions::detail),
        )
        .route(
            "/v1/teams/{team}/collections/{slug}/versions/{n}/files",
            get(versions::files),
        )
        .route(
            "/v1/teams/{team}/collections/{slug}/share",
            get(share::get).patch(share::patch),
        )
        .route(
            "/v1/teams/{team}/collections/{slug}/share/entries",
            axum::routing::post(share::add_entry),
        )
        .route(
            "/v1/teams/{team}/collections/{slug}/share/entries/{entry}",
            axum::routing::delete(share::remove_entry),
        )
        .route("/v1/public/collections", get(gallery::list))
        .route("/v1/public/collections/{team}/{slug}", get(gallery::detail))
        .route("/v1/me/shared", get(access::shared_with_me))
        .route("/v1/shared/{token}", get(access::info))
        .route("/v1/shared/{token}/snapshot", get(access::snapshot))
        .route("/v1/shared/{token}/versions/latest", get(access::latest))
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
    /// Who published the current version: `token {name}` or an email.
    publisher: Option<String>,
    /// The current version's git details: the dashboard calls a collection
    /// published from files `Git` when they're there, `Local files` when not.
    git: Option<GitMeta>,
}

#[derive(FromRow)]
struct CollectionRow {
    slug: String,
    name: String,
    source: String,
    access: String,
    current_version: Option<i32>,
    updated_at: Option<DateTime<Utc>>,
    publisher: Option<String>,
    git: Option<sqlx::types::Json<GitMeta>>,
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
            publisher: self.publisher,
            git: self.git.map(|g| g.0),
        })
    }
}

/// `updated_at` and `publisher` are the current version's (`null` before the
/// first), read from one lateral subquery so a list costs one query.
const COLLECTION_COLUMNS: &str = "c.slug::text AS slug, c.name, c.source, c.access, c.current_version, \
     cv.created_at AS updated_at, cv.published_by AS publisher, cv.git";

/// The current version's row, joined to each collection.
const CURRENT_VERSION_JOIN: &str = "LEFT JOIN LATERAL (SELECT v.created_at, v.published_by, v.git \
     FROM versions v WHERE v.collection_id = c.id AND v.number = c.current_version) cv ON true";

/// Read-only, so a team's publish token may call it as well as a member.
async fn list(
    State(state): State<AppState>,
    auth: PublishAuth,
) -> Result<Json<Vec<CollectionBody>>, ApiError> {
    let rows: Vec<CollectionRow> = sqlx::query_as(&format!(
        "SELECT {COLLECTION_COLUMNS} FROM collections c {CURRENT_VERSION_JOIN} \
         WHERE c.team_id = $1 ORDER BY c.slug"
    ))
    .bind(auth.team_id)
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
         SELECT {COLLECTION_COLUMNS} FROM c {CURRENT_VERSION_JOIN}"
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
