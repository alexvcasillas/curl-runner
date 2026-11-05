//! `/v1/admin/me` and managing who else is an admin.

use axum::Json;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use chrono::{DateTime, Utc};
use curl_runner_cloud_proto::ErrorCode;
use serde::{Deserialize, Serialize};
use serde_json::json;
use sqlx::{FromRow, PgConnection};
use uuid::Uuid;

use super::auth::{AdminAuth, Perm, Role};
use super::events::{self, Event};
use crate::error::ApiError;
use crate::extract::ApiJson;
use crate::state::AppState;
use crate::teams::{first_operator, is_operator};

#[derive(Serialize)]
pub struct AdminMe {
    role: Role,
    permissions: Vec<Perm>,
}

pub async fn me(admin: AdminAuth) -> Json<AdminMe> {
    Json(AdminMe {
        role: admin.role,
        permissions: admin.permissions,
    })
}

#[derive(Serialize)]
pub struct AddedBy {
    user_id: Uuid,
    email: String,
    name: Option<String>,
}

/// One row of the admins list. An owner listed in `OPERATOR_EMAILS` who has
/// not signed in yet has no `user_id`.
#[derive(Serialize)]
pub struct AdminRow {
    user_id: Option<Uuid>,
    email: String,
    name: Option<String>,
    role: Role,
    permissions: Vec<Perm>,
    last_active: Option<DateTime<Utc>>,
    added_by: Option<AddedBy>,
    added_at: Option<DateTime<Utc>>,
}

#[derive(FromRow)]
struct AdminSql {
    user_id: Uuid,
    email: String,
    name: Option<String>,
    permissions: Vec<String>,
    last_active: Option<DateTime<Utc>>,
    added_at: DateTime<Utc>,
    by_id: Option<Uuid>,
    by_email: Option<String>,
    by_name: Option<String>,
}

impl From<AdminSql> for AdminRow {
    fn from(r: AdminSql) -> Self {
        Self {
            user_id: Some(r.user_id),
            email: r.email,
            name: r.name,
            role: Role::Admin,
            permissions: Perm::from_column(&r.permissions),
            last_active: r.last_active,
            added_by: match (r.by_id, r.by_email) {
                (Some(user_id), Some(email)) => Some(AddedBy {
                    user_id,
                    email,
                    name: r.by_name,
                }),
                _ => None,
            },
            added_at: Some(r.added_at),
        }
    }
}

const LAST_ACTIVE: &str =
    "(SELECT max(s.last_used_at) FROM sessions s WHERE s.user_id = u.id) AS last_active";

/// Rows of `admins`, oldest first; only `user_id`'s when given.
async fn admin_rows(
    conn: &mut PgConnection,
    user_id: Option<Uuid>,
) -> Result<Vec<AdminSql>, ApiError> {
    Ok(sqlx::query_as(&format!(
        "SELECT a.user_id, u.email::text AS email, u.name, a.permissions, {LAST_ACTIVE}, \
                a.added_at, b.id AS by_id, b.email::text AS by_email, b.name AS by_name \
         FROM admins a JOIN users u ON u.id = a.user_id \
         LEFT JOIN users b ON b.id = a.added_by \
         WHERE ($1::uuid IS NULL OR a.user_id = $1) \
         ORDER BY a.added_at, u.email"
    ))
    .bind(user_id)
    .fetch_all(conn)
    .await?)
}

/// An owner's account: id, email, name, last active.
type OwnerSql = (Uuid, String, Option<String>, Option<DateTime<Utc>>);

/// Owners (in `OPERATOR_EMAILS` order, then the first operator), then admins.
pub async fn list(
    _admin: AdminAuth,
    State(state): State<AppState>,
) -> Result<Json<Vec<AdminRow>>, ApiError> {
    let mut owner_emails = state.config.operators.clone();
    if let Some(first) = first_operator(&state.db).await? {
        let first = first.to_lowercase();
        if !owner_emails.contains(&first) {
            owner_emails.push(first);
        }
    }
    let mut conn = state.db.acquire().await?;
    let mut rows = Vec::new();
    for email in &owner_emails {
        let user: Option<OwnerSql> = sqlx::query_as(&format!(
            "SELECT u.id, u.email::text, u.name, {LAST_ACTIVE} FROM users u \
                 WHERE u.email = $1::citext"
        ))
        .bind(email)
        .fetch_optional(&mut *conn)
        .await?;
        let (user_id, email, name, last_active) = match user {
            Some((id, email, name, last)) => (Some(id), email, name, last),
            None => (None, email.clone(), None, None),
        };
        rows.push(AdminRow {
            user_id,
            email,
            name,
            role: Role::Owner,
            permissions: Perm::ALL.to_vec(),
            last_active,
            added_by: None,
            added_at: None,
        });
    }
    for row in admin_rows(&mut conn, None).await? {
        if !owner_emails.contains(&row.email.to_lowercase()) {
            rows.push(row.into());
        }
    }
    Ok(Json(rows))
}

fn owner_conflict() -> ApiError {
    ApiError::new(
        ErrorCode::Conflict,
        "Owners come from OPERATOR_EMAILS and can't be changed here.",
    )
}

#[derive(Deserialize)]
pub struct AddAdmin {
    email: String,
    permissions: Vec<Perm>,
}

