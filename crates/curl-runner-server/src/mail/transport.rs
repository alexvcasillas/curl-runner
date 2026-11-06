//! Delivery backends: SMTP via lettre, or the log for local development.

use std::future::Future;
use std::pin::Pin;

use lettre::message::header::{HeaderName, HeaderValue};
use lettre::message::{Mailbox, MultiPart, SinglePart};
use lettre::transport::smtp::authentication::Credentials;
use lettre::{AsyncSmtpTransport, AsyncTransport, Message, Tokio1Executor};

use crate::config::{SmtpConfig, SmtpTls};

pub type SendFuture<'a> = Pin<Box<dyn Future<Output = Result<(), String>> + Send + 'a>>;

/// Extra headers on one message, such as `List-Unsubscribe`. Names are
/// fixed ASCII; values come from the server, never from user text.
pub type Headers = [(&'static str, String)];

/// Sends one email. The error is a short description kept in `last_error`.
pub trait Transport: Send + Sync {
    fn send<'a>(
        &'a self,
        to: &'a str,
        subject: &'a str,
        text: &'a str,
        html: Option<&'a str>,
        headers: &'a Headers,
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
        headers: &'a Headers,
    ) -> SendFuture<'a> {
        Box::pin(async move {
            let headers: String = headers
                .iter()
                .map(|(name, value)| format!("{name}: {value}\n"))
                .collect();
            tracing::info!("email to {to}\nsubject: {subject}\n{headers}\n{text}");
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
        let builder = match config.tls {
            SmtpTls::Implicit => AsyncSmtpTransport::<Tokio1Executor>::relay(&config.host),
            SmtpTls::StartTls => AsyncSmtpTransport::<Tokio1Executor>::starttls_relay(&config.host),
        }
        .map_err(|err| format!("invalid SMTP_HOST: {err}"))?
        .port(config.port);
        let builder = match &config.credentials {
            Some((user, password)) => {
                builder.credentials(Credentials::new(user.clone(), password.clone()))
            }
            None => builder,
        };
        Ok(Self {
            inner: builder.build(),
            from,
        })
    }
}

/// The message `SmtpTransport` sends, built without connecting.
pub fn build_message(
    from: &Mailbox,
    to: &str,
    subject: &str,
    text: &str,
    html: Option<&str>,
    headers: &Headers,
) -> Result<Message, String> {
    let to: Mailbox = to
        .parse()
        .map_err(|err| format!("invalid recipient: {err}"))?;
    let mut builder = Message::builder()
        .from(from.clone())
        .to(to)
        .subject(subject);
    for (name, value) in headers {
        if value.chars().any(char::is_control) {
            return Err(format!("invalid {name} header"));
        }
        builder = builder.raw_header(HeaderValue::new(
            HeaderName::new_from_ascii_str(name),
            value.clone(),
        ));
    }
    match html {
        Some(html) => builder.multipart(MultiPart::alternative_plain_html(
            text.to_owned(),
            html.to_owned(),
        )),
        None => builder.singlepart(SinglePart::plain(text.to_owned())),
    }
    .map_err(|err| format!("cannot build message: {err}"))
}

impl Transport for SmtpTransport {
    fn send<'a>(
        &'a self,
        to: &'a str,
        subject: &'a str,
        text: &'a str,
        html: Option<&'a str>,
        headers: &'a Headers,
    ) -> SendFuture<'a> {
        Box::pin(async move {
            let message = build_message(&self.from, to, subject, text, html, headers)?;
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
    fn builds_without_connecting() {
        for tls in [SmtpTls::Implicit, SmtpTls::StartTls] {
            let config = SmtpConfig {
                host: "smtp.example.com".into(),
                port: if tls == SmtpTls::Implicit { 465 } else { 587 },
                tls,
                credentials: Some(("user".into(), "pw".into())),
            };
            assert!(SmtpTransport::new(&config, "curl-runner <no-reply@example.com>").is_ok());
            assert!(SmtpTransport::new(&config, "not an address").is_err());
        }
    }

    #[test]
    fn message_carries_extra_headers() {
        let from: Mailbox = "curl-runner <no-reply@example.com>".parse().unwrap();
        let headers = [
            (
                "List-Unsubscribe",
                "<https://api.example.com/v1/unsubscribe?token=abc>".to_owned(),
            ),
            (
                "List-Unsubscribe-Post",
                "List-Unsubscribe=One-Click".to_owned(),
            ),
        ];
        let raw = build_message(
            &from,
            "ada@example.com",
            "Hi",
            "Hello",
            Some("<p>Hello</p>"),
            &headers,
        )
        .unwrap()
        .formatted();
        let raw = String::from_utf8_lossy(&raw);
        assert!(
            raw.contains(
                "List-Unsubscribe: <https://api.example.com/v1/unsubscribe?token=abc>\r\n"
            ),
            "{raw}"
        );
        assert!(
            raw.contains("List-Unsubscribe-Post: List-Unsubscribe=One-Click\r\n"),
            "{raw}"
        );
        let plain = build_message(&from, "ada@example.com", "Hi", "Hello", None, &[])
            .unwrap()
            .formatted();
        assert!(!String::from_utf8_lossy(&plain).contains("List-Unsubscribe"));
        let bad = [("List-Unsubscribe", "<x>\r\nBcc: y".to_owned())];
        assert!(build_message(&from, "ada@example.com", "Hi", "Hello", None, &bad).is_err());
    }
}
