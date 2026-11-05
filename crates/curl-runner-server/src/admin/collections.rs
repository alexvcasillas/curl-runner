//! Collections across every team: list, search, CSV, totals, and the share
//! link takedown. Any admin reads; disabling or enabling a link needs
//! `teams` and logs one event in its own transaction.
//!
//! A disabled link answers 404 to everyone and the team cannot turn it back
//! on (the one condition lives in `collections::share_blocked`). The
//! collection, its versions and its access list are untouched.

use axum::Json;
use axum::extract::State;
use axum::response::Response;
use chrono::{DateTime, Duration, SecondsFormat, Utc};
use curl_runner_cloud_proto::{Access, ErrorCode, Source};
use futures_util::{StreamExt, TryStreamExt, stream};
use serde::{Deserialize, Serialize};
use serde_json::json;
use sqlx::{FromRow, PgPool};
use uuid::Uuid;

use super::activity::{Cursor, EXPORT_PAGE, invalid, non_empty, page_limit};
use super::auth::{AdminAuth, Perm};
use super::csv;
use super::events::{AdminTx, Event};
use super::users::{clean_reason, like_pattern};
use crate::auth::session::column_enum;
use crate::billing::sync::mail_owners;
use crate::error::ApiError;
use crate::extract::{ApiJson, ApiPath, ApiQuery};
use crate::mail::Mail;
use crate::state::AppState;

const MAX_QUERY: usize = 200;
/// The window of `opens_30d`.
const OPEN_DAYS: i64 = 30;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Filter {
    All,
    Public,
    Restricted,
    Disabled,
}

impl Filter {
    fn parse(s: &str) -> Option<Self> {
        Some(match s {
            "all" => Self::All,
            "public" => Self::Public,
            "restricted" => Self::Restricted,
            "disabled" => Self::Disabled,
            _ => return None,
        })
    }

    fn as_str(self) -> &'static str {
        match self {
            Self::All => "all",
            Self::Public => "public",
            Self::Restricted => "restricted",
            Self::Disabled => "disabled",
        }
    }
}

#[derive(Deserialize)]
pub struct ListQuery {
    q: Option<String>,
    filter: Option<String>,
    cursor: Option<String>,
    limit: Option<String>,
}

#[derive(Debug, Clone)]
struct Filters {
    /// `ILIKE` pattern for `q`, already escaped.
    pattern: Option<String>,
    filter: Filter,
}

impl ListQuery {
    fn filters(&self) -> Result<Filters, ApiError> {
        let pattern = match non_empty(self.q.as_ref()) {
            Some(q) if q.chars().count() > MAX_QUERY => return Err(invalid("`q` is too long.")),
            Some(q) => Some(like_pattern(q)),
            None => None,
        };
        let filter = match non_empty(self.filter.as_ref()) {
            None => Filter::All,
            Some(f) => Filter::parse(f)
                .ok_or_else(|| invalid("`filter` must be all, public, restricted or disabled."))?,
        };
        Ok(Filters { pattern, filter })
    }

    fn cursor(&self) -> Result<Option<Cursor>, ApiError> {
        non_empty(self.cursor.as_ref())
            .map(|c| Cursor::decode(c).ok_or_else(|| invalid("`cursor` is not valid.")))
            .transpose()
    }
}

/// `$1` the pattern, `$2` the filter. `q` matches the collection's name or
/// slug, or its team's name or slug.
const WHERE_SQL: &str = "($1::text IS NULL OR c.name ILIKE $1 OR c.slug::text ILIKE $1 \
       OR t.name ILIKE $1 OR t.slug::text ILIKE $1) \
     AND (($2 = 'all') \
       OR ($2 = 'public' AND c.access = 'public') \
       OR ($2 = 'restricted' AND c.access = 'restricted') \
       OR ($2 = 'disabled' AND c.link_disabled_at IS NOT NULL))";

/// A collection with no version sorts last.
const SORT_AT: &str = "COALESCE(v.last_publish, 'epoch'::timestamptz)";

