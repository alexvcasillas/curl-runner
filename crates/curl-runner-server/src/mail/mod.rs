//! Email outbox: mail is queued in the caller's transaction and sent by a
//! background worker, so a rolled-back action sends nothing.

pub mod markdown;
pub mod templates;
pub mod transport;
pub mod unsubscribe;
pub mod worker;

use std::sync::Arc;

use chrono::{DateTime, Utc};
use curl_runner_cloud_proto::Plan;
use sqlx::PgConnection;
use uuid::Uuid;

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
        /// The Free limits in force now, from the server settings.
        free_member_limit: u32,
        free_restricted: bool,
    },
    /// Sent to an admin who asks for one from the settings page.
    TestEmail { to: String },
    /// The account was suspended and the admin asked to tell them.
    AccountSuspended { to: String, reason: Option<String> },
    /// An admin granted the team a plan override and asked to tell the owners.
    /// `until` is `None` for no end date.
    PlanGranted {
        to: String,
        team_name: String,
        slug: String,
        plan: Plan,
        until: Option<DateTime<Utc>>,
        app_url: String,
    },
    /// The team's override ends in 14 days. Sent once per end date.
    OverrideEnding {
        to: String,
        team_name: String,
        slug: String,
        plan: Plan,
        until: DateTime<Utc>,
        app_url: String,
    },
    /// The team was suspended and the admin asked to tell the owners.
    TeamSuspended {
        to: String,
        team_name: String,
        reason: Option<String>,
    },
    /// An admin turned a collection's share link off; sent to the team's owners.
    LinkDisabled {
        to: String,
        team: String,
        collection: String,
        reason: Option<String>,
    },
    /// A message an admin wrote to one person. `body` is Markdown, rendered
    /// by [`markdown::render`] without variables.
    AdminMessage {
        to: String,
        subject: String,
        body: String,
    },
    /// One recipient's copy of a broadcast. `subject` and `body_md` are the
    /// broadcast's own; `vars` fill their variables. `unsubscribe` is the
    /// recipient's one-click link. `test` is the copy an admin sends to
    /// themselves, marked in the subject.
    Broadcast {
        to: String,
        subject: String,
        body_md: String,
        vars: markdown::Vars,
        unsubscribe: String,
        test: bool,
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
            Self::TestEmail { to } => f.debug_struct("TestEmail").field("to", to).finish(),
            Self::AccountSuspended { to, .. } => {
                f.debug_struct("AccountSuspended").field("to", to).finish()
            }
            Self::PlanGranted {
                to,
                team_name,
                plan,
                until,
                ..
            } => f
                .debug_struct("PlanGranted")
                .field("to", to)
                .field("team_name", team_name)
                .field("plan", plan)
                .field("until", until)
                .finish(),
            Self::OverrideEnding {
                to,
                team_name,
                until,
                ..
            } => f
                .debug_struct("OverrideEnding")
                .field("to", to)
                .field("team_name", team_name)
                .field("until", until)
                .finish(),
            Self::TeamSuspended { to, team_name, .. } => f
                .debug_struct("TeamSuspended")
                .field("to", to)
                .field("team_name", team_name)
                .finish(),
            Self::LinkDisabled {
                to,
                team,
                collection,
                ..
            } => f
                .debug_struct("LinkDisabled")
                .field("to", to)
                .field("team", team)
                .field("collection", collection)
                .finish(),
            Self::AdminMessage { to, subject, .. } => f
                .debug_struct("AdminMessage")
                .field("to", to)
                .field("subject", subject)
                .finish(),
            Self::Broadcast {
                to, subject, test, ..
            } => f
                .debug_struct("Broadcast")
                .field("to", to)
                .field("subject", subject)
                .field("unsubscribe", &R)
                .field("test", test)
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
            | Self::PlanChanged { to, .. }
            | Self::AccountSuspended { to, .. }
            | Self::PlanGranted { to, .. }
            | Self::OverrideEnding { to, .. }
            | Self::TeamSuspended { to, .. }
            | Self::LinkDisabled { to, .. }
            | Self::AdminMessage { to, .. }
            | Self::Broadcast { to, .. }
            | Self::TestEmail { to } => to,
        }
    }

    /// The `outbox_emails.kind` it is queued with: which email it is, for
    /// the delivery log. Rows queued before 0004 say `transactional`. Only
    /// `broadcast` mail honours the product-email opt-out, and that is
    /// decided when a broadcast is expanded, not by the kind.
    pub fn kind(&self) -> &'static str {
        match self {
            Self::SignInCode { .. } => "sign_in_code",
            Self::MemberInvite { .. } => "member_invite",
            Self::CustomerInvite { .. } => "customer_invite",
            Self::PaymentFailed { .. } => "payment_failed",
            Self::PlanChanged { .. } => "plan_changed",
            Self::TestEmail { .. } => "test_email",
            Self::AccountSuspended { .. } => "account_suspended",
            Self::PlanGranted { .. } => "plan_granted",
            Self::OverrideEnding { .. } => "override_ending",
            Self::TeamSuspended { .. } => "team_suspended",
            Self::LinkDisabled { .. } => "link_disabled",
            Self::AdminMessage { .. } => "admin",
            Self::Broadcast { test: false, .. } => "broadcast",
            Self::Broadcast { test: true, .. } => "broadcast_test",
        }
    }
}

/// Every `outbox_emails.kind` value, for the delivery log's filter.
pub const KINDS: [&str; 15] = [
    "sign_in_code",
    "member_invite",
    "customer_invite",
    "payment_failed",
    "plan_changed",
    "test_email",
    "account_suspended",
    "plan_granted",
    "override_ending",
    "team_suspended",
    "link_disabled",
    "admin",
    "broadcast",
    "broadcast_test",
    "transactional",
];

/// Kinds whose subject holds a secret (a sign-in code). The worker blanks
/// their subject once sent and the delivery log never shows it. Rows from
/// before 0004 are `transactional` and may be sign-in codes too.
pub const SECRET_SUBJECT_KINDS: [&str; 2] = ["sign_in_code", "transactional"];

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
        self.enqueue_for(tx, mail, None).await
    }

    /// Like [`Mailer::enqueue`], recording which user the mail is for.
    pub async fn enqueue_for(
        &self,
        tx: &mut PgConnection,
        mail: Mail,
        user_id: Option<Uuid>,
    ) -> Result<(), ApiError> {
        let (subject, text, html) = templates::render(&mail);
        sqlx::query(
            "INSERT INTO outbox_emails \
             (to_addr, subject, text_body, html_body, next_attempt_at, created_at, kind, user_id) \
             VALUES ($1, $2, $3, $4, $5, $5, $6, $7)",
        )
        .bind(mail.to())
        .bind(subject)
        .bind(text)
        .bind(html)
        .bind(self.clock.now())
        .bind(mail.kind())
        .bind(user_id)
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
