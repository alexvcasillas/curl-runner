//! The activity log: `admin_events`, newest first, filtered and paged.

use axum::Json;
use axum::extract::State;
use axum::response::Response;
use base64::Engine;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use chrono::{DateTime, SecondsFormat, Utc};
use curl_runner_cloud_proto::ErrorCode;
use futures_util::{StreamExt, TryStreamExt, stream};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sqlx::{FromRow, PgPool};
use uuid::Uuid;

use super::auth::AdminAuth;
use super::csv;
use crate::error::ApiError;
use crate::extract::ApiQuery;
use crate::state::AppState;

const DEFAULT_LIMIT: i64 = 50;
const MAX_LIMIT: i64 = 200;
pub(super) const EXPORT_PAGE: i64 = 500;

/// The regex (Postgres `~`) on `action` for each `action_group`. Later
/// actions are named so they land in the right group.
fn group_pattern(group: &str) -> Option<&'static str> {
    match group {
        "override" => Some(r"^team\.(override|plan)\."),
        "suspension" => Some(r"^((user|team)\.(un)?suspend$|collection\.link\.)"),
        "email" => Some(r"(^broadcast\.|\.email(_owners)?$)"),
        "settings" => Some(r"^settings\."),
        "admins" => Some(r"^admin\."),
        _ => None,
    }
}

#[derive(Deserialize)]
pub struct ActivityQuery {
    admin: Option<String>,
    action_group: Option<String>,
    since: Option<String>,
    cursor: Option<String>,
    limit: Option<String>,
}

#[derive(Debug, Clone, Copy, Default)]
struct Filters {
    admin: Option<Uuid>,
    pattern: Option<&'static str>,
    since: Option<DateTime<Utc>>,
}

/// Position after the last row of a page: `(at, id)` of that row.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) struct Cursor {
    pub(super) at: DateTime<Utc>,
    pub(super) id: Uuid,
}

impl Cursor {
    pub(super) fn encode(self) -> String {
        URL_SAFE_NO_PAD.encode(format!("{}:{}", self.at.timestamp_micros(), self.id))
    }

    pub(super) fn decode(s: &str) -> Option<Self> {
        let raw = String::from_utf8(URL_SAFE_NO_PAD.decode(s).ok()?).ok()?;
        let (micros, id) = raw.split_once(':')?;
        Some(Self {
            at: DateTime::from_timestamp_micros(micros.parse().ok()?)?,
            id: id.parse().ok()?,
        })
    }
}

pub(super) fn invalid(message: &str) -> ApiError {
    ApiError::new(ErrorCode::InvalidRequest, message)
}

pub(super) fn non_empty(v: Option<&String>) -> Option<&str> {
    v.map(|s| s.trim()).filter(|s| !s.is_empty())
}

impl ActivityQuery {
    fn filters(&self) -> Result<Filters, ApiError> {
        Ok(Filters {
            admin: non_empty(self.admin.as_ref())
                .map(|a| a.parse().map_err(|_| invalid("`admin` must be a user id.")))
                .transpose()?,
            pattern: non_empty(self.action_group.as_ref())
                .map(|g| {
                    group_pattern(g).ok_or_else(|| {
                        invalid(
                            "`action_group` must be override, suspension, email, settings or admins.",
                        )
                    })
                })
                .transpose()?,
            since: non_empty(self.since.as_ref())
                .map(|s| {
                    DateTime::parse_from_rfc3339(s)
                        .map(|d| d.with_timezone(&Utc))
                        .map_err(|_| invalid("`since` must be an RFC 3339 time."))
                })
                .transpose()?,
        })
    }

    fn cursor(&self) -> Result<Option<Cursor>, ApiError> {
        non_empty(self.cursor.as_ref())
            .map(|c| Cursor::decode(c).ok_or_else(|| invalid("`cursor` is not valid.")))
            .transpose()
    }

    fn limit(&self) -> Result<i64, ApiError> {
        page_limit(self.limit.as_ref())
    }
}

/// The `limit` query value: 1 to 200, 50 when absent.
pub(super) fn page_limit(raw: Option<&String>) -> Result<i64, ApiError> {
    match non_empty(raw) {
        None => Ok(DEFAULT_LIMIT),
        Some(l) => l
            .parse()
            .ok()
            .filter(|n| (1..=MAX_LIMIT).contains(n))
            .ok_or_else(|| invalid("`limit` must be between 1 and 200.")),
    }
}

#[derive(FromRow)]
struct EventSql {
    id: Uuid,
    at: DateTime<Utc>,
    admin_user_id: Option<Uuid>,
    admin_email: String,
    admin_name: Option<String>,
    action: String,
    target_kind: Option<String>,
    target_id: Option<String>,
    target_label: Option<String>,
    detail: Value,
    ip: Option<String>,
}

#[derive(Serialize)]
pub struct AdminRef {
    user_id: Uuid,
    /// The current email, or the one at the time once the account is deleted.
    email: String,
    /// `None` once the account is deleted.
    name: Option<String>,
}

#[derive(Serialize)]
pub struct ActivityItem {
    id: Uuid,
    at: DateTime<Utc>,
    admin: Option<AdminRef>,
    action: String,
    target_kind: Option<String>,
    target_id: Option<String>,
    target_label: Option<String>,
    detail: Value,
    ip: Option<String>,
}

#[derive(Serialize)]
pub struct ActivityPage {
    items: Vec<ActivityItem>,
    next_cursor: Option<String>,
    /// Rows matching the filters, over every page.
    total: i64,
}

