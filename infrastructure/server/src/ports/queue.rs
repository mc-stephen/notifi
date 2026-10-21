use thiserror::Error;

/// Errors produced when publishing to a queue.
#[derive(Debug, Error)]
pub enum QueueError {
    #[error("failed to enqueue job on topic '{topic}': {message}")]
    Publish { topic: String, message: String },
    #[error("queue unavailable: {0}")]
    Unavailable(String),
}

/// Publishes job payloads to a queue topic.
///
/// Implemented by the pgmq-backed producer (`infra/queue`).
pub trait QueuePublisher: Send + Sync {
    fn publish(
        &self,
        topic: &str,
        payload: &[u8],
    ) -> impl std::future::Future<Output = Result<(), QueueError>> + Send;
}
