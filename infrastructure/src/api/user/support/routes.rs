use axum::Router;
use axum::routing::{get, post};
use super::handlers;

pub fn router<S>() -> Router<S>
where
    S: Clone + Send + Sync + 'static,
{
    Router::new()
        .route("/tickets", post(handlers::create_ticket).get(handlers::list_tickets))
        .route("/tickets/{ticket_id}", get(handlers::get_ticket))
        .route(
            "/tickets/{ticket_id}/messages",
            get(handlers::list_messages).post(handlers::send_reply),
        )
}
