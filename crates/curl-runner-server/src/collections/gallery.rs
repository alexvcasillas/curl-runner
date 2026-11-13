//! The public gallery (www.curl-runner.com/collections): listed collections,
//! read without an account. Listed means `listed_at` is set, access is
//! public, the link isn't taken down or expired and the team isn't
//! suspended; anything else is a 404 here.

use axum::Json;
use axum::extract::{Path, Query, State};
use chrono::{DateTime, Utc};
use curl_runner_cloud_proto::{BundleFile, Input};
use serde::{Deserialize, Serialize};
use sqlx::types::Json as Jsonb;
use uuid::Uuid;

use crate::crypto::decrypt_link_token;
use crate::error::ApiError;
use crate::extract::ClientIp;
use crate::state::AppState;

/// Gallery reads per IP a minute. The website caches its pages, so this
/// only bites a scraper.
const READS_PER_MINUTE: u32 = 240;
const PAGE: i64 = 24;
const MAX_QUERY: usize = 100;
/// Config files: not requests.
const CONFIG_FILES: [&str; 2] = ["curl-runner.yaml", "curl-runner.yml"];

/// A request as the detail page lists it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct RequestRef {
    pub method: String,
    pub name: String,
}

/// The requests in one file: `request:` is one, `requests:` each of its
/// items. A file that doesn't parse, or has neither, has none.
pub fn requests_of(text: &str) -> Vec<RequestRef> {
    let Ok(doc) = serde_yaml_ng::from_str::<serde_yaml_ng::Value>(text) else {
        return Vec::new();
    };
    let one = |r: &serde_yaml_ng::Value, ix: usize| -> RequestRef {
        let get = |k: &str| r.get(k).and_then(|v| v.as_str()).map(str::to_string);
        RequestRef {
            method: get("method").unwrap_or_else(|| "GET".into()).to_uppercase(),
            name: get("name")
                .or_else(|| get("url"))
                .unwrap_or_else(|| format!("Request {}", ix + 1)),
        }
    };
    if let Some(list) = doc.get("requests").and_then(|v| v.as_sequence()) {
        return list
            .iter()
            .enumerate()
            .filter(|(_, r)| r.is_mapping())
            .map(|(i, r)| one(r, i))
            .collect();
    }
    match doc.get("request") {
        Some(r) if r.is_mapping() => vec![one(r, 0)],
        _ => Vec::new(),
    }
}

/// How many requests a bundle holds (config files left out).
pub fn request_count(files: &[BundleFile]) -> i32 {
    files
        .iter()
        .map(|f| file_request_count(&f.path, &f.text))
        .sum()
}

/// The requests one file adds to [`request_count`]: none for the config file.
pub fn file_request_count(path: &str, text: &str) -> i32 {
    if CONFIG_FILES.contains(&path) {
        0
    } else {
        requests_of(text).len() as i32
    }
}

#[derive(Serialize)]
struct TeamRef {
    slug: String,
    name: String,
}

#[derive(Serialize)]
struct CardBody {
    team: TeamRef,
    slug: String,
    name: String,
    description: Option<String>,
    requests: i64,
    version: Option<i32>,
    updated_at: Option<DateTime<Utc>>,
    listed_at: DateTime<Utc>,
    link: String,
}

#[derive(Serialize)]
pub(super) struct ListBody {
    collections: Vec<CardBody>,
    next_cursor: Option<String>,
    /// Matching collections, and the teams they come from.
    total: i64,
    teams: i64,
}

#[derive(Deserialize)]
pub(super) struct ListQuery {
    q: Option<String>,
    team: Option<String>,
    cursor: Option<String>,
}

/// What a listed collection must be, `c` and `t` joined.
const LISTED: &str = "c.listed_at IS NOT NULL AND c.access = 'public' \
     AND c.link_disabled_at IS NULL \
     AND (c.link_expires_at IS NULL OR c.link_expires_at > $1) \
     AND t.suspended_at IS NULL";

/// `$2` the pattern (or NULL), `$3` the team (or NULL).
const MATCHES: &str = "($2::text IS NULL OR c.name ILIKE $2 OR c.description ILIKE $2 \
       OR t.name ILIKE $2 OR t.slug::text ILIKE $2) \
     AND ($3::text IS NULL OR t.slug = $3)";

#[derive(sqlx::FromRow)]
struct CardRow {
    id: Uuid,
    team_slug: String,
    team_name: String,
    slug: String,
    name: String,
    description: Option<String>,
    requests: Option<i64>,
    version: Option<i32>,
    updated_at: Option<DateTime<Utc>>,
    listed_at: DateTime<Utc>,
    link_token_enc: Option<String>,
}

