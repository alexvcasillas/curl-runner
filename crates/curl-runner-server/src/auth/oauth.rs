//! GitHub and Google sign-in: `start` returns the consent URL, `callback`
//! trades the provider's code for a session.

use axum::Json;
use axum::extract::{Path, State};
use base64::Engine as _;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use chrono::Duration;
use curl_runner_cloud_proto::ErrorCode;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::auth::email::Client;
use crate::auth::providers::Provider;
use crate::auth::session::{UserBody, create_session, ensure_not_suspended};
use crate::error::ApiError;
use crate::extract::{ApiJson, ClientIp};
use crate::hash::{new_token, secret_hash};
use crate::state::AppState;

const STATE_TTL: Duration = Duration::minutes(10);
const WINDOW: Duration = Duration::minutes(15);
const PER_IP: u32 = 20;

/// PKCE S256: `base64url(SHA-256(verifier))`, no padding.
pub fn pkce_challenge(verifier: &str) -> String {
    URL_SAFE_NO_PAD.encode(Sha256::digest(verifier.as_bytes()))
}

fn provider_from(name: &str) -> Result<Provider, ApiError> {
    Provider::parse(name).ok_or_else(ApiError::not_found)
}

/// Where the dashboard sends the user after sign-in: a path on its own origin.
fn valid_return_to(path: &str) -> bool {
    path.starts_with('/')
        && !path.starts_with("//")
        && !path.contains('\\')
        && !path.contains(char::is_control)
        && path.len() <= 2048
}

pub fn redirect_uri(app_url: &str, provider: Provider) -> String {
    format!("{app_url}/sign-in/{}/callback", provider.as_str())
}

#[derive(Deserialize)]
pub struct StartRequest {
    client: Client,
    return_to: Option<String>,
}

#[derive(Serialize)]
pub struct StartBody {
    url: String,
    /// Secret for the starting browser to keep (an HttpOnly cookie) and send
    /// back at callback, so a forged callback link cannot sign a victim in.
    binding: String,
}

pub async fn start(
    State(state): State<AppState>,
    Path(provider): Path<String>,
    ClientIp(ip): ClientIp,
    ApiJson(req): ApiJson<StartRequest>,
) -> Result<Json<StartBody>, ApiError> {
    let provider = provider_from(&provider)?;
    state
        .limiter
        .check(&format!("oauth-start:ip:{ip}"), PER_IP, WINDOW)?;
    if req
        .return_to
        .as_deref()
        .is_some_and(|p| !valid_return_to(p))
    {
        return Err(ApiError::new(
            ErrorCode::InvalidRequest,
            "return_to must be a path on this site.",
        ));
    }
    let pepper = state.config.secret_key.as_bytes();
    let oauth_state = new_token("");
    let verifier = new_token("");
    let binding = new_token("crb_");
    let url = state
        .oauth
        .authorize_url(
            provider,
            &oauth_state,
            &pkce_challenge(&verifier),
            &redirect_uri(&state.config.app_url, provider),
        )
        .ok_or_else(ApiError::not_found)?;
    let now = state.clock.now();
    sqlx::query("DELETE FROM oauth_states WHERE expires_at <= $1")
        .bind(now)
        .execute(&state.db)
        .await?;
    sqlx::query(
        "INSERT INTO oauth_states (state_hash, provider, verifier, return_to, expires_at, client, binding_hash) \
         VALUES ($1, $2, $3, $4, $5, $6, $7)",
    )
    .bind(secret_hash(pepper, &oauth_state))
    .bind(provider.as_str())
    .bind(&verifier)
    .bind(&req.return_to)
    .bind(now + STATE_TTL)
    .bind(req.client.as_str())
    .bind(secret_hash(pepper, &binding))
    .execute(&state.db)
    .await?;
    Ok(Json(StartBody { url, binding }))
}

#[derive(Deserialize)]
pub struct CallbackRequest {
    code: String,
    state: String,
    /// Optional on the wire so a missing one is rejected after the state is spent.
    binding: Option<String>,
}

#[derive(Serialize)]
pub struct CallbackBody {
    session: String,
    user: UserBody,
    return_to: Option<String>,
    /// `web` or `device`: for `device` the dashboard now calls `/v1/auth/handoff`.
    client: String,
}

#[derive(sqlx::FromRow)]
struct StateRow {
    verifier: String,
    return_to: Option<String>,
    client: String,
    binding_hash: String,
    expires_at: chrono::DateTime<chrono::Utc>,
}

