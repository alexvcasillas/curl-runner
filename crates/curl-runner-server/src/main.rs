use std::net::SocketAddr;
use std::sync::Arc;

use curl_runner_server::billing::sync;
use curl_runner_server::clock::{Clock, SystemClock};
use curl_runner_server::config::Config;
use curl_runner_server::mail::{LogTransport, SmtpTransport, Transport, worker};
use curl_runner_server::settings;
use curl_runner_server::{AppState, healthcheck, port_from_env, serve};
use sqlx::postgres::PgPoolOptions;
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

    let config = match Config::from_env(|key| std::env::var(key).ok()) {
        Ok(config) => config,
        Err(err) => {
            tracing::error!("{err}");
            return std::process::ExitCode::FAILURE;
        }
    };
    let pool = match PgPoolOptions::new()
        .max_connections(10)
        .connect_with(config.database.clone())
        .await
    {
        Ok(pool) => pool,
        Err(err) => {
            tracing::error!("cannot connect to Postgres: {err}");
            return std::process::ExitCode::FAILURE;
        }
    };
    if let Err(err) = sqlx::migrate!().run(&pool).await {
        tracing::error!("migrations failed: {err}");
        return std::process::ExitCode::FAILURE;
    }
    match settings::seed(&pool, &config, SystemClock.now()).await {
        Ok(outcome) => {
            if outcome.inserted {
                tracing::info!(
                    "server settings created from SIGNUP, SIGNUP_DOMAINS and DEFAULT_PLAN"
                );
            }
            for var in outcome.drifted {
                tracing::warn!(
                    "{var} changed since the server settings were created and is ignored. \
                     Change it on the admin settings page instead."
                );
            }
        }
        Err(err) => {
            tracing::error!("cannot seed the server settings: {err:?}");
            return std::process::ExitCode::FAILURE;
        }
    }
    let transport: Arc<dyn Transport> = match &config.smtp {
        Some(smtp) => match SmtpTransport::new(smtp, &config.mail_from) {
            Ok(transport) => Arc::new(transport),
            Err(err) => {
                tracing::error!("{err}");
                return std::process::ExitCode::FAILURE;
            }
        },
        None => {
            tracing::warn!("SMTP not configured; emails are written to the log");
            Arc::new(LogTransport)
        }
    };
    let state = AppState::new(pool, config, Arc::new(SystemClock));
    let (stop_tx, mut stop_rx) = tokio::sync::watch::channel(false);
    let mut sync_stop = stop_rx.clone();
    let mut broadcast_stop = stop_rx.clone();
    let mail_worker = tokio::spawn(worker::run(
        state.db.clone(),
        transport,
        state.clock.clone(),
        state.config.clone(),
        state.metrics.clone(),
        async move {
            let _ = stop_rx.wait_for(|stop| *stop).await;
        },
    ));

    let seat_worker = tokio::spawn(sync::run(state.clone(), async move {
        let _ = sync_stop.wait_for(|stop| *stop).await;
    }));
    let broadcast_worker = tokio::spawn(worker::run_broadcasts(state.clone(), async move {
        let _ = broadcast_stop.wait_for(|stop| *stop).await;
    }));

    let addr = SocketAddr::from(([0, 0, 0, 0], port));
    let listener = match TcpListener::bind(addr).await {
        Ok(listener) => listener,
        Err(err) => {
            tracing::error!("cannot bind {addr}: {err}");
            return std::process::ExitCode::FAILURE;
        }
    };
    tracing::info!("listening on {addr}");

    let result = serve(state, listener, async move {
        shutdown_signal().await;
        let _ = stop_tx.send(true);
    })
    .await;
    let _ = mail_worker.await;
    let _ = seat_worker.await;
    let _ = broadcast_worker.await;
    match result {
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
