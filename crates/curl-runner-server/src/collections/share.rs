//! The team side of sharing: link, access mode, expiry and the access list.

use axum::Json;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use chrono::{DateTime, Duration, Utc};
use curl_runner_cloud_proto::{Access, ErrorCode, Plan};
use serde::{Deserialize, Deserializer, Serialize};
use sqlx::{FromRow, PgConnection};
use uuid::Uuid;

use crate::auth::UserAuth;
use crate::auth::session::column_enum;
use crate::crypto::{decrypt_link_token, encrypt_link_token};
use crate::error::ApiError;
use crate::extract::ApiJson;
use crate::hash::{new_token, secret_hash};
use crate::mail::Mail;
use crate::plans::limits;
use crate::state::AppState;
use crate::teams::{lock_as, team_access};

/// Longest link lifetime a team can ask for (ten years).
const MAX_EXPIRY_DAYS: u32 = 3650;

#[derive(Serialize)]
pub(super) struct EntryBody {
    entry: String,
    kind: String,
    invited_at: DateTime<Utc>,
    /// Someone matching this entry has opened the collection.
    opened: bool,
}

#[derive(Serialize)]
pub(super) struct ShareBody {
    access: Access,
    link: String,
    expires_at: Option<DateTime<Utc>>,
    entries: Vec<EntryBody>,
}

#[derive(FromRow)]
struct ShareRow {
    id: Uuid,
    access: String,
    link_token_enc: Option<String>,
    link_expires_at: Option<DateTime<Utc>>,
}

async fn share_body(state: &AppState, row: ShareRow) -> Result<ShareBody, ApiError> {
    let token = row
        .link_token_enc
        .as_deref()
        .and_then(|enc| decrypt_link_token(state.config.secret_key.as_bytes(), enc))
        .ok_or_else(|| {
            tracing::error!("collection {} has no readable share link token", row.id);
            ApiError::unavailable()
        })?;
    let entries: Vec<(String, String, DateTime<Utc>, bool)> = sqlx::query_as(
        "SELECT e.entry::text, e.kind, e.invited_at, \
                EXISTS (SELECT 1 FROM share_opens o \
                        WHERE o.collection_id = e.collection_id AND o.entry = e.entry) \
         FROM access_entries e WHERE e.collection_id = $1 ORDER BY e.kind, e.entry",
    )
    .bind(row.id)
    .fetch_all(&state.db)
    .await?;
    Ok(ShareBody {
        access: column_enum(row.access)?,
        link: format!("{}/s/{token}", state.config.app_url),
        expires_at: row.link_expires_at,
        entries: entries
            .into_iter()
            .map(|(entry, kind, invited_at, opened)| EntryBody {
                entry,
                kind,
                invited_at,
                opened,
            })
            .collect(),
    })
}

const SHARE_COLUMNS: &str = "id, access, link_token_enc, link_expires_at";

pub(super) async fn get(
    State(state): State<AppState>,
    auth: UserAuth,
    Path((team, slug)): Path<(String, String)>,
) -> Result<Json<ShareBody>, ApiError> {
    let access = team_access(&state.db, auth.user_id, &team).await?;
    let row: Option<ShareRow> = sqlx::query_as(&format!(
        "SELECT {SHARE_COLUMNS} FROM collections WHERE team_id = $1 AND slug = $2"
    ))
    .bind(access.team_id)
    .bind(&slug)
    .fetch_optional(&state.db)
    .await?;
    Ok(Json(
        share_body(&state, row.ok_or_else(ApiError::not_found)?).await?,
    ))
}

/// `null` clears a field, absent leaves it alone.
fn nullable<'de, D, T>(de: D) -> Result<Option<Option<T>>, D::Error>
where
    D: Deserializer<'de>,
    T: Deserialize<'de>,
{
    Option::<T>::deserialize(de).map(Some)
}

#[derive(Deserialize)]
pub(super) struct PatchShare {
    access: Option<Access>,
    #[serde(default, deserialize_with = "nullable")]
    expires_in_days: Option<Option<u32>>,
    #[serde(default)]
    reset_link: bool,
}

fn restricted_needs_team(slug: &str) -> ApiError {
    ApiError::new(
        ErrorCode::PlanLimit,
        format!("Restricted sharing needs the Team plan. Upgrade, or keep {slug} public or off."),
    )
}

fn access_str(access: Access) -> &'static str {
    match access {
        Access::Off => "off",
        Access::Public => "public",
        Access::Restricted => "restricted",
    }
}

