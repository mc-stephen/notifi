use axum::Router;
use axum::routing::{delete, get, post};
use super::handlers;

pub fn router<S>() -> Router<S>
where
    S: Clone + Send + Sync + 'static,
{
    Router::new()
        .route("/", post(handlers::broadcast).get(handlers::list_broadcasts))
        .route("/{broadcast_id}", get(handlers::get_broadcast))
        .route(
            "/scheduled/{broadcast_id}",
            delete(handlers::cancel_scheduled),
        )
}
