use crate::id::Ulid;
use serde::{Deserialize, Serialize};

/// Immutable event record that crosses module boundaries.
///
/// Produced by a domain aggregate via [`DomainEvent::into_envelope`], written
/// to the transactional outbox, and eventually published to the event stream.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EventEnvelope {
    /// Unique event id (ULID; timestamp encodes `occurred_at`).
    pub id: Ulid,
    /// Aggregate type that produced the event (e.g. `notification`).
    pub aggregate_type: String,
    /// Aggregate id the event refers to.
    pub aggregate_id: String,
    /// Event type in `<domain>.<past_tense>` form (e.g. `notification.failed`).
    pub event_type: String,
    /// Milliseconds since epoch the event occurred.
    pub occurred_at_ms: u64,
    /// Correlation id of the originating request, if any.
    pub correlation_id: Option<String>,
    /// Event-specific payload.
    pub payload: serde_json::Value,
}

/// A domain event: named, immutable, and owned by one aggregate.
pub trait DomainEvent: Send + Sync {
    /// Event type in `<domain>.<past_tense>` form.
    fn event_type(&self) -> &'static str;

    /// Aggregate type that produced this event.
    fn aggregate_type(&self) -> &'static str;

    /// Aggregate id this event refers to.
    fn aggregate_id(&self) -> &str;

    /// Serialized event payload.
    fn payload(&self) -> Result<serde_json::Value, serde_json::Error>;

    /// Wraps this event into an immutable [`EventEnvelope`].
    fn into_envelope(
        self,
        correlation_id: Option<String>,
    ) -> Result<EventEnvelope, serde_json::Error>
    where
        Self: Sized,
    {
        let id = Ulid::new();
        Ok(EventEnvelope {
            id,
            aggregate_type: self.aggregate_type().to_string(),
            aggregate_id: self.aggregate_id().to_string(),
            event_type: self.event_type().to_string(),
            occurred_at_ms: id.timestamp_ms(),
            correlation_id,
            payload: self.payload()?,
        })
    }
}
