//! Device-facing gateway surface — the route door only.
//!
//! `GET /gateway/{topic}` is the door client apps hold open: this file
//! registers the path, parses the request shape, and maps the read's
//! verdicts onto HTTP problems. The read's rules live in
//! `other_channel::gateway` — the key check, the 1-200 topic bound, the
//! 20-second long-poll cap, and the poll-and-delete queue call
//! (one secret route; queued messages come back oldest-first and leave
//! the queue at fetch time). The key arrives as `?key=`; a missing key
//! is refused with 401; a wrong key pairs with the topic to the same
//! harmless empty queue, so no route-existence signal leaks.
//!
//! No session middleware here: the route's secret is the auth. Catalog
//! row: [`crate::api::catalog::GATEWAY_ROUTES`]. Proven by
//! `tests/gateway_api.rs`.

use std::sync::Arc;

use axum::Json;
use axum::Router;
use axum::extract::{Extension, Path, Query};
use serde::Deserialize;
use serde::Serialize;

use crate::api::routes::app::auth::Problem;
use crate::api::state::AppState;
use crate::channels::other_channel::gateway::{PollRouteError, poll_route};
use crate::domain::error::AppError;
use crate::ports::gateway::{GatewayMessage, GatewayStore};

/// The `/gateway` router: device routes over the gateway queue. Without a
/// store (no database) the routes answer 503 problem documents.
pub fn router(state: &AppState) -> Router<AppState> {
    let mut routes = Router::new().route("/{topic}", axum::routing::get(poll_topic));
    if let Some(store) = state.gateway.clone() {
        routes = routes.layer(axum::Extension(store));
    }
    routes
}

type MaybeGatewayStore = Option<Extension<Arc<dyn GatewayStore + Send + Sync>>>;

fn require_store(store: MaybeGatewayStore) -> Result<Arc<dyn GatewayStore + Send + Sync>, Problem> {
    store
        .map(|Extension(store)| store)
        .ok_or_else(|| AppError::NotConfigured.into())
}

#[derive(Deserialize)]
pub struct PollParams {
    /// Route secret (the generated recipe key).
    key: Option<String>,
    /// Long-poll hold in seconds (capped inside the read core; omitted = 0).
    wait: Option<u64>,
}

#[derive(Serialize)]
pub struct PollResponse {
    /// Queued messages, oldest first; empty when nothing waits.
    messages: Vec<GatewayMessage>,
}

/// `GET /gateway/{topic}` — the door: hand the poll read to
/// `other_channel::gateway` and translate its verdicts to problems.
pub async fn poll_topic(
    Path(topic): Path<String>,
    Query(params): Query<PollParams>,
    store: MaybeGatewayStore,
) -> Result<Json<PollResponse>, Problem> {
    let store = require_store(store)?;
    let key = params.key.as_deref().unwrap_or("");
    let wait = params.wait.unwrap_or(0);
    let messages = poll_route(store, &topic, key, wait)
        .await
        .map_err(|e| match e {
            PollRouteError::MissingKey => Problem::from(AppError::Unauthorized),
            PollRouteError::BadTopic(msg) => Problem::from(AppError::Validation(msg.to_string())),
            PollRouteError::Storage(msg) => Problem::from(AppError::Storage(msg)),
        })?;
    Ok(Json(PollResponse { messages }))
}
