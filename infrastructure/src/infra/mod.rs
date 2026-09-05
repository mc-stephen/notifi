//! Concrete drivers for the ports: Postgres repositories, the OAuth HTTP
//! client, config loading, telemetry, and cache connections.

pub mod audit_repository_pg;
pub mod channel_provider_repository_pg;
pub mod config;
pub mod db;
pub mod notifications_repository_pg;
pub mod oauth_provider_http;
pub mod provider_tester;
pub mod recipients_repository_pg;
pub mod redis;
pub mod repository_pg;
pub mod telemetry;
pub mod templates_repository_pg;
pub mod tickets_repository_pg;

pub use channel_provider_repository_pg::PgChannelProviderStore;
pub use notifications_repository_pg::PgNotificationsStore;
pub use provider_tester::ConfigProviderTester;

pub use oauth_provider_http::{ProviderCredentials, http_oauth_provider};
pub use recipients_repository_pg::PgRecipientsStore;
pub use repository_pg::PgAuthStore;
pub use templates_repository_pg::PgTemplatesStore;
pub use tickets_repository_pg::PgTicketsStore;
