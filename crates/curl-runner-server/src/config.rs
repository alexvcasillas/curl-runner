//! Server configuration, read once from the environment at startup.
//!
//! An empty value counts as unset: compose passes `${VAR:-}` through as "".

use std::fmt;
use std::str::FromStr;

use curl_runner_cloud_proto::SignupMode;
use sqlx::postgres::{PgConnectOptions, PgSslMode};

const REDACTED: &str = "<redacted>";
const MIN_SECRET_KEY_BYTES: usize = 32;

#[derive(Clone)]
pub struct Config {
    pub database: PgConnectOptions,
    /// Public base URL of this API, without a trailing `/`.
    pub api_url: String,
    /// Public base URL of the dashboard, without a trailing `/`.
    pub app_url: String,
    /// Pepper for stored hashes and the key source for encrypted link tokens.
    pub secret_key: String,
    pub smtp: Option<SmtpConfig>,
    pub mail_from: String,
    pub github: Option<OAuthApp>,
    pub google: Option<OAuthApp>,
    pub signup: SignupMode,
    /// Lower-cased domains allowed to sign up when `signup` is `Domains`.
    pub signup_domains: Vec<String>,
    /// Lower-cased operator emails.
    pub operators: Vec<String>,
}

/// How to reach the SMTP relay.
#[derive(Clone, PartialEq, Eq)]
pub enum SmtpConfig {
    /// From `SMTP_HOST`, `SMTP_PORT`, `SMTP_USER` and `SMTP_PASSWORD`, taken as-is.
    Parts {
        host: String,
        port: u16,
        tls: SmtpTls,
        credentials: Option<(String, String)>,
    },
    /// From `SMTP_URL`, e.g. `smtps://user:pass@smtp.example.com:465`.
    Url(String),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SmtpTls {
    /// TLS from the first byte (port 465).
    Implicit,
    /// Plain connection upgraded with STARTTLS (port 587).
    StartTls,
}

#[derive(Clone, PartialEq, Eq)]
pub struct OAuthApp {
    pub client_id: String,
    pub client_secret: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConfigError {
    Missing(&'static str),
    Invalid { var: &'static str, reason: String },
}

impl fmt::Display for ConfigError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Missing(what) => write!(f, "missing config: {what}"),
            Self::Invalid { var, reason } => write!(f, "invalid {var}: {reason}"),
        }
    }
}

impl std::error::Error for ConfigError {}

impl Config {
    /// Read the config through `get` (the process env in production).
    pub fn from_env(get: impl Fn(&str) -> Option<String>) -> Result<Config, ConfigError> {
        let get = |key: &str| get(key).filter(|v| !v.is_empty());

        let database = database(&get)?;

        let secret_key = get("SECRET_KEY").ok_or(ConfigError::Missing("SECRET_KEY"))?;
        if secret_key.len() < MIN_SECRET_KEY_BYTES {
            return Err(ConfigError::Invalid {
                var: "SECRET_KEY",
                reason: format!("must be at least {MIN_SECRET_KEY_BYTES} bytes"),
            });
        }

        let signup = match get("SIGNUP").map(|v| v.trim().to_ascii_lowercase()) {
            None => SignupMode::Domains,
            Some(v) => match v.as_str() {
                "open" => SignupMode::Open,
                "domains" => SignupMode::Domains,
                "closed" => SignupMode::Closed,
                _ => {
                    return Err(ConfigError::Invalid {
                        var: "SIGNUP",
                        reason: "expected open, domains or closed".into(),
                    });
                }
            },
        };

        Ok(Config {
            database,
            api_url: base_url(get("PUBLIC_API_URL"), "http://localhost:8080"),
            app_url: base_url(get("PUBLIC_APP_URL"), "http://localhost:3000"),
            secret_key,
            smtp: smtp(&get)?,
            mail_from: get("MAIL_FROM")
                .unwrap_or_else(|| "curl-runner <no-reply@curl-runner.com>".into()),
            github: oauth_app(&get, "GITHUB_CLIENT_ID", "GITHUB_CLIENT_SECRET")?,
            google: oauth_app(&get, "GOOGLE_CLIENT_ID", "GOOGLE_CLIENT_SECRET")?,
            signup,
            signup_domains: lower_list(get("SIGNUP_DOMAINS")),
            operators: lower_list(get("OPERATOR_EMAILS")),
        })
    }
}

/// `DATABASE_URL` if set, else the `DB_*` parts. Parts are passed to
/// `PgConnectOptions` as-is, so a password needs no URL encoding.
fn database(get: &impl Fn(&str) -> Option<String>) -> Result<PgConnectOptions, ConfigError> {
    if let Some(url) = get("DATABASE_URL") {
        if !(url.starts_with("postgres://") || url.starts_with("postgresql://")) {
            return Err(ConfigError::Invalid {
                var: "DATABASE_URL",
                reason: "must start with postgres:// or postgresql://".into(),
            });
        }
        // The parse error may quote the URL, password included, so it's not shown.
        return PgConnectOptions::from_str(&url).map_err(|_| ConfigError::Invalid {
            var: "DATABASE_URL",
            reason: "not a valid Postgres URL".into(),
        });
    }
    let (Some(user), Some(password), Some(name)) =
        (get("DB_USER"), get("DB_PASSWORD"), get("DB_NAME"))
    else {
        return Err(ConfigError::Missing(
            "DATABASE_URL, or DB_USER, DB_PASSWORD and DB_NAME",
        ));
    };
    let port = parse_port(get("DB_PORT"), "DB_PORT", 5432)?;
    Ok(PgConnectOptions::new_without_pgpass()
        .host(get("DB_HOST").as_deref().unwrap_or("db"))
        .port(port)
        .username(&user)
        .password(&password)
        .database(&name)
        .ssl_mode(PgSslMode::Disable))
}

/// The `SMTP_*` parts if `SMTP_HOST` is set, else `SMTP_URL`, else none.
fn smtp(get: &impl Fn(&str) -> Option<String>) -> Result<Option<SmtpConfig>, ConfigError> {
    if let Some(host) = get("SMTP_HOST") {
        let port = parse_port(get("SMTP_PORT"), "SMTP_PORT", 465)?;
        let tls = if port == 587 {
            SmtpTls::StartTls
        } else {
            SmtpTls::Implicit
        };
        let credentials = match (get("SMTP_USER"), get("SMTP_PASSWORD")) {
            (Some(user), Some(password)) => Some((user, password)),
            (None, None) => None,
            _ => {
                return Err(ConfigError::Invalid {
                    var: "SMTP_USER",
                    reason: "SMTP_USER and SMTP_PASSWORD go together".into(),
                });
            }
        };
        return Ok(Some(SmtpConfig::Parts {
            host,
            port,
            tls,
            credentials,
        }));
    }
    match get("SMTP_URL") {
        Some(url) if url.starts_with("smtp://") || url.starts_with("smtps://") => {
            Ok(Some(SmtpConfig::Url(url)))
        }
        Some(_) => Err(ConfigError::Invalid {
            var: "SMTP_URL",
            reason: "must start with smtp:// or smtps://".into(),
        }),
        None => Ok(None),
    }
}

fn oauth_app(
    get: &impl Fn(&str) -> Option<String>,
    id_var: &'static str,
    secret_var: &'static str,
) -> Result<Option<OAuthApp>, ConfigError> {
    match (get(id_var), get(secret_var)) {
        (Some(client_id), Some(client_secret)) => Ok(Some(OAuthApp {
            client_id,
            client_secret,
        })),
        (None, None) => Ok(None),
        _ => Err(ConfigError::Invalid {
            var: id_var,
            reason: format!("{id_var} and {secret_var} go together"),
        }),
    }
}

fn parse_port(value: Option<String>, var: &'static str, default: u16) -> Result<u16, ConfigError> {
    value.map_or(Ok(default), |v| {
        v.trim().parse().map_err(|_| ConfigError::Invalid {
            var,
            reason: format!("{v:?} is not a port"),
        })
    })
}

fn base_url(value: Option<String>, default: &str) -> String {
    value
        .as_deref()
        .unwrap_or(default)
        .trim()
        .trim_end_matches('/')
        .to_owned()
}

fn lower_list(value: Option<String>) -> Vec<String> {
    value
        .unwrap_or_default()
        .split(',')
        .map(|item| item.trim().to_lowercase())
        .filter(|item| !item.is_empty())
        .collect()
}

impl fmt::Debug for Config {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // PgConnectOptions' own Debug prints the password, so list the safe parts.
        let db = &self.database;
        let database = format!(
            "{}@{}:{}/{}",
            db.get_username(),
            db.get_host(),
            db.get_port(),
            db.get_database().unwrap_or_default()
        );
        f.debug_struct("Config")
            .field("database", &database)
            .field("api_url", &self.api_url)
            .field("app_url", &self.app_url)
            .field("secret_key", &REDACTED)
            .field("smtp", &self.smtp)
            .field("mail_from", &self.mail_from)
            .field("github", &self.github)
            .field("google", &self.google)
            .field("signup", &self.signup)
            .field("signup_domains", &self.signup_domains)
            .field("operators", &self.operators)
            .finish()
    }
}

