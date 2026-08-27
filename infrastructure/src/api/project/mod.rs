//! Project API surface — the product API customers integrate with.
//!
//! Served at the root (`/notifications`, `/templates`, … once built) and
//! authenticated by API keys (M6). Deliberately separate from the user
//! surface ([`crate::api::user`]) so product logic and dashboard logic
//! never share routes or middleware.

// TODO(M2): mount the first product endpoints here (e.g. POST
// /notifications via the notifications domain). Until then the surface is
// intentionally empty — merging an empty router is harmless and keeps the
// composition shape visible in `build_router`.

use axum::Router;

use crate::api::state::AppState;

pub fn router() -> Router<AppState> {
    Router::new()
}
