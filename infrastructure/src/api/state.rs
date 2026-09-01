//! Application state shared across handlers.

use sqlx::PgPool;
use std::sync::Arc;

/// Shared dependencies owned by the HTTP layer.
///
/// Database and Redis are optional so the API can boot (and report readiness)
/// before external services are configured. Auth and projects are built only
/// when the database exists; without it their routes answer 503 problem
/// documents.
#[derive(Clone)]
pub struct AppState {
    pub db: Option<PgPool>,
    pub redis: Option<redis::Client>,
    pub auth: Option<Arc<crate::domain::auth::AuthService>>,
    /// OAuth providers wired only when at least one has credentials.
    pub oauth: Option<Arc<crate::ports::oauth::OAuthRuntime>>,
    /// Project listing and environment gate — wired alongside auth.
    pub projects: Option<Arc<crate::domain::projects::ProjectService>>,
}
