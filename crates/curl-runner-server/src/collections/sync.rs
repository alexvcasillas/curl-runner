//! Two-way sync for collections whose source is `cloud`, and creating one.
//!
//! `POST /v1/teams/{team}/collections/{slug}/sync` runs the per-file
//! three-way merge ([`curl_runner_cloud_proto::sync::merge`]) under a row
//! lock on the collection, so two teammates syncing at once take turns: the
//! second sees the first's version as the server's copy. Any change the
//! server takes makes a new version, authored by the member.

use std::collections::{HashMap, HashSet};

use axum::Json;
use axum::body::Body;
use axum::extract::{Path, State};
use axum::http::{HeaderMap, StatusCode};
use chrono::{DateTime, Utc};
use curl_runner_cloud_proto::sync::{PulledFile, SyncConflict, SyncRequest, SyncResponse, merge};
use curl_runner_cloud_proto::{
    BundleFile, ErrorCode, MAX_FILE_BYTES, bundle_hash_from_file_hashes, file_hash, validate_path,
    validate_tree,
};
use serde::Deserialize;
use sqlx::PgConnection;
use uuid::Uuid;

use super::gallery::file_request_count;
use super::publish::{
    MAX_BODY, PUBLISHING_PAUSED, TEAM_SUSPENDED, TOO_LARGE, decode_body, insert_blobs,
    invalid_bundle, invalid_request, is_length_limit, lock_collection,
};
use super::{COLLECTION_COLUMNS, CURRENT_VERSION_JOIN, CollectionBody, CollectionRow};
use crate::auth::UserAuth;
use crate::billing::effective::team_plan;
use crate::crypto::encrypt_link_token;
use crate::error::ApiError;
use crate::extract::ApiJson;
use crate::hash::{new_token, secret_hash};
use crate::plans::limits;
use crate::state::AppState;
use crate::teams::{team_access, valid_slug};

/// Saves sync a moment after they happen and every member checks every few
/// minutes, so this is generous; it only stops a runaway client.
const SYNCS_PER_USER: u32 = 120;
const SYNC_WINDOW: chrono::Duration = chrono::Duration::minutes(1);
const MAX_NAME_CHARS: usize = 100;

fn is_sha256_hex(s: &str) -> bool {
    s.len() == 64 && s.bytes().all(|b| matches!(b, b'0'..=b'9' | b'a'..=b'f'))
}

/// CPU-bound: decode, parse and check the request (kept off the async
/// workers). Paths are valid and unique, hashes are SHA-256 hex, and every
/// text matches its hash.
fn check(headers: &HeaderMap, body: &[u8]) -> Result<SyncRequest, ApiError> {
    let req: SyncRequest = serde_json::from_slice(&decode_body(headers, body)?)
        .map_err(|_| invalid_request("The request body is not a valid sync request."))?;
    let mut seen = HashSet::with_capacity(req.files.len() + req.deleted.len());
    let paths = req
        .files
        .iter()
        .map(|f| &f.path)
        .chain(req.deleted.iter().map(|d| &d.path));
    for path in paths {
        validate_path(path).map_err(|e| invalid_bundle(format!("{path}: {e}")))?;
        if !seen.insert(path.as_str()) {
            return Err(invalid_bundle(format!("{path} is listed twice.")));
        }
    }
    let hashes = req
        .files
        .iter()
        .flat_map(|f| std::iter::once(&f.hash).chain(f.base.as_ref()))
        .chain(req.deleted.iter().map(|d| &d.base));
    if !hashes.into_iter().all(|h| is_sha256_hex(h)) {
        return Err(invalid_request("Hashes must be lowercase SHA-256 hex."));
    }
    for f in &req.files {
        let Some(text) = &f.text else { continue };
        if text.len() > MAX_FILE_BYTES {
            return Err(invalid_bundle(format!(
                "{}: file is larger than 1 MiB",
                f.path
            )));
        }
        // Postgres text cannot hold NUL.
        if text.contains('\0') {
            return Err(invalid_bundle(format!("{}: text contains a NUL", f.path)));
        }
        if file_hash(text) != f.hash {
            return Err(invalid_bundle(format!(
                "{}: the hash does not match the text.",
                f.path
            )));
        }
    }
    Ok(req)
}

