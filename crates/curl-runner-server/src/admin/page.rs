//! What every admin list shares: query parsing, keyset cursors, search
//! patterns, and the owners' emails.

use base64::Engine;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use chrono::{DateTime, Utc};
use curl_runner_cloud_proto::ErrorCode;
use uuid::Uuid;

use crate::error::ApiError;
use crate::state::AppState;
use crate::teams::first_operator;

const DEFAULT_LIMIT: i64 = 50;
const MAX_LIMIT: i64 = 200;
/// Rows read per query by a CSV export.
pub(super) const EXPORT_PAGE: i64 = 500;
/// The longest `q` a list searches for, in characters.
pub(super) const MAX_QUERY: usize = 200;

/// `400 invalid_request` with `message`.
pub(super) fn invalid(message: impl Into<String>) -> ApiError {
    ApiError::new(ErrorCode::InvalidRequest, message)
}

/// A query value, trimmed; empty is `None`.
pub(super) fn non_empty(v: Option<&String>) -> Option<&str> {
    v.map(|s| s.trim()).filter(|s| !s.is_empty())
}

/// The `limit` query value: 1 to 200, 50 when absent.
pub(super) fn page_limit(raw: Option<&String>) -> Result<i64, ApiError> {
    match non_empty(raw) {
        None => Ok(DEFAULT_LIMIT),
        Some(l) => l
            .parse()
            .ok()
            .filter(|n| (1..=MAX_LIMIT).contains(n))
            .ok_or_else(|| invalid("`limit` must be between 1 and 200.")),
    }
}

/// `s` as an `ILIKE` contains-pattern, with `%`, `_` and `\` taken literally.
pub(super) fn like_pattern(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 2);
    out.push('%');
    for c in s.chars() {
        if matches!(c, '%' | '_' | '\\') {
            out.push('\\');
        }
        out.push(c);
    }
    out.push('%');
    out
}

/// The `q` query value as an `ILIKE` pattern; at most [`MAX_QUERY`]
/// characters.
pub(super) fn search_pattern(q: Option<&String>) -> Result<Option<String>, ApiError> {
    match non_empty(q) {
        Some(q) if q.chars().count() > MAX_QUERY => Err(invalid("`q` is too long.")),
        Some(q) => Ok(Some(like_pattern(q))),
        None => Ok(None),
    }
}

/// Position after the last row of a page: `(at, id)` of that row.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) struct Cursor {
    pub(super) at: DateTime<Utc>,
    pub(super) id: Uuid,
}

impl Cursor {
    pub(super) fn encode(self) -> String {
        URL_SAFE_NO_PAD.encode(format!("{}:{}", self.at.timestamp_micros(), self.id))
    }

    pub(super) fn decode(s: &str) -> Option<Self> {
        let raw = String::from_utf8(URL_SAFE_NO_PAD.decode(s).ok()?).ok()?;
        let (micros, id) = raw.split_once(':')?;
        Some(Self {
            at: DateTime::from_timestamp_micros(micros.parse().ok()?)?,
            id: id.parse().ok()?,
        })
    }

    /// The `cursor` query value; `400` when it is not one.
    pub(super) fn parse(raw: Option<&String>) -> Result<Option<Self>, ApiError> {
        non_empty(raw)
            .map(|c| Self::decode(c).ok_or_else(|| invalid("`cursor` is not valid.")))
            .transpose()
    }
}

/// The owners' emails, lowercased, once each: `OPERATOR_EMAILS` in order,
/// then the first operator.
pub(super) async fn owner_emails(state: &AppState) -> Result<Vec<String>, ApiError> {
    let mut owners: Vec<String> = Vec::new();
    let first = first_operator(&state.db).await?;
    for email in state.config.operators.iter().chain(first.as_ref()) {
        let email = email.to_lowercase();
        if !owners.contains(&email) {
            owners.push(email);
        }
    }
    Ok(owners)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cursor_round_trips() {
        let c = Cursor {
            at: DateTime::from_timestamp_micros(1_762_250_400_123_456).unwrap(),
            id: Uuid::new_v4(),
        };
        assert_eq!(Cursor::decode(&c.encode()), Some(c));
        assert_eq!(Cursor::decode("nope"), None);
        assert_eq!(Cursor::decode(""), None);
        assert!(Cursor::parse(Some(&"nope".to_owned())).is_err());
        assert_eq!(Cursor::parse(Some(&"  ".to_owned())).unwrap(), None);
    }

    #[test]
    fn like_patterns_take_wildcards_literally() {
        assert_eq!(like_pattern("ada"), "%ada%");
        assert_eq!(like_pattern("100%_\\"), "%100\\%\\_\\\\%");
    }

    #[test]
    fn search_is_trimmed_and_capped() {
        assert_eq!(search_pattern(None).unwrap(), None);
        assert_eq!(search_pattern(Some(&" ".to_owned())).unwrap(), None);
        assert_eq!(
            search_pattern(Some(&" ada ".to_owned()))
                .unwrap()
                .as_deref(),
            Some("%ada%")
        );
        assert!(search_pattern(Some(&"x".repeat(MAX_QUERY + 1))).is_err());
    }

    #[test]
    fn limits() {
        assert_eq!(page_limit(None).unwrap(), DEFAULT_LIMIT);
        assert_eq!(page_limit(Some(&"200".to_owned())).unwrap(), 200);
        for bad in ["0", "201", "x"] {
            assert!(page_limit(Some(&bad.to_owned())).is_err(), "{bad}");
        }
    }
}