pub(super) async fn patch(
    State(state): State<AppState>,
    auth: UserAuth,
    Path((team, slug)): Path<(String, String)>,
    ApiJson(req): ApiJson<PatchShare>,
) -> Result<Json<ShareBody>, ApiError> {
    let expires_in = match req.expires_in_days {
        Some(Some(days)) if days == 0 || days > MAX_EXPIRY_DAYS => {
            return Err(ApiError::new(
                ErrorCode::InvalidRequest,
                "expires_in_days must be between 1 and 3650, or null for no expiry.",
            ));
        }
        other => other,
    };
    let now = state.clock.now();
    let found = team_access(&state.db, auth.user_id, &team).await?;
    let mut tx = state.db.begin().await?;
    let (plan, _) = lock_as(&mut tx, found.team_id, auth.user_id).await?;
    let row: Option<(Uuid, String)> = sqlx::query_as(
        "SELECT id, access FROM collections WHERE team_id = $1 AND slug = $2 FOR UPDATE",
    )
    .bind(found.team_id)
    .bind(&slug)
    .fetch_optional(&mut *tx)
    .await?;
    let (id, current) = row.ok_or_else(ApiError::not_found)?;
    // Re-sending the access it already has is not a change, so no plan check.
    if let Some(access) = req.access
        && access == Access::Restricted
        && current != access_str(access)
        && !limits(plan).restricted
    {
        return Err(restricted_needs_team(&slug));
    }

    if let Some(access) = req.access {
        sqlx::query("UPDATE collections SET access = $2 WHERE id = $1")
            .bind(id)
            .bind(access_str(access))
            .execute(&mut *tx)
            .await?;
    }
    if let Some(days) = expires_in {
        let at = days.map(|d| now + Duration::days(i64::from(d)));
        sqlx::query("UPDATE collections SET link_expires_at = $2 WHERE id = $1")
            .bind(id)
            .bind(at)
            .execute(&mut *tx)
            .await?;
    }
    if req.reset_link {
        let secret = state.config.secret_key.as_bytes();
        let token = new_token("crsl_");
        sqlx::query("UPDATE collections SET link_hash = $2, link_token_enc = $3 WHERE id = $1")
            .bind(id)
            .bind(secret_hash(secret, &token))
            .bind(encrypt_link_token(secret, &token))
            .execute(&mut *tx)
            .await?;
    }
    let row: ShareRow = sqlx::query_as(&format!(
        "SELECT {SHARE_COLUMNS} FROM collections WHERE id = $1"
    ))
    .bind(id)
    .fetch_one(&mut *tx)
    .await?;
    tx.commit().await?;
    Ok(Json(share_body(&state, row).await?))
}

/// `raw` as a lower-cased email address under the same rules as an email
/// entry, or `None`. Also used for member invites.
pub(crate) fn parse_email(raw: &str) -> Option<String> {
    match parse_entry(raw) {
        Ok((email, "email")) => Some(email),
        _ => None,
    }
}

/// Lower-case `raw` and return it with its kind: an email (one `@`, both sides
/// non-empty) or a domain entry `*@domain.tld` (the domain has a dot, no
/// empty labels).
pub(super) fn parse_entry(raw: &str) -> Result<(String, &'static str), ApiError> {
    let bad = || {
        ApiError::new(
            ErrorCode::InvalidRequest,
            "An entry is an email like ada@example.com or a domain like *@example.com.",
        )
    };
    let entry = raw.trim().to_lowercase();
    if entry.chars().any(|c| c.is_whitespace() || c.is_control()) {
        return Err(bad());
    }
    let mut parts = entry.split('@');
    let (Some(local), Some(domain), None) = (parts.next(), parts.next(), parts.next()) else {
        return Err(bad());
    };
    if domain.is_empty() {
        return Err(bad());
    }
    if local == "*" {
        let ok = domain.contains('.') && domain.split('.').all(|label| !label.is_empty());
        return if ok {
            Ok((entry, "domain"))
        } else {
            Err(bad())
        };
    }
    if local.is_empty() || local.contains('*') {
        return Err(bad());
    }
    Ok((entry, "email"))
}

#[derive(Deserialize)]
pub(super) struct AddEntry {
    entry: String,
    #[serde(default)]
    notify: bool,
}

struct Locked {
    id: Uuid,
    access: String,
    name: String,
    plan: Plan,
}

/// Lock the team and the collection for an access-list change.
async fn lock_collection(
    conn: &mut PgConnection,
    team_id: Uuid,
    user_id: Uuid,
    slug: &str,
) -> Result<Locked, ApiError> {
    let (plan, _) = lock_as(&mut *conn, team_id, user_id).await?;
    let row: Option<(Uuid, String, String)> = sqlx::query_as(
        "SELECT id, access, name FROM collections WHERE team_id = $1 AND slug = $2 FOR UPDATE",
    )
    .bind(team_id)
    .bind(slug)
    .fetch_optional(conn)
    .await?;
    let (id, access, name) = row.ok_or_else(ApiError::not_found)?;
    Ok(Locked {
        id,
        access,
        name,
        plan,
    })
}

