//! `GET /v1/info`: what this server offers, so clients can adapt.

use axum::Json;
use axum::extract::State;
use curl_runner_cloud_proto::{Banner, BannerTone, ServerInfo, SignInMethod};

use crate::auth::UserAuth;
use crate::config::Config;
use crate::error::ApiError;
use crate::settings::Settings;
use crate::state::AppState;

/// `auth` is a `Result` on purpose: this route is public, and older CLIs send
/// a publish token or an expired session to it. Only a valid session counts
/// as signed in; anything else is an anonymous caller, never a `401`.
///
/// Discovery answers while the database is down: the settings fall back to
/// the last ones read, else to the env's (logged), never a `503`.
pub async fn info(
    State(state): State<AppState>,
    auth: Result<UserAuth, ApiError>,
) -> Json<ServerInfo> {
    let settings = state.settings_or_last().await;
    Json(server_info(
        &state.config,
        &settings,
        state.billing.is_some(),
        auth.is_ok(),
    ))
}

/// Email sign-in is always on; GitHub and Google appear when configured.
pub fn server_info(
    config: &Config,
    settings: &Settings,
    billing: bool,
    signed_in: bool,
) -> ServerInfo {
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
        signup: settings.signup_mode,
        billing,
        app_url: Some(config.app_url.clone()),
        banner: banner(settings, signed_in),
    }
}

/// The banner this caller may see: none without a message, and none for an
/// anonymous caller when it is for signed-in people only.
fn banner(settings: &Settings, signed_in: bool) -> Option<Banner> {
    let message = settings.banner_message.as_deref()?.trim();
    if message.is_empty() || (settings.banner_audience == "signed_in" && !signed_in) {
        return None;
    }
    Some(Banner {
        message: message.to_owned(),
        tone: if settings.banner_tone == "warning" {
            BannerTone::Warning
        } else {
            BannerTone::Info
        },
        link: settings.banner_link.clone(),
        version: settings.banner_version,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn with_banner(audience: &str) -> Settings {
        Settings {
            banner_message: Some("  Maintenance  ".into()),
            banner_audience: audience.into(),
            banner_tone: "warning".into(),
            banner_version: 4,
            ..Settings::default()
        }
    }

    #[test]
    fn banner_follows_the_audience() {
        let all = with_banner("everyone");
        let b = banner(&all, false).unwrap();
        assert_eq!(
            (b.message.as_str(), b.tone, b.version),
            ("Maintenance", BannerTone::Warning, 4)
        );
        let signed = with_banner("signed_in");
        assert!(banner(&signed, false).is_none());
        assert!(banner(&signed, true).is_some());
    }

    #[test]
    fn no_message_no_banner() {
        assert!(banner(&Settings::default(), true).is_none());
        let blank = Settings {
            banner_message: Some("   ".into()),
            ..Settings::default()
        };
        assert!(banner(&blank, true).is_none());
    }
}
