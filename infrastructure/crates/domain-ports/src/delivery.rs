use serde::{Deserialize, Serialize};
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

/// Boxed send future (keeps [`DeliveryProvider`] object-safe).
pub type BoxSendFuture<'a> =
    Pin<Box<dyn Future<Output = Result<DeliveryReceipt, DeliveryError>> + Send + 'a>>;

/// A normalized message handed to a channel adapter for delivery.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DeliveryMessage {
    /// Channel identifier (`email`, `sms`, `fcm`, `apns`, `web-push`, ...).
    pub channel: String,
    /// Recipient address/target (email address, phone number, push token, ...).
    pub recipient: String,
    /// Message title (unused by channels that have no title concept).
    pub title: String,
    /// Message body.
    pub body: String,
    /// Channel-specific extra payload (e.g. template params, deep links).
    pub metadata: Option<serde_json::Value>,
}

/// Successful delivery result reported by an adapter.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DeliveryReceipt {
    /// Provider that accepted the message.
    pub provider: String,
    /// Provider-side message/notification id, if known.
    pub provider_message_id: Option<String>,
}

/// Delivery failures normalized for retry-policy decisions.
///
/// Adapters wrap provider-specific errors into this type; the core only needs
/// the variant to decide whether to retry, back off, or dead-letter.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum DeliveryError {
    /// Provider authentication/authorization failed. Not retryable until config changes.
    #[error("provider authentication failed: {0}")]
    Auth(String),
    /// Provider rate limited the sender. Retryable with backoff.
    #[error("provider rate limited: {0}")]
    RateLimited(String),
    /// The recipient address/token is invalid. Not retryable.
    #[error("invalid recipient: {0}")]
    InvalidRecipient(String),
    /// The provider request timed out. Retryable.
    #[error("provider timed out: {0}")]
    Timeout(String),
    /// The provider permanently rejected the message. Not retryable.
    #[error("provider rejected the message: {0}")]
    Rejected(String),
    /// Any other provider-side error. Retryable with caution.
    #[error("provider error: {0}")]
    Provider(String),
}

/// Port every channel adapter implements.
///
/// Adapters must be cheap to construct and `Send + Sync`; a registry in the
/// `delivery` domain constructs them from tenant config and calls [`send`](DeliveryProvider::send).
pub trait DeliveryProvider: Send + Sync {
    /// Channel this provider serves (e.g. `"email"`, `"sms"`).
    fn channel(&self) -> &'static str;

    /// Provider name (e.g. `"smtp"`, `"twilio"`, `"fcm"`).
    fn provider_name(&self) -> &'static str;

    /// Attempts to deliver the message.
    fn send(&self, message: &DeliveryMessage) -> BoxSendFuture<'_>;
}

/// Builds channel adapters from per-project provider config JSON.
/// Returns `None` when the channel/provider has no adapter yet — the send
/// worker treats that as a provider-missing failure and falls through to
/// the next priority (it must never panic or block).
pub trait AdapterFactory: Send + Sync {
    fn adapter(
        &self,
        channel: &str,
        provider_id: &str,
        config: &serde_json::Value,
    ) -> Option<Arc<dyn DeliveryProvider>>;
}
