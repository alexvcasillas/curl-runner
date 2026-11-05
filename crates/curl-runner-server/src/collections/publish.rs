//! `PUT /v1/teams/{team}/collections/{slug}/versions`: the CI publish endpoint.
//!
//! One transaction holds a row lock on the collection from the upsert to the
//! commit, so concurrent publishes get consecutive numbers and a retry of the
//! same content is `unchanged`, never a duplicate or a half-written version.

use std::collections::HashMap;
use std::io::Read;

use axum::Json;
use axum::body::Body;
use axum::extract::{Path, State};
use axum::http::{HeaderMap, StatusCode, header};
use curl_runner_cloud_proto::{
    BundleFile, ErrorCode, FileChanges, MAX_BUNDLE_BYTES, PublishRequest, PublishResponse,
    PublishStatus, bundle_hash_from_file_hashes, file_hash, validate_bundle, validate_inputs,
};
use flate2::read::GzDecoder;
use sqlx::PgConnection;
use uuid::Uuid;

use super::versions::diff;
use crate::auth::session::column_enum;
use crate::crypto::encrypt_link_token;
use crate::error::ApiError;
use crate::hash::{new_token, secret_hash};
use crate::plans::limits;
use crate::state::AppState;
use crate::teams::valid_slug;
use crate::tokens::PublishAuth;

/// Largest request body as sent, and largest JSON after decompression. JSON
/// escaping can double the size of a bundle that is within 25 MiB, so the cap
/// is twice the bundle limit plus overhead; `validate_bundle` enforces the real
/// limit on the decoded files. This only stops a body from exhausting memory.
const MAX_JSON: usize = 2 * MAX_BUNDLE_BYTES + (1 << 20);
const MAX_BODY: usize = MAX_JSON;
const TOO_LARGE: &str = "The request body is larger than 51 MiB.";
const MAX_NAME_CHARS: usize = 100;
const PUBLISHES_PER_TEAM: u32 = 60;
const PUBLISH_WINDOW: chrono::Duration = chrono::Duration::minutes(1);
/// What a publish gets while an admin has paused publishing.
pub const PUBLISHING_PAUSED: &str =
    "Publishing is paused for maintenance. Try again in a few minutes.";
const MAX_DESCRIPTION_CHARS: usize = 500;

fn invalid_bundle(message: impl Into<String>) -> ApiError {
    ApiError::new(ErrorCode::InvalidBundle, message)
}

fn invalid_request(message: &'static str) -> ApiError {
    ApiError::new(ErrorCode::InvalidRequest, message)
}

