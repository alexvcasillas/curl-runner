//! `/v1/admin/settings`: the server settings page. Any admin reads. Writes
//! need the `settings` permission and log one `settings.update` event.

use axum::Json;
use axum::extract::State;
use axum::http::StatusCode;
use chrono::Duration;
use curl_runner_cloud_proto::{BannerTone, Plan, SignupMode};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value, json};

use super::auth::{AdminAuth, Perm};
use super::events::{AdminTx, Event};
use super::page::invalid;
use crate::config::Config;
use crate::error::ApiError;
use crate::extract::{ApiJson, ApiPath};
use crate::mail::Mail;
use crate::state::AppState;
use crate::teams::enum_str;

const MAX_DOMAINS: usize = 100;
const MAX_BANNER_CHARS: usize = 280;
const MAX_LINK_CHARS: usize = 500;
const MAX_GRACE_DAYS: u32 = 365;
const TEST_EMAILS_PER_WINDOW: u32 = 5;
const TEST_EMAIL_WINDOW: Duration = Duration::minutes(10);

/// One variable `config.rs` reads. `value` is `null` for a secret, and for a
/// variable that is not set.
#[derive(Serialize, Debug, PartialEq, Eq)]
pub struct EnvVar {
    key: &'static str,
    value: Option<String>,
    secret: bool,
}

/// Every variable `config.rs` reads, in the order the page lists them.
/// Values show only for the allowlist; everything else is a secret.
pub fn env_listing(config: &Config) -> Vec<EnvVar> {
    let shown = |key: &'static str, value: Option<String>| EnvVar {
        key,
        value,
        secret: false,
    };
    let secret = |key: &'static str| EnvVar {
        key,
        value: None,
        secret: true,
    };
    let billing = config.billing.as_ref();
    vec![
        shown("PUBLIC_APP_URL", Some(config.app_url.clone())),
        shown("PUBLIC_API_URL", Some(config.api_url.clone())),
        shown("MAIL_FROM", Some(config.mail_from.clone())),
        shown("SMTP_HOST", config.smtp.as_ref().map(|s| s.host.clone())),
        shown(
            "SMTP_PORT",
            config.smtp.as_ref().map(|s| s.port.to_string()),
        ),
        shown(
            "STRIPE_PRICE_MONTHLY",
            billing.map(|b| b.price_monthly.clone()),
        ),
        shown(
            "STRIPE_PRICE_ANNUAL",
            billing.map(|b| b.price_annual.clone()),
        ),
        shown(
            "TRUSTED_PROXY_HOPS",
            Some(config.trusted_proxy_hops.to_string()),
        ),
        secret("SMTP_USER"),
        secret("SMTP_PASSWORD"),
        secret("GITHUB_CLIENT_ID"),
        secret("GITHUB_CLIENT_SECRET"),
        secret("GOOGLE_CLIENT_ID"),
        secret("GOOGLE_CLIENT_SECRET"),
        secret("STRIPE_SECRET_KEY"),
        secret("STRIPE_WEBHOOK_SECRET"),
        secret("OPERATOR_EMAILS"),
        secret("SECRET_KEY"),
        secret("DATABASE_URL"),
        secret("DB_HOST"),
        secret("DB_PORT"),
        secret("DB_USER"),
        secret("DB_PASSWORD"),
        secret("DB_NAME"),
        // These three only seed the settings row now.
        secret("SIGNUP"),
        secret("SIGNUP_DOMAINS"),
        secret("DEFAULT_PLAN"),
    ]
}

/// The settings row plus the env listing.
async fn view(state: &AppState) -> Result<Json<Value>, ApiError> {
    let settings = state.settings().await?;
    let mut out = serde_json::to_value(&*settings).map_err(|_| ApiError::unavailable())?;
    out["env"] =
        serde_json::to_value(env_listing(&state.config)).map_err(|_| ApiError::unavailable())?;
    Ok(Json(out))
}

pub async fn get(admin: AdminAuth, State(state): State<AppState>) -> Result<Json<Value>, ApiError> {
    let _ = admin;
    view(&state).await
}

/// A lowercase DNS name with at least one dot, such as `acme.com`.
fn valid_domain(domain: &str) -> bool {
    if domain.len() > 253 || !domain.contains('.') {
        return false;
    }
    let labels: Vec<&str> = domain.split('.').collect();
    let label_ok = |l: &&str| {
        (1..=63).contains(&l.len())
            && !l.starts_with('-')
            && !l.ends_with('-')
            && l.bytes()
                .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-')
    };
    labels.iter().all(label_ok)
        && labels
            .last()
            .is_some_and(|tld| !tld.bytes().all(|b| b.is_ascii_digit()))
}

