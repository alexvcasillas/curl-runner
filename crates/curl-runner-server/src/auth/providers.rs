//! GitHub and Google sign-in: authorize URLs and code exchange.
//!
//! [`RealProviders`] talks to the providers over rustls. Tokens and provider
//! responses are never logged. [`FakeProviders`] backs the tests.

use std::future::Future;
use std::pin::Pin;
use std::sync::Mutex;
use std::time::Duration;

use reqwest::Url;
use serde::Deserialize;

use crate::config::OAuthApp;

pub type ExchangeFuture<'a> =
    Pin<Box<dyn Future<Output = Result<VerifiedIdentity, String>> + Send + 'a>>;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Provider {
    Github,
    Google,
}

impl Provider {
    pub fn parse(name: &str) -> Option<Self> {
        match name {
            "github" => Some(Self::Github),
            "google" => Some(Self::Google),
            _ => None,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Github => "github",
            Self::Google => "google",
        }
    }
}

/// Who the provider says signed in. `email` is always a verified address.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VerifiedIdentity {
    pub subject: String,
    pub email: String,
    pub name: Option<String>,
}

pub trait OAuthProviders: Send + Sync {
    /// The provider's consent URL, or `None` when it is not configured.
    fn authorize_url(
        &self,
        provider: Provider,
        state: &str,
        challenge: &str,
        redirect_uri: &str,
    ) -> Option<String>;

    /// Trade an authorization code for a verified identity. The error is a
    /// short description for the log, never shown to the user.
    fn exchange<'a>(
        &'a self,
        provider: Provider,
        code: &'a str,
        verifier: &'a str,
        redirect_uri: &'a str,
    ) -> ExchangeFuture<'a>;
}

const GITHUB_AUTHORIZE: &str = "https://github.com/login/oauth/authorize";
const GITHUB_TOKEN: &str = "https://github.com/login/oauth/access_token";
const GITHUB_USER: &str = "https://api.github.com/user";
const GITHUB_EMAILS: &str = "https://api.github.com/user/emails";
const GOOGLE_AUTHORIZE: &str = "https://accounts.google.com/o/oauth2/v2/auth";
const GOOGLE_TOKEN: &str = "https://oauth2.googleapis.com/token";
const GOOGLE_USERINFO: &str = "https://openidconnect.googleapis.com/v1/userinfo";

fn authorize_url(
    base: &str,
    scope: &str,
    client_id: &str,
    state: &str,
    challenge: &str,
    redirect_uri: &str,
) -> String {
    let mut url = Url::parse(base).expect("static url");
    url.query_pairs_mut()
        .append_pair("client_id", client_id)
        .append_pair("redirect_uri", redirect_uri)
        .append_pair("response_type", "code")
        .append_pair("scope", scope)
        .append_pair("state", state)
        .append_pair("code_challenge", challenge)
        .append_pair("code_challenge_method", "S256");
    url.into()
}

#[derive(Deserialize)]
struct TokenResponse {
    access_token: Option<String>,
}

#[derive(Deserialize)]
struct GithubUser {
    id: u64,
    name: Option<String>,
}

#[derive(Deserialize)]
struct GithubEmail {
    email: String,
    primary: bool,
    verified: bool,
}

#[derive(Deserialize)]
struct GoogleUser {
    sub: String,
    email: Option<String>,
    email_verified: Option<serde_json::Value>,
    name: Option<String>,
}

fn github_identity(user: GithubUser, emails: Vec<GithubEmail>) -> Result<VerifiedIdentity, String> {
    let email = emails
        .into_iter()
        .find(|e| e.primary && e.verified)
        .ok_or("github has no primary verified email")?
        .email;
    Ok(VerifiedIdentity {
        subject: user.id.to_string(),
        email,
        name: user.name.filter(|n| !n.trim().is_empty()),
    })
}

fn google_identity(user: GoogleUser) -> Result<VerifiedIdentity, String> {
    let verified = match &user.email_verified {
        Some(serde_json::Value::Bool(b)) => *b,
        Some(serde_json::Value::String(s)) => s == "true",
        _ => false,
    };
    let email = user
        .email
        .filter(|e| verified && !e.is_empty())
        .ok_or("google email is missing or not verified")?;
    Ok(VerifiedIdentity {
        subject: user.sub,
        email,
        name: user.name.filter(|n| !n.trim().is_empty()),
    })
}

pub struct RealProviders {
    http: reqwest::Client,
    github: Option<OAuthApp>,
    google: Option<OAuthApp>,
}

impl RealProviders {
    pub fn new(github: Option<OAuthApp>, google: Option<OAuthApp>) -> Self {
        let http = reqwest::Client::builder()
            .timeout(Duration::from_secs(10))
            .user_agent("curl-runner-server")
            .build()
            .expect("http client");
        Self {
            http,
            github,
            google,
        }
    }

    fn app(&self, provider: Provider) -> Option<&OAuthApp> {
        match provider {
            Provider::Github => self.github.as_ref(),
            Provider::Google => self.google.as_ref(),
        }
    }

