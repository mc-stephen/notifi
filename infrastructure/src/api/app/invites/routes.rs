use super::handlers;
use axum::Router;
use axum::routing::{get, post};

pub fn router<S>() -> Router<S>
where
    S: Clone + Send + Sync + 'static,
{
    Router::new()
        .route("/{token}", get(handlers::get_invite))
        .route("/{token}/accept", post(handlers::accept_invite))
        .route("/{token}/decline", post(handlers::decline_invite))
}