/// Perm `admins`. The person must have signed in once.
pub async fn add(
    admin: AdminAuth,
    State(state): State<AppState>,
    ApiJson(req): ApiJson<AddAdmin>,
) -> Result<(StatusCode, Json<AdminRow>), ApiError> {
    admin.require(Perm::Admins)?;
    let user: Option<(Uuid, String)> =
        sqlx::query_as("SELECT id, email::text FROM users WHERE email = $1::citext")
            .bind(req.email.trim())
            .fetch_optional(&state.db)
            .await?;
    let (user_id, email) = user.ok_or_else(|| {
        ApiError::new(
            ErrorCode::NotFound,
            "No account with that email yet. Ask them to sign in once.",
        )
    })?;
    if is_operator(&state.db, &state.config, &email).await? {
        return Err(owner_conflict());
    }
    let permissions = Perm::normalize(&req.permissions);
    let now = state.clock.now();
    let mut tx = state.db.begin().await?;
    let inserted: Option<Uuid> = sqlx::query_scalar(
        "INSERT INTO admins (user_id, permissions, added_by, added_at) VALUES ($1, $2, $3, $4) \
         ON CONFLICT (user_id) DO NOTHING RETURNING user_id",
    )
    .bind(user_id)
    .bind(Perm::to_column(&permissions))
    .bind(admin.user_id)
    .bind(now)
    .fetch_optional(&mut *tx)
    .await?;
    if inserted.is_none() {
        return Err(ApiError::new(
            ErrorCode::Conflict,
            "That person is already an admin.",
        ));
    }
    events::record(
        &mut tx,
        &admin,
        now,
        Event::new("admin.add")
            .target("user", user_id, &email)
            .detail(json!({"permissions": permissions})),
    )
    .await?;
    let row = one_admin(&mut tx, user_id).await?;
    tx.commit().await?;
    Ok((StatusCode::CREATED, Json(row)))
}

async fn one_admin(conn: &mut PgConnection, user_id: Uuid) -> Result<AdminRow, ApiError> {
    admin_rows(conn, Some(user_id))
        .await?
        .into_iter()
        .next()
        .map(Into::into)
        .ok_or_else(ApiError::not_found)
}

/// The email of `user_id`; an owner is `409`, an unknown user `404`.
async fn non_owner_email(state: &AppState, user_id: Uuid) -> Result<String, ApiError> {
    let email: Option<String> = sqlx::query_scalar("SELECT email::text FROM users WHERE id = $1")
        .bind(user_id)
        .fetch_optional(&state.db)
        .await?;
    let email = email.ok_or_else(ApiError::not_found)?;
    if is_operator(&state.db, &state.config, &email).await? {
        return Err(owner_conflict());
    }
    Ok(email)
}

/// The admin's stored permissions, row-locked. Not an admin: `404`.
async fn lock_admin(conn: &mut PgConnection, user_id: Uuid) -> Result<Vec<Perm>, ApiError> {
    let perms: Option<Vec<String>> =
        sqlx::query_scalar("SELECT permissions FROM admins WHERE user_id = $1 FOR UPDATE")
            .bind(user_id)
            .fetch_optional(conn)
            .await?;
    perms
        .map(|p| Perm::from_column(&p))
        .ok_or_else(ApiError::not_found)
}

#[derive(Deserialize)]
pub struct SetPermissions {
    permissions: Vec<Perm>,
}

/// Perm `admins`. Takes effect on the admin's next request.
pub async fn update(
    admin: AdminAuth,
    State(state): State<AppState>,
    Path(user_id): Path<Uuid>,
    ApiJson(req): ApiJson<SetPermissions>,
) -> Result<Json<AdminRow>, ApiError> {
    admin.require(Perm::Admins)?;
    let email = non_owner_email(&state, user_id).await?;
    let permissions = Perm::normalize(&req.permissions);
    let now = state.clock.now();
    let mut tx = state.db.begin().await?;
    let before = lock_admin(&mut tx, user_id).await?;
    sqlx::query("UPDATE admins SET permissions = $2 WHERE user_id = $1")
        .bind(user_id)
        .bind(Perm::to_column(&permissions))
        .execute(&mut *tx)
        .await?;
    events::record(
        &mut tx,
        &admin,
        now,
        Event::new("admin.update")
            .target("user", user_id, &email)
            .detail(json!({"from": before, "to": permissions})),
    )
    .await?;
    let row = one_admin(&mut tx, user_id).await?;
    tx.commit().await?;
    Ok(Json(row))
}

/// Perm `admins`. The person keeps their account and teams.
pub async fn remove(
    admin: AdminAuth,
    State(state): State<AppState>,
    Path(user_id): Path<Uuid>,
) -> Result<StatusCode, ApiError> {
    admin.require(Perm::Admins)?;
    let email = non_owner_email(&state, user_id).await?;
    let now = state.clock.now();
    let mut tx = state.db.begin().await?;
    let before = lock_admin(&mut tx, user_id).await?;
    sqlx::query("DELETE FROM admins WHERE user_id = $1")
        .bind(user_id)
        .execute(&mut *tx)
        .await?;
    events::record(
        &mut tx,
        &admin,
        now,
        Event::new("admin.remove")
            .target("user", user_id, &email)
            .detail(json!({"permissions": before})),
    )
    .await?;
    tx.commit().await?;
    Ok(StatusCode::NO_CONTENT)
}
