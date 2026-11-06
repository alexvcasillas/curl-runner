//! Sends queued email, retrying failures with exponential backoff, and
//! expands broadcasts into the outbox.

use std::future::Future;
use std::sync::Arc;
use std::time::Duration;

use chrono::{DateTime, Duration as ChronoDuration, Utc};
use sqlx::{FromRow, PgPool};
use uuid::Uuid;

use super::{SECRET_SUBJECT_KINDS, Transport, templates, unsubscribe};
use crate::admin::emails::{Audience, Recipient, copy_for, recipients_sql};
use crate::clock::Clock;
use crate::config::Config;
use crate::error::ApiError;
use crate::state::AppState;

/// Rows handled per batch.
const BATCH: i64 = 20;
/// Attempts before a message is given up on.
pub const MAX_ATTEMPTS: i32 = 8;
const POLL: Duration = Duration::from_secs(2);
const BASE_BACKOFF_SECS: i64 = 30;
/// How often the broadcast worker looks for due broadcasts.
const BROADCAST_POLL: Duration = Duration::from_secs(5);
/// Recipients queued per transaction when a broadcast is expanded.
pub const FANOUT_CHUNK: i64 = 500;
const MAX_BACKOFF_SECS: i64 = 3600;

#[derive(FromRow)]
struct Row {
    id: uuid::Uuid,
    to_addr: String,
    subject: String,
    text_body: String,
    html_body: Option<String>,
    attempts: i32,
    kind: String,
    user_id: Option<Uuid>,
}

/// `last_error` of a broadcast copy dropped at send time.
pub const SKIPPED_OPTED_OUT: &str = "Skipped: they turned off product emails before it was sent.";
pub const SKIPPED_NO_ACCOUNT: &str = "Skipped: the account was deleted before it was sent.";

/// Delay before the next try after `attempts` failures so far (0 for the first).
pub fn backoff(attempts: i32) -> ChronoDuration {
    let factor = 1i64 << attempts.clamp(0, 20);
    ChronoDuration::seconds((BASE_BACKOFF_SECS * factor).min(MAX_BACKOFF_SECS))
}

/// Poll the outbox every 2 s until `shutdown` resolves.
pub async fn run(
    db: PgPool,
    transport: Arc<dyn Transport>,
    clock: Arc<dyn Clock>,
    config: Arc<Config>,
    shutdown: impl Future<Output = ()>,
) {
    tokio::pin!(shutdown);
    loop {
        if let Err(err) = run_once(&db, transport.as_ref(), clock.as_ref(), &config).await {
            tracing::error!("mail worker: {err}");
        }
        tokio::select! {
            () = &mut shutdown => return,
            () = tokio::time::sleep(POLL) => {},
        }
    }
}

/// Process up to one batch of due messages, each in its own short transaction
/// so a slow send holds one row lock and a later failure never undoes an
/// earlier send. Returns how many were sent.
pub async fn run_once(
    db: &PgPool,
    transport: &dyn Transport,
    clock: &dyn Clock,
    config: &Config,
) -> Result<usize, sqlx::Error> {
    let mut sent = 0;
    for _ in 0..BATCH {
        match send_next(db, transport, clock, config).await? {
            Some(true) => sent += 1,
            Some(false) => {}
            None => break,
        }
    }
    Ok(sent)
}

/// Due rows: `{extra}` narrows the pick. Both picks walk the pending index
/// on `next_attempt_at`.
fn pick_sql(extra: &str) -> String {
    format!(
        "SELECT id, to_addr, subject, text_body, html_body, attempts, kind, user_id \
         FROM outbox_emails \
         WHERE sent_at IS NULL AND failed_at IS NULL AND attempts < $1 \
           AND next_attempt_at <= $2 {extra} \
         ORDER BY next_attempt_at LIMIT 1 FOR UPDATE SKIP LOCKED"
    )
}

