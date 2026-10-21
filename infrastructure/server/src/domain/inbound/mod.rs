//! Inbound webhook trigger support.
//!
//! A webhook trigger node carries an unguessable token. External systems
//! POST it to fire the owning flow. The flow definition is the source of
//! truth; `flow_trigger_tokens` is the lookup index synced on saves.

pub mod entities;
pub mod services;

pub use entities::{InboundPayload, WebhookNodeConfig};
pub use services::InboundService;
