//! The Notifi gateway card (channel `other`) — Notifi's own hosted
//! pull-route, so this folder owns both of its halves: the SEND side
//! queues a notification into Notifi's gateway queue, and the READ
//! core behind `GET /gateway/{topic}?key=...` lives here too — the
//! route door itself is a thin axum registration over at
//! `src/api/routes/gateway.rs`.
//!
//! No external provider at all: the store IS the vendor's listening
//! path (`ports::gateway`; production driver `infra::gateway_store`).
//! The recipe carries the route's two halves (topic + secret key), so
//! the recipient contact is unused. Proven by `tests/gateway_api.rs`.

use std::sync::Arc;

use crate::ports::gateway::{GatewayMessage, GatewayStore};
use crate::ports::{
    BoxSendFuture, DeliveryError, DeliveryMessage, DeliveryProvider, DeliveryReceipt,
};

/// Long-poll hold cap, in seconds.
const MAX_WAIT_SECS: u64 = 20;

/// The Notifi gateway (channel `other`, provider `gateway`): queues the
/// notification into Notifi's own gateway queue — the user's app polls
/// its secret route (`GET /gateway/{topic}?key=...`) and reads it away.
/// No external provider at all: the store IS the vendor's listening
/// path (see `ports::gateway`). The recipe carries the route's two
/// halves, so the recipient contact is unused here.
pub struct GatewayAdapter {
    store: Arc<dyn GatewayStore + Send + Sync>,
    topic: String,
    secret_key: String,
}

impl GatewayAdapter {
    /// Arms the card: the queue store comes from the factory (the
    /// server's wired database), the route pair from the saved recipe.
    pub fn new(
        store: Arc<dyn GatewayStore + Send + Sync>,
        topic: String,
        secret_key: String,
    ) -> Self {
        Self {
            store,
            topic,
            secret_key,
        }
    }
}

impl DeliveryProvider for GatewayAdapter {
    fn channel(&self) -> &'static str {
        "other"
    }

    fn provider_name(&self) -> &'static str {
        "gateway"
    }

    fn send(&self, message: &DeliveryMessage) -> BoxSendFuture<'_> {
        let store = self.store.clone();
        let topic = self.topic.clone();
        let secret_key = self.secret_key.clone();
        let title = message.title.clone();
        let body = message.body.clone();
        Box::pin(async move {
            store
                .push(&topic, &secret_key, &title, &body)
                .await
                .map(|_| DeliveryReceipt {
                    provider: "gateway".to_string(),
                    provider_message_id: None,
                })
                .map_err(|e| DeliveryError::Provider(e.to_string()))
        })
    }
}

/// The read's verdicts, translated to HTTP problems by the route door
/// (`src/api/routes/gateway.rs`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PollRouteError {
    /// The route key is required — the (topic, key) pair is the auth.
    MissingKey,
    /// Topic outside 1-200 characters after trimming.
    BadTopic(&'static str),
    /// The queue store failed underneath the read.
    Storage(String),
}

/// The card's read core: validate the route pair, cap the long-poll
/// hold, then poll-and-delete from the queue. The route door passes its
/// parsed request straight here.
pub async fn poll_route(
    store: Arc<dyn GatewayStore + Send + Sync>,
    topic: &str,
    key: &str,
    wait_secs: u64,
) -> Result<Vec<GatewayMessage>, PollRouteError> {
    let key = key.trim().to_string();
    if key.is_empty() {
        return Err(PollRouteError::MissingKey);
    }

    let topic = topic.trim().to_string();
    if topic.is_empty() || topic.len() > 200 {
        return Err(PollRouteError::BadTopic(
            "route topic must be 1-200 characters",
        ));
    }

    let wait = wait_secs.min(MAX_WAIT_SECS);
    store
        .poll(&topic, &key, wait)
        .await
        .map_err(|e| PollRouteError::Storage(e.to_string()))
}