/// Paused publishing and suspended teams stop syncs as they stop publishes.
async fn refuse_when_stopped(state: &AppState, team_id: Uuid) -> Result<(), ApiError> {
    if state.settings().await?.publishing_paused {
        return Err(ApiError::new(ErrorCode::Unavailable, PUBLISHING_PAUSED));
    }
    let suspended: bool =
        sqlx::query_scalar("SELECT suspended_at IS NOT NULL FROM teams WHERE id = $1")
            .bind(team_id)
            .fetch_one(&state.db)
            .await?;
    if suspended {
        return Err(ApiError::new(ErrorCode::TeamSuspended, TEAM_SUSPENDED));
    }
    Ok(())
}

/// One file of the current version.
struct ServerFile {
    hash: String,
    len: usize,
}

async fn current_files(
    conn: &mut PgConnection,
    collection_id: Uuid,
    version: Option<i32>,
) -> Result<HashMap<String, ServerFile>, ApiError> {
    let Some(n) = version else {
        return Ok(HashMap::new());
    };
    let rows: Vec<(String, String, i32)> = sqlx::query_as(
        "SELECT vf.path, vf.hash, octet_length(b.text) FROM versions v \
         JOIN version_files vf ON vf.version_id = v.id JOIN blobs b ON b.hash = vf.hash \
         WHERE v.collection_id = $1 AND v.number = $2",
    )
    .bind(collection_id)
    .bind(n)
    .fetch_all(conn)
    .await?;
    Ok(rows
        .into_iter()
        .map(|(path, hash, len)| {
            let len = usize::try_from(len).unwrap_or_default();
            (path, ServerFile { hash, len })
        })
        .collect())
}

async fn texts(
    conn: &mut PgConnection,
    hashes: Vec<&str>,
) -> Result<HashMap<String, String>, ApiError> {
    if hashes.is_empty() {
        return Ok(HashMap::new());
    }
    let rows: Vec<(String, String)> =
        sqlx::query_as("SELECT hash, text FROM blobs WHERE hash = ANY($1::text[])")
            .bind(hashes)
            .fetch_all(conn)
            .await?;
    Ok(rows.into_iter().collect())
}

/// Who made the server's copy of `path` (its hash, `None` when deleted), and
/// when: the first version of the latest run of versions where it reads so.
async fn last_change(
    conn: &mut PgConnection,
    collection_id: Uuid,
    path: &str,
    hash: Option<&str>,
    now: DateTime<Utc>,
) -> Result<(String, DateTime<Utc>), ApiError> {
    let row: Option<(String, DateTime<Utc>)> = sqlx::query_as(
        "SELECT v.published_by, v.created_at FROM versions v \
         WHERE v.collection_id = $1 AND v.number > COALESCE(( \
             SELECT max(v2.number) FROM versions v2 \
             LEFT JOIN version_files f ON f.version_id = v2.id AND f.path = $2 \
             WHERE v2.collection_id = $1 AND f.hash IS DISTINCT FROM $3::text), 0) \
         ORDER BY v.number LIMIT 1",
    )
    .bind(collection_id)
    .bind(path)
    .bind(hash)
    .fetch_optional(conn)
    .await?;
    Ok(row.unwrap_or_else(|| (String::new(), now)))
}