const CARD_COLUMNS: &str = "c.id, t.slug::text AS team_slug, t.name AS team_name, \
     c.slug::text AS slug, c.name, c.description, \
     COALESCE(cv.request_count::bigint, \
       (SELECT count(*) FROM version_files f WHERE f.version_id = cv.id)) AS requests, \
     c.current_version AS version, cv.created_at AS updated_at, c.listed_at, c.link_token_enc";

const CARD_FROM: &str = "FROM collections c JOIN teams t ON t.id = c.team_id \
     LEFT JOIN versions cv ON cv.collection_id = c.id AND cv.number = c.current_version";

fn link(state: &AppState, enc: Option<&str>) -> Option<String> {
    let token = decrypt_link_token(state.config.secret_key.as_bytes(), enc?)?;
    Some(format!("{}/s/{token}", state.config.app_url))
}

fn card(state: &AppState, r: CardRow) -> Option<CardBody> {
    Some(CardBody {
        link: link(state, r.link_token_enc.as_deref())?,
        team: TeamRef {
            slug: r.team_slug,
            name: r.team_name,
        },
        slug: r.slug,
        name: r.name,
        description: r.description,
        requests: r.requests.unwrap_or(0),
        version: r.version,
        updated_at: r.updated_at,
        listed_at: r.listed_at,
    })
}

/// `{listed_at micros}.{id}`: the last card of a page.
fn cursor_of(r: &CardRow) -> String {
    format!("{}.{}", r.listed_at.timestamp_micros(), r.id)
}

fn parse_cursor(s: &str) -> Option<(DateTime<Utc>, Uuid)> {
    let (at, id) = s.split_once('.')?;
    Some((
        DateTime::from_timestamp_micros(at.parse().ok()?)?,
        id.parse().ok()?,
    ))
}

/// `%q%` with LIKE's wildcards escaped.
fn pattern(q: Option<&str>) -> Option<String> {
    let q = q?.trim();
    if q.is_empty() {
        return None;
    }
    let q: String = q.chars().take(MAX_QUERY).collect();
    let escaped = q
        .replace('\\', "\\\\")
        .replace('%', "\\%")
        .replace('_', "\\_");
    Some(format!("%{escaped}%"))
}

/// `GET /v1/public/collections?q=&team=&cursor=`: listed collections,
/// newest listed first, 24 a page.
pub(super) async fn list(
    State(state): State<AppState>,
    ClientIp(ip): ClientIp,
    Query(q): Query<ListQuery>,
) -> Result<Json<ListBody>, ApiError> {
    state.limiter.check(
        &format!("gallery:ip:{ip}"),
        READS_PER_MINUTE,
        chrono::Duration::minutes(1),
    )?;
    let now = state.clock.now();
    let pat = pattern(q.q.as_deref());
    let team = q.team.as_deref().map(str::to_lowercase);
    let cursor = q.cursor.as_deref().and_then(parse_cursor);
    let rows: Vec<CardRow> = sqlx::query_as(&format!(
        "SELECT {CARD_COLUMNS} {CARD_FROM} WHERE {LISTED} AND {MATCHES} \
         AND ($4::timestamptz IS NULL OR (c.listed_at, c.id) < ($4, $5)) \
         ORDER BY c.listed_at DESC, c.id DESC LIMIT {}",
        PAGE + 1
    ))
    .bind(now)
    .bind(&pat)
    .bind(&team)
    .bind(cursor.map(|c| c.0))
    .bind(cursor.map(|c| c.1))
    .fetch_all(&state.db)
    .await?;
    let (total, teams): (i64, i64) = sqlx::query_as(&format!(
        "SELECT count(*), count(DISTINCT t.id) FROM collections c JOIN teams t ON t.id = c.team_id \
         WHERE {LISTED} AND {MATCHES}"
    ))
    .bind(now)
    .bind(&pat)
    .bind(&team)
    .fetch_one(&state.db)
    .await?;
    let more = rows.len() as i64 > PAGE;
    let next_cursor = more
        .then(|| rows.get(PAGE as usize - 1).map(cursor_of))
        .flatten();
    Ok(Json(ListBody {
        collections: rows
            .into_iter()
            .take(PAGE as usize)
            .filter_map(|r| card(&state, r))
            .collect(),
        next_cursor,
        total,
        teams,
    }))
}

#[derive(Serialize)]
struct FileBody {
    path: String,
    text: String,
    requests: Vec<RequestRef>,
}

#[derive(Serialize)]
struct InputBody {
    name: String,
    description: Option<String>,
    secret: bool,
}

