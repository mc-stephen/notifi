//! Domain layer — framework-free business rules (no axum/sqlx types).

pub mod auth;
pub mod projects;

/// Re-exported so the rest of the crate can `use crate::domain::AuthService`.
pub use auth::AuthService;