#[derive(FromRow)]
struct CollectionSql {
    id: Uuid,
    team: String,
    slug: String,
    name: String,
    source: String,
    access: String,
    link_disabled: bool,
    versions: i64,
    size_bytes: i64,
    opens_30d: i64,
    last_publish: Option<DateTime<Utc>>,
    sort_at: DateTime<Utc>,
}

#[derive(Serialize)]
pub struct CollectionRow {
    team: String,
    slug: String,
    name: String,
    source: Source,
    access: Access,
    versions: i64,
    /// The current version's files, in bytes.
    size_bytes: i64,
    /// `share_opens` rows in the last 30 days. Only signed-in opens are
    /// recorded, at most one per person a day.
    opens_30d: i64,
    last_publish: Option<DateTime<Utc>>,
    link_disabled: bool,
}

impl CollectionSql {
    fn into_row(self) -> Result<(CollectionRow, Cursor), ApiError> {
        let cursor = Cursor {
            at: self.sort_at,
            id: self.id,
        };
        Ok((
            CollectionRow {
                team: self.team,
                slug: self.slug,
                name: self.name,
                source: column_enum(self.source)?,
                access: column_enum(self.access)?,
                versions: self.versions,
                size_bytes: self.size_bytes,
                opens_30d: self.opens_30d,
                last_publish: self.last_publish,
                link_disabled: self.link_disabled,
            },
            cursor,
        ))
    }
}

/// Up to `limit` collections after `cursor` (most recently published first),
/// in one query: the version count, the current version's size and the
/// recent opens come from lateral subqueries, never a query per row.
async fn page(
    db: &PgPool,
    f: &Filters,
    now: DateTime<Utc>,
    cursor: Option<Cursor>,
    limit: i64,
) -> Result<(Vec<CollectionRow>, Option<Cursor>), ApiError> {
    let rows: Vec<CollectionSql> = sqlx::query_as(&format!(
        "SELECT c.id, t.slug::text AS team, c.slug::text AS slug, c.name, c.source, c.access, \
                c.link_disabled_at IS NOT NULL AS link_disabled, \
                v.versions, COALESCE(s.bytes, 0) AS size_bytes, o.opens AS opens_30d, \
                v.last_publish, {SORT_AT} AS sort_at \
         FROM collections c \
         JOIN teams t ON t.id = c.team_id \
         LEFT JOIN LATERAL (SELECT count(*) AS versions, max(created_at) AS last_publish \
                            FROM versions WHERE collection_id = c.id) v ON true \
         LEFT JOIN LATERAL (SELECT sum(octet_length(b.text))::bigint AS bytes \
                            FROM versions cv \
                            JOIN version_files vf ON vf.version_id = cv.id \
                            JOIN blobs b ON b.hash = vf.hash \
                            WHERE cv.collection_id = c.id AND cv.number = c.current_version) s ON true \
         LEFT JOIN LATERAL (SELECT count(*) AS opens FROM share_opens so \
                            WHERE so.collection_id = c.id AND so.opened_at > $3) o ON true \
         WHERE {WHERE_SQL} AND ($4::timestamptz IS NULL OR ({SORT_AT}, c.id) < ($4, $5::uuid)) \
         ORDER BY {SORT_AT} DESC, c.id DESC LIMIT $6"
    ))
    .bind(&f.pattern)
    .bind(f.filter.as_str())
    .bind(now - Duration::days(OPEN_DAYS))
    .bind(cursor.map(|c| c.at))
    .bind(cursor.map(|c| c.id))
    .bind(limit + 1)
    .fetch_all(db)
    .await?;
    let more = rows.len() as i64 > limit;
    let mut out = Vec::with_capacity(rows.len());
    let mut last = None;
    for row in rows.into_iter().take(usize::try_from(limit).unwrap_or(0)) {
        let (row, cursor) = row.into_row()?;
        out.push(row);
        last = Some(cursor);
    }
    Ok((out, if more { last } else { None }))
}

#[derive(Serialize)]
pub struct CollectionsPage {
    items: Vec<CollectionRow>,
    next_cursor: Option<String>,
    /// Collections matching the filters, over every page.
    total: i64,
}

