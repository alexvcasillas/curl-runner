//! One-click unsubscribe from product emails (broadcasts).
//!
//! The token is `base64url(user_id || HMAC-SHA256(SECRET_KEY, "unsubscribe:" || user_id))`
//! with the id's 16 raw bytes. It never expires and needs no sign-in: it can
//! only turn product emails off, and doing it twice is the same as once.
//!
//! `GET` only shows a confirm page, so a link scanner that fetches the link
//! changes nothing. `POST` (the page's button, or a mail client's RFC 8058
//! one-click `List-Unsubscribe=One-Click` form post, token in the query)
//! turns them off. Every answer is an HTML page, and a bad token gets the
//! same page as a good one, so tokens cannot be probed.

use axum::Router;
use axum::extract::rejection::QueryRejection;
use axum::extract::{Query, State};
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

use super::templates::{confirm_page, notice_page};
use crate::config::Config;
use crate::extract::ClientIp;
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
    Router::new().route("/v1/unsubscribe", get(confirm).post(unsubscribe))
}

#[derive(Deserialize)]
pub struct UnsubscribeQuery {
    token: Option<String>,
}

fn html(status: StatusCode, page: String) -> Response {
    (
        status,
        [
            (header::CONTENT_TYPE, "text/html; charset=utf-8"),
            (header::CACHE_CONTROL, "no-store"),
            (header::REFERRER_POLICY, "no-referrer"),
        ],
        page,
    )
        .into_response()
}

/// The per-IP limit: the page to answer once it is hit.
fn limited(state: &AppState, ip: &str) -> Option<Response> {
    state
        .limiter
        .check(
            &format!("unsubscribe:ip:{ip}"),
            PER_MINUTE,
            Duration::minutes(1),
        )
        .map_err(|err| {
            let mut res = html(
                StatusCode::TOO_MANY_REQUESTS,
                notice_page(
                    "Too many tries.",
                    "Wait a minute, then open the link from the email again.",
                ),
            );
            if let Some(secs) = err.retry_after
                && let Ok(value) = secs.to_string().parse()
            {
                res.headers_mut().insert(header::RETRY_AFTER, value);
            }
            res
        })
        .err()
}

/// The token from the query, or `None` for a missing or unreadable one.
fn token_of(q: Result<Query<UnsubscribeQuery>, QueryRejection>) -> Option<String> {
    q.ok().and_then(|Query(q)| q.token)
}

/// `GET /v1/unsubscribe?token=`: asks first, changes nothing.
async fn confirm(
    State(state): State<AppState>,
    ClientIp(ip): ClientIp,
    q: Result<Query<UnsubscribeQuery>, QueryRejection>,
) -> Response {
    if let Some(res) = limited(&state, &ip) {
        return res;
    }
    // Only base64url characters go back into the form's URL.
    let token: String = token_of(q)
        .unwrap_or_default()
        .chars()
        .filter(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_'))
        .take(128)
        .collect();
    html(
        StatusCode::OK,
        confirm_page(
            "Stop product emails from curl-runner?",
            "You\u{2019}ll still get sign-in codes, invites and billing emails.",
            &format!("{}/v1/unsubscribe?token={token}", state.config.api_url),
            "Unsubscribe",
        ),
    )
}

/// `POST /v1/unsubscribe?token=`: turn product emails off. The body is
/// ignored: the confirm page's form sends none, a one-click client sends
/// `List-Unsubscribe=One-Click`.
async fn unsubscribe(
    State(state): State<AppState>,
    ClientIp(ip): ClientIp,
    q: Result<Query<UnsubscribeQuery>, QueryRejection>,
) -> Response {
    if let Some(res) = limited(&state, &ip) {
        return res;
    }
    if let Some(user_id) = token_of(q).and_then(|t| verify(state.config.secret_key.as_bytes(), &t))
    {
        let done =
            sqlx::query("UPDATE users SET product_emails = false WHERE id = $1 AND product_emails")
                .bind(user_id)
                .execute(&state.db)
                .await;
        if let Err(err) = done {
            tracing::error!("unsubscribe: {err}");
            return html(
                StatusCode::SERVICE_UNAVAILABLE,
                notice_page(
                    "We couldn't do that right now.",
                    "Try the link again in a few minutes.",
                ),
            );
        }
    }
    html(
        StatusCode::OK,
        notice_page(
            "You won't get product emails from curl-runner anymore.",
            "Sign-in codes, invites and billing emails still reach you.",
        ),
    )
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
