//! User API surface — the dashboard backend, versioned under `/v1`.
//!
//! Versioning contract: breaking changes ship as `/v2` alongside `/v1`;
//! v1 never changes incompatibly. New dashboard features mount inside
//! [`v1_router`] (auth today; onboarding/organizations next).

pub mod auth;

use axum::Router;

use crate::api::state::AppState;

/// The `/v1` router: nests feature routers under their path segments.
pub fn v1_router(state: &AppState) -> Router<AppState> {
    // Auth routes pull their service from a scoped Extension; without it
    // (no database) they answer 503 problem documents.
    let auth_routes = match state.auth.clone() {
        Some(service) => auth::routes::router().layer(axum::Extension(service)),
        None => auth::routes::router(),
    };
    // The OAuth runtime is optional independently of the service; without
    // it (no provider credentials) the /oauth routes answer 503. Layered
    // conditionally so the extension carries the bare `Arc<OAuthRuntime>`
    // the handlers extract — NOT `Extension<Option<Arc<...>>>`, which would
    // silently never match.
    let auth_routes = match state.oauth.clone() {
        Some(runtime) => auth_routes.layer(axum::Extension(runtime)),
        None => auth_routes,
    };

    Router::new().nest("/auth", auth_routes)
}
