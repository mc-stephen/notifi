//! Application state shared across handlers.

use sqlx::PgPool;

/// Shared dependencies owned by the HTTP layer.
///
/// Database and Redis are optional so the API can boot (and report readiness)
/// before external services are configured.
#[derive(Clone)]
pub struct AppState {
    pub db: Option<PgPool>,
    pub redis: Option<redis::Client>,
}