impl fmt::Debug for SmtpConfig {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Parts {
                host,
                port,
                tls,
                credentials,
            } => f
                .debug_struct("Parts")
                .field("host", host)
                .field("port", port)
                .field("tls", tls)
                .field("user", &credentials.as_ref().map(|(user, _)| user))
                .field("password", &credentials.as_ref().map(|_| REDACTED))
                .finish(),
            Self::Url(_) => f.debug_tuple("Url").field(&REDACTED).finish(),
        }
    }
}

impl fmt::Debug for OAuthApp {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("OAuthApp")
            .field("client_id", &self.client_id)
            .field("client_secret", &REDACTED)
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    const KEY: &str = "0123456789abcdef0123456789abcdef";

    fn parse(vars: &[(&str, &str)]) -> Result<Config, ConfigError> {
        let env: HashMap<String, String> = vars
            .iter()
            .map(|(k, v)| ((*k).to_owned(), (*v).to_owned()))
            .collect();
        Config::from_env(|k| env.get(k).cloned())
    }

    fn minimal() -> Vec<(&'static str, &'static str)> {
        vec![
            ("DB_USER", "curl_runner"),
            ("DB_PASSWORD", "pw"),
            ("DB_NAME", "curl_runner"),
            ("SECRET_KEY", KEY),
        ]
    }

