use axum::Router;
use axum::routing::{get, patch};
use super::handlers;

pub fn router<S>() -> Router<S>
where
    S: Clone + Send + Sync + 'static,
{
    Router::new()
        .route(
            "/",
            get(handlers::list_notifications),
        )
        .route(
            "/count",
            get(handlers::count_unread),
        )
        .route(
            "/read-all",
            patch(handlers::mark_all_read),
        )
        .route(
            "/{notification_id}",
            get(handlers::get_notification).delete(handlers::delete_notification),
        )
        .route(
            "/{notification_id}/read",
            patch(handlers::set_read),
        )
}
