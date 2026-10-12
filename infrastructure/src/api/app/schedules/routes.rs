use super::handlers;
use axum::Router;
use axum::routing::{delete, get, patch, post};

pub fn router<S>() -> Router<S>
where
    S: Clone + Send + Sync + 'static,
{
    Router::new()
        .route("/", get(handlers::list_schedules))
        .route("/", post(handlers::create_schedule))
        .route("/{id}", get(handlers::get_schedule))
        .route("/{id}", patch(handlers::update_schedule))
        .route("/{id}", delete(handlers::delete_schedule))
        .route("/{id}/run-now", post(handlers::run_schedule_now))
}
