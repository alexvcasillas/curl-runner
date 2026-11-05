//! Email outbox: mail is queued in the caller's transaction and sent by a
//! background worker, so a rolled-back action sends nothing.

pub mod templates;
pub mod transport;
pub mod worker;

use std::sync::Arc;

use chrono::{DateTime, Utc};
use sqlx::PgConnection;

use crate::clock::Clock;
use crate::error::ApiError;

pub use transport::{LogTransport, SmtpTransport, Transport};

/// An email the server can send. `Debug` hides codes and links.
#[derive(Clone, PartialEq, Eq)]
pub enum Mail {
    /// `link` is `None` for a device sign-in: the code goes back to the
    /// terminal, and a link opened in a browser would use it up.
    SignInCode {
        to: String,
        code: String,
        link: Option<String>,
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
    /// First failed payment. `deadline` is when the team moves to Free.
    PaymentFailed {
        to: String,
        team_name: String,
        slug: String,
        deadline: DateTime<Utc>,
        app_url: String,
    },
    /// The team moved from Team to Free.
    PlanChanged {
        to: String,
        team_name: String,
        slug: String,
        app_url: String,
    },
}

impl std::fmt::Debug for Mail {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        const R: &str = "<redacted>";
        match self {
            Self::SignInCode { to, .. } => f
                .debug_struct("SignInCode")
                .field("to", to)
                .field("code", &R)
                .field("link", &R)
                .finish(),
            Self::MemberInvite {
                to,
                team_name,
                inviter,
                ..
            } => f
                .debug_struct("MemberInvite")
                .field("to", to)
                .field("team_name", team_name)
                .field("inviter", inviter)
                .field("link", &R)
                .finish(),
            Self::CustomerInvite {
                to,
                team_name,
                collection_name,
                ..
            } => f
                .debug_struct("CustomerInvite")
                .field("to", to)
                .field("team_name", team_name)
                .field("collection_name", collection_name)
                .field("link", &R)
                .finish(),
            Self::PaymentFailed {
                to,
                team_name,
                slug,
                deadline,
                ..
            } => f
                .debug_struct("PaymentFailed")
                .field("to", to)
                .field("team_name", team_name)
                .field("slug", slug)
                .field("deadline", deadline)
                .finish(),
            Self::PlanChanged {
                to,
                team_name,
                slug,
                ..
            } => f
                .debug_struct("PlanChanged")
                .field("to", to)
                .field("team_name", team_name)
                .field("slug", slug)
                .finish(),
        }
    }
}

impl Mail {
    pub fn to(&self) -> &str {
        match self {
            Self::SignInCode { to, .. }
            | Self::MemberInvite { to, .. }
            | Self::CustomerInvite { to, .. }
            | Self::PaymentFailed { to, .. }
            | Self::PlanChanged { to, .. } => to,
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
            "INSERT INTO outbox_emails \
             (to_addr, subject, text_body, html_body, next_attempt_at, created_at) \
             VALUES ($1, $2, $3, $4, $5, $5)",
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn debug_redacts_codes_and_links() {
        let mails = [
            Mail::SignInCode {
                to: "ada@example.com".into(),
                code: "123456".into(),
                link: Some("https://app.example.com/x?token=SECRET".into()),
            },
            Mail::MemberInvite {
                to: "ada@example.com".into(),
                team_name: "Acme".into(),
                inviter: "Jane".into(),
                link: "https://app.example.com/x?token=SECRET".into(),
            },
            Mail::CustomerInvite {
                to: "ada@example.com".into(),
                team_name: "Acme".into(),
                collection_name: "payments-api".into(),
                link: "https://app.example.com/x?token=SECRET".into(),
            },
        ];
        for mail in mails {
            let shown = format!("{mail:?}");
            assert!(!shown.contains("123456"), "{shown}");
            assert!(!shown.contains("SECRET"), "{shown}");
            assert!(shown.contains("<redacted>"), "{shown}");
            assert!(shown.contains("ada@example.com"), "{shown}");
        }
    }
}