    fn with(extra: &[(&'static str, &'static str)]) -> Result<Config, ConfigError> {
        let mut vars = minimal();
        vars.extend_from_slice(extra);
        parse(&vars)
    }

    #[test]
    fn config_requires_database_and_secret() {
        assert!(matches!(
            parse(&[("SECRET_KEY", KEY)]),
            Err(ConfigError::Missing(_))
        ));
        assert_eq!(
            parse(&[("DB_USER", "u"), ("DB_PASSWORD", "p"), ("DB_NAME", "d")]).unwrap_err(),
            ConfigError::Missing("SECRET_KEY")
        );
        // Part of the DB_* form is not enough.
        assert!(parse(&[("DB_USER", "u"), ("SECRET_KEY", KEY)]).is_err());
        // Empty values count as unset.
        assert!(with(&[("SECRET_KEY", "")]).is_err());
        // Too short a key.
        assert!(matches!(
            with(&[("SECRET_KEY", "short")]),
            Err(ConfigError::Invalid {
                var: "SECRET_KEY",
                ..
            })
        ));
        assert!(minimal().len() == 4 && with(&[]).is_ok());
        assert!(parse(&[("DATABASE_URL", "postgres://u:p@h/d"), ("SECRET_KEY", KEY)]).is_ok());
    }

