//! Global HTTP middleware chain — infrastructure concerns shared by every
//! request: request-id correlation, logging/tracing, CORS.
//!
//! Two-tier model:
//! * **Global chain** (this module) — owned by the `api` binary, applies to
//!   every request, contains no business logic.
//! * **Feature security** — e.g. the session-auth `auth_required` middleware
//!   belongs to the `auth` feature's `presentation/` layer, not here. Protected
//!   route subtrees layer it on themselves (`ARCHITECTURE.md` §3).
//!
//! Ordering: Tower applies layers so the one added LAST is the OUTERMOST
//! (it sees the request first, its response last).

pub mod cors;
pub mod request_id;

use axum::Router;
use tower_http::trace::TraceLayer;

/// Applies the global middleware stack to a router (outermost last).
pub fn apply<S>(router: Router<S>) -> Router<S>
where
    S: Clone + Send + Sync + 'static,
{
    router
        .layer(request_id::propagate())
        .layer(request_id::assign())
        .layer(cors::permissive())
        .layer(TraceLayer::new_for_http())
}
