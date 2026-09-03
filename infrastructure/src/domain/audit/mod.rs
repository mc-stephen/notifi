//! The audit slice: an in-process listener that records system actions.
//!
//! Mutating domains emit an action after a successful mutation; the audit
//! service appends an immutable entry and can serve it back to the dashboard.
//! The outbox/pgmq event bus can later subscribe to the same emissions
//! without rework (ARCHITECTURE.md §6).

pub mod entities;
pub mod services;

pub use entities::{AuditAction, AuditEntry};
pub use services::AuditService;
