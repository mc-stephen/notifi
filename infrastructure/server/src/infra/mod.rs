//! Concrete drivers for the ports: Postgres repositories, the OAuth HTTP
//! client, config loading, telemetry, and cache connections.

pub mod admin_repository_pg;
pub mod api_keys_repository_pg;
pub mod audit_repository_pg;
pub mod billing_repository_pg;
pub mod campaign_repository_pg;
pub mod channel_provider_repository_pg;
pub mod config;
pub mod db;
pub mod delivery_adapters;
pub mod flow_repository_pg;
pub mod http_requester;
pub mod notifications_repository_pg;
pub mod oauth_provider_http;
pub mod provider_tester;
pub mod rabbit;
pub mod recipients_repository_pg;
pub mod redis;
pub mod repository_pg;
pub mod schedule_repository_pg;
pub mod send_repository_pg;
pub mod smtp_mailer;
pub mod system_templates;
pub mod telemetry;
pub mod templates_repository_pg;
pub mod tickets_repository_pg;
pub mod trigger_token_repository_pg;
pub mod webhook_repository_pg;

pub use admin_repository_pg::PgAdminStore;
pub use api_keys_repository_pg::PgApiKeysStore;
pub use billing_repository_pg::PgBillingStore;
pub use campaign_repository_pg::PgCampaignStore;
pub use channel_provider_repository_pg::PgChannelProviderStore;
pub use flow_repository_pg::PgFlowStore;
pub use http_requester::ReqwestHttpRequester;
pub use notifications_repository_pg::PgNotificationsStore;
pub use provider_tester::ConfigProviderTester;

pub use oauth_provider_http::{ProviderCredentials, http_oauth_provider};
pub use recipients_repository_pg::PgRecipientsStore;
pub use repository_pg::PgAuthStore;
pub use schedule_repository_pg::PgScheduleStore;
pub use send_repository_pg::PgSendStore;
pub use smtp_mailer::LiveSmtpMailer;
pub use system_templates::{DEFAULT_TEMPLATES_DIR, FileSystemTemplates};
pub use templates_repository_pg::PgTemplatesStore;
pub use tickets_repository_pg::PgTicketsStore;
pub use trigger_token_repository_pg::PgTriggerTokenStore;
pub use webhook_repository_pg::PgWebhookStore;
