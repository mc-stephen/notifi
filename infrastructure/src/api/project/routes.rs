use super::handlers;
use axum::Router;
use axum::routing::post;

use crate::api::state::AppState;

/// The `/v1` router: product sends + inbound webhook fires. Services
/// arrive as scoped Extensions; without them (no database) the routes
/// answer 503.
pub fn router(state: &AppState) -> Router<AppState> {
    let mut routes = Router::new()
        .route("/notifications", post(handlers::send_notification))
        .route("/hooks/{token}", post(handlers::fire_webhook));
    if let Some(auth_service) = state.auth.clone() {
        routes = routes.layer(axum::Extension(auth_service));
    }
    if let Some(service) = state.sends.clone() {
        routes = routes.layer(axum::Extension(service));
    }
    if let Some(service) = state.inbound.clone() {
        routes = routes.layer(axum::Extension(service));
    }
    routes
}
