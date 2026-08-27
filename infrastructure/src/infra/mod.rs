//! Concrete drivers for the ports: Postgres repositories, the OAuth HTTP
//! client, config loading, telemetry, and cache connections.

pub mod config;
pub mod db;
pub mod oauth_provider_http;
pub mod redis;
pub mod repository_pg;
pub mod telemetry;

pub use oauth_provider_http::{ProviderCredentials, http_oauth_provider};
pub use repository_pg::PgAuthStore;
