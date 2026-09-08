use axum::Router;
use axum::routing::{get, patch, post};
use super::handlers;

pub fn router<S>() -> Router<S>
where
    S: Clone + Send + Sync + 'static,
{
    Router::new()
        .route("/", get(handlers::list_users))
        .route("/{user_id}", get(handlers::get_user))
        .route("/{user_id}/status", patch(handlers::set_user_status))
        .route(
            "/{user_id}/sessions/revoke",
            post(handlers::revoke_user_sessions),
        )
}
