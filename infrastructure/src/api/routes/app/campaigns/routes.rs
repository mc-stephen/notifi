use super::handlers;
use axum::Router;
use axum::routing::{delete, get, patch, post};

pub fn router<S>() -> Router<S>
where
    S: Clone + Send + Sync + 'static,
{
    Router::new()
        .route("/", get(handlers::list_campaigns))
        .route("/", post(handlers::create_campaign))
        .route("/{id}", get(handlers::get_campaign))
        .route("/{id}", patch(handlers::update_campaign))
        .route("/{id}", delete(handlers::delete_campaign))
        .route("/{id}/run-now", post(handlers::run_campaign_now))
        .route("/{id}/pause", post(handlers::pause_campaign))
        .route("/{id}/resume", post(handlers::resume_campaign))
        .route("/{id}/cancel", post(handlers::cancel_campaign))
}