pub(super) async fn add_entry(
    State(state): State<AppState>,
    auth: UserAuth,
    Path((team, slug)): Path<(String, String)>,
    ApiJson(req): ApiJson<AddEntry>,
) -> Result<(StatusCode, Json<EntryBody>), ApiError> {
    let (entry, kind) = parse_entry(&req.entry)?;
    let now = state.clock.now();
    let found = team_access(&state.db, auth.user_id, &team).await?;
    let mut tx = state.db.begin().await?;
    let Locked {
        id,
        access,
        name,
        plan,
    } = lock_collection(&mut tx, found.team_id, auth.user_id, &slug).await?;
    if access != "restricted" {
        return Err(ApiError::new(
            ErrorCode::Conflict,
            format!("{slug} isn't restricted. Set its access to restricted before adding people."),
        ));
    }
    if !limits(plan).restricted {
        return Err(restricted_needs_team(&slug));
    }
    let inserted: Option<DateTime<Utc>> = sqlx::query_scalar(
        "INSERT INTO access_entries (collection_id, entry, kind, invited_at) \
         VALUES ($1, $2, $3, $4) ON CONFLICT (collection_id, entry) DO NOTHING \
         RETURNING invited_at",
    )
    .bind(id)
    .bind(&entry)
    .bind(kind)
    .bind(now)
    .fetch_optional(&mut *tx)
    .await?;
    let invited_at = match inserted {
        Some(at) => at,
        None => {
            sqlx::query_scalar(
                "SELECT invited_at FROM access_entries WHERE collection_id = $1 AND entry = $2",
            )
            .bind(id)
            .bind(&entry)
            .fetch_one(&mut *tx)
            .await?
        }
    };
    if inserted.is_some() && req.notify && kind == "email" {
        let (team_name, enc): (String, Option<String>) = sqlx::query_as(
            "SELECT t.name, c.link_token_enc FROM collections c \
             JOIN teams t ON t.id = c.team_id WHERE c.id = $1",
        )
        .bind(id)
        .fetch_one(&mut *tx)
        .await?;
        let token = enc
            .as_deref()
            .and_then(|enc| decrypt_link_token(state.config.secret_key.as_bytes(), enc))
            .ok_or_else(|| {
                tracing::error!("collection {id} has no readable share link token");
                ApiError::unavailable()
            })?;
        state
            .mailer
            .enqueue(
                &mut tx,
                Mail::CustomerInvite {
                    to: entry.clone(),
                    team_name,
                    collection_name: name,
                    link: format!("{}/s/{token}", state.config.app_url),
                },
            )
            .await?;
    }
    let opened: bool = sqlx::query_scalar(
        "SELECT EXISTS (SELECT 1 FROM share_opens WHERE collection_id = $1 AND entry = $2)",
    )
    .bind(id)
    .bind(&entry)
    .fetch_one(&mut *tx)
    .await?;
    tx.commit().await?;
    Ok((
        StatusCode::CREATED,
        Json(EntryBody {
            entry,
            kind: kind.to_owned(),
            invited_at,
            opened,
        }),
    ))
}

pub(super) async fn remove_entry(
    State(state): State<AppState>,
    auth: UserAuth,
    Path((team, slug, entry)): Path<(String, String, String)>,
) -> Result<StatusCode, ApiError> {
    let access = team_access(&state.db, auth.user_id, &team).await?;
    let done = sqlx::query(
        "DELETE FROM access_entries WHERE entry = $3 AND collection_id = \
         (SELECT id FROM collections WHERE team_id = $1 AND slug = $2)",
    )
    .bind(access.team_id)
    .bind(&slug)
    .bind(entry.trim().to_lowercase())
    .execute(&state.db)
    .await?;
    if done.rows_affected() == 0 {
        return Err(ApiError::not_found());
    }
    Ok(StatusCode::NO_CONTENT)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ok(raw: &str) -> (String, &'static str) {
        parse_entry(raw).unwrap()
    }

    #[test]
    fn emails_are_lowercased_and_typed() {
        assert_eq!(ok(" Ada@Example.COM "), ("ada@example.com".into(), "email"));
    }

    #[test]
    fn parse_email_takes_emails_only() {
        assert_eq!(
            parse_email(" Ada@Example.COM ").as_deref(),
            Some("ada@example.com")
        );
        for bad in ["*@example.com", "a@b@c.com", "@example.com", "ada@", "ada"] {
            assert_eq!(parse_email(bad), None, "{bad}");
        }
    }

    #[test]
    fn domains_need_a_dot() {
        assert_eq!(ok("*@Example.com"), ("*@example.com".into(), "domain"));
        for bad in ["*@example", "*@", "*@.com", "*@example..com", "*@example."] {
            assert!(parse_entry(bad).is_err(), "{bad}");
        }
    }

    #[test]
    fn malformed_emails_are_rejected() {
        for bad in [
            "",
            "ada",
            "@example.com",
            "ada@",
            "a@b@c.com",
            "a b@example.com",
            "a*@example.com",
        ] {
            assert!(parse_entry(bad).is_err(), "{bad}");
        }
    }
}
