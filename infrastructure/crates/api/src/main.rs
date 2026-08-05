//! Notifi HTTP API — composition root.
//!
//! M1: axum application core — routing, middleware (trace/request-id/logging/
//! CORS), RFC 9457 problem-details errors, Postgres pool + migrations runner,
//! Redis client, graceful shutdown.
//!
//! Bootstrap order: telemetry → config → database → redis → router → serve.

mod http;
mod infra;
mod state;

use state::AppState;

#[tokio::main]
async fn main() {
    let _guard = match notifi_infra_telemetry::init() {
        Ok(guard) => guard,
        Err(e) => {
            eprintln!("failed to initialize telemetry: {e}");
            return;
        }
    };

    let config = match notifi_infra_config::AppConfig::from_env() {
        Ok(config) => config,
        Err(e) => {
            tracing::error!("invalid configuration: {e}");
            return;
        }
    };

    tracing::info!(
        host = %config.server.host,
        port = config.server.port,
        "api bootstrap complete"
    );

    let db = infra::db::connect(&config).await;
    let redis = infra::redis::connect(&config);

    if db.is_none() {
        tracing::warn!("database unavailable/disabled; readiness will report 503");
    }
    if redis.is_none() {
        tracing::warn!("redis unavailable/disabled; readiness will report 503");
    }

    let app = http::build_router(AppState { db, redis });

    let listener = match tokio::net::TcpListener::bind((
        config.server.host.as_str(),
        config.server.port,
    ))
    .await
    {
        Ok(listener) => listener,
        Err(e) => {
            tracing::error!(
                host = %config.server.host,
                port = config.server.port,
                error = %e,
                "failed to bind listener"
            );
            return;
        }
    };

    tracing::info!(
        host = %config.server.host,
        port = config.server.port,
        "api listening"
    );

    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await
        .unwrap_or_else(|e| tracing::error!("server error: {e}"));

    tracing::info!("api shutdown complete");
}

/// Resolves on SIGINT (Ctrl-C) or SIGTERM; triggers graceful shutdown.
async fn shutdown_signal() {
    let ctrl_c = async {
        tokio::signal::ctrl_c()
            .await
            .expect("failed to install ctrl-c handler");
    };

    #[cfg(unix)]
    let terminate = async {
        tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
            .expect("failed to install SIGTERM handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {},
        _ = terminate => {},
    }
    tracing::info!("shutdown signal received");
}
