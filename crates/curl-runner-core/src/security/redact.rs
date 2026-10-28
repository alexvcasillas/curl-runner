//! Secret redaction (port of `utils/secret-redactor.ts`). No global state:
//! build a [`Redactor`] and pass it around.

use std::sync::LazyLock;

use indexmap::IndexMap;
use regex::{Captures, Regex};

use crate::model::RequestConfig;

pub const REDACTED: &str = "[REDACTED]";

/// Header names whose values are secrets (`SENSITIVE_HEADER_RE`).
static SENSITIVE_HEADER: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?i)authorization|api[-_]?key|token|secret|cookie").unwrap());

/// `^Bearer\s+(\S+)$`, case-insensitive.
static BEARER_VALUE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?i)^Bearer\s+(\S+)$").unwrap());

/// Whole-match patterns (`SECRET_PATTERNS`, same order as TS).
static PATTERNS: LazyLock<Vec<Regex>> = LazyLock::new(|| {
    [
        // Stripe
        r"sk_live_[a-zA-Z0-9]{24,}",
        r"sk_test_[a-zA-Z0-9]{24,}",
        r"pk_live_[a-zA-Z0-9]{24,}",
        r"pk_test_[a-zA-Z0-9]{24,}",
        r"rk_live_[a-zA-Z0-9]{24,}",
        r"rk_test_[a-zA-Z0-9]{24,}",
        // AWS
        r"AKIA[0-9A-Z]{16}",
        // GitHub
        r"ghp_[a-zA-Z0-9]{36,}",
        r"gho_[a-zA-Z0-9]{36,}",
        r"ghu_[a-zA-Z0-9]{36,}",
        r"ghs_[a-zA-Z0-9]{36,}",
        r"ghr_[a-zA-Z0-9]{36,}",
        // NPM
        r"npm_[a-zA-Z0-9]{36,}",
        // Slack
        r"xox[baprs]-[a-zA-Z0-9-]{10,}",
        // Paddle
        r"pdl_[a-zA-Z0-9]{20,}",
        // OpenAI
        r"sk-[a-zA-Z0-9]{48,}",
        // Anthropic
        r"sk-ant-[a-zA-Z0-9-]{40,}",
        // Generic Bearer tokens
        r"Bearer [a-zA-Z0-9_-]{40,}",
        // JWT
        r"eyJ[A-Za-z0-9_-]+\.[A-Za-z0-9_-]+\.[A-Za-z0-9_-]+",
        // Google API key
        r"AIza[0-9A-Za-z_-]{35}",
    ]
    .iter()
    .map(|p| Regex::new(p).unwrap())
    .collect()
});

/// `api_key=` / `token=` / `secret=` assignment: only the value (group 1) is
/// redacted.
static ASSIGNMENT: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r#"(?:api[_-]?key|token|secret)=([^&\s"']+)"#).unwrap());

/// Masks secret values in output. Explicit secrets (longest first) are
/// replaced before the built-in token patterns run.
#[derive(Debug, Clone)]
pub struct Redactor {
    /// Distinct non-empty secrets, longest (UTF-16 length, like JS) first;
    /// ties keep insertion order.
    secrets: Vec<String>,
    patterns: bool,
}

impl Default for Redactor {
    fn default() -> Self {
        Self {
            secrets: Vec::new(),
            patterns: true,
        }
    }
}

fn js_len(s: &str) -> usize {
    s.encode_utf16().count()
}

impl Redactor {
    pub fn new(secrets: impl IntoIterator<Item = String>) -> Self {
        let mut r = Self::default();
        r.add_secrets(secrets);
        r
    }

    /// Redactor for the `SECRET_*` variables (non-empty values) of `vars`.
    /// When `!enabled` (`--no-redact` / `env.redactSecrets: false`) no
    /// explicit secret is registered; built-in patterns stay on, as in TS.
    pub fn from_env_vars(vars: &IndexMap<String, String>, enabled: bool) -> Self {
        let mut r = Self::default();
        if enabled {
            r.add_secrets(
                vars.iter()
                    .filter(|(k, _)| k.starts_with("SECRET_"))
                    .map(|(_, v)| v.clone()),
            );
        }
        r
    }