/// Lock one due message, try it and record the outcome. `None` when nothing
/// is due, else whether the send succeeded.
///
/// Everything else goes before broadcast copies, so a large broadcast never
/// holds back a sign-in code: first the oldest due non-broadcast row, and
/// only when there is none, the oldest due row of any kind.
///
/// A broadcast copy whose recipient has since turned product emails off, or
/// whose account is gone, is not sent: it is marked failed with
/// [`SKIPPED_OPTED_OUT`] or [`SKIPPED_NO_ACCOUNT`]. Copies that go out carry
/// `List-Unsubscribe` (RFC 2369) and `List-Unsubscribe-Post` (RFC 8058).
async fn send_next(
    db: &PgPool,
    transport: &dyn Transport,
    clock: &dyn Clock,
    config: &Config,
) -> Result<Option<bool>, sqlx::Error> {
    let now = clock.now();
    let mut tx = db.begin().await?;
    let mut row: Option<Row> = sqlx::query_as(&pick_sql("AND kind <> 'broadcast'"))
        .bind(MAX_ATTEMPTS)
        .bind(now)
        .fetch_optional(&mut *tx)
        .await?;
    if row.is_none() {
        row = sqlx::query_as(&pick_sql(""))
            .bind(MAX_ATTEMPTS)
            .bind(now)
            .fetch_optional(&mut *tx)
            .await?;
    }
    let Some(row) = row else {
        return Ok(None);
    };

    let mut headers = Vec::new();
    if row.kind == "broadcast" {
        let opted_in: Option<bool> = match row.user_id {
            Some(user_id) => {
                sqlx::query_scalar("SELECT product_emails FROM users WHERE id = $1")
                    .bind(user_id)
                    .fetch_optional(&mut *tx)
                    .await?
            }
            None => None,
        };
        let skip = match (row.user_id, opted_in) {
            (Some(user_id), Some(true)) => {
                headers.push((
                    "List-Unsubscribe",
                    format!("<{}>", unsubscribe::link(config, user_id)),
                ));
                headers.push((
                    "List-Unsubscribe-Post",
                    "List-Unsubscribe=One-Click".to_owned(),
                ));
                None
            }
            (Some(_), Some(false)) => Some(SKIPPED_OPTED_OUT),
            _ => Some(SKIPPED_NO_ACCOUNT),
        };
        if let Some(reason) = skip {
            sqlx::query(
                "UPDATE outbox_emails SET failed_at = $2, last_error = $3, \
                 text_body = '', html_body = '' WHERE id = $1",
            )
            .bind(row.id)
            .bind(now)
            .bind(reason)
            .execute(&mut *tx)
            .await?;
            tx.commit().await?;
            return Ok(Some(false));
        }
    }

    let result = transport
        .send(
            &row.to_addr,
            &row.subject,
            &row.text_body,
            row.html_body.as_deref(),
            &headers,
        )
        .await;
    let ok = result.is_ok();
    match result {
        Ok(()) => {
            // Bodies hold sign-in codes and links: blank them once sent. The
            // subject stays for the delivery log unless it holds a code.
            sqlx::query(
                "UPDATE outbox_emails SET sent_at = $2, last_error = NULL, \
                 subject = CASE WHEN kind = ANY($3) THEN '' ELSE subject END, \
                 text_body = '', html_body = '' WHERE id = $1",
            )
            .bind(row.id)
            .bind(now)
            .bind(&SECRET_SUBJECT_KINDS[..])
            .execute(&mut *tx)
            .await?;
        }
        Err(error) => {
            let attempts = row.attempts + 1;
            let gave_up = attempts >= MAX_ATTEMPTS;
            if gave_up {
                tracing::error!(
                    "giving up on email {} after {attempts} attempts: {error}",
                    row.id
                );
            } else {
                tracing::warn!("email {} failed (attempt {attempts}): {error}", row.id);
            }
            // After the last attempt nothing will read the bodies again.
            sqlx::query(
                "UPDATE outbox_emails SET attempts = $2, next_attempt_at = $3, last_error = $4, \
                 failed_at = CASE WHEN $5 THEN $6 ELSE failed_at END, \
                 subject = CASE WHEN $5 AND kind = ANY($7) THEN '' ELSE subject END, \
                 text_body = CASE WHEN $5 THEN '' ELSE text_body END, \
                 html_body = CASE WHEN $5 THEN '' ELSE html_body END WHERE id = $1",
            )
            .bind(row.id)
            .bind(attempts)
            .bind(now + backoff(row.attempts))
            .bind(error)
            .bind(gave_up)
            .bind(now)
            .bind(&SECRET_SUBJECT_KINDS[..])
            .execute(&mut *tx)
            .await?;
        }
    }
    tx.commit().await?;
    Ok(Some(ok))
}

