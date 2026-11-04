//! Delivery backends: SMTP via lettre, or the log for local development.

use std::future::Future;
use std::pin::Pin;

use lettre::message::{Mailbox, MultiPart, SinglePart};
use lettre::transport::smtp::authentication::Credentials;
use lettre::{AsyncSmtpTransport, AsyncTransport, Message, Tokio1Executor};

use crate::config::{SmtpConfig, SmtpTls};

pub type SendFuture<'a> = Pin<Box<dyn Future<Output = Result<(), String>> + Send + 'a>>;

/// Sends one email. The error is a short description kept in `last_error`.
pub trait Transport: Send + Sync {
    fn send<'a>(
        &'a self,
        to: &'a str,
        subject: &'a str,
        text: &'a str,
        html: Option<&'a str>,
    ) -> SendFuture<'a>;
}

/// Writes the whole message to the log, codes included. For local development
/// only: used when no SMTP settings exist.
pub struct LogTransport;

impl Transport for LogTransport {
    fn send<'a>(
        &'a self,
        to: &'a str,
        subject: &'a str,
        text: &'a str,
        _html: Option<&'a str>,
    ) -> SendFuture<'a> {
        Box::pin(async move {
            tracing::info!("email to {to}\nsubject: {subject}\n\n{text}");
            Ok(())
        })
    }
}

pub struct SmtpTransport {
    inner: AsyncSmtpTransport<Tokio1Executor>,
    from: Mailbox,
}

impl SmtpTransport {
    pub fn new(config: &SmtpConfig, mail_from: &str) -> Result<Self, String> {
        let from: Mailbox = mail_from
            .parse()
            .map_err(|err| format!("invalid MAIL_FROM: {err}"))?;
        let builder = match config {
            SmtpConfig::Url(url) => AsyncSmtpTransport::<Tokio1Executor>::from_url(url)
                .map_err(|err| format!("invalid SMTP_URL: {err}"))?,
            SmtpConfig::Parts {
                host,
                port,
                tls,
                credentials,
            } => {
                let builder = match tls {
                    SmtpTls::Implicit => AsyncSmtpTransport::<Tokio1Executor>::relay(host),
                    SmtpTls::StartTls => AsyncSmtpTransport::<Tokio1Executor>::starttls_relay(host),
                }
                .map_err(|err| format!("invalid SMTP_HOST: {err}"))?
                .port(*port);
                match credentials {
                    Some((user, password)) => {
                        builder.credentials(Credentials::new(user.clone(), password.clone()))
                    }
                    None => builder,
                }
            }
        };
        Ok(Self {
            inner: builder.build(),
            from,
        })
    }
}

impl Transport for SmtpTransport {
    fn send<'a>(
        &'a self,
        to: &'a str,
        subject: &'a str,
        text: &'a str,
        html: Option<&'a str>,
    ) -> SendFuture<'a> {
        Box::pin(async move {
            let to: Mailbox = to
                .parse()
                .map_err(|err| format!("invalid recipient: {err}"))?;
            let builder = Message::builder()
                .from(self.from.clone())
                .to(to)
                .subject(subject);
            let message = match html {
                Some(html) => builder.multipart(MultiPart::alternative_plain_html(
                    text.to_owned(),
                    html.to_owned(),
                )),
                None => builder.singlepart(SinglePart::plain(text.to_owned())),
            }
            .map_err(|err| format!("cannot build message: {err}"))?;
            self.inner
                .send(message)
                .await
                .map(|_| ())
                .map_err(|err| err.to_string())
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builds_from_parts_and_url_without_connecting() {
        for tls in [SmtpTls::Implicit, SmtpTls::StartTls] {
            let parts = SmtpConfig::Parts {
                host: "smtp.example.com".into(),
                port: if tls == SmtpTls::Implicit { 465 } else { 587 },
                tls,
                credentials: Some(("user".into(), "pw".into())),
            };
            assert!(SmtpTransport::new(&parts, "curl-runner <no-reply@example.com>").is_ok());
        }
        let url = SmtpConfig::Url("smtps://u:p@smtp.example.com:465".into());
        assert!(SmtpTransport::new(&url, "no-reply@example.com").is_ok());
        assert!(SmtpTransport::new(&url, "not an address").is_err());
    }
}