pub async fn callback(
    State(state): State<AppState>,
    Path(provider): Path<String>,
    ClientIp(ip): ClientIp,
    ApiJson(req): ApiJson<CallbackRequest>,
) -> Result<Json<CallbackBody>, ApiError> {
    let provider = provider_from(&provider)?;
    state
        .limiter
        .check(&format!("oauth-callback:ip:{ip}"), PER_IP, WINDOW)?;
    let pepper = state.config.secret_key.as_bytes();
    let now = state.clock.now();

    // Consume the state first: a replay, or a failed exchange, cannot retry it.
    let row: Option<StateRow> = sqlx::query_as(
        "DELETE FROM oauth_states WHERE state_hash = $1 AND provider = $2 \
         RETURNING verifier, return_to, client, binding_hash, expires_at",
    )
    .bind(secret_hash(pepper, req.state.trim()))
    .bind(provider.as_str())
    .fetch_optional(&state.db)
    .await?;
    let row = row
        .filter(|r| {
            r.expires_at > now
                && r.binding_hash
                    == secret_hash(pepper, req.binding.as_deref().unwrap_or_default().trim())
        })
        .ok_or_else(|| {
            ApiError::new(
                ErrorCode::InvalidRequest,
                "That sign-in has expired. Start again.",
            )
        })?;

    let identity = state
        .oauth
        .exchange(
            provider,
            req.code.trim(),
            &row.verifier,
            &redirect_uri(&state.config.app_url, provider),
        )
        .await
        .map_err(|reason| {
            tracing::warn!("{} sign-in refused: {reason}", provider.as_str());
            ApiError::new(
                ErrorCode::Unauthorized,
                "We could not verify your account with that provider.",
            )
        })?;
    let email = identity.email.trim().to_lowercase();

    let mut tx = state.db.begin().await?;
    let existing: Option<UserBody> = sqlx::query_as(
        "SELECT u.id, u.email::text AS email, u.name FROM identities i \
         JOIN users u ON u.id = i.user_id WHERE i.provider = $1 AND i.subject = $2",
    )
    .bind(provider.as_str())
    .bind(&identity.subject)
    .fetch_optional(&mut *tx)
    .await?;
    let user = match existing {
        Some(user) => {
            sqlx::query("UPDATE identities SET email = $3 WHERE provider = $1 AND subject = $2")
                .bind(provider.as_str())
                .bind(&identity.subject)
                .bind(&email)
                .execute(&mut *tx)
                .await?;
            user
        }
        None => {
            // Link to the user with this (provider-verified) email, or create one.
            let user: UserBody = sqlx::query_as(
                "INSERT INTO users (email, name, created_at) VALUES ($1, $2, $3) \
                 ON CONFLICT (email) DO UPDATE SET name = COALESCE(users.name, EXCLUDED.name) \
                 RETURNING id, email::text AS email, name",
            )
            .bind(&email)
            .bind(&identity.name)
            .bind(now)
            .fetch_one(&mut *tx)
            .await?;
            sqlx::query(
                "INSERT INTO identities (user_id, provider, subject, email, created_at) \
                 VALUES ($1, $2, $3, $4, $5) ON CONFLICT (provider, subject) DO NOTHING",
            )
            .bind(user.id)
            .bind(provider.as_str())
            .bind(&identity.subject)
            .bind(&email)
            .bind(now)
            .execute(&mut *tx)
            .await?;
            user
        }
    };
    ensure_not_suspended(&mut tx, user.id).await?;
    let session = create_session(&mut tx, &state, user.id, "web", None).await?;
    tx.commit().await?;
    Ok(Json(CallbackBody {
        session,
        user,
        return_to: row.return_to,
        client: row.client,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pkce_challenge_matches_rfc_7636_vector() {
        assert_eq!(
            pkce_challenge("dBjftJeZ4CVP-mB92K27uhbUJU1p1r_wW1gFWFOEjXk"),
            "E9Melhoa2OwvFrEMTJguCHaoeK1t8URWbuGJSstw-cM"
        );
    }

    #[test]
    fn return_to_must_be_a_local_path() {
        for ok in ["/", "/teams/acme?tab=1", "/a/b#c"] {
            assert!(valid_return_to(ok), "{ok}");
        }
        for bad in [
            "",
            "teams",
            "//evil.example.com",
            "https://evil.example.com",
            "/\\evil.example.com",
            "/a\nb",
        ] {
            assert!(!valid_return_to(bad), "{bad:?}");
        }
    }

    #[test]
    fn redirect_uri_is_under_the_app_url() {
        assert_eq!(
            redirect_uri("https://app.example.com", Provider::Github),
            "https://app.example.com/sign-in/github/callback"
        );
    }
}
