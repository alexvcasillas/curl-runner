//! Request extractors shared by every route.

use std::net::SocketAddr;

use axum::extract::rejection::JsonRejection;
use axum::extract::{ConnectInfo, FromRequest, FromRequestParts, Request};
use axum::http::request::Parts;
use curl_runner_cloud_proto::ErrorCode;
use serde::de::DeserializeOwned;

use crate::error::ApiError;
use crate::state::AppState;

/// `Json<T>` whose rejections are `400 invalid_request` in the API error shape.
pub struct ApiJson<T>(pub T);

impl<S, T> FromRequest<S> for ApiJson<T>
where
    S: Send + Sync,
    T: DeserializeOwned,
{
    type Rejection = ApiError;

    async fn from_request(req: Request, state: &S) -> Result<Self, Self::Rejection> {
        match axum::Json::<T>::from_request(req, state).await {
            Ok(axum::Json(value)) => Ok(Self(value)),
            Err(rejection) => Err(json_rejection(&rejection)),
        }
    }
}

fn json_rejection(rejection: &JsonRejection) -> ApiError {
    let message = match rejection {
        JsonRejection::MissingJsonContentType(_) => "Send the body as application/json.",
        JsonRejection::BytesRejection(_) => "The request body could not be read.",
        _ => "The request body is not valid JSON for this endpoint.",
    };
    ApiError::new(ErrorCode::InvalidRequest, message)
}

/// `Query<T>` whose rejections are `400 invalid_request` in the API error shape.
pub struct ApiQuery<T>(pub T);

impl<S, T> FromRequestParts<S> for ApiQuery<T>
where
    S: Send + Sync,
    T: DeserializeOwned,
{
    type Rejection = ApiError;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        match axum::extract::Query::<T>::from_request_parts(parts, state).await {
            Ok(axum::extract::Query(value)) => Ok(Self(value)),
            Err(_) => Err(ApiError::new(
                ErrorCode::InvalidRequest,
                "The query string is not valid for this endpoint.",
            )),
        }
    }
}

/// The caller's IP. Behind `TRUSTED_PROXY_HOPS` proxies it is the entry that
/// many places from the right of `X-Forwarded-For` (earlier entries are
/// client-controlled). With too few entries, or 0 hops, it is the socket peer;
/// without that, `"unknown"`.
pub struct ClientIp(pub String);

impl FromRequestParts<AppState> for ClientIp {
    type Rejection = std::convert::Infallible;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        Ok(Self(client_ip(parts, state.config.trusted_proxy_hops)))
    }
}

fn client_ip(parts: &Parts, hops: usize) -> String {
    let forwarded = parts
        .headers
        .get_all("x-forwarded-for")
        .iter()
        .filter_map(|v| v.to_str().ok())
        .flat_map(|v| v.split(','))
        .map(str::trim)
        .filter(|ip| !ip.is_empty())
        .collect::<Vec<_>>();
    if hops > 0 && forwarded.len() >= hops {
        return forwarded[forwarded.len() - hops].to_owned();
    }
    parts
        .extensions
        .get::<ConnectInfo<SocketAddr>>()
        .map_or_else(|| "unknown".to_owned(), |c| c.0.ip().to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::Router;
    use axum::body::Body;
    use axum::http::{Request, StatusCode, header};
    use axum::routing::post;
    use http_body_util::BodyExt;
    use serde::Deserialize;
    use tower::ServiceExt;

    #[derive(Deserialize)]
    struct Body1 {
        n: u32,
    }

    async fn echo(ApiJson(b): ApiJson<Body1>) -> String {
        b.n.to_string()
    }

    async fn send(content_type: Option<&str>, body: &str) -> (StatusCode, String) {
        let mut req = Request::builder().method("POST").uri("/");
        if let Some(ct) = content_type {
            req = req.header(header::CONTENT_TYPE, ct);
        }
        let res = Router::new()
            .route("/", post(echo))
            .oneshot(req.body(Body::from(body.to_owned())).unwrap())
            .await
            .unwrap();
        let status = res.status();
        let bytes = res.into_body().collect().await.unwrap().to_bytes();
        (status, String::from_utf8(bytes.to_vec()).unwrap())
    }

    #[tokio::test]
    async fn valid_json_passes_through() {
        let (status, body) = send(Some("application/json"), r#"{"n":7}"#).await;
        assert_eq!((status, body.as_str()), (StatusCode::OK, "7"));
    }

    #[tokio::test]
    async fn malformed_json_is_400_invalid_request() {
        let (status, body) = send(Some("application/json"), "{nope").await;
        assert_eq!(status, StatusCode::BAD_REQUEST);
        assert!(body.contains(r#""error":"invalid_request""#), "{body}");
        assert!(body.contains(r#""message""#));
    }

    #[tokio::test]
    async fn wrong_shape_and_missing_content_type_are_400() {
        let (status, body) = send(Some("application/json"), r#"{"n":"x"}"#).await;
        assert_eq!(status, StatusCode::BAD_REQUEST);
        assert!(body.contains(r#""error":"invalid_request""#));
        let (status, body) = send(None, r#"{"n":1}"#).await;
        assert_eq!(status, StatusCode::BAD_REQUEST);
        assert!(body.contains(r#""error":"invalid_request""#));
    }

    fn parts(forwarded: Option<&str>, peer: Option<&str>) -> Parts {
        let mut req = Request::builder().uri("/");
        if let Some(f) = forwarded {
            req = req.header("x-forwarded-for", f);
        }
        let mut req = req.body(()).unwrap();
        if let Some(p) = peer {
            let addr: SocketAddr = p.parse().unwrap();
            req.extensions_mut().insert(ConnectInfo(addr));
        }
        req.into_parts().0
    }

    #[test]
    fn client_ip_takes_nth_entry_from_the_right() {
        let p = parts(Some("spoofed, 1.2.3.4"), Some("10.0.0.2:5000"));
        assert_eq!(client_ip(&p, 1), "1.2.3.4");
        assert_eq!(client_ip(&p, 2), "spoofed");
    }

    #[test]
    fn client_ip_uses_peer_for_zero_hops_or_short_list() {
        let p = parts(Some("spoofed, 1.2.3.4"), Some("10.0.0.2:5000"));
        assert_eq!(client_ip(&p, 0), "10.0.0.2");
        assert_eq!(client_ip(&p, 3), "10.0.0.2");
        assert_eq!(
            client_ip(&parts(None, Some("10.0.0.2:5000")), 1),
            "10.0.0.2"
        );
        assert_eq!(client_ip(&parts(None, None), 1), "unknown");
        assert_eq!(client_ip(&parts(Some(" "), None), 1), "unknown");
    }
}
