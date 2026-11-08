//! Inbound webhook trigger support.
//!
//! A webhook trigger node carries an unguessable token. External systems
//! POST it to fire the owning flow. The flow definition is the source of
//! truth; `flow_trigger_tokens` is the lookup index synced on saves.
//!
//! The door also carries its own auth on the node: listed `secrets`
//! require a matching `x-notifi-secret` header (401) and listed IP rules
//! (`allowedIps`, plain IP or CIDR) require the caller inside them (403).
//! Empty lists keep the door open — the pre-auth default.

pub mod entities;
pub mod services;

pub use entities::{InboundPayload, WebhookNodeConfig};
pub use services::InboundService;
