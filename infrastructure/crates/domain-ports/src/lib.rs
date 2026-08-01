//! Trait-only contracts between domains, infrastructure, and channel adapters.
//!
//! This crate contains **interfaces only** — no implementations. Concrete
//! infrastructure (`infra/*`) and channel adapters implement these ports;
//! domains depend on them so business logic never touches concrete
//! infrastructure.
//!
//! Modules:
//!
//! * [`channel`] — per-channel config loading (adopted from `notification_core`).
//! * [`delivery`] — the port every channel adapter implements.
//! * [`event_bus`] — publishing domain event envelopes.
//! * [`queue`] — publishing job payloads to a queue.

pub mod channel;
pub mod delivery;
pub mod event_bus;
pub mod queue;

pub use channel::ChannelConfigLoader;
pub use delivery::{DeliveryError, DeliveryMessage, DeliveryProvider, DeliveryReceipt};
pub use event_bus::{EventBus, EventBusError};
pub use queue::{QueueError, QueuePublisher};
