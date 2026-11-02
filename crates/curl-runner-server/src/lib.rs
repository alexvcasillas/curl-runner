//! curl-runner server: the sync/sharing backend. Today it only serves `GET /health`
//! (and `GET /`, which answers the same).

use std::future::Future;
use std::time::Duration;

use axum::routing::get;
use axum::{Json, Router};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tower_http::cors::CorsLayer;
use tower_http::trace::{DefaultMakeSpan, DefaultOnResponse, TraceLayer};
use tracing::Level;

const DEFAULT_PORT: u16 = 8080;

/// The application router.
pub fn app() -> Router {
    Router::new()
        .route("/", get(health))
        .route("/health", get(health))
        .layer(CorsLayer::permissive())
        .layer(
            TraceLayer::new_for_http()
                .make_span_with(DefaultMakeSpan::new().level(Level::INFO))
                .on_response(DefaultOnResponse::new().level(Level::INFO)),
        )
}

async fn health() -> Json<serde_json::Value> {
    Json(serde_json::json!({"status": "ok", "version": env!("CARGO_PKG_VERSION")}))
}

/// Serve [`app`] on `listener` until `shutdown` resolves, draining in-flight requests.
pub async fn serve(
    listener: TcpListener,
    shutdown: impl Future<Output = ()> + Send + 'static,
) -> std::io::Result<()> {
    serve_router(app(), listener, shutdown).await
}

async fn serve_router(
    router: Router,
    listener: TcpListener,
    shutdown: impl Future<Output = ()> + Send + 'static,
) -> std::io::Result<()> {
    axum::serve(listener, router)
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

    async fn send(method: &str, path: &str, origin: Option<&str>) -> axum::response::Response {
        let mut req = Request::builder().method(method).uri(path);
        if let Some(o) = origin {
            req = req.header(header::ORIGIN, o);
        }
        app()
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
    async fn unknown_path_is_404() {
        assert_eq!(
            send("GET", "/nope", None).await.status(),
            StatusCode::NOT_FOUND
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
        let server = tokio::spawn(serve(listener, async {
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
        let slow = app().route(
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
}
