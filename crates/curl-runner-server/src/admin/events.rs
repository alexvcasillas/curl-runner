//! The audit log: one `admin_events` row per admin write.

use std::ops::{Deref, DerefMut};

use chrono::{DateTime, Months, Utc};
use serde_json::Value;
use sqlx::{PgConnection, PgPool, Postgres, Transaction};
use uuid::Uuid;

use super::auth::AdminAuth;
use crate::error::ApiError;
use crate::state::AppState;

/// Events older than this are deleted.
pub const RETENTION: Months = Months::new(24);

/// One admin action. `action` is a stable snake_case verb such as
/// `user.suspend` or `team.override.grant`; the activity filters group on it.
#[derive(Debug, Clone)]
pub struct Event {
    pub action: &'static str,
    pub target_kind: Option<&'static str>,
    pub target_id: Option<String>,
    pub target_label: Option<String>,
    pub detail: Value,
}

impl Event {
    pub fn new(action: &'static str) -> Self {
        Self {
            action,
            target_kind: None,
            target_id: None,
            target_label: None,
            detail: Value::Object(serde_json::Map::new()),
        }
    }

    /// What the action was done to: a kind (`user`, `team`, `collection`,
    /// `broadcast`, `settings`), its id and a human label (an email, a slug).
    pub fn target(
        mut self,
        kind: &'static str,
        id: impl ToString,
        label: impl Into<String>,
    ) -> Self {
        self.target_kind = Some(kind);
        self.target_id = Some(id.to_string());
        self.target_label = Some(label.into());
        self
    }

    pub fn detail(mut self, detail: Value) -> Self {
        self.detail = detail;
        self
    }
}

/// Insert `event` by `admin` at `at`. Call it inside the write's own
/// transaction, so the change and its record commit together. Prefer
/// [`AdminTx`], which cannot commit without an event.
pub async fn record(
    conn: &mut PgConnection,
    admin: &AdminAuth,
    at: DateTime<Utc>,
    event: Event,
) -> Result<(), ApiError> {
    sqlx::query(
        "INSERT INTO admin_events \
         (id, admin_user_id, admin_email, action, target_kind, target_id, target_label, \
          detail, ip, at) \
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)",
    )
    .bind(Uuid::new_v4())
    .bind(admin.user_id)
    .bind(&admin.email)
    .bind(event.action)
    .bind(event.target_kind)
    .bind(event.target_id)
    .bind(event.target_label)
    .bind(event.detail)
    .bind(&admin.ip)
    .bind(at)
    .execute(conn)
    .await?;
    Ok(())
}

/// The transaction of one admin write. It derefs to the connection for the
/// write's queries, and the only way to commit is [`AdminTx::commit`], which
/// records the event first. Dropping it rolls back.
pub struct AdminTx<'a> {
    tx: Transaction<'static, Postgres>,
    admin: &'a AdminAuth,
    at: DateTime<Utc>,
}

impl<'a> AdminTx<'a> {
    pub async fn begin(state: &AppState, admin: &'a AdminAuth) -> Result<Self, ApiError> {
        Ok(Self {
            tx: state.db.begin().await?,
            admin,
            at: state.clock.now(),
        })
    }

    /// When the write happens: the event's `at`.
    pub fn at(&self) -> DateTime<Utc> {
        self.at
    }

    /// Record `event`, then commit the write and the event together.
    pub async fn commit(mut self, event: Event) -> Result<(), ApiError> {
        record(&mut self.tx, self.admin, self.at, event).await?;
        self.tx.commit().await?;
        Ok(())
    }
}

impl Deref for AdminTx<'_> {
    type Target = PgConnection;

    fn deref(&self) -> &PgConnection {
        &self.tx
    }
}

impl DerefMut for AdminTx<'_> {
    fn deref_mut(&mut self) -> &mut PgConnection {
        &mut self.tx
    }
}

/// Delete events older than [`RETENTION`]. Returns how many went.
pub async fn prune(db: &PgPool, now: DateTime<Utc>) -> Result<u64, ApiError> {
    let cutoff = now.checked_sub_months(RETENTION).unwrap_or(now);
    let done = sqlx::query("DELETE FROM admin_events WHERE at < $1")
        .bind(cutoff)
        .execute(db)
        .await?;
    Ok(done.rows_affected())
}
