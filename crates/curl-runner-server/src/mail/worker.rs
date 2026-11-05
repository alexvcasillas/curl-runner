//! Sends queued email, retrying failures with exponential backoff.

use std::future::Future;
use std::sync::Arc;
use std::time::Duration;

use chrono::Duration as ChronoDuration;
use sqlx::{FromRow, PgPool};

use super::Transport;
use crate::clock::Clock;

/// Rows handled per batch.
const BATCH: i64 = 20;
/// Attempts before a message is given up on.
pub const MAX_ATTEMPTS: i32 = 8;
const POLL: Duration = Duration::from_secs(2);
const BASE_BACKOFF_SECS: i64 = 30;
const MAX_BACKOFF_SECS: i64 = 3600;

#[derive(FromRow)]
struct Row {
    id: uuid::Uuid,
    to_addr: String,
    subject: String,
    text_body: String,
    html_body: Option<String>,
    attempts: i32,
}

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
    shutdown: impl Future<Output = ()>,
) {
    tokio::pin!(shutdown);
    loop {
        if let Err(err) = run_once(&db, transport.as_ref(), clock.as_ref()).await {
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
) -> Result<usize, sqlx::Error> {
    let mut sent = 0;
    for _ in 0..BATCH {
        match send_next(db, transport, clock).await? {
            Some(true) => sent += 1,
            Some(false) => {}
            None => break,
        }
    }
    Ok(sent)
}

/// Lock one due message, try it and record the outcome. `None` when nothing
/// is due, else whether the send succeeded.
async fn send_next(
    db: &PgPool,
    transport: &dyn Transport,
    clock: &dyn Clock,
) -> Result<Option<bool>, sqlx::Error> {
    let now = clock.now();
    let mut tx = db.begin().await?;
    let row: Option<Row> = sqlx::query_as(
        "SELECT id, to_addr, subject, text_body, html_body, attempts FROM outbox_emails \
         WHERE sent_at IS NULL AND attempts < $1 AND next_attempt_at <= $2 \
         ORDER BY next_attempt_at LIMIT 1 FOR UPDATE SKIP LOCKED",
    )
    .bind(MAX_ATTEMPTS)
    .bind(now)
    .fetch_optional(&mut *tx)
    .await?;
    let Some(row) = row else {
        return Ok(None);
    };

    let result = transport
        .send(
            &row.to_addr,
            &row.subject,
            &row.text_body,
            row.html_body.as_deref(),
        )
        .await;
    let ok = result.is_ok();
    match result {
        Ok(()) => {
            // Bodies hold sign-in codes and links: blank them once sent.
            sqlx::query(
                "UPDATE outbox_emails SET sent_at = $2, last_error = NULL, \
                 subject = '', text_body = '', html_body = '' WHERE id = $1",
            )
            .bind(row.id)
            .bind(now)
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
                 subject = CASE WHEN $5 THEN '' ELSE subject END, \
                 text_body = CASE WHEN $5 THEN '' ELSE text_body END, \
                 html_body = CASE WHEN $5 THEN '' ELSE html_body END WHERE id = $1",
            )
            .bind(row.id)
            .bind(attempts)
            .bind(now + backoff(row.attempts))
            .bind(error)
            .bind(gave_up)
            .bind(now)
            .execute(&mut *tx)
            .await?;
        }
    }
    tx.commit().await?;
    Ok(Some(ok))
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
