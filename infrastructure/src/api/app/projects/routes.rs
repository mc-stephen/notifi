//! Projects routes — mounted by the parent `v1_router` under `/projects`.

use super::handlers;
use axum::Router;
use axum::routing::{get, patch, post};

pub fn router<S>() -> Router<S>
where
    S: Clone + Send + Sync + 'static,
{
    Router::new()
        .route(
            "/",
            get(handlers::list_projects).post(handlers::create_project),
        )
        .route("/{id}/environment", patch(handlers::update_environment))
        .route("/{id}/require-2fa", patch(handlers::set_require_2fa))
        .route(
            "/{id}/members",
            post(handlers::add_member).get(handlers::list_members),
        )
}
