//! Ports — the traits the domain depends on and `infra`/channels implement.
//!
//! Framework-free by rule: no axum/sqlx/reqwest types may appear here.
//! Includes the delivery-channel contracts formerly in `crates/domain-ports`
//! (`channel`, `delivery`, `event_bus`, `queue`).

pub mod admin_store;
pub mod api_keys_store;
pub mod audit_store;
pub mod auth_store;
pub mod billing_store;
pub mod campaigns_store;
pub mod channel;
pub mod channel_provider_store;
pub mod config_store;
pub mod delivery;
pub mod event_bus;
pub mod flow_store;
pub mod http_requester;
pub mod mailer;
pub mod notifications_store;
pub mod oauth;
pub mod projects_store;
pub mod provider_tester;
pub mod queue;
pub mod recipients_store;
pub mod schedule_store;
pub mod send_store;
pub mod templates_store;
pub mod tickets_store;
pub mod trigger_token_store;
pub mod webhook_store;

pub use audit_store::{AdminAuditFilters, AuditFilters, AuditStore};
pub use auth_store::{AuthStore, BoxFut, OnboardingInput, StoreError};
pub use channel::ChannelConfigLoader;
pub use channel_provider_store::ChannelProviderStore;
pub use config_store::ConfigStore;
pub use delivery::{
    AdapterFactory, BoxSendFuture, DeliveryError, DeliveryMessage, DeliveryProvider,
    DeliveryReceipt,
};
pub use event_bus::{EventBus, EventBusError};
pub use http_requester::{HttpCall, HttpRequester, HttpResult};
pub use mailer::{MailError, RenderedTemplate, SmtpMailer, SystemSender, SystemTemplates};
pub use oauth::{AuthorizeStart, OAppError, OAuthIdentityProvider, OAuthProfile, OAuthRuntime};
pub use projects_store::{ProjectSummary, ProjectsStore};
pub use provider_tester::{ProviderTester, TestResult};
pub use queue::{QueueError, QueuePublisher};
pub use recipients_store::{RecipientRecord, RecipientsStore};
pub use templates_store::{AttachmentInput, AttachmentRecord, TemplateRecord, TemplatesStore};
pub use tickets_store::{TicketMessageRecord, TicketRecord, TicketsStore};
