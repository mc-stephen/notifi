use notifi_core::event::EventEnvelope;
use thiserror::Error;

/// Errors produced when publishing events.
#[derive(Debug, Error)]
pub enum EventBusError {
    #[error("failed to publish event: {0}")]
    Publish(String),
    #[error("event bus unavailable: {0}")]
    Unavailable(String),
}

/// Publishes domain event envelopes to the event stream.
///
/// Implemented by the queue-backed bus (outbox dispatcher → pgmq topic).
pub trait EventBus: Send + Sync {
    fn publish(
        &self,
        envelope: EventEnvelope,
    ) -> impl std::future::Future<Output = Result<(), EventBusError>> + Send;
}