pub(super) async fn sync(
    State(state): State<AppState>,
    auth: UserAuth,
    Path((team, slug)): Path<(String, String)>,
    headers: HeaderMap,
    body: Body,
) -> Result<Json<SyncResponse>, ApiError> {
    let access = team_access(&state.db, auth.user_id, &team).await?;
    refuse_when_stopped(&state, access.team_id).await?;
    state.limiter.check(
        &format!("sync:user:{}", auth.user_id),
        SYNCS_PER_USER,
        SYNC_WINDOW,
    )?;
    let bytes = axum::body::to_bytes(body, MAX_BODY).await.map_err(|e| {
        if is_length_limit(&e) {
            invalid_bundle(TOO_LARGE)
        } else {
            invalid_request("Couldn't read the request body.")
        }
    })?;
    let req = tokio::task::spawn_blocking(move || check(&headers, &bytes))
        .await
        .map_err(|_| ApiError::unavailable())??;
    let now = state.clock.now();

    let mut tx = state.db.begin().await?;
    // Under a share lock on the team row, as a publish: a suspension either
    // waits for this sync or is seen here.
    let suspended: bool =
        sqlx::query_scalar("SELECT suspended_at IS NOT NULL FROM teams WHERE id = $1 FOR SHARE")
            .bind(access.team_id)
            .fetch_one(&mut *tx)
            .await?;
    if suspended {
        return Err(ApiError::new(ErrorCode::TeamSuspended, TEAM_SUSPENDED));
    }
    let locked = lock_collection(&mut tx, access.team_id, &slug)
        .await?
        .ok_or_else(ApiError::not_found)?;
    if locked.source != "cloud" {
        return Err(ApiError::new(
            ErrorCode::WrongSource,
            format!(
                "{slug} is published from files. Switch its source to cloud in the dashboard to sync it from the studio."
            ),
        ));
    }

    // Read after the lock: a snapshot taken before waiting on it would miss
    // the version a teammate's sync just committed.
    let server = current_files(&mut tx, locked.id, locked.current_version).await?;
    let hashes: HashMap<String, String> = server
        .iter()
        .map(|(p, f)| (p.clone(), f.hash.clone()))
        .collect();
    let plan = merge(&hashes, &req);

    let mut version = locked.current_version.unwrap_or(0);
    let mut pushed = Vec::new();
    if plan.changes_server() {
        if locked.access == "restricted" {
            let settings = state.settings().await?;
            let (team_plan, _) = team_plan(&mut tx, access.team_id, now, false).await?;
            if !limits(team_plan, &settings).restricted {
                return Err(ApiError::new(
                    ErrorCode::PlanLimit,
                    format!(
                        "{slug} is restricted, which needs the Team plan. Upgrade, or set its access to public or off."
                    ),
                ));
            }
        }
        let by_path: HashMap<&str, &str> = req
            .files
            .iter()
            .filter_map(|f| f.text.as_deref().map(|t| (f.path.as_str(), t)))
            .collect();
        let mut files: Vec<BundleFile> = Vec::with_capacity(plan.push.len());
        for (path, _) in &plan.push {
            let text = by_path.get(path.as_str()).ok_or_else(|| {
                ApiError::new(
                    ErrorCode::InvalidRequest,
                    format!("{path} changed but its text wasn't sent."),
                )
            })?;
            files.push(BundleFile {
                path: path.clone(),
                text: (*text).to_string(),
            });
        }

        // The new tree: the server's files, less removals, with the pushes.
        let mut tree: HashMap<&str, (&str, usize)> = server
            .iter()
            .map(|(p, f)| (p.as_str(), (f.hash.as_str(), f.len)))
            .collect();
        for path in &plan.remove {
            tree.remove(path.as_str());
        }
        for ((path, hash), file) in plan.push.iter().zip(&files) {
            tree.insert(path.as_str(), (hash.as_str(), file.text.len()));
        }
        let sizes: Vec<(&str, usize)> = tree.iter().map(|(p, (_, len))| (*p, *len)).collect();
        validate_tree(&sizes).map_err(|e| invalid_bundle(e.to_string()))?;
        let pairs: Vec<(&str, &str)> = tree.iter().map(|(p, (h, _))| (*p, *h)).collect();

        // Requests: the previous count less the files replaced or removed,
        // plus the new texts. Only the changed files' old texts are read.
        let replaced: Vec<&str> = plan
            .push
            .iter()
            .map(|(p, _)| p)
            .chain(&plan.remove)
            .filter_map(|p| server.get(p).map(|f| f.hash.as_str()))
            .collect();
        let old_texts = texts(&mut tx, replaced).await?;
        let (previous_count, inputs): (i32, serde_json::Value) = match locked.current_version {
            Some(n) => {
                sqlx::query_as(
                    "SELECT request_count, inputs FROM versions \
                     WHERE collection_id = $1 AND number = $2",
                )
                .bind(locked.id)
                .bind(n)
                .fetch_one(&mut *tx)
                .await?
            }
            None => (0, serde_json::json!([])),
        };
        let removed_requests: i32 = plan
            .push
            .iter()
            .map(|(p, _)| p)
            .chain(&plan.remove)
            .filter_map(|p| {
                let hash = &server.get(p)?.hash;
                Some(file_request_count(p, old_texts.get(hash)?))
            })
            .sum();
        let added_requests: i32 = files
            .iter()
            .map(|f| file_request_count(&f.path, &f.text))
            .sum();

        let file_hashes: Vec<String> = plan.push.iter().map(|(_, h)| h.clone()).collect();
        insert_blobs(&mut tx, &files, &file_hashes).await?;
        version += 1;
        let version_id: Uuid = sqlx::query_scalar(
            "INSERT INTO versions (collection_id, number, hash, inputs, published_by, git, \
                                   created_at, request_count) \
             VALUES ($1, $2, $3, $4, $5, NULL, $6, $7) RETURNING id",
        )
        .bind(locked.id)
        .bind(version)
        .bind(bundle_hash_from_file_hashes(&pairs))
        .bind(inputs)
        .bind(&auth.email)
        .bind(now)
        .bind((previous_count - removed_requests + added_requests).max(0))
        .fetch_one(&mut *tx)
        .await?;
        let (paths, hashes): (Vec<&str>, Vec<&str>) = pairs.into_iter().unzip();
        sqlx::query(
            "INSERT INTO version_files (version_id, path, hash) \
             SELECT $1, p, h FROM UNNEST($2::text[], $3::text[]) AS t(p, h)",
        )
        .bind(version_id)
        .bind(&paths)
        .bind(&hashes)
        .execute(&mut *tx)
        .await?;
        sqlx::query("UPDATE collections SET current_version = $2 WHERE id = $1")
            .bind(locked.id)
            .bind(version)
            .execute(&mut *tx)
            .await?;
        pushed = plan.push.iter().map(|(p, _)| p.clone()).collect();
        pushed.extend(plan.remove.iter().cloned());
        pushed.sort();
    }

    // Pulls and conflicts read the server's copy as it was before this sync:
    // neither touches a pushed or removed path.
    let wanted: Vec<&str> = plan
        .pull
        .iter()
        .chain(&plan.conflicts)
        .filter_map(|p| server.get(p).map(|f| f.hash.as_str()))
        .collect();
    let blob = texts(&mut tx, wanted).await?;
    let mut pulled = Vec::with_capacity(plan.pull.len());
    for path in &plan.pull {
        let hash = &server[path].hash;
        let text = blob.get(hash).ok_or_else(ApiError::unavailable)?;
        pulled.push(PulledFile {
            path: path.clone(),
            hash: hash.clone(),
            text: text.clone(),
        });
    }
    let mut conflicts = Vec::with_capacity(plan.conflicts.len());
    for path in &plan.conflicts {
        let server_hash = server.get(path).map(|f| f.hash.clone());
        let (server_author, server_at) =
            last_change(&mut tx, locked.id, path, server_hash.as_deref(), now).await?;
        conflicts.push(SyncConflict {
            path: path.clone(),
            server_text: server_hash.as_ref().and_then(|h| blob.get(h).cloned()),
            deleted: server_hash.is_none(),
            server_hash,
            server_author,
            server_at,
        });
    }
    tx.commit().await?;

    Ok(Json(SyncResponse {
        version: u32::try_from(version).unwrap_or_default(),
        pushed,
        pulled,
        conflicts,
        deleted: plan.delete,
    }))
}

