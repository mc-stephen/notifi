//! Redis client creation.

use crate::infra::config::AppConfig;

/// The one label everything this platform writes on a Redis pad starts
/// with (`app:feature:key` read left to right: `notifi:` is the app,
/// each feature adds its own page label — e.g. the config cache keys
/// `notifi:cfg:…`). Shared-pad etiquette: any sweeper that only knows
/// this label can never erase a neighbour's notes.
pub const APP_KEY_PREFIX: &str = "notifi:";

/// Builds a Redis client — required: the runtime boots with it or not at
/// all.
///
/// Returns an error naming the gap when the URL is unset or invalid.
/// The machine must be reachable too — boot performs the real handshake
/// and aborts on failure.
pub fn connect(config: &AppConfig) -> Result<redis::Client, String> {
    let url = config.redis.url.as_deref().ok_or_else(|| {
        "redis not configured — set NOTIFI_REDIS_URL (sessions and the config cache ride it)"
            .to_string()
    })?;

    redis::Client::open(url).map_err(|e| format!("invalid redis url: {e}"))
}