#[derive(Serialize)]
pub(super) struct DetailBody {
    team: TeamRef,
    slug: String,
    name: String,
    description: Option<String>,
    version: Option<i32>,
    updated_at: Option<DateTime<Utc>>,
    link: String,
    inputs: Vec<InputBody>,
    files: Vec<FileBody>,
}

/// `GET /v1/public/collections/{team}/{slug}`: a listed collection with its
/// current version's files and the inputs people fill in. 404 otherwise.
pub(super) async fn detail(
    State(state): State<AppState>,
    ClientIp(ip): ClientIp,
    Path((team, slug)): Path<(String, String)>,
) -> Result<Json<DetailBody>, ApiError> {
    state.limiter.check(
        &format!("gallery:ip:{ip}"),
        READS_PER_MINUTE,
        chrono::Duration::minutes(1),
    )?;
    let now = state.clock.now();
    let row: Option<CardRow> = sqlx::query_as(&format!(
        "SELECT {CARD_COLUMNS} {CARD_FROM} WHERE {LISTED} AND t.slug = $2 AND c.slug = $3"
    ))
    .bind(now)
    .bind(team.to_lowercase())
    .bind(slug.to_lowercase())
    .fetch_optional(&state.db)
    .await?;
    let row = row.ok_or_else(ApiError::not_found)?;
    let id = row.id;
    let card = card(&state, row).ok_or_else(ApiError::not_found)?;
    let version: Option<(Uuid, Jsonb<Vec<Input>>)> = sqlx::query_as(
        "SELECT v.id, v.inputs FROM versions v JOIN collections c ON c.id = v.collection_id \
         WHERE c.id = $1 AND v.number = c.current_version",
    )
    .bind(id)
    .fetch_optional(&state.db)
    .await?;
    let (files, inputs) = match version {
        Some((vid, inputs)) => {
            let rows: Vec<(String, String)> = sqlx::query_as(
                "SELECT f.path, b.text FROM version_files f JOIN blobs b ON b.hash = f.hash \
                 WHERE f.version_id = $1 ORDER BY f.path",
            )
            .bind(vid)
            .fetch_all(&state.db)
            .await?;
            (rows, inputs.0)
        }
        None => (Vec::new(), Vec::new()),
    };
    Ok(Json(DetailBody {
        team: card.team,
        slug: card.slug,
        name: card.name,
        description: card.description,
        version: card.version,
        updated_at: card.updated_at,
        link: card.link,
        inputs: inputs
            .into_iter()
            .map(|i| InputBody {
                name: i.name,
                description: i.description,
                secret: i.secret,
            })
            .collect(),
        files: files
            .into_iter()
            .map(|(path, text)| FileBody {
                requests: if CONFIG_FILES.contains(&path.as_str()) {
                    Vec::new()
                } else {
                    requests_of(&text)
                },
                path,
                text,
            })
            .collect(),
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn requests_from_one_or_many() {
        let one = requests_of("request:\n  name: Create\n  method: post\n  url: /x\n");
        assert_eq!(
            one,
            [RequestRef {
                method: "POST".into(),
                name: "Create".into()
            }]
        );
        let many = requests_of("requests:\n  - url: /a\n  - name: B\n    method: DELETE\n  - 3\n");
        assert_eq!(
            many,
            [
                RequestRef {
                    method: "GET".into(),
                    name: "/a".into()
                },
                RequestRef {
                    method: "DELETE".into(),
                    name: "B".into()
                }
            ]
        );
        assert!(requests_of(": [").is_empty());
        assert!(requests_of("cloud:\n  collection: x\n").is_empty());
    }

    #[test]
    fn counts_skip_the_config() {
        let f = |p: &str, t: &str| BundleFile {
            path: p.into(),
            text: t.into(),
        };
        let files = [
            f("curl-runner.yaml", "requests:\n  - url: /x\n"),
            f("a.yaml", "request:\n  url: /a\n"),
            f("b.yaml", "requests:\n  - url: /b\n  - url: /c\n"),
        ];
        assert_eq!(request_count(&files), 3);
    }

    #[test]
    fn patterns_escape_wildcards() {
        assert_eq!(pattern(Some(" 50%_off ")).as_deref(), Some("%50\\%\\_off%"));
        assert_eq!(pattern(Some("  ")), None);
        assert_eq!(pattern(None), None);
    }

    #[test]
    fn cursors_round_trip() {
        let at = DateTime::from_timestamp_micros(1_794_500_000_123_456).unwrap();
        let id = Uuid::new_v4();
        let s = format!("{}.{}", at.timestamp_micros(), id);
        assert_eq!(parse_cursor(&s), Some((at, id)));
        assert_eq!(parse_cursor("nope"), None);
    }
}