    pub fn set_pattern_redaction(&mut self, enabled: bool) {
        self.patterns = enabled;
    }

    pub fn add_secret(&mut self, value: impl Into<String>) {
        let v = value.into();
        if v.is_empty() || self.secrets.contains(&v) {
            return;
        }
        // Stable insert: after every secret of >= length.
        let n = js_len(&v);
        let at = self.secrets.partition_point(|s| js_len(s) >= n);
        self.secrets.insert(at, v);
    }

    pub fn add_secrets(&mut self, values: impl IntoIterator<Item = String>) {
        for v in values {
            self.add_secret(v);
        }
    }

    /// Register inline secrets of the (interpolated) requests: bearer
    /// tokens, basic passwords and `user:pass`, sensitive header values and
    /// the bare token of `Authorization: Bearer <tok>`.
    pub fn register_request_secrets(&mut self, requests: &[RequestConfig]) {
        for req in requests {
            if let Some(auth) = &req.auth {
                let nz = |o: &Option<String>| o.clone().filter(|s| !s.is_empty());
                match auth.auth_type.as_str() {
                    "bearer" => {
                        if let Some(t) = nz(&auth.token) {
                            self.add_secret(t);
                        }
                    }
                    "basic" => {
                        if let Some(p) = nz(&auth.password) {
                            self.add_secret(p.clone());
                            if let Some(u) = nz(&auth.username) {
                                self.add_secret(format!("{u}:{p}"));
                            }
                        }
                    }
                    _ => {}
                }
            }
            if let Some(headers) = &req.headers {
                for (name, value) in headers {
                    if value.is_empty() || !SENSITIVE_HEADER.is_match(name) {
                        continue;
                    }
                    self.add_secret(value.clone());
                    if let Some(c) = BEARER_VALUE.captures(value) {
                        self.add_secret(c[1].to_string());
                    }
                }
            }
        }
    }

    pub fn clear(&mut self) {
        self.secrets.clear();
    }

    pub fn has_secrets(&self) -> bool {
        !self.secrets.is_empty()
    }

    pub fn secret_count(&self) -> usize {
        self.secrets.len()
    }

    /// Replace only the explicit secrets (`.env` `SECRET_*` values and
    /// registered request secrets); built-in token patterns are not applied.
    pub fn redact_secrets(&self, input: &str) -> String {
        let mut result = input.to_string();
        for secret in &self.secrets {
            if result.contains(secret.as_str()) {
                result = result.replace(secret.as_str(), REDACTED);
            }
        }
        result
    }

