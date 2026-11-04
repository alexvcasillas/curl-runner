//! curl-runner server: the Cloud backend (accounts, teams, publish, sharing).

pub mod auth;
pub mod clock;
pub mod collections;
pub mod config;
pub mod crypto;
pub mod error;
pub mod extract;
pub mod hash;
pub mod info;
pub mod mail;
pub mod plans;
pub mod state;
pub mod teams;
pub mod tokens;

use std::future::Future;
use std::time::Duration;

use axum::routing::get;
use axum::{Json, Router};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tower_http::cors::CorsLayer;
use tower_http::trace::{DefaultOnResponse, TraceLayer};
use tracing::{Level, Span};

pub use error::ApiError;
pub use state::AppState;

const DEFAULT_PORT: u16 = 8080;

/// The application router.
pub fn app(state: AppState) -> Router {
    Router::new()
        .route("/", get(health))
        .route("/health", get(health))
        .route("/v1/info", get(info::info))
        .merge(auth::routes())
        .merge(teams::routes())
        .merge(tokens::routes())
        .merge(collections::routes())
        .fallback(not_found)
        .with_state(state)
        .layer(CorsLayer::permissive())
        .layer(
            TraceLayer::new_for_http()
                .make_span_with(make_span)
                .on_response(DefaultOnResponse::new().level(Level::INFO)),
        )
}

/// The request span. Share-link and invite tokens travel in the path, so it
/// records the method and the matched route (`/v1/shared/{token}`), never the
/// raw URI or query. Unmatched requests are a fixed `unmatched`.
fn make_span<B>(req: &axum::http::Request<B>) -> Span {
    let route = req
        .extensions()
        .get::<axum::extract::MatchedPath>()
        .map_or("unmatched", axum::extract::MatchedPath::as_str);
    tracing::span!(Level::INFO, "request", method = %req.method(), route = %route)
}

async fn health() -> Json<serde_json::Value> {
    Json(serde_json::json!({"status": "ok", "version": env!("CARGO_PKG_VERSION")}))
}

async fn not_found() -> ApiError {
    ApiError::not_found()
}

/// Serve [`app`] on `listener` until `shutdown` resolves, draining in-flight requests.
pub async fn serve(
    state: AppState,
    listener: TcpListener,
    shutdown: impl Future<Output = ()> + Send + 'static,
) -> std::io::Result<()> {
    serve_router(app(state), listener, shutdown).await
}

async fn serve_router(
    router: Router,
    listener: TcpListener,
    shutdown: impl Future<Output = ()> + Send + 'static,
) -> std::io::Result<()> {
    axum::serve(
        listener,
        router.into_make_service_with_connect_info::<std::net::SocketAddr>(),
    )
    .with_graceful_shutdown(shutdown)
    .await
}

/// Parse the `PORT` env value; missing or invalid values fall back to 8080.
pub fn port_from_env(value: Option<&str>) -> u16 {
    value
        .and_then(|v| v.trim().parse().ok())
        .unwrap_or(DEFAULT_PORT)
}

/// True when `GET http://127.0.0.1:{port}/health` answers 200. Raw TCP, so the
/// minimal container image needs no HTTP client.
pub async fn healthcheck(port: u16) -> bool {
    get_status(port, "/health").await == Some(200)
}

