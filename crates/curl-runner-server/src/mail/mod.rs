//! Email outbox: mail is queued in the caller's transaction and sent by a
//! background worker, so a rolled-back action sends nothing.

pub mod templates;
pub mod transport;
pub mod worker;

use std::sync::Arc;

use sqlx::PgConnection;

use crate::clock::Clock;
use crate::error::ApiError;

pub use transport::{LogTransport, SmtpTransport, Transport};

/// An email the server can send.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Mail {
    SignInCode {
        to: String,
        code: String,
        link: String,
    },
    MemberInvite {
        to: String,
        team_name: String,
        inviter: String,
        link: String,
    },
    CustomerInvite {
        to: String,
        team_name: String,
        collection_name: String,
        link: String,
    },
}

impl Mail {
    pub fn to(&self) -> &str {
        match self {
            Self::SignInCode { to, .. }
            | Self::MemberInvite { to, .. }
            | Self::CustomerInvite { to, .. } => to,
        }
    }
}

/// Queues email in the `outbox_emails` table, always on the caller's connection.
#[derive(Clone)]
pub struct Mailer {
    clock: Arc<dyn Clock>,
}

impl Mailer {
    pub fn new(clock: Arc<dyn Clock>) -> Self {
        Self { clock }
    }

    /// Queue `mail` on the caller's connection. It is only sent once the
    /// caller's transaction commits.
    pub async fn enqueue(&self, tx: &mut PgConnection, mail: Mail) -> Result<(), ApiError> {
        let (subject, text, html) = templates::render(&mail);
        sqlx::query(
            "INSERT INTO outbox_emails (to_addr, subject, text_body, html_body, next_attempt_at) \
             VALUES ($1, $2, $3, $4, $5)",
        )
        .bind(mail.to())
        .bind(subject)
        .bind(text)
        .bind(html)
        .bind(self.clock.now())
        .execute(tx)
        .await
        .map_err(|err| {
            tracing::error!("cannot queue email: {err}");
            ApiError::unavailable()
        })?;
        Ok(())
    }
}
