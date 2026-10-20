use super::handlers;
use axum::Router;
use axum::routing::{delete, get, post, put};

pub fn router<S>() -> Router<S>
where
    S: Clone + Send + Sync + 'static,
{
    Router::new()
        .route("/", get(handlers::list_flows))
        .route("/", post(handlers::create_flow))
        .route("/{id}", get(handlers::get_flow))
        .route("/{id}", put(handlers::update_flow))
        .route("/{id}", delete(handlers::delete_flow))
}
