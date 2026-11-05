//! The API error type. Every error response is an `ErrorBody`:
//! `{"error": "<code>", "message": "<one sentence>"}`.

use std::fmt;

use axum::Json;
use axum::http::{HeaderValue, StatusCode, header};
use axum::response::{IntoResponse, Response};
use curl_runner_cloud_proto::{ErrorBody, ErrorCode};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ApiError {
    pub code: ErrorCode,
    pub message: String,
    /// Seconds until a rate limit resets, sent as `Retry-After`.
    pub retry_after: Option<u64>,
}

impl ApiError {
    pub fn new(code: ErrorCode, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
            retry_after: None,
        }
    }

    /// `429 rate_limited`, retryable after `seconds`.
    pub fn rate_limited(seconds: u64) -> Self {
        Self {
            retry_after: Some(seconds),
            ..Self::new(
                ErrorCode::RateLimited,
                "Too many requests. Try again in a few minutes.",
            )
        }
    }

    pub fn not_found() -> Self {
        Self::new(ErrorCode::NotFound, "Not found.")
    }

    pub fn unavailable() -> Self {
        Self::new(
            ErrorCode::Unavailable,
            "The service is unavailable. Try again shortly.",
        )
    }

    pub fn status(&self) -> StatusCode {
        match self.code {
            ErrorCode::InvalidRequest | ErrorCode::InvalidBundle => StatusCode::BAD_REQUEST,
            ErrorCode::Unauthorized | ErrorCode::CodeInvalid | ErrorCode::CodeExpired => {
                StatusCode::UNAUTHORIZED
            }
            ErrorCode::Forbidden | ErrorCode::SignupClosed | ErrorCode::AccountSuspended => {
                StatusCode::FORBIDDEN
            }
            ErrorCode::NotFound => StatusCode::NOT_FOUND,
            ErrorCode::Conflict | ErrorCode::WrongSource => StatusCode::CONFLICT,
            ErrorCode::PlanLimit => StatusCode::PAYMENT_REQUIRED,
            ErrorCode::RateLimited => StatusCode::TOO_MANY_REQUESTS,
            ErrorCode::Unavailable => StatusCode::SERVICE_UNAVAILABLE,
            // Client-side only (a code from a newer server); never built here.
            ErrorCode::Unknown => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }
}

impl fmt::Display for ApiError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}: {}", self.code, self.message)
    }
}

impl std::error::Error for ApiError {}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let status = self.status();
        let body = ErrorBody {
            error: self.code,
            message: self.message,
        };
        let mut res = (status, Json(body)).into_response();
        if let Some(seconds) = self.retry_after {
            res.headers_mut()
                .insert(header::RETRY_AFTER, HeaderValue::from(seconds));
        }
        res
    }
}

impl From<sqlx::Error> for ApiError {
    fn from(err: sqlx::Error) -> Self {
        tracing::error!("database error: {err}");
        Self::unavailable()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use http_body_util::BodyExt;

    #[test]
    fn codes_map_to_statuses() {
        let cases = [
            (ErrorCode::InvalidRequest, 400),
            (ErrorCode::InvalidBundle, 400),
            (ErrorCode::Unauthorized, 401),
            (ErrorCode::CodeInvalid, 401),
            (ErrorCode::CodeExpired, 401),
            (ErrorCode::Forbidden, 403),
            (ErrorCode::SignupClosed, 403),
            (ErrorCode::AccountSuspended, 403),
            (ErrorCode::NotFound, 404),
            (ErrorCode::Conflict, 409),
            (ErrorCode::WrongSource, 409),
            (ErrorCode::PlanLimit, 402),
            (ErrorCode::RateLimited, 429),
            (ErrorCode::Unavailable, 503),
        ];
        for (code, status) in cases {
            assert_eq!(
                ApiError::new(code, "x").status().as_u16(),
                status,
                "{code:?}"
            );
        }
    }

    #[tokio::test]
    async fn response_body_is_error_and_message() {
        let res = ApiError::new(ErrorCode::WrongSource, "Edited in the cloud.").into_response();
        assert_eq!(res.status(), StatusCode::CONFLICT);
        let body = res.into_body().collect().await.unwrap().to_bytes();
        assert_eq!(
            body,
            r#"{"error":"wrong_source","message":"Edited in the cloud."}"#.as_bytes()
        );
    }

    #[test]
    fn rate_limited_sets_retry_after() {
        let res = ApiError::rate_limited(42).into_response();
        assert_eq!(res.status(), StatusCode::TOO_MANY_REQUESTS);
        assert_eq!(res.headers()[header::RETRY_AFTER], "42");
        let res = ApiError::not_found().into_response();
        assert!(res.headers().get(header::RETRY_AFTER).is_none());
    }

    #[test]
    fn sqlx_error_becomes_unavailable() {
        let err = ApiError::from(sqlx::Error::PoolTimedOut);
        assert_eq!(err.code, ErrorCode::Unavailable);
        assert_eq!(err.status(), StatusCode::SERVICE_UNAVAILABLE);
    }
}
