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
//!
//! Every ping that passes the door is recorded (best-effort) as the
//! door's last-ping probe in `flow_inbound_probes` — caller IP,
//! whitelisted headers, capped body (`InboundProbeRecord`) — which the
//! builder's webhook trigger "Incoming data" tab reads via
//! `GET /app/flows/{id}/inbound/last` (`InboundService::last_probe`).
//! Pings rejected by the door never reach the flow, so they're never
//! probed.

pub mod entities;
pub mod services;

pub use entities::{InboundPayload, InboundProbe, WebhookNodeConfig};
pub use services::InboundService;