const WHERE: &str = "($1::uuid IS NULL OR e.admin_user_id = $1) \
     AND ($2::text IS NULL OR e.action ~ $2) \
     AND ($3::timestamptz IS NULL OR e.at >= $3)";

/// Up to `limit` rows after `cursor`, and the cursor for the next page.
async fn page(
    db: &PgPool,
    filters: Filters,
    cursor: Option<Cursor>,
    limit: i64,
) -> Result<(Vec<EventSql>, Option<Cursor>), ApiError> {
    let mut rows: Vec<EventSql> = sqlx::query_as(&format!(
        "SELECT e.id, e.at, e.admin_user_id, \
                COALESCE(u.email::text, e.admin_email) AS admin_email, u.name AS admin_name, \
                e.action, e.target_kind, e.target_id, e.target_label, e.detail, e.ip \
         FROM admin_events e LEFT JOIN users u ON u.id = e.admin_user_id \
         WHERE {WHERE} AND ($4::timestamptz IS NULL OR (e.at, e.id) < ($4, $5::uuid)) \
         ORDER BY e.at DESC, e.id DESC LIMIT $6"
    ))
    .bind(filters.admin)
    .bind(filters.pattern)
    .bind(filters.since)
    .bind(cursor.map(|c| c.at))
    .bind(cursor.map(|c| c.id))
    .bind(limit + 1)
    .fetch_all(db)
    .await?;
    let more = rows.len() as i64 > limit;
    rows.truncate(usize::try_from(limit).unwrap_or(0));
    let next = if more {
        rows.last().map(|r| Cursor { at: r.at, id: r.id })
    } else {
        None
    };
    Ok((rows, next))
}

/// `GET /v1/admin/activity?admin=&action_group=&since=&cursor=&limit=`
pub async fn list(
    _admin: AdminAuth,
    State(state): State<AppState>,
    ApiQuery(q): ApiQuery<ActivityQuery>,
) -> Result<Json<ActivityPage>, ApiError> {
    let filters = q.filters()?;
    let cursor = q.cursor()?;
    let limit = q.limit()?;
    let (rows, next) = page(&state.db, filters, cursor, limit).await?;
    let total: i64 = sqlx::query_scalar(&format!(
        "SELECT count(*) FROM admin_events e WHERE {WHERE}"
    ))
    .bind(filters.admin)
    .bind(filters.pattern)
    .bind(filters.since)
    .fetch_one(&state.db)
    .await?;
    Ok(Json(ActivityPage {
        items: rows
            .into_iter()
            .map(|r| ActivityItem {
                id: r.id,
                at: r.at,
                admin: r.admin_user_id.map(|user_id| AdminRef {
                    user_id,
                    email: r.admin_email,
                    name: r.admin_name,
                }),
                action: r.action,
                target_kind: r.target_kind,
                target_id: r.target_id,
                target_label: r.target_label,
                detail: r.detail,
                ip: r.ip,
            })
            .collect(),
        next_cursor: next.map(Cursor::encode),
        total,
    }))
}

const CSV_HEADER: [&str; 8] = [
    "at",
    "admin_email",
    "action",
    "target_kind",
    "target_id",
    "target_label",
    "detail",
    "ip",
];

fn csv_row(r: EventSql) -> Vec<String> {
    vec![
        r.at.to_rfc3339_opts(SecondsFormat::Secs, true),
        r.admin_email,
        r.action,
        r.target_kind.unwrap_or_default(),
        r.target_id.unwrap_or_default(),
        r.target_label.unwrap_or_default(),
        r.detail.to_string(),
        r.ip.unwrap_or_default(),
    ]
}

/// `GET /v1/admin/activity.csv` with the same filters, every matching row,
/// read in pages of 500 as the download goes.
pub async fn export(
    _admin: AdminAuth,
    State(state): State<AppState>,
    ApiQuery(q): ApiQuery<ActivityQuery>,
) -> Result<Response, ApiError> {
    let filters = q.filters()?;
    let db = state.db.clone();
    // State: `None` once the last page is out, else the cursor to read after.
    let pages = stream::try_unfold(Some(None), move |next: Option<Option<Cursor>>| {
        let db = db.clone();
        async move {
            let Some(cursor) = next else {
                return Ok(None);
            };
            let (rows, next) = page(&db, filters, cursor, EXPORT_PAGE).await?;
            let rows: Vec<Vec<String>> = rows.into_iter().map(csv_row).collect();
            Ok::<_, ApiError>(Some((rows, next.map(Some))))
        }
    });
    let rows = pages
        .map_ok(|rows| stream::iter(rows.into_iter().map(Ok)))
        .try_flatten()
        .boxed();
    Ok(csv::stream(
        "activity",
        state.clock.now().date_naive(),
        &CSV_HEADER,
        rows,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cursor_round_trips() {
        let c = Cursor {
            at: "2026-11-04T10:00:00.123456Z".parse().unwrap(),
            id: Uuid::new_v4(),
        };
        assert_eq!(Cursor::decode(&c.encode()), Some(c));
        assert_eq!(Cursor::decode("nope"), None);
        assert_eq!(Cursor::decode(""), None);
    }

    #[test]
    fn every_group_has_a_pattern() {
        for g in ["override", "suspension", "email", "settings", "admins"] {
            assert!(group_pattern(g).is_some(), "{g}");
        }
        assert!(group_pattern("all").is_none());
    }
}
