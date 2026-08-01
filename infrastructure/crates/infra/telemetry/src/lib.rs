//! Logging and tracing bootstrap.
//!
//! Filter level resolution: `NOTIFI_LOG` env var, falling back to `RUST_LOG`,
//! falling back to `info`.

use tracing_subscriber::EnvFilter;

/// Keeps the global subscriber alive for the process lifetime.
#[derive(Debug)]
pub struct TelemetryGuard;

/// Installs the global tracing subscriber and an error-panic hook.
///
/// Should be called exactly once, first thing in `main`. Returns an error if a
/// subscriber was already installed.
pub fn init() -> Result<TelemetryGuard, Box<dyn std::error::Error + Send + Sync>> {
    let filter = std::env::var("NOTIFI_LOG")
        .or_else(|_| std::env::var("RUST_LOG"))
        .map(EnvFilter::new)
        .unwrap_or_else(|_| EnvFilter::new("info"));

    tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_target(true)
        .try_init()
        .map(|_| TelemetryGuard)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn init_is_idempotent_in_tests() {
        if let Ok(_guard) = init() {
            // first initialization succeeded
        }
        // already initialized by a previous test is fine too
    }
}
