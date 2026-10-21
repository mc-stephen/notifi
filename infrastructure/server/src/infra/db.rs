//! Postgres pool creation + migration runner.

use crate::infra::config::AppConfig;
use sqlx::PgPool;
use sqlx::postgres::PgPoolOptions;

/// Builds the pool: explicit size/timeouts so pressure fails loud instead
/// of queueing silently behind sqlx defaults.
fn pool_options(config: &AppConfig) -> PgPoolOptions {
    PgPoolOptions::new()
        .max_connections(config.database.max_connections.max(1))
        // Fail fast: a waiter longer than this means real pressure, and the
        // error names the pool instead of a 30s mystery hang.
        .acquire_timeout(std::time::Duration::from_secs(10))
        // Recycle idle and aging connections; a load balancer or a DB
        // restart never leaves the pool holding dead sockets.
        .idle_timeout(std::time::Duration::from_secs(600))
        .max_lifetime(std::time::Duration::from_secs(1800))
}

/// Connects to Postgres and applies pending migrations, if configured.
///
/// Returns `None` when `NOTIFI_DATABASE_URL` is unset or the connection/migration
/// fails — the API keeps booting and readiness reports the gap.
pub async fn connect(config: &AppConfig) -> Option<PgPool> {
    let url = config.database.url.as_deref()?;

    let pool = match pool_options(config).connect(url).await {
        Ok(pool) => pool,
        Err(e) => {
            tracing::error!(error = %e, "postgres connection failed");
            return None;
        }
    };

    match sqlx::migrate!("./assets/migrations").run(&pool).await {
        Ok(_) => tracing::info!("postgres connected; migrations applied"),
        Err(e) => {
            tracing::error!(error = %e, "migrations failed; continuing without database");
            return None;
        }
    }

    Some(pool)
}

/// Drops the entire `public` schema and re-applies all migrations.
///
/// Used by `cargo run -- --reset-db` for a clean dev start.
pub async fn reset(config: &AppConfig) -> Result<(), String> {
    let url = config
        .database
        .url
        .as_ref()
        .ok_or("NOTIFI_DATABASE_URL not set")?;

    let pool = pool_options(config)
        .connect(url)
        .await
        .map_err(|e| format!("connection failed: {e}"))?;

    tracing::info!("dropping all tables...");
    sqlx::query("DROP SCHEMA public CASCADE")
        .execute(&pool)
        .await
        .map_err(|e| format!("drop failed: {e}"))?;

    sqlx::query("CREATE SCHEMA public")
        .execute(&pool)
        .await
        .map_err(|e| format!("create schema failed: {e}"))?;

    tracing::info!("re-applying migrations...");
    sqlx::migrate!("./assets/migrations")
        .run(&pool)
        .await
        .map_err(|e| format!("migrate failed: {e}"))?;

    tracing::info!("database reset complete");
    Ok(())
}