    async fn get_json<T: for<'de> Deserialize<'de>>(
        &self,
        url: &str,
        token: &str,
    ) -> Result<T, String> {
        let res = self
            .http
            .get(url)
            .bearer_auth(token)
            .header("accept", "application/json")
            .send()
            .await
            .map_err(|_| "request failed".to_owned())?;
        if !res.status().is_success() {
            return Err(format!("provider answered {}", res.status().as_u16()));
        }
        res.json()
            .await
            .map_err(|_| "unreadable response".to_owned())
    }

    async fn access_token(
        &self,
        url: &str,
        app: &OAuthApp,
        code: &str,
        verifier: &str,
        redirect_uri: &str,
        grant: bool,
    ) -> Result<String, String> {
        let mut form = vec![
            ("client_id", app.client_id.as_str()),
            ("client_secret", app.client_secret.as_str()),
            ("code", code),
            ("code_verifier", verifier),
            ("redirect_uri", redirect_uri),
        ];
        if grant {
            form.push(("grant_type", "authorization_code"));
        }
        let res = self
            .http
            .post(url)
            .header("accept", "application/json")
            .form(&form)
            .send()
            .await
            .map_err(|_| "token request failed".to_owned())?;
        if !res.status().is_success() {
            return Err(format!("token endpoint answered {}", res.status().as_u16()));
        }
        let body: TokenResponse = res
            .json()
            .await
            .map_err(|_| "unreadable token response".to_owned())?;
        body.access_token
            .filter(|t| !t.is_empty())
            .ok_or_else(|| "no access token".to_owned())
    }
}

impl OAuthProviders for RealProviders {
    fn authorize_url(
        &self,
        provider: Provider,
        state: &str,
        challenge: &str,
        redirect_uri: &str,
    ) -> Option<String> {
        let app = self.app(provider)?;
        let (base, scope) = match provider {
            Provider::Github => (GITHUB_AUTHORIZE, "read:user user:email"),
            Provider::Google => (GOOGLE_AUTHORIZE, "openid email profile"),
        };
        Some(authorize_url(
            base,
            scope,
            &app.client_id,
            state,
            challenge,
            redirect_uri,
        ))
    }

    fn exchange<'a>(
        &'a self,
        provider: Provider,
        code: &'a str,
        verifier: &'a str,
        redirect_uri: &'a str,
    ) -> ExchangeFuture<'a> {
        Box::pin(async move {
            let app = self.app(provider).ok_or("provider not configured")?;
            match provider {
                Provider::Github => {
                    let token = self
                        .access_token(GITHUB_TOKEN, app, code, verifier, redirect_uri, false)
                        .await?;
                    let user: GithubUser = self.get_json(GITHUB_USER, &token).await?;
                    let emails: Vec<GithubEmail> = self.get_json(GITHUB_EMAILS, &token).await?;
                    github_identity(user, emails)
                }
                Provider::Google => {
                    let token = self
                        .access_token(GOOGLE_TOKEN, app, code, verifier, redirect_uri, true)
                        .await?;
                    let user: GoogleUser = self.get_json(GOOGLE_USERINFO, &token).await?;
                    google_identity(user)
                }
            }
        })
    }
}

/// What a [`FakeProviders`] exchange saw last.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SeenExchange {
    pub provider: Provider,
    pub code: String,
    pub verifier: String,
    pub redirect_uri: String,
}

struct FakeState {
    configured: Vec<Provider>,
    result: Result<VerifiedIdentity, String>,
    seen: Option<SeenExchange>,
}

/// Test double: every code exchanges to the identity set by the test, or to an
/// error. Both providers are configured until [`only_configured`] says otherwise.
///
/// [`only_configured`]: FakeProviders::only_configured
pub struct FakeProviders {
    inner: Mutex<FakeState>,
}

impl Default for FakeProviders {
    fn default() -> Self {
        Self::new()
    }
}

impl FakeProviders {
    pub fn new() -> Self {
        Self {
            inner: Mutex::new(FakeState {
                configured: vec![Provider::Github, Provider::Google],
                result: Err("no identity set".to_owned()),
                seen: None,
            }),
        }
    }

    fn lock(&self) -> std::sync::MutexGuard<'_, FakeState> {
        self.inner.lock().unwrap_or_else(|e| e.into_inner())
    }

    pub fn only_configured(&self, providers: &[Provider]) {
        self.lock().configured = providers.to_vec();
    }

    pub fn set_identity(&self, subject: &str, email: &str, name: Option<&str>) {
        self.lock().result = Ok(VerifiedIdentity {
            subject: subject.to_owned(),
            email: email.to_owned(),
            name: name.map(str::to_owned),
        });
    }

    pub fn set_error(&self, message: &str) {
        self.lock().result = Err(message.to_owned());
    }

    pub fn last_exchange(&self) -> Option<SeenExchange> {
        self.lock().seen.clone()
    }
}

impl OAuthProviders for FakeProviders {
    fn authorize_url(
        &self,
        provider: Provider,
        state: &str,
        challenge: &str,
        redirect_uri: &str,
    ) -> Option<String> {
        if !self.lock().configured.contains(&provider) {
            return None;
        }
        Some(authorize_url(
            &format!("https://{}.example.com/authorize", provider.as_str()),
            "fake",
            "fake-client",
            state,
            challenge,
            redirect_uri,
        ))
    }

