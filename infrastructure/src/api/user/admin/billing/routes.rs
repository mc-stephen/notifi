use super::handlers;
use axum::Router;
use axum::routing::{get, patch};

pub fn router<S>() -> Router<S>
where
    S: Clone + Send + Sync + 'static,
{
    Router::new()
        .route(
            "/plans",
            get(handlers::list_plans).post(handlers::create_plan),
        )
        .route(
            "/plans/{plan_id}",
            patch(handlers::update_plan).delete(handlers::delete_plan),
        )
        .route(
            "/plans/{plan_id}/subscriptions",
            get(handlers::list_subscribers),
        )
        .route("/subscribers/history", get(handlers::subscription_history))
}
