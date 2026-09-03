//! Domain layer — framework-free business rules (no axum/sqlx types).

pub mod audit;
pub mod auth;
pub mod channels;
pub mod projects;
pub mod recipients;
pub mod templates;

/// Re-exported so the rest of the crate can `use crate::domain::AuthService`.
pub use auth::AuthService;
