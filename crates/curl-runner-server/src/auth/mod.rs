//! Sign-in and sessions.

pub mod email;
pub mod handoff;
pub mod oauth;
pub mod providers;
pub mod session;

use axum::Router;
use axum::routing::{delete, get, post};

use crate::state::AppState;

pub use session::UserAuth;

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/v1/auth/email/start", post(email::start))
        .route("/v1/auth/email/verify", post(email::verify))
        .route("/v1/auth/oauth/{provider}/start", post(oauth::start))
        .route("/v1/auth/oauth/{provider}/callback", post(oauth::callback))
        .route("/v1/auth/handoff", post(handoff::create))
        .route("/v1/auth/handoff/redeem", post(handoff::redeem))
        .route("/v1/auth/sign-out", post(session::sign_out))
        .route("/v1/me", get(session::me))
        .route("/v1/sessions", get(session::list))
        .route("/v1/sessions/{id}", delete(session::revoke))
}