/// `GET /v1/admin/collections?q=&filter=all|public|restricted|disabled&cursor=&limit=`.
pub async fn list(
    _admin: AdminAuth,
    State(state): State<AppState>,
    ApiQuery(q): ApiQuery<ListQuery>,
) -> Result<Json<CollectionsPage>, ApiError> {
    let filters = q.filters()?;
    let cursor = q.cursor()?;
    let limit = page_limit(q.limit.as_ref())?;
    let now = state.clock.now();
    let (items, next) = page(&state.db, &filters, now, cursor, limit).await?;
    let total: i64 = sqlx::query_scalar(&format!(
        "SELECT count(*) FROM collections c JOIN teams t ON t.id = c.team_id WHERE {WHERE_SQL}"
    ))
    .bind(&filters.pattern)
    .bind(filters.filter.as_str())
    .fetch_one(&state.db)
    .await?;
    Ok(Json(CollectionsPage {
        items,
        next_cursor: next.map(Cursor::encode),
        total,
    }))
}

const CSV_HEADER: [&str; 10] = [
    "team",
    "slug",
    "name",
    "source",
    "access",
    "versions",
    "size_bytes",
    "opens_30d",
    "last_publish",
    "link_disabled",
];

fn csv_row(r: CollectionRow) -> Vec<String> {
    vec![
        r.team,
        r.slug,
        r.name,
        crate::teams::enum_str(r.source),
        crate::teams::enum_str(r.access),
        r.versions.to_string(),
        r.size_bytes.to_string(),
        r.opens_30d.to_string(),
        r.last_publish
            .map(|d| d.to_rfc3339_opts(SecondsFormat::Secs, true))
            .unwrap_or_default(),
        r.link_disabled.to_string(),
    ]
}

/// `GET /v1/admin/collections.csv` with the same `q` and `filter`, every
/// matching collection, read in pages of 500 as the download goes.
pub async fn export(
    _admin: AdminAuth,
    State(state): State<AppState>,
    ApiQuery(q): ApiQuery<ListQuery>,
) -> Result<Response, ApiError> {
    let filters = q.filters()?;
    let now = state.clock.now();
    let db = state.db.clone();
    // State: `None` once the last page is out, else the cursor to read after.
    let pages = stream::try_unfold(Some(None), move |next: Option<Option<Cursor>>| {
        let db = db.clone();
        let filters = filters.clone();
        async move {
            let Some(cursor) = next else {
                return Ok(None);
            };
            let (rows, next) = page(&db, &filters, now, cursor, EXPORT_PAGE).await?;
            let rows: Vec<Vec<String>> = rows.into_iter().map(csv_row).collect();
            Ok::<_, ApiError>(Some((rows, next.map(Some))))
        }
    });
    let rows = pages
        .map_ok(|rows| stream::iter(rows.into_iter().map(Ok)))
        .try_flatten()
        .boxed();
    Ok(csv::stream(
        "collections",
        now.date_naive(),
        &CSV_HEADER,
        rows,
    ))
}

#[derive(Serialize, FromRow)]
pub struct Stats {
    /// Collections on the server.
    count: i64,
    /// File bytes summed over every version of every collection.
    bytes_raw: i64,
    /// File bytes of the distinct contents. Files are stored once per hash in
    /// `blobs` (0001), so identical files across versions and collections
    /// count once here.
    bytes_stored: i64,
}

/// `GET /v1/admin/collections/stats`: the list footer's totals, over every
/// collection whatever the list's filters.
pub async fn stats(
    _admin: AdminAuth,
    State(state): State<AppState>,
) -> Result<Json<Stats>, ApiError> {
    let stats = sqlx::query_as(
        "SELECT (SELECT count(*) FROM collections) AS count, \
                (SELECT COALESCE(sum(octet_length(b.text)), 0)::bigint \
                 FROM version_files f JOIN blobs b ON b.hash = f.hash) AS bytes_raw, \
                (SELECT COALESCE(sum(octet_length(b.text)), 0)::bigint FROM blobs b \
                 WHERE EXISTS (SELECT 1 FROM version_files f WHERE f.hash = b.hash)) AS bytes_stored",
    )
    .fetch_one(&state.db)
    .await?;
    Ok(Json(stats))
}