/// True when reading the body stopped at the size cap (not a client abort).
fn is_length_limit(err: &(dyn std::error::Error + 'static)) -> bool {
    let mut cur = Some(err);
    while let Some(e) = cur {
        if e.is::<http_body_util::LengthLimitError>() {
            return true;
        }
        cur = e.source();
    }
    false
}

/// The request JSON bytes: gunzipped when `Content-Encoding: gzip`. The read
/// stops one byte past the cap, so a zip bomb never grows past it in memory.
fn decode_body(headers: &HeaderMap, body: &[u8]) -> Result<Vec<u8>, ApiError> {
    let encoding = headers
        .get(header::CONTENT_ENCODING)
        .and_then(|v| v.to_str().ok())
        .map(|v| v.trim().to_ascii_lowercase());
    let json = match encoding.as_deref() {
        None | Some("" | "identity") => body.to_vec(),
        Some("gzip") => {
            let mut out = Vec::new();
            GzDecoder::new(body)
                .take(MAX_JSON as u64 + 1)
                .read_to_end(&mut out)
                .map_err(|_| invalid_request("The request body is not valid gzip."))?;
            out
        }
        Some(_) => return Err(invalid_request("Content-Encoding must be gzip or absent.")),
    };
    if json.len() > MAX_JSON {
        return Err(invalid_bundle(TOO_LARGE));
    }
    Ok(json)
}

/// A request that passed every check that needs no database, with each file's hash.
struct Checked {
    req: PublishRequest,
    file_hashes: Vec<String>,
}

/// CPU-bound: decode, parse and validate (kept off the async workers).
fn check(headers: &HeaderMap, body: &[u8], slug: &str) -> Result<Checked, ApiError> {
    if !valid_slug(slug) {
        return Err(invalid_bundle(
            "The collection slug must be lowercase letters, digits and dashes.",
        ));
    }
    let req: PublishRequest = serde_json::from_slice(&decode_body(headers, body)?)
        .map_err(|_| invalid_request("The request body is not a valid publish request."))?;
    validate_bundle(&req.files).map_err(|e| invalid_bundle(e.to_string()))?;
    validate_inputs(&req.inputs).map_err(|e| invalid_bundle(e.to_string()))?;
    if let Some(name) = &req.name
        && (name.trim().is_empty() || name.chars().count() > MAX_NAME_CHARS)
    {
        return Err(invalid_bundle(format!(
            "The collection name must be 1 to {MAX_NAME_CHARS} characters."
        )));
    }
    if let Some(description) = &req.description
        && description.chars().count() > MAX_DESCRIPTION_CHARS
    {
        return Err(invalid_bundle(format!(
            "The description must be at most {MAX_DESCRIPTION_CHARS} characters."
        )));
    }
    // Postgres text cannot hold NUL.
    if let Some(f) = req.files.iter().find(|f| f.text.contains('\0')) {
        return Err(invalid_bundle(format!("{}: text contains a NUL", f.path)));
    }
    let file_hashes: Vec<String> = req.files.iter().map(|f| file_hash(&f.text)).collect();
    let pairs: Vec<(&str, &str)> = req
        .files
        .iter()
        .zip(&file_hashes)
        .map(|(f, h)| (f.path.as_str(), h.as_str()))
        .collect();
    if bundle_hash_from_file_hashes(&pairs) != req.hash {
        return Err(invalid_bundle("The hash does not match the files."));
    }
    Ok(Checked { req, file_hashes })
}

/// A collection row held `FOR UPDATE`.
struct Locked {
    id: Uuid,
    source: String,
    access: String,
    current_version: Option<i32>,
}

async fn lock_collection(
    conn: &mut PgConnection,
    team_id: Uuid,
    slug: &str,
) -> Result<Option<Locked>, ApiError> {
    let row: Option<(Uuid, String, String, Option<i32>)> = sqlx::query_as(
        "SELECT id, source, access, current_version FROM collections \
         WHERE team_id = $1 AND slug = $2 FOR UPDATE",
    )
    .bind(team_id)
    .bind(slug)
    .fetch_optional(conn)
    .await?;
    Ok(row.map(|(id, source, access, current_version)| Locked {
        id,
        source,
        access,
        current_version,
    }))
}

pub(super) async fn publish(
    State(state): State<AppState>,
    auth: PublishAuth,
    Path((_team, slug)): Path<(String, String)>,
    headers: HeaderMap,
    body: Body,
) -> Result<(StatusCode, Json<PublishResponse>), ApiError> {
    // First, so a paused server reads nothing and creates nothing.
    let settings = state.settings().await?;
    if settings.publishing_paused {
        return Err(ApiError::new(ErrorCode::Unavailable, PUBLISHING_PAUSED));
    }
    state.limiter.check(
        &format!("publish:team:{}", auth.team_id),
        PUBLISHES_PER_TEAM,
        PUBLISH_WINDOW,
    )?;
    let bytes = axum::body::to_bytes(body, MAX_BODY).await.map_err(|e| {
        if is_length_limit(&e) {
            invalid_bundle(TOO_LARGE)
        } else {
            invalid_request("Couldn't read the request body.")
        }
    })?;
    let checked = {
        let slug = slug.clone();
        tokio::task::spawn_blocking(move || check(&headers, &bytes, &slug))
            .await
            .map_err(|_| ApiError::unavailable())??
    };
    let Checked { req, file_hashes } = checked;

    let published_by = auth.publisher.label();
    let now = state.clock.now();

    let mut tx = state.db.begin().await?;
    let locked = match lock_collection(&mut tx, auth.team_id, &slug).await? {
        Some(locked) => locked,
        None => {
            // New collection: only now is a share link token needed. Two first
            // publishes of one slug: the loser waits for the winner's commit,
            // does nothing, and finds the row below.
            let secret = state.config.secret_key.as_bytes();
            let token = new_token("crsl_");
            sqlx::query(
                "INSERT INTO collections (team_id, slug, name, link_hash, link_token_enc) \
                 VALUES ($1, $2, $3, $4, $5) ON CONFLICT (team_id, slug) DO NOTHING",
            )
            .bind(auth.team_id)
            .bind(&slug)
            .bind(req.name.as_deref().unwrap_or(&slug))
            .bind(secret_hash(secret, &token))
            .bind(encrypt_link_token(secret, &token))
            .execute(&mut *tx)
            .await?;
            lock_collection(&mut tx, auth.team_id, &slug)
                .await?
                .ok_or_else(ApiError::unavailable)?
        }
    };

    if locked.source == "cloud" {
        return Err(ApiError::new(
            ErrorCode::WrongSource,
            format!(
                "{slug} is synced from the studio. Switch its source to git in the dashboard to publish from CI."
            ),
        ));
    }
    if locked.access == "restricted" {
        let plan: String = sqlx::query_scalar("SELECT plan FROM teams WHERE id = $1")
            .bind(auth.team_id)
            .fetch_one(&mut *tx)
            .await?;
        if !limits(column_enum(plan)?, &settings).restricted {
            return Err(ApiError::new(
                ErrorCode::PlanLimit,
                format!(
                    "{slug} is restricted, which needs the Team plan. Upgrade, or set its access to public or off."
                ),
            ));
        }
    }

    // A separate statement, run after the lock: after waiting on the lock, a
    // joined read would still see the pre-wait snapshot and miss a version the
    // other publisher just committed.
    let previous: Option<(Uuid, String)> = match locked.current_version {
        Some(n) => {
            sqlx::query_as("SELECT id, hash FROM versions WHERE collection_id = $1 AND number = $2")
                .bind(locked.id)
                .bind(n)
                .fetch_optional(&mut *tx)
                .await?
        }
        None => None,
    };
    if let (Some(n), Some((_, hash))) = (locked.current_version, &previous)
        && *hash == req.hash
    {
        tx.commit().await?;
        return Ok((
            StatusCode::OK,
            Json(PublishResponse {
                collection: slug,
                version: u32::try_from(n).unwrap_or_default(),
                status: PublishStatus::Unchanged,
                changed: FileChanges::default(),
            }),
        ));
    }

    let changed = match &previous {
        Some((version_id, _)) => {
            let old = file_hashes_of(&mut tx, *version_id).await?;
            let new = req
                .files
                .iter()
                .zip(&file_hashes)
                .map(|(f, h)| (f.path.clone(), h.clone()))
                .collect();
            diff(&old, &new)
        }
        None => FileChanges {
            added: sorted_paths(&req.files),
            ..FileChanges::default()
        },
    };

    insert_blobs(&mut tx, &req.files, &file_hashes).await?;
    let number = locked.current_version.unwrap_or(0) + 1;
    let inputs = serde_json::to_value(&req.inputs).map_err(|_| ApiError::unavailable())?;
    let git = req
        .git
        .as_ref()
        .map(serde_json::to_value)
        .transpose()
        .map_err(|_| ApiError::unavailable())?;
    let version_id: Uuid = sqlx::query_scalar(
        "INSERT INTO versions (collection_id, number, hash, inputs, published_by, git, created_at) \
         VALUES ($1, $2, $3, $4, $5, $6, $7) RETURNING id",
    )
    .bind(locked.id)
    .bind(number)
    .bind(&req.hash)
    .bind(inputs)
    .bind(&published_by)
    .bind(git)
    .bind(now)
    .fetch_one(&mut *tx)
    .await?;
    let paths: Vec<&str> = req.files.iter().map(|f| f.path.as_str()).collect();
    sqlx::query(
        "INSERT INTO version_files (version_id, path, hash) \
         SELECT $1, p, h FROM UNNEST($2::text[], $3::text[]) AS t(p, h)",
    )
    .bind(version_id)
    .bind(&paths)
    .bind(&file_hashes)
    .execute(&mut *tx)
    .await?;
    sqlx::query(
        "UPDATE collections SET current_version = $2, \
         name = COALESCE($3, name), description = COALESCE($4, description) WHERE id = $1",
    )
    .bind(locked.id)
    .bind(number)
    .bind(req.name.as_deref())
    .bind(req.description.as_deref())
    .execute(&mut *tx)
    .await?;
    tx.commit().await?;

    Ok((
        StatusCode::CREATED,
        Json(PublishResponse {
            collection: slug,
            version: u32::try_from(number).unwrap_or_default(),
            status: PublishStatus::Created,
            changed,
        }),
    ))
}

fn sorted_paths(files: &[BundleFile]) -> Vec<String> {
    let mut paths: Vec<String> = files.iter().map(|f| f.path.clone()).collect();
    paths.sort();
    paths
}

async fn file_hashes_of(
    conn: &mut PgConnection,
    version_id: Uuid,
) -> Result<HashMap<String, String>, ApiError> {
    let rows: Vec<(String, String)> =
        sqlx::query_as("SELECT path, hash FROM version_files WHERE version_id = $1")
            .bind(version_id)
            .fetch_all(conn)
            .await?;
    Ok(rows.into_iter().collect())
}

/// Insert the files' texts under their hashes, skipping ones already stored.
/// Sorted and deduplicated, so two overlapping publishes lock blob rows in the
/// same order and cannot deadlock.
async fn insert_blobs(
    conn: &mut PgConnection,
    files: &[BundleFile],
    hashes: &[String],
) -> Result<(), ApiError> {
    let mut unique: Vec<(&str, &str)> = hashes
        .iter()
        .zip(files)
        .map(|(h, f)| (h.as_str(), f.text.as_str()))
        .collect();
    unique.sort_unstable_by_key(|(h, _)| *h);
    unique.dedup_by_key(|(h, _)| *h);
    let (hs, texts): (Vec<&str>, Vec<&str>) = unique.into_iter().unzip();
    sqlx::query(
        "INSERT INTO blobs (hash, text) SELECT * FROM UNNEST($1::text[], $2::text[]) \
         ON CONFLICT (hash) DO NOTHING",
    )
    .bind(&hs)
    .bind(&texts)
    .execute(conn)
    .await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use flate2::Compression;
    use flate2::write::GzEncoder;
    use std::io::Write;

    fn gz(data: &[u8]) -> Vec<u8> {
        let mut e = GzEncoder::new(Vec::new(), Compression::fast());
        e.write_all(data).unwrap();
        e.finish().unwrap()
    }

    fn gzip_headers() -> HeaderMap {
        let mut h = HeaderMap::new();
        h.insert(header::CONTENT_ENCODING, "gzip".parse().unwrap());
        h
    }

    #[test]
    fn plain_and_gzip_bodies_decode() {
        assert_eq!(decode_body(&HeaderMap::new(), b"{}").unwrap(), b"{}");
        assert_eq!(decode_body(&gzip_headers(), &gz(b"{}")).unwrap(), b"{}");
    }

    #[test]
    fn zip_bomb_is_rejected_as_invalid_bundle() {
        let bomb = gz(&vec![b' '; MAX_JSON + 10]);
        assert!(bomb.len() < MAX_JSON / 10);
        let err = decode_body(&gzip_headers(), &bomb).unwrap_err();
        assert_eq!(err.code, ErrorCode::InvalidBundle);
    }

    #[test]
    fn bad_gzip_and_unknown_encoding_are_invalid_request() {
        let err = decode_body(&gzip_headers(), b"not gzip").unwrap_err();
        assert_eq!(err.code, ErrorCode::InvalidRequest);
        let mut h = HeaderMap::new();
        h.insert(header::CONTENT_ENCODING, "br".parse().unwrap());
        assert_eq!(
            decode_body(&h, b"{}").unwrap_err().code,
            ErrorCode::InvalidRequest
        );
    }

    #[tokio::test]
    async fn only_the_size_cap_counts_as_length_limit() {
        let over = axum::body::to_bytes(Body::from(vec![0u8; 10]), 5)
            .await
            .unwrap_err();
        assert!(is_length_limit(&over));
        assert!(!is_length_limit(&std::io::Error::other("connection reset")));
    }
}
