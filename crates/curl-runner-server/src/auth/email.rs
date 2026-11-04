//! Email code sign-in: `start` mails a code and a link, `verify` trades either
//! for a session. Codes and link tokens are stored hashed.

use axum::Json;
use axum::extract::State;
use axum::http::StatusCode;
use chrono::{DateTime, Duration, Utc};
use curl_runner_cloud_proto::ErrorCode;
use rand::Rng as _;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::auth::session::{UserBody, create_session};
use crate::error::ApiError;
use crate::extract::{ApiJson, ClientIp};
use crate::hash::{new_token, secret_hash};
use crate::mail::Mail;
use crate::state::AppState;

const CODE_TTL: Duration = Duration::minutes(10);
/// Wrong guesses allowed per code; the next try finds it expired.
const MAX_ATTEMPTS: i32 = 5;
const START_WINDOW: Duration = Duration::minutes(15);
const START_PER_EMAIL: u32 = 5;
const START_PER_IP: u32 = 20;
/// A daily cap on mail to one address, on top of the 15 minute window.
const START_DAY: Duration = Duration::hours(24);
const START_PER_EMAIL_PER_DAY: u32 = 20;
const VERIFY_WINDOW: Duration = Duration::minutes(15);
const VERIFY_PER_IP: u32 = 30;
const VERIFY_PER_EMAIL: u32 = 10;
const MAX_LABEL: usize = 100;

#[derive(Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Client {
    Web,
    Device,
}

impl Client {
    pub(crate) fn as_str(&self) -> &'static str {
        match self {
            Self::Web => "web",
            Self::Device => "device",
        }
    }
}

#[derive(Deserialize)]
pub struct StartRequest {
    email: String,
    client: Client,
    label: Option<String>,
}

fn invalid(message: &str) -> ApiError {
    ApiError::new(ErrorCode::InvalidRequest, message)
}

/// Trim and lowercase; reject anything that is not plausibly an address.
fn normalize_email(raw: &str) -> Result<String, ApiError> {
    let email = raw.trim().to_lowercase();
    let ok = email.len() <= 254
        && email
            .split_once('@')
            .is_some_and(|(local, domain)| !local.is_empty() && domain.contains('.'))
        && !email.contains(char::is_whitespace);
    if ok {
        Ok(email)
    } else {
        Err(invalid("Enter a valid email address."))
    }
}

/// Six digits from the CSPRNG, zero-padded.
fn new_code() -> String {
    format!("{:06}", rand::rng().random_range(0..1_000_000u32))
}

pub async fn start(
    State(state): State<AppState>,
    ClientIp(ip): ClientIp,
    ApiJson(req): ApiJson<StartRequest>,
) -> Result<(StatusCode, Json<serde_json::Value>), ApiError> {
    let email = normalize_email(&req.email)?;
    let label = req
        .label
        .map(|l| l.trim().to_owned())
        .filter(|l| !l.is_empty());
    if label
        .as_ref()
        .is_some_and(|l| l.chars().count() > MAX_LABEL)
    {
        return Err(invalid("The label is too long."));
    }
    state
        .limiter
        .check(&format!("email-start:ip:{ip}"), START_PER_IP, START_WINDOW)?;
    state.limiter.check(
        &format!("email-start:email:{email}"),
        START_PER_EMAIL,
        START_WINDOW,
    )?;
    state.limiter.check(
        &format!("email-start:email-day:{email}"),
        START_PER_EMAIL_PER_DAY,
        START_DAY,
    )?;

    let pepper = state.config.secret_key.as_bytes();
    let code = new_code();
    // A device signs in with the code alone. A link opened in a browser
    // would spend the code the terminal is waiting to use.
    let link_token = match req.client {
        Client::Web => Some(new_token("crl_")),
        Client::Device => None,
    };
    let link = link_token.as_ref().map(|token| {
        format!(
            "{}/sign-in/email?token={token}&client=web",
            state.config.app_url
        )
    });
    let mut tx = state.db.begin().await?;
    // Serialise starts for one address so concurrent ones leave a single live code.
    sqlx::query("SELECT pg_advisory_xact_lock(hashtext($1))")
        .bind(&email)
        .execute(&mut *tx)
        .await?;
    sqlx::query("DELETE FROM email_codes WHERE email = $1 AND used_at IS NULL")
        .bind(&email)
        .execute(&mut *tx)
        .await?;
    sqlx::query(
        "INSERT INTO email_codes (email, code_hash, link_hash, expires_at, client, label) \
         VALUES ($1, $2, $3, $4, $5, $6)",
    )
    .bind(&email)
    .bind(secret_hash(pepper, &code))
    .bind(link_token.as_ref().map(|t| secret_hash(pepper, t)))
    .bind(state.clock.now() + CODE_TTL)
    .bind(req.client.as_str())
    .bind(&label)
    .execute(&mut *tx)
    .await?;
    state
        .mailer
        .enqueue(
            &mut tx,
            Mail::SignInCode {
                to: email,
                code,
                link,
            },
        )
        .await?;
    tx.commit().await?;
    Ok((StatusCode::ACCEPTED, Json(serde_json::json!({}))))
}