#[derive(Deserialize)]
pub struct DisableReq {
    /// Shown to the team's owners.
    reason: Option<String>,
}

#[derive(Serialize)]
pub struct LinkBody {
    team: String,
    slug: String,
    link_disabled: bool,
}

#[derive(FromRow)]
struct Locked {
    id: Uuid,
    name: String,
    team_id: Uuid,
    team_name: String,
    disabled: bool,
}

async fn lock_collection(
    conn: &mut sqlx::PgConnection,
    team: &str,
    slug: &str,
) -> Result<Locked, ApiError> {
    sqlx::query_as(
        "SELECT c.id, c.name, t.id AS team_id, t.name AS team_name, \
                c.link_disabled_at IS NOT NULL AS disabled \
         FROM collections c JOIN teams t ON t.id = c.team_id \
         WHERE t.slug = $1 AND c.slug = $2 FOR UPDATE OF c",
    )
    .bind(team)
    .bind(slug)
    .fetch_optional(conn)
    .await?
    .ok_or_else(ApiError::not_found)
}

fn conflict(message: &str) -> ApiError {
    ApiError::new(ErrorCode::Conflict, message)
}

/// `POST /v1/admin/collections/{team}/{slug}/disable-link`, perm `teams`.
/// `409` if already disabled. Emails the team's owners.
pub async fn disable_link(
    admin: AdminAuth,
    State(state): State<AppState>,
    ApiPath((team, slug)): ApiPath<(String, String)>,
    ApiJson(req): ApiJson<DisableReq>,
) -> Result<Json<LinkBody>, ApiError> {
    admin.require(Perm::Teams)?;
    let reason = clean_reason(req.reason)?;
    let mut tx = AdminTx::begin(&state, &admin).await?;
    let c = lock_collection(&mut tx, &team, &slug).await?;
    if c.disabled {
        return Err(conflict("That share link is already disabled."));
    }
    sqlx::query(
        "UPDATE collections SET link_disabled_at = $2, link_disabled_by = $3, \
                link_disabled_reason = $4 WHERE id = $1",
    )
    .bind(c.id)
    .bind(tx.at())
    .bind(admin.user_id)
    .bind(&reason)
    .execute(&mut *tx)
    .await?;
    mail_owners(&state, &mut tx, c.team_id, |to| Mail::LinkDisabled {
        to,
        team: c.team_name.clone(),
        collection: c.name.clone(),
        reason: reason.clone(),
    })
    .await?;
    tx.commit(
        Event::new("collection.link.disable")
            .target("collection", c.id, format!("{team}/{slug}"))
            .detail(json!({"reason": reason})),
    )
    .await?;
    Ok(Json(LinkBody {
        team,
        slug,
        link_disabled: true,
    }))
}

/// `POST /v1/admin/collections/{team}/{slug}/enable-link`, perm `teams`.
/// `409` if not disabled. The team's link token is unchanged, so the old
/// link works again.
pub async fn enable_link(
    admin: AdminAuth,
    State(state): State<AppState>,
    ApiPath((team, slug)): ApiPath<(String, String)>,
) -> Result<Json<LinkBody>, ApiError> {
    admin.require(Perm::Teams)?;
    let mut tx = AdminTx::begin(&state, &admin).await?;
    let c = lock_collection(&mut tx, &team, &slug).await?;
    if !c.disabled {
        return Err(conflict("That share link is not disabled."));
    }
    sqlx::query(
        "UPDATE collections SET link_disabled_at = NULL, link_disabled_by = NULL, \
                link_disabled_reason = NULL WHERE id = $1",
    )
    .bind(c.id)
    .execute(&mut *tx)
    .await?;
    tx.commit(Event::new("collection.link.enable").target(
        "collection",
        c.id,
        format!("{team}/{slug}"),
    ))
    .await?;
    Ok(Json(LinkBody {
        team,
        slug,
        link_disabled: false,
    }))
}