fn clean_domains(raw: &[String]) -> Result<Vec<String>, ApiError> {
    let mut out: Vec<String> = Vec::new();
    for d in raw {
        let d = d.trim().to_ascii_lowercase();
        if !valid_domain(&d) {
            return Err(invalid(format!("{d} is not a valid domain.")));
        }
        if !out.contains(&d) {
            out.push(d);
        }
    }
    if out.len() > MAX_DOMAINS {
        return Err(invalid(format!("List at most {MAX_DOMAINS} domains.")));
    }
    Ok(out)
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct SignupBody {
    mode: SignupMode,
    domains: Vec<String>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct PlansBody {
    default_plan: Plan,
    free_member_limit: i64,
    free_restricted: bool,
    grace_days: i64,
}

#[derive(Deserialize, Clone, Copy)]
#[serde(rename_all = "snake_case")]
enum Audience {
    Everyone,
    SignedIn,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct BannerBody {
    message: Option<String>,
    tone: BannerTone,
    audience: Audience,
    link: Option<String>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct PublishingBody {
    paused: bool,
}

fn body<T: serde::de::DeserializeOwned>(value: Value) -> Result<T, ApiError> {
    serde_json::from_value(value)
        .map_err(|_| invalid("The request body is not valid for this section."))
}

/// A banner link must be an http(s) URL with no whitespace.
fn clean_link(link: Option<String>) -> Result<Option<String>, ApiError> {
    let Some(link) = link.map(|l| l.trim().to_owned()).filter(|l| !l.is_empty()) else {
        return Ok(None);
    };
    let lower = link.to_ascii_lowercase();
    let host_ok = lower
        .strip_prefix("https://")
        .or_else(|| lower.strip_prefix("http://"))
        .is_some_and(|rest| !rest.is_empty() && !rest.starts_with('/'));
    if !host_ok || link.chars().any(|c| c.is_whitespace() || c.is_control()) {
        return Err(invalid("The link must be an http or https URL."));
    }
    if link.chars().count() > MAX_LINK_CHARS {
        return Err(invalid(format!(
            "The link must be at most {MAX_LINK_CHARS} characters."
        )));
    }
    Ok(Some(link))
}

/// An empty message clears the banner.
fn clean_message(message: Option<String>) -> Result<Option<String>, ApiError> {
    let Some(message) = message
        .map(|m| m.trim().to_owned())
        .filter(|m| !m.is_empty())
    else {
        return Ok(None);
    };
    if message.chars().count() > MAX_BANNER_CHARS {
        return Err(invalid(format!(
            "The message must be at most {MAX_BANNER_CHARS} characters."
        )));
    }
    if message.chars().any(char::is_control) {
        return Err(invalid("The message must be one line of text."));
    }
    Ok(Some(message))
}

fn in_range(name: &str, value: i64, min: i64, max: i64) -> Result<i64, ApiError> {
    if (min..=max).contains(&value) {
        Ok(value)
    } else {
        Err(invalid(format!("{name} must be between {min} and {max}.")))
    }
}

/// API field to column, per section: what the event's `from` reads.
fn columns(section: &str) -> &'static [(&'static str, &'static str)] {
    match section {
        "signup" => &[("mode", "signup_mode"), ("domains", "signup_domains")],
        "plans" => &[
            ("default_plan", "default_plan"),
            ("free_member_limit", "free_member_limit"),
            ("free_restricted", "free_restricted"),
            ("grace_days", "grace_days"),
        ],
        "banner" => &[
            ("message", "banner_message"),
            ("tone", "banner_tone"),
            ("audience", "banner_audience"),
            ("link", "banner_link"),
        ],
        _ => &[("paused", "publishing_paused")],
    }
}

pub async fn update(
    admin: AdminAuth,
    State(state): State<AppState>,
    ApiPath(section): ApiPath<String>,
    ApiJson(input): ApiJson<Value>,
) -> Result<Json<Value>, ApiError> {
    admin.require(Perm::Settings)?;
    let section = match section.as_str() {
        "signup" => "signup",
        "plans" => "plans",
        "banner" => "banner",
        "publishing" => "publishing",
        _ => return Err(ApiError::not_found()),
    };
    // Validate everything before opening the transaction. `after` is the new
    // value of each field, keyed by API name.
    let after: Map<String, Value> = match section {
        "signup" => {
            let req: SignupBody = body(input)?;
            let domains = clean_domains(&req.domains)?;
            Map::from_iter([
                ("mode".into(), json!(enum_str(req.mode))),
                ("domains".into(), json!(domains)),
            ])
        }
        "plans" => {
            let req: PlansBody = body(input)?;
            Map::from_iter([
                ("default_plan".into(), json!(enum_str(req.default_plan))),
                (
                    "free_member_limit".into(),
                    json!(in_range(
                        "free_member_limit",
                        req.free_member_limit,
                        1,
                        100
                    )?),
                ),
                ("free_restricted".into(), json!(req.free_restricted)),
                (
                    "grace_days".into(),
                    json!(in_range(
                        "grace_days",
                        req.grace_days,
                        0,
                        i64::from(MAX_GRACE_DAYS)
                    )?),
                ),
            ])
        }
        "banner" => {
            let req: BannerBody = body(input)?;
            Map::from_iter([
                ("message".into(), json!(clean_message(req.message)?)),
                ("tone".into(), json!(enum_str(req.tone))),
                (
                    "audience".into(),
                    json!(match req.audience {
                        Audience::Everyone => "everyone",
                        Audience::SignedIn => "signed_in",
                    }),
                ),
                ("link".into(), json!(clean_link(req.link)?)),
            ])
        }
        _ => {
            let req: PublishingBody = body(input)?;
            Map::from_iter([("paused".into(), json!(req.paused))])
        }
    };

    // The row exists once settings have been read once.
    state.settings().await?;
    let mut tx = AdminTx::begin(&state, &admin).await?;
    let old: Value = sqlx::query_scalar("SELECT to_jsonb(s) FROM server_settings s FOR UPDATE")
        .fetch_one(&mut *tx)
        .await?;
    let before: Map<String, Value> = columns(section)
        .iter()
        .map(|(field, column)| ((*field).to_owned(), old[*column].clone()))
        .collect();
    let at = tx.at();
    let arg = |name: &str| after[name].clone();
    let query = match section {
        "signup" => sqlx::query(
            "UPDATE server_settings SET signup_mode = $3, signup_domains = $4, \
             updated_at = $1, updated_by = $2",
        )
        .bind(at)
        .bind(admin.user_id)
        .bind(arg("mode").as_str().map(str::to_owned))
        .bind(arg("domains").as_array().map(|a| {
            a.iter()
                .filter_map(|d| d.as_str().map(str::to_owned))
                .collect::<Vec<_>>()
        })),
        "plans" => sqlx::query(
            "UPDATE server_settings SET default_plan = $3, free_member_limit = $4, \
             free_restricted = $5, grace_days = $6, updated_at = $1, updated_by = $2",
        )
        .bind(at)
        .bind(admin.user_id)
        .bind(arg("default_plan").as_str().map(str::to_owned))
        .bind(
            arg("free_member_limit")
                .as_i64()
                .and_then(|n| i32::try_from(n).ok()),
        )
        .bind(arg("free_restricted").as_bool())
        .bind(
            arg("grace_days")
                .as_i64()
                .and_then(|n| i32::try_from(n).ok()),
        ),
        "banner" => sqlx::query(
            "UPDATE server_settings SET banner_message = $3, banner_tone = $4, \
             banner_audience = $5, banner_link = $6, banner_version = banner_version + 1, \
             updated_at = $1, updated_by = $2",
        )
        .bind(at)
        .bind(admin.user_id)
        .bind(arg("message").as_str().map(str::to_owned))
        .bind(arg("tone").as_str().map(str::to_owned))
        .bind(arg("audience").as_str().map(str::to_owned))
        .bind(arg("link").as_str().map(str::to_owned)),
        _ => sqlx::query(
            "UPDATE server_settings SET publishing_paused = $3, updated_at = $1, updated_by = $2",
        )
        .bind(at)
        .bind(admin.user_id)
        .bind(arg("paused").as_bool()),
    };
    query.execute(&mut *tx).await?;
    tx.commit(
        Event::new("settings.update")
            .target("settings", section, "Server settings")
            .detail(json!({"section": section, "from": before, "to": after})),
    )
    .await?;
    state.settings_cache.invalidate();
    view(&state).await
}

pub async fn test_email(
    admin: AdminAuth,
    State(state): State<AppState>,
) -> Result<(StatusCode, Json<Value>), ApiError> {
    admin.require(Perm::Settings)?;
    state.limiter.check(
        &format!("admin:test-email:{}", admin.user_id),
        TEST_EMAILS_PER_WINDOW,
        TEST_EMAIL_WINDOW,
    )?;
    let mut tx = AdminTx::begin(&state, &admin).await?;
    state
        .mailer
        .enqueue(
            &mut tx,
            Mail::TestEmail {
                to: admin.email.clone(),
            },
        )
        .await?;
    tx.commit(Event::new("settings.test_email").target("user", admin.user_id, &admin.email))
        .await?;
    Ok((StatusCode::ACCEPTED, Json(json!({"to": admin.email}))))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn domains_are_hostnames() {
        for ok in ["acme.com", "a-b.example.co.uk", "x1.io"] {
            assert!(valid_domain(ok), "{ok}");
        }
        for bad in [
            "", "acme", "-a.com", "a-.com", "a..com", "a b.com", "a.com/x", "1.2.3.4", "A.com",
            "@a.com",
        ] {
            assert!(!valid_domain(bad), "{bad}");
        }
        assert!(!valid_domain(&format!("{}.com", "a".repeat(64))));
    }

    #[test]
    fn domains_are_lowercased_and_deduplicated() {
        let got = clean_domains(&[" Acme.COM ".into(), "acme.com".into(), "b.io".into()]).unwrap();
        assert_eq!(got, ["acme.com", "b.io"]);
        assert!(clean_domains(&["nope".into()]).is_err());
    }

    #[test]
    fn links_are_http_urls() {
        assert_eq!(clean_link(None).unwrap(), None);
        assert_eq!(clean_link(Some("  ".into())).unwrap(), None);
        assert!(
            clean_link(Some("https://a.com/x".into()))
                .unwrap()
                .is_some()
        );
        for bad in [
            "javascript:alert(1)",
            "ftp://a.com",
            "https://",
            "https:///x",
            "https://a.com/ b",
        ] {
            assert!(clean_link(Some(bad.into())).is_err(), "{bad}");
        }
    }
}