#[derive(Deserialize)]
pub struct VerifyRequest {
    email: Option<String>,
    code: Option<String>,
    token: Option<String>,
}

#[derive(Serialize)]
pub struct VerifyBody {
    session: String,
    user: UserBody,
}

#[derive(sqlx::FromRow)]
struct CodeRow {
    id: Uuid,
    email: String,
    code_hash: String,
    attempts: i32,
    expires_at: DateTime<Utc>,
    client: String,
    label: Option<String>,
}

fn code_invalid() -> ApiError {
    ApiError::new(ErrorCode::CodeInvalid, "That code is not right.")
}

fn code_expired() -> ApiError {
    ApiError::new(
        ErrorCode::CodeExpired,
        "That code has expired. Ask for a new one.",
    )
}

pub async fn verify(
    State(state): State<AppState>,
    ClientIp(ip): ClientIp,
    ApiJson(req): ApiJson<VerifyRequest>,
) -> Result<Json<VerifyBody>, ApiError> {
    state.limiter.check(
        &format!("email-verify:ip:{ip}"),
        VERIFY_PER_IP,
        VERIFY_WINDOW,
    )?;
    if let Some(email) = &req.email {
        state.limiter.check(
            &format!("email-verify:email:{}", normalize_email(email)?),
            VERIFY_PER_EMAIL,
            VERIFY_WINDOW,
        )?;
    }
    let pepper = state.config.secret_key.as_bytes();
    let now = state.clock.now();
    let mut tx = state.db.begin().await?;

    // Lock the row so concurrent guesses cannot beat the attempt counter.
    let (row, guess) = match (&req.token, &req.email, &req.code) {
        (Some(token), None, None) => {
            let row: Option<CodeRow> = sqlx::query_as(
                "SELECT id, email::text AS email, code_hash, attempts, expires_at, client, label \
                 FROM email_codes WHERE link_hash = $1 AND used_at IS NULL FOR UPDATE",
            )
            .bind(secret_hash(pepper, token.trim()))
            .fetch_optional(&mut *tx)
            .await?;
            (row, None)
        }
        (None, Some(email), Some(code)) => {
            let email = normalize_email(email)?;
            let row: Option<CodeRow> = sqlx::query_as(
                "SELECT id, email::text AS email, code_hash, attempts, expires_at, client, label \
                 FROM email_codes WHERE email = $1 AND used_at IS NULL \
                 ORDER BY expires_at DESC LIMIT 1 FOR UPDATE",
            )
            .bind(&email)
            .fetch_optional(&mut *tx)
            .await?;
            (row, Some(secret_hash(pepper, code.trim())))
        }
        _ => return Err(invalid("Send an email and code, or a token.")),
    };
    let row = row.ok_or_else(code_invalid)?;
    if req.token.is_some() && row.client == Client::Device.as_str() {
        // Device codes are made without a link. Refuse one found anyway, so a
        // browser can never spend a code the terminal is waiting on.
        return Err(invalid("Enter this code in your terminal."));
    }
    if row.expires_at <= now || row.attempts >= MAX_ATTEMPTS {
        return Err(code_expired());
    }
    if guess.is_some_and(|g| g != row.code_hash) {
        sqlx::query("UPDATE email_codes SET attempts = attempts + 1 WHERE id = $1")
            .bind(row.id)
            .execute(&mut *tx)
            .await?;
        tx.commit().await?;
        return Err(code_invalid());
    }

    sqlx::query("UPDATE email_codes SET used_at = $2 WHERE id = $1")
        .bind(row.id)
        .bind(now)
        .execute(&mut *tx)
        .await?;
    // A no-op update on conflict, so RETURNING yields the existing user too.
    let user: UserBody = sqlx::query_as(
        "INSERT INTO users (email, created_at) VALUES ($1, $2) \
         ON CONFLICT (email) DO UPDATE SET email = EXCLUDED.email \
         RETURNING id, email::text AS email, name",
    )
    .bind(&row.email)
    .bind(now)
    .fetch_one(&mut *tx)
    .await?;
    let session =
        create_session(&mut tx, &state, user.id, &row.client, row.label.as_deref()).await?;
    tx.commit().await?;
    Ok(Json(VerifyBody { session, user }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn email_is_trimmed_and_lowercased() {
        assert_eq!(
            normalize_email("  Jane@Example.COM ").unwrap(),
            "jane@example.com"
        );
    }

    #[test]
    fn bad_emails_are_rejected() {
        for bad in [
            "",
            "jane",
            "@example.com",
            "jane@",
            "jane@localhost",
            "a b@c.de",
        ] {
            assert!(normalize_email(bad).is_err(), "{bad}");
        }
    }

    #[test]
    fn codes_are_six_digits() {
        for _ in 0..200 {
            let code = new_code();
            assert_eq!(code.len(), 6);
            assert!(code.bytes().all(|b| b.is_ascii_digit()));
        }
    }
}
