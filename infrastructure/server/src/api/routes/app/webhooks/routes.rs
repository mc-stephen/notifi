use super::handlers;
use axum::Router;
use axum::routing::{delete, get, patch, post};

pub fn router<S>() -> Router<S>
where
    S: Clone + Send + Sync + 'static,
{
    Router::new()
        .route("/", get(handlers::list_webhooks))
        .route("/", post(handlers::create_webhook))
        .route("/{id}", get(handlers::get_webhook))
        .route("/{id}", patch(handlers::update_webhook))
        .route("/{id}", delete(handlers::delete_webhook))
}
