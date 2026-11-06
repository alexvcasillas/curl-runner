//! One-click unsubscribe from product emails (broadcasts).
//!
//! The token is `base64url(user_id || HMAC-SHA256(SECRET_KEY, "unsubscribe:" || user_id))`
//! with the id's 16 raw bytes. It never expires and needs no sign-in: it can
//! only turn product emails off, and doing it twice is the same as once.

use axum::Router;
use axum::extract::State;
use axum::http::{StatusCode, header};
use axum::response::{IntoResponse, Response};
use axum::routing::get;
use base64::Engine;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use chrono::Duration;
use hmac::{Hmac, Mac};
use serde::Deserialize;
use sha2::Sha256;
use uuid::Uuid;

use super::templates::notice_page;
use crate::config::Config;
use crate::error::ApiError;
use crate::extract::{ApiQuery, ClientIp};
use crate::state::AppState;

const PREFIX: &[u8] = b"unsubscribe:";
/// Unsubscribe requests per IP per minute.
const PER_MINUTE: u32 = 30;

fn mac(secret: &[u8], user_id: Uuid) -> Hmac<Sha256> {
    let mut mac = Hmac::<Sha256>::new_from_slice(secret).expect("HMAC takes any key length");
    mac.update(PREFIX);
    mac.update(user_id.as_bytes());
    mac
}

/// The unsubscribe token of `user_id`.
pub fn token(secret: &[u8], user_id: Uuid) -> String {
    let mut raw = user_id.as_bytes().to_vec();
    raw.extend_from_slice(&mac(secret, user_id).finalize().into_bytes());
    URL_SAFE_NO_PAD.encode(raw)
}

/// The user a token was made for, if its MAC checks out (compared in
/// constant time).
pub fn verify(secret: &[u8], token: &str) -> Option<Uuid> {
    let raw = URL_SAFE_NO_PAD.decode(token.trim()).ok()?;
    if raw.len() != 16 + 32 {
        return None;
    }
    let user_id = Uuid::from_slice(&raw[..16]).ok()?;
    mac(secret, user_id).verify_slice(&raw[16..]).ok()?;
    Some(user_id)
}

/// The link in `user_id`'s broadcast copies.
pub fn link(config: &Config, user_id: Uuid) -> String {
    format!(
        "{}/v1/unsubscribe?token={}",
        config.api_url,
        token(config.secret_key.as_bytes(), user_id)
    )
}

pub fn routes() -> Router<AppState> {
    Router::new().route("/v1/unsubscribe", get(unsubscribe))
}

#[derive(Deserialize)]
pub struct UnsubscribeQuery {
    token: Option<String>,
}

fn page(status: StatusCode, heading: &str, body: &str) -> Response {
    (
        status,
        [
            (header::CONTENT_TYPE, "text/html; charset=utf-8"),
            (header::CACHE_CONTROL, "no-store"),
        ],
        notice_page(heading, body),
    )
        .into_response()
}

/// `GET /v1/unsubscribe?token=`: turn product emails off for the token's
/// user and answer a small page. A deleted user gets the same page.
async fn unsubscribe(
    State(state): State<AppState>,
    ClientIp(ip): ClientIp,
    ApiQuery(q): ApiQuery<UnsubscribeQuery>,
) -> Result<Response, ApiError> {
    state.limiter.check(
        &format!("unsubscribe:ip:{ip}"),
        PER_MINUTE,
        Duration::minutes(1),
    )?;
    let user_id = q
        .token
        .as_deref()
        .and_then(|t| verify(state.config.secret_key.as_bytes(), t));
    let Some(user_id) = user_id else {
        return Ok(page(
            StatusCode::BAD_REQUEST,
            "This unsubscribe link doesn't work.",
            "Copy the whole link from the email and try again, or reply to the email to ask us.",
        ));
    };
    sqlx::query("UPDATE users SET product_emails = false WHERE id = $1 AND product_emails")
        .bind(user_id)
        .execute(&state.db)
        .await?;
    Ok(page(
        StatusCode::OK,
        "You won't get product emails from curl-runner anymore.",
        "Sign-in codes, invites and billing emails still reach you.",
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    const KEY: &[u8] = b"test-secret-key-test-secret-key-0123";

    #[test]
    fn token_round_trips_and_rejects_tampering() {
        let id = Uuid::new_v4();
        let t = token(KEY, id);
        assert_eq!(t.len(), 64);
        assert_eq!(verify(KEY, &t), Some(id));
        assert_eq!(verify(b"other-key-other-key-other-key-000000", &t), None);
        let mut raw = URL_SAFE_NO_PAD.decode(&t).unwrap();
        raw[0] ^= 1;
        assert_eq!(verify(KEY, &URL_SAFE_NO_PAD.encode(&raw)), None);
        assert_eq!(verify(KEY, &t[..60]), None);
        assert_eq!(verify(KEY, "!!"), None);
    }
}
