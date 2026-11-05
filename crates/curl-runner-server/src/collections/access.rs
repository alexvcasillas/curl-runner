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
use crate::crypto::decrypt_link_token;
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
    team_slug: String,
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
        "SELECT c.id, c.team_id, t.name, t.slug::text, c.slug::text, c.name, c.description, c.access, \
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
        team_slug,
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
        team_slug,
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

/// The values an access-list entry is matched against for `email`: the
/// lowercased address and its `*@domain` form. Entries are `citext`, so the
/// comparison ignores case on the stored side too.
pub(super) fn entry_keys(email: &str) -> (String, String) {
    let email = email.to_lowercase();
    let domain = email.rsplit_once('@').map_or("", |(_, d)| d).to_owned();
    (email, format!("*@{domain}"))
}

/// SQL: whether the `access_entries` row aliased `alias` covers the caller.
/// `email` and `domain` are the placeholders (`$2`, `$3`) bound to [`entry_keys`].
pub(super) fn entry_covers(alias: &str, email: &str, domain: &str) -> String {
    format!(
        "(({alias}.kind = 'email' AND {alias}.entry = {email}) \
          OR ({alias}.kind = 'domain' AND {alias}.entry = {domain}))"
    )
}

/// The access-list entry covering `email`: the email itself, else its domain.
async fn matching_entry(
    state: &AppState,
    collection_id: Uuid,
    email: &str,
) -> Result<Option<String>, ApiError> {
    let (email, domain) = entry_keys(email);
    Ok(sqlx::query_scalar(&format!(
        "SELECT e.entry::text FROM access_entries e WHERE e.collection_id = $1 AND {} \
         ORDER BY e.kind = 'email' DESC LIMIT 1",
        entry_covers("e", "$2", "$3")
    ))
    .bind(collection_id)
    .bind(&email)
    .bind(&domain)
    .fetch_optional(&state.db)
    .await?)
}

#[derive(Serialize)]
pub(crate) struct SharedWithMe {
    pub(crate) team: String,
    pub(crate) team_slug: String,
    pub(crate) collection: String,
    pub(crate) name: String,
    pub(crate) version: u32,
    pub(crate) updated_at: DateTime<Utc>,
    pub(crate) access: Access,
    /// The share link. `None` when the reader (the admin panel) must not see it.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) link: Option<String>,
}

type SharedRow = (
    String,
    String,
    String,
    String,
    i32,
    DateTime<Utc>,
    String,
    Option<String>,
);

/// SQL `FROM ... WHERE` (aliases `c`, `t`, `v`) for the collections shared
/// with a user by teams they are not in: a live link, a published version,
/// and either an access-list entry covering them (restricted) or a recorded
/// open (public). The arguments are SQL expressions: the user's id, their
/// lowercased email, the `*@domain` form of it, and the current time.
pub(crate) fn shared_from(user: &str, email: &str, domain: &str, now: &str) -> String {
    format!(
        "FROM collections c \
         JOIN teams t ON t.id = c.team_id \
         JOIN versions v ON v.collection_id = c.id AND v.number = c.current_version \
         WHERE c.access <> 'off' AND c.link_hash IS NOT NULL \
           AND (c.link_expires_at IS NULL OR c.link_expires_at > {now}) \
           AND NOT EXISTS (SELECT 1 FROM memberships m \
                           WHERE m.team_id = c.team_id AND m.user_id = {user}) \
           AND ((c.access = 'restricted' AND EXISTS ( \
                    SELECT 1 FROM access_entries e WHERE e.collection_id = c.id AND {})) \
                OR (c.access = 'public' AND EXISTS ( \
                    SELECT 1 FROM share_opens o \
                    WHERE o.collection_id = c.id AND o.user_id = {user})))",
        entry_covers("e", email, domain)
    )
}

/// The collections shared with `user_id` (whose account email is `email`),
/// newest first. A restricted collection the user opened but is no longer
/// listed on is left out, since they cannot open it now. With `with_link`
/// false the share link is neither decrypted nor returned.
pub(crate) async fn shared_for(
    state: &AppState,
    user_id: Uuid,
    email: &str,
    with_link: bool,
) -> Result<Vec<SharedWithMe>, ApiError> {
    let (email, domain) = entry_keys(email);
    let rows: Vec<SharedRow> = sqlx::query_as(&format!(
        "SELECT t.name, t.slug::text, c.slug::text, c.name, v.number, v.created_at, c.access, c.link_token_enc \
         {} ORDER BY v.created_at DESC, t.name, c.slug",
        shared_from("$1", "$2", "$3", "$4")
    ))
    .bind(user_id)
    .bind(&email)
    .bind(&domain)
    .bind(state.clock.now())
    .fetch_all(&state.db)
    .await?;
    let mut out = Vec::with_capacity(rows.len());
    for (team, team_slug, collection, name, version, updated_at, access, enc) in rows {
        let link = if with_link {
            let Some(token) = enc
                .as_deref()
                .and_then(|enc| decrypt_link_token(state.config.secret_key.as_bytes(), enc))
            else {
                tracing::error!("collection {team}/{collection} has no readable share link token");
                continue;
            };
            Some(format!("{}/s/{token}", state.config.app_url))
        } else {
            None
        };
        out.push(SharedWithMe {
            team,
            team_slug,
            collection,
            name,
            version: u32::try_from(version).unwrap_or_default(),
            updated_at,
            access: column_enum(access)?,
            link,
        });
    }
    Ok(out)
}

/// `GET /v1/me/shared`: [`shared_for`] the caller, with links.
pub(super) async fn shared_with_me(
    State(state): State<AppState>,
    auth: UserAuth,
) -> Result<Json<Vec<SharedWithMe>>, ApiError> {
    Ok(Json(
        shared_for(&state, auth.user_id, &auth.email, true).await?,
    ))
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
        team_slug: shared.team_slug.clone(),
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