/// Status code of a plain HTTP/1.1 `GET` against localhost, or `None` on any failure.
async fn get_status(port: u16, path: &str) -> Option<u16> {
    let exchange = async {
        let mut stream = TcpStream::connect(("127.0.0.1", port)).await.ok()?;
        let request =
            format!("GET {path} HTTP/1.1\r\nHost: 127.0.0.1\r\nConnection: close\r\n\r\n");
        stream.write_all(request.as_bytes()).await.ok()?;
        let mut response = Vec::new();
        stream.read_to_end(&mut response).await.ok()?;
        let line = std::str::from_utf8(&response).ok()?.lines().next()?;
        line.split(' ').nth(1)?.parse().ok()
    };
    tokio::time::timeout(Duration::from_secs(5), exchange)
        .await
        .ok()
        .flatten()
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::Body;
    use axum::http::{Request, StatusCode, header};
    use axum::routing::get;
    use http_body_util::BodyExt;
    use std::time::Duration;
    use tokio::net::TcpListener;
    use tower::ServiceExt;

    use std::sync::{Arc, Mutex};

    use crate::clock::SystemClock;
    use crate::config::Config;

    /// State over a pool that never connects: these routes don't touch the DB.
    fn state() -> AppState {
        let config = Config::from_env(|k| match k {
            "DATABASE_URL" => Some("postgres://u@localhost/d".into()),
            "SECRET_KEY" => Some("k".repeat(32)),
            _ => None,
        })
        .unwrap();
        let pool = sqlx::postgres::PgPoolOptions::new().connect_lazy_with(config.database.clone());
        AppState::new(pool, config, std::sync::Arc::new(SystemClock))
    }

    async fn send(method: &str, path: &str, origin: Option<&str>) -> axum::response::Response {
        let mut req = Request::builder().method(method).uri(path);
        if let Some(o) = origin {
            req = req.header(header::ORIGIN, o);
        }
        app(state())
            .oneshot(req.body(Body::empty()).unwrap())
            .await
            .unwrap()
    }

    #[tokio::test]
    async fn health_returns_ok_and_version() {
        let res = send("GET", "/health", None).await;
        assert_eq!(res.status(), StatusCode::OK);
        assert_eq!(res.headers()[header::CONTENT_TYPE], "application/json");
        let body = res.into_body().collect().await.unwrap().to_bytes();
        let expected = format!(
            r#"{{"status":"ok","version":"{}"}}"#,
            env!("CARGO_PKG_VERSION")
        );
        assert_eq!(body, expected.as_bytes());
    }

    #[tokio::test]
    async fn health_allows_any_origin() {
        let res = send("GET", "/health", Some("https://example.com")).await;
        assert_eq!(res.headers()[header::ACCESS_CONTROL_ALLOW_ORIGIN], "*");
    }

    #[tokio::test]
    async fn root_returns_the_same_as_health() {
        let health = send("GET", "/health", None).await;
        let health_body = health.into_body().collect().await.unwrap().to_bytes();

        let res = send("GET", "/", None).await;
        assert_eq!(res.status(), StatusCode::OK);
        assert_eq!(res.headers()[header::CONTENT_TYPE], "application/json");
        let body = res.into_body().collect().await.unwrap().to_bytes();
        assert_eq!(body, health_body);
    }

    #[tokio::test]
    async fn root_allows_any_origin() {
        let res = send("GET", "/", Some("https://example.com")).await;
        assert_eq!(res.status(), StatusCode::OK);
        assert_eq!(res.headers()[header::ACCESS_CONTROL_ALLOW_ORIGIN], "*");
    }

    #[tokio::test]
    async fn unknown_path_is_404_json() {
        let res = send("GET", "/nope", None).await;
        assert_eq!(res.status(), StatusCode::NOT_FOUND);
        let body = res.into_body().collect().await.unwrap().to_bytes();
        assert_eq!(
            body,
            r#"{"error":"not_found","message":"Not found."}"#.as_bytes()
        );
    }

    #[tokio::test]
    async fn post_health_is_405() {
        let res = send("POST", "/health", None).await;
        assert_eq!(res.status(), StatusCode::METHOD_NOT_ALLOWED);
    }

    #[tokio::test]
    async fn post_root_is_405() {
        let res = send("POST", "/", None).await;
        assert_eq!(res.status(), StatusCode::METHOD_NOT_ALLOWED);
    }

    #[test]
    fn port_defaults_to_8080() {
        assert_eq!(port_from_env(None), 8080);
        assert_eq!(port_from_env(Some("")), 8080);
        assert_eq!(port_from_env(Some("abc")), 8080);
        assert_eq!(port_from_env(Some("9000")), 9000);
    }

    #[tokio::test]
    async fn healthcheck_true_when_serving_false_when_not() {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let port = listener.local_addr().unwrap().port();
        let (tx, rx) = tokio::sync::oneshot::channel::<()>();
        let server = tokio::spawn(serve(state(), listener, async {
            let _ = rx.await;
        }));
        assert!(healthcheck(port).await);
        tx.send(()).unwrap();
        server.await.unwrap().unwrap();
        // Nothing listens on the port any more.
        assert!(!healthcheck(port).await);
    }

    #[tokio::test]
    async fn shutdown_finishes_in_flight_request() {
        let slow = app(state()).route(
            "/slow",
            get(|| async {
                tokio::time::sleep(Duration::from_millis(300)).await;
                "done"
            }),
        );
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let port = listener.local_addr().unwrap().port();
        let (tx, rx) = tokio::sync::oneshot::channel::<()>();
        let server = tokio::spawn(serve_router(slow, listener, async {
            let _ = rx.await;
        }));
        let request = tokio::spawn(get_status(port, "/slow"));
        tokio::time::sleep(Duration::from_millis(100)).await;
        tx.send(()).unwrap();
        assert_eq!(request.await.unwrap(), Some(200));
        server.await.unwrap().unwrap();
    }

    #[derive(Clone, Default)]
    struct Capture(Arc<Mutex<Vec<u8>>>);

    impl std::io::Write for Capture {
        fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
            self.0.lock().unwrap().extend_from_slice(buf);
            Ok(buf.len())
        }
        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }

    #[tokio::test]
    async fn request_logs_carry_the_route_not_the_token() {
        let capture = Capture::default();
        let sink = capture.clone();
        let subscriber = tracing_subscriber::fmt()
            .with_writer(move || sink.clone())
            .with_ansi(false)
            .finish();
        let _guard = tracing::subscriber::set_default(subscriber);
        let router = Router::new()
            .route("/v1/shared/{token}", get(|| async { "ok" }))
            .layer(
                TraceLayer::new_for_http()
                    .make_span_with(make_span)
                    .on_response(DefaultOnResponse::new().level(Level::INFO)),
            );
        for uri in ["/v1/shared/crsl_SECRET?k=crsl_SECRET", "/nope/crsl_SECRET"] {
            router
                .clone()
                .oneshot(Request::builder().uri(uri).body(Body::empty()).unwrap())
                .await
                .unwrap();
        }
        let out = String::from_utf8(capture.0.lock().unwrap().clone()).unwrap();
        assert!(!out.contains("crsl_SECRET"), "{out}");
        assert!(out.contains("route=/v1/shared/{token}"), "{out}");
        assert!(out.contains("route=unmatched"), "{out}");
    }
}
