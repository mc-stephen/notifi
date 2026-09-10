use super::handlers;
use axum::Router;
use axum::routing::{get, post};

pub fn router<S>() -> Router<S>
where
    S: Clone + Send + Sync + 'static,
{
    Router::new()
        .route("/plans", get(handlers::list_plans))
        .route("/subscription", get(handlers::get_subscription))
        .route("/subscriptions", post(handlers::subscribe))
        .route("/subscriptions/cancel", post(handlers::cancel))
        .route("/subscriptions/renew", post(handlers::renew))
}
