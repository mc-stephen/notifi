//! Ports — the traits the domain depends on and `infra` implements.
//!
//! Framework-free by rule: no axum/sqlx/reqwest types may appear here.

pub mod audit_store;
pub mod auth_store;
pub mod channel_provider_store;
pub mod oauth;
pub mod projects_store;
pub mod provider_tester;
pub mod recipients_store;
pub mod templates_store;
pub mod tickets_store;

pub use audit_store::{AuditFilters, AuditStore};
pub use auth_store::{AuthStore, BoxFut, OnboardingInput, StoreError};
pub use channel_provider_store::ChannelProviderStore;
pub use oauth::{
    AuthorizeStart, OAuthError, OAuthIdentityProvider, OAuthProfile, OAuthRuntime,
};
pub use projects_store::{ProjectSummary, ProjectsStore};
pub use provider_tester::{ProviderTester, TestResult};
pub use recipients_store::{RecipientRecord, RecipientsStore};
pub use templates_store::{AttachmentInput, AttachmentRecord, TemplateRecord, TemplatesStore};
pub use tickets_store::{TicketMessageRecord, TicketRecord, TicketsStore};