/// Expand due broadcasts every 5 s until `shutdown` resolves.
pub async fn run_broadcasts(state: AppState, shutdown: impl Future<Output = ()>) {
    tokio::pin!(shutdown);
    loop {
        if let Err(err) = fan_out_due(&state, state.clock.now()).await {
            tracing::error!("broadcast worker: {err:?}");
        }
        tokio::select! {
            () = &mut shutdown => return,
            () = tokio::time::sleep(BROADCAST_POLL) => {},
        }
    }
}

/// Start the scheduled broadcasts that are due at `now`, then expand every
/// broadcast that is `sending`: new ones, and any a crash left half done.
/// Returns how many finished.
pub async fn fan_out_due(state: &AppState, now: DateTime<Utc>) -> Result<usize, ApiError> {
    // A due broadcast with nothing to send goes back to draft instead.
    // (The API refuses to leave a scheduled one empty; this is the backstop.)
    let emptied: Vec<Uuid> = sqlx::query_scalar(
        "UPDATE broadcasts SET status = 'draft', scheduled_at = NULL, updated_at = $1 \
         WHERE status = 'scheduled' AND scheduled_at <= $1 \
           AND (btrim(subject) = '' OR btrim(body_md) = '') RETURNING id",
    )
    .bind(now)
    .fetch_all(&state.db)
    .await?;
    for id in emptied {
        tracing::warn!("broadcast {id} was due with no subject or message: back to draft");
    }
    sqlx::query(
        "UPDATE broadcasts SET status = 'sending', updated_at = $1 \
         WHERE status = 'scheduled' AND scheduled_at <= $1",
    )
    .bind(now)
    .execute(&state.db)
    .await?;
    let sending: Vec<Uuid> = sqlx::query_scalar(
        "SELECT id FROM broadcasts WHERE status = 'sending' ORDER BY scheduled_at, created_at",
    )
    .fetch_all(&state.db)
    .await?;
    let mut done = 0;
    for id in sending {
        // One broadcast failing must not hold up the others.
        match expand_broadcast(state, id, usize::MAX).await {
            Ok(true) => done += 1,
            Ok(false) => {}
            Err(err) => tracing::error!("broadcast {id}: cannot expand it: {err:?}"),
        }
    }
    Ok(done)
}

#[derive(FromRow)]
struct Sending {
    subject: String,
    body_md: String,
    audience: String,
    status: String,
}

