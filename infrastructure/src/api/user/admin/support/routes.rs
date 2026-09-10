use super::handlers;
use axum::Router;
use axum::routing::get;

pub fn router<S>() -> Router<S>
where
    S: Clone + Send + Sync + 'static,
{
    Router::new()
        .route("/tickets", get(handlers::list_tickets))
        .route(
            "/tickets/{ticket_id}",
            get(handlers::get_ticket).patch(handlers::set_status),
        )
        .route(
            "/tickets/{ticket_id}/messages",
            get(handlers::list_messages).post(handlers::send_reply),
        )
}
