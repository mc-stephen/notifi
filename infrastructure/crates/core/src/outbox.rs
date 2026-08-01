use crate::event::EventEnvelope;
use serde::{Deserialize, Serialize};

/// Lifecycle of an outbox record.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OutboxStatus {
    /// Written in the aggregate transaction, not yet published.
    Pending,
    /// Published to the event stream.
    Published,
    /// Delivery permanently failed; parked for inspection.
    Failed,
}

/// Row model for the transactional outbox.
///
/// Persistence happens inside the same database transaction as the aggregate
/// change, so an event is never lost when its aggregate commits. A dispatcher
/// later drains `Pending` records and publishes them.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OutboxRecord {
    /// Monotonic database id.
    pub id: i64,
    /// The event to deliver.
    pub envelope: EventEnvelope,
    pub status: OutboxStatus,
    pub created_at_ms: u64,
    pub published_at_ms: Option<u64>,
    /// Publish attempts so far (for retry/backoff decisions).
    pub attempts: u32,
}

impl OutboxRecord {
    pub fn new(id: i64, envelope: EventEnvelope, now_ms: u64) -> Self {
        Self {
            id,
            envelope,
            status: OutboxStatus::Pending,
            created_at_ms: now_ms,
            published_at_ms: None,
            attempts: 0,
        }
    }
}
