//! Device-facing gateway contracts — the queue behind the routes client
//! apps poll.
//!
//! Recipe: a user powers a route from their dashboard (a topic name they
//! pick + a generated secret key). Sends queue a message onto the route;
//! the app's poll fetches whatever waits and reads it away (poll-and-delete:
//! a message leaves its route at first read — crash-window loss is accepted
//! at dev stage). The long-poll hold lives in the store, so the HTTP layer
//! only translates wait seconds.
//!
//! Production driver: [`crate::infra::gateway_store::PgGatewayStore`].
//! Route surface: `src/api/routes/gateway.rs`. Proven by
//! `tests/gateway_api.rs`.

use serde::Deserialize;
use serde::Serialize;
use std::future::Future;
use std::pin::Pin;

/// One message waiting at a gateway route, as the device receives it.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GatewayMessage {
    /// Queue identity (monotonic per route) — handy in device logs.
    pub id: String,
    /// Headline (the notification title).
    pub title: String,
    /// Message body (the notification text).
    pub body: String,
    /// When the send queued it (RFC 3339).
    pub received_at: String,
}

/// Gateway queue failures. Dev keeps this small: persistence either worked
/// or it did not.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum GatewayError {
    #[error("gateway storage failure: {0}")]
    Storage(String),
}

/// Boxed gateway future (keeps [`GatewayStore`] object-safe).
pub type BoxGatewayFuture<'a, T> =
    Pin<Box<dyn Future<Output = Result<T, GatewayError>> + Send + 'a>>;

/// The gateway queue: route-keyed message storage with poll-and-delete
/// reads. The route is the (topic, key) pair from the saved recipe — both
/// sides come from the same recipe, so the queue needs no registry of its
/// own.
pub trait GatewayStore: Send + Sync {
    /// Queues one message for the secret route.
    fn push(&self, topic: &str, key: &str, title: &str, body: &str) -> BoxGatewayFuture<'_, ()>;

    /// Fetches-and-deletes the route's oldest queued messages (up to 50).
    /// `wait_secs` is the long-poll hold: the call may block up to that long
    /// waiting for the first message, then returns whatever waits (possibly
    /// nothing). `wait_secs: 0` is the plain check.
    fn poll(
        &self,
        topic: &str,
        key: &str,
        wait_secs: u64,
    ) -> BoxGatewayFuture<'_, Vec<GatewayMessage>>;
}
