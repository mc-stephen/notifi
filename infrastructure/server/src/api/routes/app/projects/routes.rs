//! Projects routes — mounted by the parent `v1_router` under `/projects`.

use super::handlers;
use axum::Router;
use axum::routing::{delete, get, patch, post};

pub fn router<S>() -> Router<S>
where
    S: Clone + Send + Sync + 'static,
{
    Router::new()
        .route(
            "/",
            get(handlers::list_projects).post(handlers::create_project),
        )
        .route("/{id}/active", patch(handlers::update_active))
        .route("/{id}/require-2fa", patch(handlers::set_require_2fa))
        .route(
            "/{id}/members",
            post(handlers::add_member).get(handlers::list_members),
        )
        // Deletion lifecycle: request (freeze + countdown), cancel inside
        // the window, and the always-available data export.
        .route("/{id}", delete(handlers::request_deletion))
        .route("/{id}/deletion/cancel", post(handlers::cancel_deletion))
        .route("/{id}/export", get(handlers::export_zip))
}
