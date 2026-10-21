//! Domain layer — framework-free business rules (no axum/sqlx types).

pub mod admin;
pub mod api_keys;
pub mod audit;
pub mod auth;
pub mod billing;
pub mod campaigns;
pub mod channels;
pub mod error;
pub mod flows;
pub mod inbound;
pub mod notifications;
pub mod projects;
pub mod recipients;
pub mod schedules;
pub mod sends;
pub mod support;
pub mod templates;
pub mod webhooks;

/// Re-exported so the rest of the crate can `use crate::domain::AuthService`.
pub use auth::AuthService;
