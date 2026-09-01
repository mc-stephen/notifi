//! Projects routes — mounted by the parent `v1_router` under `/projects`.

use axum::Router;
use axum::routing::{get, patch};
use super::handlers;

pub fn router<S>() -> Router<S>
where
    S: Clone + Send + Sync + 'static,
{
    Router::new()
        .route("/", get(handlers::list_projects))
        .route("/{id}/environment", patch(handlers::update_environment))
}
