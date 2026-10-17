//! Framework-free kernel for Notifi.
//!
//! This crate has **no** dependencies on web frameworks, databases, or HTTP
//! clients. It is the foundation every domain, port, adapter, and binary
//! builds upon.
//!
//! Modules:
//!
//! * [`config`] — tenant config directory resolution and JSON loading.
//! * [`error`] — unified [`ApiError`] model (RFC 9457 mapping at the edge).
//! * [`event`] — domain event trait and envelope.
//! * [`id`] — ULID-backed typed identifiers.
//! * [`outbox`] — transactional outbox data model.

pub mod config;
pub mod error;
pub mod event;
pub mod id;
pub mod outbox;

pub use error::{ApiError, IntoApiError};
pub use event::{DomainEvent, EventEnvelope};
pub use id::Ulid;