#[derive(Deserialize)]
pub(super) struct CreateBody {
    slug: String,
    name: String,
}

/// `POST /v1/teams/{team}/collections`: a new, empty collection that syncs
/// from the studio. Its first sync pushes the files. Collections published
/// from files are made by their first publish instead.
pub(super) async fn create(
    State(state): State<AppState>,
    auth: UserAuth,
    Path(team): Path<String>,
    ApiJson(body): ApiJson<CreateBody>,
) -> Result<(StatusCode, Json<CollectionBody>), ApiError> {
    let access = team_access(&state.db, auth.user_id, &team).await?;
    refuse_when_stopped(&state, access.team_id).await?;
    if !valid_slug(&body.slug) {
        return Err(invalid_bundle(
            "The collection slug must be lowercase letters, digits and dashes.",
        ));
    }
    let name = body.name.trim();
    if name.is_empty() || name.chars().count() > MAX_NAME_CHARS {
        return Err(invalid_bundle(format!(
            "The collection name must be 1 to {MAX_NAME_CHARS} characters."
        )));
    }
    let secret = state.config.secret_key.as_bytes();
    let token = new_token("crsl_");
    let row: Option<CollectionRow> = sqlx::query_as(&format!(
        "WITH c AS (INSERT INTO collections (team_id, slug, name, source, link_hash, link_token_enc) \
                    VALUES ($1, $2, $3, 'cloud', $4, $5) \
                    ON CONFLICT (team_id, slug) DO NOTHING RETURNING *) \
         SELECT {COLLECTION_COLUMNS} FROM c {CURRENT_VERSION_JOIN}"
    ))
    .bind(access.team_id)
    .bind(&body.slug)
    .bind(name)
    .bind(secret_hash(secret, &token))
    .bind(encrypt_link_token(secret, &token))
    .fetch_optional(&state.db)
    .await?;
    let row = row.ok_or_else(|| {
        ApiError::new(
            ErrorCode::Conflict,
            format!("{} already exists in this team.", body.slug),
        )
    })?;
    Ok((StatusCode::CREATED, Json(row.into_body()?)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use curl_runner_cloud_proto::sync::{SyncDeletion, SyncFile};

    fn body(req: &SyncRequest) -> Vec<u8> {
        serde_json::to_vec(req).unwrap()
    }

    fn file(path: &str, text: &str) -> SyncFile {
        SyncFile {
            path: path.into(),
            hash: file_hash(text),
            base: None,
            text: Some(text.into()),
        }
    }

    #[test]
    fn a_valid_request_passes() {
        let req = SyncRequest {
            files: vec![file("a.yaml", "x")],
            deleted: vec![SyncDeletion {
                path: "b.yaml".into(),
                base: file_hash("y"),
            }],
        };
        assert_eq!(check(&HeaderMap::new(), &body(&req)).unwrap(), req);
    }

    #[test]
    fn a_path_listed_twice_is_refused() {
        let req = SyncRequest {
            files: vec![file("a.yaml", "x")],
            deleted: vec![SyncDeletion {
                path: "a.yaml".into(),
                base: file_hash("y"),
            }],
        };
        let err = check(&HeaderMap::new(), &body(&req)).unwrap_err();
        assert_eq!(err.code, ErrorCode::InvalidBundle);
    }

    #[test]
    fn bad_paths_hashes_and_texts_are_refused() {
        let bad_path = SyncRequest {
            files: vec![file("../a.yaml", "x")],
            deleted: vec![],
        };
        assert_eq!(
            check(&HeaderMap::new(), &body(&bad_path)).unwrap_err().code,
            ErrorCode::InvalidBundle
        );
        let mut short = file("a.yaml", "x");
        short.base = Some("abc".into());
        let bad_hash = SyncRequest {
            files: vec![short],
            deleted: vec![],
        };
        assert_eq!(
            check(&HeaderMap::new(), &body(&bad_hash)).unwrap_err().code,
            ErrorCode::InvalidRequest
        );
        let mut lying = file("a.yaml", "x");
        lying.text = Some("not x".into());
        let mismatch = SyncRequest {
            files: vec![lying],
            deleted: vec![],
        };
        assert_eq!(
            check(&HeaderMap::new(), &body(&mismatch)).unwrap_err().code,
            ErrorCode::InvalidBundle
        );
        assert_eq!(
            check(&HeaderMap::new(), b"[]").unwrap_err().code,
            ErrorCode::InvalidRequest
        );
    }

    #[test]
    fn sha256_hex_is_64_lowercase_hex_chars() {
        assert!(is_sha256_hex(&file_hash("")));
        assert!(!is_sha256_hex(&file_hash("").to_uppercase()));
        assert!(!is_sha256_hex("abc"));
    }
}
