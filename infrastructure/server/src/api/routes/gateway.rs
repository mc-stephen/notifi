//! Device-facing gateway surface — the paths client apps hold open.
//!
//! `GET /gateway/{topic}` serves one secret route (topic + generated
//! key from the saved recipe): queued messages come back oldest-first and
//! leave the queue at fetch time (poll-and-delete — no second read).
//! Long-poll via `?wait=<seconds>` (cap 20; omitted = plain instant check).
//! The key arrives as `?key=`; a missing key is refused with 401; a wrong
//! key pairs with the topic to the same harmless empty queue, so no
//! route-existence signal leaks.
//!
//! No session middleware here: the route's secret is the auth. Catalog row:
//! [`crate::api::catalog::GATEWAY_ROUTES`]. Proven by
//! `tests/gateway_api.rs`.

use std::sync::Arc;

use axum::Json;
use axum::Router;
use axum::extract::{Extension, Path, Query};
use serde::Deserialize;
use serde::Serialize;

use crate::api::routes::app::auth::Problem;
use crate::api::state::AppState;
use crate::domain::error::AppError;
use crate::ports::gateway::{GatewayMessage, GatewayStore};

/// Long-poll hold cap, in seconds.
const MAX_WAIT_SECS: u64 = 20;

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
    /// Long-poll hold in seconds (capped at [`MAX_WAIT_SECS`]; omitted = 0).
    wait: Option<u64>,
}

#[derive(Serialize)]
pub struct PollResponse {
    /// Queued messages, oldest first; empty when nothing waits.
    messages: Vec<GatewayMessage>,
}

/// `GET /gateway/{topic}` — poll-and-delete read for one secret route.
pub async fn poll_topic(
    Path(topic): Path<String>,
    Query(params): Query<PollParams>,
    store: MaybeGatewayStore,
) -> Result<Json<PollResponse>, Problem> {
    let store = require_store(store)?;

    let key = params.key.as_deref().unwrap_or("").trim().to_string();
    if key.is_empty() {
        return Err(AppError::Unauthorized.into());
    }

    let topic = topic.trim().to_string();
    if topic.is_empty() || topic.len() > 200 {
        return Err(AppError::Validation("route topic must be 1-200 characters".into()).into());
    }

    let wait = params.wait.unwrap_or(0).min(MAX_WAIT_SECS);
    let messages = store
        .poll(&topic, &key, wait)
        .await
        .map_err(|e| AppError::Storage(e.to_string()))?;
    Ok(Json(PollResponse { messages }))
}
