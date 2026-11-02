use std::net::SocketAddr;

use curl_runner_server::{healthcheck, port_from_env, serve};
use tokio::net::TcpListener;
use tokio::signal;
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() -> std::process::ExitCode {
    let port = port_from_env(std::env::var("PORT").ok().as_deref());

    // `healthcheck` is for the container HEALTHCHECK: the image has no curl.
    if std::env::args().nth(1).as_deref() == Some("healthcheck") {
        return if healthcheck(port).await {
            std::process::ExitCode::SUCCESS
        } else {
            std::process::ExitCode::FAILURE
        };
    }

    tracing_subscriber::fmt()
        .with_ansi(false)
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .init();

    let addr = SocketAddr::from(([0, 0, 0, 0], port));
    let listener = match TcpListener::bind(addr).await {
        Ok(listener) => listener,
        Err(err) => {
            tracing::error!("cannot bind {addr}: {err}");
            return std::process::ExitCode::FAILURE;
        }
    };
    tracing::info!("listening on {addr}");

    match serve(listener, shutdown_signal()).await {
        Ok(()) => std::process::ExitCode::SUCCESS,
        Err(err) => {
            tracing::error!("server error: {err}");
            std::process::ExitCode::FAILURE
        }
    }
}

/// Resolves on Ctrl-C or SIGTERM (what Docker and Coolify send on stop).
async fn shutdown_signal() {
    let ctrl_c = async {
        let _ = signal::ctrl_c().await;
    };
    #[cfg(unix)]
    let terminate = async {
        if let Ok(mut sig) = signal::unix::signal(signal::unix::SignalKind::terminate()) {
            sig.recv().await;
        }
    };
    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();
    tokio::select! {
        () = ctrl_c => {},
        () = terminate => {},
    }
}
