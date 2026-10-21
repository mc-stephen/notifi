//! Redis client creation.

use crate::infra::config::AppConfig;

/// Builds a Redis client, if configured.
///
/// Returns `None` when `NOTIFI_REDIS_URL` is unset or the URL is invalid —
/// the API keeps booting and readiness reports the gap.
pub fn connect(config: &AppConfig) -> Option<redis::Client> {
    let url = config.redis.url.as_deref()?;

    match redis::Client::open(url) {
        Ok(client) => {
            tracing::info!("redis configured");
            Some(client)
        }
        Err(e) => {
            tracing::error!(error = %e, "invalid redis url; continuing without redis");
            None
        }
    }
}