    #[test]
    fn config_db_password_with_special_chars() {
        let cfg = with(&[
            ("DB_HOST", "pg.internal"),
            ("DB_PORT", "6543"),
            ("DB_PASSWORD", "p@ss:w/rd#?%"),
        ])
        .unwrap();
        let db = &cfg.database;
        assert_eq!(db.get_host(), "pg.internal");
        assert_eq!(db.get_port(), 6543);
        assert_eq!(db.get_username(), "curl_runner");
        assert_eq!(db.get_database(), Some("curl_runner"));
        assert!(matches!(db.get_ssl_mode(), PgSslMode::Disable));
        // PgConnectOptions has no password getter; its Debug shows the raw value.
        assert!(format!("{db:?}").contains(r#"password: Some("p@ss:w/rd#?%")"#));
    }

    #[test]
    fn config_db_defaults_host_and_port() {
        let cfg = with(&[]).unwrap();
        assert_eq!(cfg.database.get_host(), "db");
        assert_eq!(cfg.database.get_port(), 5432);
        assert!(with(&[("DB_PORT", "nope")]).is_err());
    }

    #[test]
    fn config_database_url_wins() {
        let cfg = with(&[("DATABASE_URL", "postgres://u:p@elsewhere:5555/other")]).unwrap();
        assert_eq!(cfg.database.get_host(), "elsewhere");
        assert_eq!(cfg.database.get_port(), 5555);
        assert_eq!(cfg.database.get_database(), Some("other"));
        assert!(with(&[("DATABASE_URL", "mysql://nope")]).is_err());
    }

    #[test]
    fn config_signup_defaults_to_domains() {
        let cfg = with(&[]).unwrap();
        assert_eq!(cfg.signup, SignupMode::Domains);
        assert_eq!(
            with(&[("SIGNUP", "open")]).unwrap().signup,
            SignupMode::Open
        );
        assert_eq!(
            with(&[("SIGNUP", "Closed")]).unwrap().signup,
            SignupMode::Closed
        );
        assert!(with(&[("SIGNUP", "maybe")]).is_err());
    }

    #[test]
    fn config_defaults() {
        let cfg = with(&[]).unwrap();
        assert_eq!(cfg.api_url, "http://localhost:8080");
        assert_eq!(cfg.app_url, "http://localhost:3000");
        assert_eq!(cfg.mail_from, "curl-runner <no-reply@curl-runner.com>");
        assert!(cfg.smtp.is_none());
        assert!(cfg.github.is_none());
        assert!(cfg.google.is_none());
        assert!(cfg.signup_domains.is_empty());
        assert!(cfg.operators.is_empty());
    }

    #[test]
    fn config_urls_drop_trailing_slash() {
        let cfg = with(&[
            ("PUBLIC_API_URL", "https://api.example.com/"),
            ("PUBLIC_APP_URL", "https://app.example.com"),
        ])
        .unwrap();
        assert_eq!(cfg.api_url, "https://api.example.com");
        assert_eq!(cfg.app_url, "https://app.example.com");
    }

    #[test]
    fn config_lists_are_trimmed_and_lower_cased() {
        let cfg = with(&[
            ("SIGNUP_DOMAINS", " Example.com, ,acme.io "),
            ("OPERATOR_EMAILS", "Ops@Example.com"),
        ])
        .unwrap();
        assert_eq!(cfg.signup_domains, ["example.com", "acme.io"]);
        assert_eq!(cfg.operators, ["ops@example.com"]);
    }

    #[test]
    fn config_oauth_needs_id_and_secret() {
        let cfg = with(&[("GITHUB_CLIENT_ID", "id"), ("GITHUB_CLIENT_SECRET", "s")]).unwrap();
        assert_eq!(
            cfg.github,
            Some(OAuthApp {
                client_id: "id".into(),
                client_secret: "s".into()
            })
        );
        assert!(cfg.google.is_none());
        assert!(with(&[("GOOGLE_CLIENT_ID", "id")]).is_err());
        assert!(with(&[("GITHUB_CLIENT_SECRET", "s")]).is_err());
    }

    #[test]
    fn config_smtp_parts() {
        let cfg = with(&[
            ("SMTP_HOST", "email-smtp.example.com"),
            ("SMTP_USER", "AKIA"),
            ("SMTP_PASSWORD", "a/b+c=="),
        ])
        .unwrap();
        assert_eq!(
            cfg.smtp,
            Some(SmtpConfig::Parts {
                host: "email-smtp.example.com".into(),
                port: 465,
                tls: SmtpTls::Implicit,
                credentials: Some(("AKIA".into(), "a/b+c==".into())),
            })
        );
        let starttls = with(&[("SMTP_HOST", "h"), ("SMTP_PORT", "587")]).unwrap();
        assert!(matches!(
            starttls.smtp,
            Some(SmtpConfig::Parts {
                port: 587,
                tls: SmtpTls::StartTls,
                credentials: None,
                ..
            })
        ));
        assert!(with(&[("SMTP_HOST", "h"), ("SMTP_USER", "u")]).is_err());
        assert!(with(&[("SMTP_HOST", "h"), ("SMTP_PORT", "x")]).is_err());
    }

    #[test]
    fn config_smtp_url_and_parts_win() {
        let url = "smtps://user:pass@smtp.example.com:465";
        let cfg = with(&[("SMTP_URL", url)]).unwrap();
        assert_eq!(cfg.smtp, Some(SmtpConfig::Url(url.into())));
        let both = with(&[("SMTP_URL", url), ("SMTP_HOST", "other.example.com")]).unwrap();
        assert!(
            matches!(both.smtp, Some(SmtpConfig::Parts { ref host, .. }) if host == "other.example.com")
        );
        assert!(with(&[("SMTP_URL", "https://smtp.example.com")]).is_err());
    }

    #[test]
    fn config_debug_redacts_secrets() {
        let cfg = with(&[
            ("DB_PASSWORD", "db-pass-123"),
            ("SECRET_KEY", "super-secret-key-super-secret-key-0"),
            (
                "SMTP_URL",
                "smtps://user:smtp-pass-456@smtp.example.com:465",
            ),
            ("GITHUB_CLIENT_ID", "gh-id"),
            ("GITHUB_CLIENT_SECRET", "gh-secret-789"),
            ("GOOGLE_CLIENT_ID", "g-id"),
            ("GOOGLE_CLIENT_SECRET", "g-secret-012"),
        ])
        .unwrap();
        let debug = format!("{cfg:?}");
        for secret in [
            "db-pass-123",
            "super-secret-key",
            "smtp-pass-456",
            "gh-secret-789",
            "g-secret-012",
        ] {
            assert!(!debug.contains(secret), "{secret} leaked: {debug}");
        }
        assert!(debug.contains("<redacted>"));
        assert!(debug.contains("gh-id"), "non-secrets stay visible: {debug}");

        let parts = with(&[
            ("SMTP_HOST", "h"),
            ("SMTP_USER", "u"),
            ("SMTP_PASSWORD", "smtp-part-pass"),
        ])
        .unwrap();
        assert!(!format!("{parts:?}").contains("smtp-part-pass"));
    }
}