/// Queue up to `max_chunks` chunks of [`FANOUT_CHUNK`] recipients of a
/// `sending` broadcast. Returns whether it is done (and now `sent`).
///
/// Each chunk is one transaction holding the broadcast's row lock: it picks
/// the next recipients who have no row for this broadcast yet, renders their
/// copies, inserts them with `ON CONFLICT DO NOTHING` on the unique
/// `(broadcast_id, user_id)`, and adds what it inserted to `sent_count`. So
/// progress is the outbox itself: after a crash the next run starts with
/// whoever is still missing, and nobody gets two copies. The chunk that
/// comes back short marks the broadcast `sent`.
pub async fn expand_broadcast(
    state: &AppState,
    id: Uuid,
    max_chunks: usize,
) -> Result<bool, ApiError> {
    for _ in 0..max_chunks {
        let mut tx = state.db.begin().await?;
        let b: Option<Sending> = sqlx::query_as(
            "SELECT subject, body_md, audience, status FROM broadcasts WHERE id = $1 FOR UPDATE",
        )
        .bind(id)
        .fetch_optional(&mut *tx)
        .await?;
        let Some(b) = b else {
            return Ok(true);
        };
        if b.status != "sending" {
            return Ok(b.status == "sent");
        }
        let now = state.clock.now();
        let audience = Audience::parse(&b.audience).unwrap_or(Audience::Everyone);
        let recipients: Vec<Recipient> = sqlx::query_as(&format!(
            "{} LIMIT $3",
            recipients_sql(
                audience,
                false,
                "AND NOT EXISTS (SELECT 1 FROM outbox_emails o \
                                 WHERE o.broadcast_id = $2 AND o.user_id = u.id)",
            )
        ))
        .bind(now)
        .bind(id)
        .bind(FANOUT_CHUNK)
        .fetch_all(&mut *tx)
        .await?;
        let mut to = Vec::with_capacity(recipients.len());
        let mut subjects = Vec::with_capacity(recipients.len());
        let mut texts = Vec::with_capacity(recipients.len());
        let mut htmls = Vec::with_capacity(recipients.len());
        let mut users = Vec::with_capacity(recipients.len());
        for r in &recipients {
            let link = unsubscribe::link(&state.config, r.user_id);
            let mail = copy_for(r, &b.subject, &b.body_md, link, now, false);
            let (subject, text, html) = templates::render(&mail);
            to.push(r.email.clone());
            subjects.push(subject);
            texts.push(text);
            htmls.push(html);
            users.push(r.user_id);
        }
        let inserted = sqlx::query(
            "INSERT INTO outbox_emails \
             (to_addr, subject, text_body, html_body, next_attempt_at, created_at, kind, \
              broadcast_id, user_id) \
             SELECT r.to_addr, r.subject, r.text_body, r.html_body, $1, $1, 'broadcast', $2, \
                    r.user_id \
             FROM UNNEST($3::text[], $4::text[], $5::text[], $6::text[], $7::uuid[]) \
                  AS r(to_addr, subject, text_body, html_body, user_id) \
             ON CONFLICT (broadcast_id, user_id) WHERE broadcast_id IS NOT NULL DO NOTHING",
        )
        .bind(now)
        .bind(id)
        .bind(&to)
        .bind(&subjects)
        .bind(&texts)
        .bind(&htmls)
        .bind(&users)
        .execute(&mut *tx)
        .await?
        .rows_affected();
        let finished = (recipients.len() as i64) < FANOUT_CHUNK;
        sqlx::query(
            "UPDATE broadcasts SET sent_count = sent_count + $2, updated_at = $3, \
                    status = CASE WHEN $4 THEN 'sent' ELSE status END, \
                    sent_at = CASE WHEN $4 THEN $3 ELSE sent_at END \
             WHERE id = $1",
        )
        .bind(id)
        .bind(i32::try_from(inserted).unwrap_or(i32::MAX))
        .bind(now)
        .bind(finished)
        .execute(&mut *tx)
        .await?;
        tx.commit().await?;
        if finished {
            return Ok(true);
        }
    }
    Ok(false)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn backoff_doubles_and_caps_at_one_hour() {
        assert_eq!(backoff(0), ChronoDuration::seconds(30));
        assert_eq!(backoff(1), ChronoDuration::seconds(60));
        assert_eq!(backoff(2), ChronoDuration::seconds(120));
        assert_eq!(backoff(7), ChronoDuration::seconds(3600));
        assert_eq!(backoff(40), ChronoDuration::seconds(3600));
    }
}
