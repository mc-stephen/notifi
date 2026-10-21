//! Webhook management: owner-side CRUD for flow-owning webhooks.

pub mod entities;
pub mod services;

/// Re-exported so the rest of the crate can `use crate::domain::webhooks::WebhookService`.
pub use entities::Webhook;
pub use services::WebhookService;