    /// Explicit secrets, then (when enabled) the built-in token patterns.
    pub fn redact(&self, input: &str) -> String {
        let mut result = self.redact_secrets(input);
        if self.patterns {
            for p in PATTERNS.iter() {
                result = p
                    .replace_all(&result, regex::NoExpand(REDACTED))
                    .into_owned();
            }
            result = ASSIGNMENT
                .replace_all(&result, |c: &Captures| {
                    let whole = c.get(0).unwrap();
                    let val = c.get(1).unwrap();
                    format!(
                        "{}{REDACTED}",
                        &whole.as_str()[..val.start() - whole.start()]
                    )
                })
                .into_owned();
        }
        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::AuthConfig;

    #[test]
    fn redact_secrets_skips_patterns() {
        let r = Redactor::new(["s3cr3t".to_string()]);
        let input = "s3cr3t token=abcdef";
        assert_eq!(r.redact_secrets(input), format!("{REDACTED} token=abcdef"));
        assert_eq!(r.redact(input), format!("{REDACTED} token={REDACTED}"));
    }

    fn key(prefix: &str, suffix: &str) -> String {
        format!("{prefix}{suffix}")
    }
    fn r(secrets: &[&str]) -> Redactor {
        Redactor::new(secrets.iter().map(|s| s.to_string()))
    }
    fn out(s: &str) -> String {
        Redactor::default().redact(s)
    }

    // --- explicit secrets ---
    #[test]
    fn redacts_registered_secret() {
        assert_eq!(
            r(&["supersecret123"]).redact("Authorization: Bearer supersecret123"),
            "Authorization: Bearer [REDACTED]"
        );
    }
    #[test]
    fn redacts_multiple_secrets() {
        assert_eq!(
            r(&["key123", "token456"]).redact("key: key123, token: token456"),
            "key: [REDACTED], token: [REDACTED]"
        );
    }
    #[test]
    fn longer_secrets_first() {
        assert_eq!(
            r(&["abc", "abcdef"]).redact("secret: abcdef"),
            "secret: [REDACTED]"
        );
    }
    #[test]
    fn redacts_all_occurrences() {
        assert_eq!(
            r(&["secret"]).redact("first: secret, second: secret, third: secret"),
            "first: [REDACTED], second: [REDACTED], third: [REDACTED]"
        );
    }
    #[test]
    fn ignores_empty_secrets() {
        let mut x = r(&[""]);
        x.add_secret("");
        x.add_secrets(["".to_string()]);
        assert_eq!(x.secret_count(), 0);
        assert!(!x.has_secrets());
        assert_eq!(x.redact("abc"), "abc");
    }
    #[test]
    fn unchanged_when_no_match() {
        assert_eq!(
            r(&["mysecret"]).redact("no secrets here"),
            "no secrets here"
        );
    }
    #[test]
    fn clear_removes_all() {
        let mut x = r(&["secret1", "secret2"]);
        assert!(x.has_secrets());
        x.clear();
        assert!(!x.has_secrets());
        assert_eq!(x.secret_count(), 0);
    }

    // --- patterns ---
    #[test]
    fn stripe_keys() {
        for p in [
            "sk_live_", "sk_test_", "pk_live_", "pk_test_", "rk_live_", "rk_test_",
        ] {
            assert_eq!(
                out(&format!("key: {}", key(p, "abcdefghijklmnopqrstuvwx"))),
                "key: [REDACTED]"
            );
        }
    }
    #[test]
    fn aws_key() {
        assert_eq!(out("AWS_KEY=AKIAIOSFODNN7EXAMPLE"), "AWS_KEY=[REDACTED]");
    }
    #[test]
    fn github_tokens() {
        assert_eq!(
            out("token: ghp_abcdefghijklmnopqrstuvwxyz0123456789"),
            "token: [REDACTED]"
        );
        for p in ["gho_", "ghu_", "ghs_", "ghr_", "npm_"] {
            assert_eq!(
                out(&key(p, "abcdefghijklmnopqrstuvwxyz0123456789")),
                "[REDACTED]"
            );
        }
    }
    #[test]
    fn slack_paddle_openai_anthropic() {
        assert_eq!(out("xoxb-123456789-abcdefghij"), "[REDACTED]");
        assert_eq!(out("xoxp-123456789-abcdefghij"), "[REDACTED]");
        assert_eq!(out("pdl_abcdefghijklmnopqrst"), "[REDACTED]");
        assert_eq!(
            out("sk-abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWX"),
            "[REDACTED]"
        );
        assert_eq!(
            out("sk-ant-api03-abcdefghijklmnopqrstuvwxyzABCDEFGHIJKL"),
            "[REDACTED]"
        );
    }
    #[test]
    fn bearer_token() {
        assert_eq!(
            out("Authorization: Bearer abcdefghijklmnopqrstuvwxyz0123456789ABCD"),
            "Authorization: [REDACTED]"
        );
    }
    #[test]
    fn pattern_redaction_toggle() {
        let k = key("sk_live_", "abcdefghijklmnopqrstuvwx");
        let mut x = Redactor::default();
        x.set_pattern_redaction(false);
        assert_eq!(x.redact(&k), k);
        x.set_pattern_redaction(true);
        assert_eq!(x.redact(&k), "[REDACTED]");
    }
    #[test]
    fn multiple_patterns() {
        let s = key("sk_live_", "abcdefghijklmnopqrstuvwx");
        let g = key("ghp_", "abcdefghijklmnopqrstuvwxyz0123456789");
        assert_eq!(
            out(&format!("stripe: {s}, github: {g}")),
            "stripe: [REDACTED], github: [REDACTED]"
        );
    }
    #[test]
    fn combined_explicit_and_patterns() {
        let s = key("sk_live_", "abcdefghijklmnopqrstuvwx");
        assert_eq!(
            r(&["mysuperpassword"]).redact(&format!("password: mysuperpassword, stripe: {s}")),
            "password: [REDACTED], stripe: [REDACTED]"
        );
        assert_eq!(r(&[&s]).redact(&format!("key: {s}")), "key: [REDACTED]");
    }
    #[test]
    fn has_secrets_and_count() {
        let mut x = Redactor::default();
        assert!(!x.has_secrets());
        assert_eq!(x.secret_count(), 0);
        x.add_secret("value1");
        assert_eq!(x.secret_count(), 1);
        x.add_secrets(["value2".to_string(), "value3".to_string()]);
        assert_eq!(x.secret_count(), 3);
        assert!(x.has_secrets());
    }
    #[test]
    fn jwt_and_google_key() {
        let jwt = "eyJhbGciOiJIUzI1NiJ9.eyJzdWIiOiIxMjM0NTY3ODkwIn0.SflKxwRJSMeKKF2QT4fwpMeJf36POk6yJV_adQssw5c";
        assert_eq!(out(&format!("token: {jwt}")), "token: [REDACTED]");
        assert_eq!(
            out("key=AIzaSyD-9tSrke72I6e0dvos8Fy7T5EeHzNs9MA"),
            "key=[REDACTED]"
        );
    }
    #[test]
    fn query_param_values() {
        assert_eq!(
            out("https://api.example.com?api_key=supersecret123&other=ok"),
            "https://api.example.com?api_key=[REDACTED]&other=ok"
        );
        assert_eq!(out("url?token=abc123xyz"), "url?token=[REDACTED]");
        assert_eq!(out("body: secret=mysecretvalue"), "body: secret=[REDACTED]");
    }
    #[test]
    fn query_param_value_equal_to_key_name_is_fully_redacted() {
        // TS replaces the first textual occurrence of the value inside the
        // match, which leaves `se[REDACTED]=cret` here (leak); we redact the
        // value span itself.
        assert_eq!(out("secret=cret"), "secret=[REDACTED]");
        assert_eq!(out("token=token"), "token=[REDACTED]");
    }
    #[test]
    fn no_over_redaction() {
        let b = "Hello world, the count is 42, status=ok";
        assert_eq!(out(b), b);
    }

    // --- request secrets ---
    fn rq(f: impl FnOnce(&mut RequestConfig)) -> RequestConfig {
        let mut c = RequestConfig {
            url: "https://example.com".into(),
            ..Default::default()
        };
        f(&mut c);
        c
    }
    fn reg(reqs: &[RequestConfig]) -> Redactor {
        let mut x = Redactor::default();
        x.register_request_secrets(reqs);
        x
    }
    fn hdr(k: &str, v: &str) -> Option<IndexMap<String, String>> {
        Some(IndexMap::from([(k.to_string(), v.to_string())]))
    }
    #[test]
    fn request_bearer_token() {
        let x = reg(&[rq(|c| {
            c.auth = Some(AuthConfig {
                auth_type: "bearer".into(),
                token: Some("mysupersecretbearertoken123".into()),
                ..Default::default()
            })
        })]);
        assert_eq!(
            x.redact("Authorization: Bearer mysupersecretbearertoken123"),
            "Authorization: Bearer [REDACTED]"
        );
    }
    #[test]
    fn request_basic_password_and_combined() {
        let x = reg(&[rq(|c| {
            c.auth = Some(AuthConfig {
                auth_type: "basic".into(),
                username: Some("admin".into()),
                password: Some("p@ssw0rd!".into()),
                ..Default::default()
            })
        })]);
        assert_eq!(x.redact("password is p@ssw0rd!"), "password is [REDACTED]");
        assert_eq!(x.redact("-u admin:p@ssw0rd!"), "-u [REDACTED]");
    }
    #[test]
    fn request_sensitive_header() {
        let x = reg(&[rq(|c| {
            c.headers = hdr("X-Api-Key", "header-secret-value-xyz")
        })]);
        assert_eq!(
            x.redact("x-api-key: header-secret-value-xyz"),
            "x-api-key: [REDACTED]"
        );
    }
    #[test]
    fn request_bare_token_from_authorization() {
        let x = reg(&[rq(|c| {
            c.headers = hdr("Authorization", "Bearer rawtoken987654")
        })]);
        assert_eq!(
            x.redact("token rawtoken987654 found"),
            "token [REDACTED] found"
        );
    }
    #[test]
    fn request_non_sensitive_header_untouched() {
        let x = reg(&[rq(|c| c.headers = hdr("Content-Type", "application/json"))]);
        assert_eq!(
            x.redact("Content-Type: application/json"),
            "Content-Type: application/json"
        );
    }
    #[test]
    fn request_multiple_requests() {
        let tok = |t: &str| {
            rq(|c| {
                c.auth = Some(AuthConfig {
                    auth_type: "bearer".into(),
                    token: Some(t.into()),
                    ..Default::default()
                })
            })
        };
        let x = reg(&[tok("tokenAAA"), tok("tokenBBB")]);
        assert_eq!(
            x.redact("a=tokenAAA b=tokenBBB"),
            "a=[REDACTED] b=[REDACTED]"
        );
    }
    #[test]
    fn request_tokens_sharing_a_prefix_are_both_registered() {
        // TS keys secrets by the first 8 chars, so the second overwrote the
        // first; we key by value.
        let tok = |t: &str| {
            rq(|c| {
                c.auth = Some(AuthConfig {
                    auth_type: "bearer".into(),
                    token: Some(t.into()),
                    ..Default::default()
                })
            })
        };
        let x = reg(&[tok("samepref-1"), tok("samepref-2")]);
        assert_eq!(x.redact("samepref-1 samepref-2"), "[REDACTED] [REDACTED]");
    }
    #[test]
    fn summary_json_is_redacted() {
        let x = reg(&[rq(|c| {
            c.auth = Some(AuthConfig {
                auth_type: "bearer".into(),
                token: Some("json-secret-token".into()),
                ..Default::default()
            })
        })]);
        let s = serde_json::to_string_pretty(
            &serde_json::json!({"request": {"auth": {"token": "json-secret-token"}}}),
        )
        .unwrap();
        let red = x.redact(&s);
        assert!(!red.contains("json-secret-token"));
        assert!(red.contains("[REDACTED]"));
    }

    // --- env vars ---
    fn vars(p: &[(&str, &str)]) -> IndexMap<String, String> {
        p.iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect()
    }
    #[test]
    fn from_env_vars_uses_secret_prefix_only() {
        let x = Redactor::from_env_vars(
            &vars(&[
                ("SECRET_A", "alpha-value"),
                ("PUBLIC", "pubvalue"),
                ("SECRET_EMPTY", ""),
            ]),
            true,
        );
        assert_eq!(x.secret_count(), 1);
        assert_eq!(x.redact("alpha-value pubvalue"), "[REDACTED] pubvalue");
    }
    #[test]
    fn from_env_vars_disabled_registers_nothing_but_patterns_stay_on() {
        // TS: --no-redact / env.redactSecrets=false skip explicit secrets
        // only; pattern redaction stays on.
        let x = Redactor::from_env_vars(&vars(&[("SECRET_A", "alpha-value")]), false);
        assert!(!x.has_secrets());
        assert_eq!(x.redact("alpha-value"), "alpha-value");
        assert_eq!(x.redact("AKIAIOSFODNN7EXAMPLE"), "[REDACTED]");
    }

    // --- edge cases ---
    #[test]
    fn edge_cases() {
        let x = r(&["secret"]);
        assert_eq!(x.redact(""), "");
        assert_eq!(x.redact("   \n\t  "), "   \n\t  ");
        let re = r(&["secret.*+?^${}()|[]\\"]);
        assert_eq!(
            re.redact("value: secret.*+?^${}()|[]\\"),
            "value: [REDACTED]"
        );
        assert_eq!(
            r(&["пароль123"]).redact("password: пароль123"),
            "password: [REDACTED]"
        );
        let long = format!("{}secret{}", "prefix".repeat(10000), "suffix".repeat(10000));
        let red = x.redact(&long);
        assert!(red.contains("[REDACTED]") && !red.contains("secret"));
    }
}
