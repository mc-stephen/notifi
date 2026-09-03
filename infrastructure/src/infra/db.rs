//! Postgres pool creation + migration runner.

use crate::infra::config::AppConfig;
use sqlx::PgPool;

/// Connects to Postgres and applies pending migrations, if configured.
///
/// Returns `None` when `NOTIFI_DATABASE_URL` is unset or the connection/migration
/// fails — the API keeps booting and readiness reports the gap.
pub async fn connect(config: &AppConfig) -> Option<PgPool> {
    let url = config.database.url.as_deref()?;

    let pool = match PgPool::connect(url).await {
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

    let pool = PgPool::connect(url)
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