    fn exchange<'a>(
        &'a self,
        provider: Provider,
        code: &'a str,
        verifier: &'a str,
        redirect_uri: &'a str,
    ) -> ExchangeFuture<'a> {
        Box::pin(async move {
            let mut inner = self.lock();
            inner.seen = Some(SeenExchange {
                provider,
                code: code.to_owned(),
                verifier: verifier.to_owned(),
                redirect_uri: redirect_uri.to_owned(),
            });
            inner.result.clone()
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn real() -> RealProviders {
        let app = |id: &str| OAuthApp {
            client_id: id.to_owned(),
            client_secret: "s3cret".to_owned(),
        };
        RealProviders::new(Some(app("gh-id")), None).with_google(app("g-id"))
    }

    impl RealProviders {
        fn with_google(mut self, app: OAuthApp) -> Self {
            self.google = Some(app);
            self
        }
    }

    fn query(url: &str) -> std::collections::HashMap<String, String> {
        Url::parse(url)
            .unwrap()
            .query_pairs()
            .map(|(k, v)| (k.into_owned(), v.into_owned()))
            .collect()
    }

    #[test]
    fn github_url_has_scope_pkce_state_and_redirect() {
        let redirect = "https://app.example.com/sign-in/github/callback";
        let url = real()
            .authorize_url(Provider::Github, "st ate", "chal", redirect)
            .unwrap();
        assert!(url.starts_with("https://github.com/login/oauth/authorize?"));
        let q = query(&url);
        assert_eq!(q["client_id"], "gh-id");
        assert_eq!(q["redirect_uri"], redirect);
        assert_eq!(q["scope"], "read:user user:email");
        assert_eq!(q["state"], "st ate");
        assert_eq!(q["code_challenge"], "chal");
        assert_eq!(q["code_challenge_method"], "S256");
        assert_eq!(q["response_type"], "code");
        assert!(!url.contains("s3cret"));
    }

    #[test]
    fn google_url_has_scope_and_pkce() {
        let url = real()
            .authorize_url(Provider::Google, "s", "c", "https://a.example.com/cb")
            .unwrap();
        assert!(url.starts_with("https://accounts.google.com/o/oauth2/v2/auth?"));
        let q = query(&url);
        assert_eq!(q["client_id"], "g-id");
        assert_eq!(q["scope"], "openid email profile");
        assert_eq!(q["code_challenge_method"], "S256");
    }

    #[test]
    fn unconfigured_provider_has_no_url() {
        let p = RealProviders::new(None, None);
        assert!(p.authorize_url(Provider::Github, "s", "c", "r").is_none());
        assert!(p.authorize_url(Provider::Google, "s", "c", "r").is_none());
    }

    #[test]
    fn github_picks_primary_verified_email() {
        let user: GithubUser =
            serde_json::from_str(r#"{"id": 583231, "name": "Jane Doe"}"#).unwrap();
        let emails: Vec<GithubEmail> = serde_json::from_str(
            r#"[
                {"email": "old@example.com", "primary": false, "verified": true},
                {"email": "new@example.com", "primary": true, "verified": false},
                {"email": "jane@example.com", "primary": true, "verified": true, "visibility": "public"}
            ]"#,
        )
        .unwrap();
        let id = github_identity(user, emails).unwrap();
        assert_eq!(id.subject, "583231");
        assert_eq!(id.email, "jane@example.com");
        assert_eq!(id.name.as_deref(), Some("Jane Doe"));
    }

    #[test]
    fn github_without_primary_verified_email_is_an_error() {
        let user: GithubUser = serde_json::from_str(r#"{"id": 1, "name": null}"#).unwrap();
        let emails: Vec<GithubEmail> = serde_json::from_str(
            r#"[{"email": "a@example.com", "primary": true, "verified": false},
                {"email": "b@example.com", "primary": false, "verified": true}]"#,
        )
        .unwrap();
        assert!(github_identity(user, emails).is_err());
    }

    #[test]
    fn google_requires_email_verified() {
        let ok: GoogleUser = serde_json::from_str(
            r#"{"sub": "1088", "email": "jane@example.com", "email_verified": true, "name": "Jane"}"#,
        )
        .unwrap();
        let id = google_identity(ok).unwrap();
        assert_eq!(id.subject, "1088");
        assert_eq!(id.email, "jane@example.com");
        for bad in [
            r#"{"sub": "1", "email": "a@example.com", "email_verified": false}"#,
            r#"{"sub": "1", "email": "a@example.com"}"#,
            r#"{"sub": "1", "email_verified": true}"#,
        ] {
            let user: GoogleUser = serde_json::from_str(bad).unwrap();
            assert!(google_identity(user).is_err(), "{bad}");
        }
    }

    #[test]
    fn provider_names_round_trip() {
        for p in [Provider::Github, Provider::Google] {
            assert_eq!(Provider::parse(p.as_str()), Some(p));
        }
        assert_eq!(Provider::parse("gitlab"), None);
    }
}
