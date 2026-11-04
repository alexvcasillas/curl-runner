//! `GET /v1/info`: what this server offers, so clients can adapt.

use axum::Json;
use axum::extract::State;
use curl_runner_cloud_proto::{ServerInfo, SignInMethod};

use crate::config::Config;
use crate::state::AppState;

pub async fn info(State(state): State<AppState>) -> Json<ServerInfo> {
    Json(server_info(&state.config))
}

/// Email sign-in is always on; GitHub and Google appear when configured.
pub fn server_info(config: &Config) -> ServerInfo {
    let mut sign_in = vec![SignInMethod::Email];
    if config.github.is_some() {
        sign_in.push(SignInMethod::Github);
    }
    if config.google.is_some() {
        sign_in.push(SignInMethod::Google);
    }
    ServerInfo {
        version: env!("CARGO_PKG_VERSION").to_owned(),
        sign_in,
        signup: config.signup,
        billing: false,
    }
}
