//! Postgres pool creation + migration runner.

use notifi_infra_config::AppConfig;
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

    match sqlx::migrate!("../../migrations").run(&pool).await {
        Ok(_) => tracing::info!("postgres connected; migrations applied"),
        Err(e) => {
            tracing::error!(error = %e, "migrations failed; continuing without database");
            return None;
        }
    }

    Some(pool)
}
