//! Recipient routes — mounted by the parent `v1_router` under
//! `/projects/{project_id}/recipients`.

use axum::Router;
use axum::routing::{get, post};
use super::handlers;

pub fn router<S>() -> Router<S>
where
    S: Clone + Send + Sync + 'static,
{
    Router::new()
        .route("/", post(handlers::create_recipient).get(handlers::list_recipients))
        .route(
            "/{recipient_id}",
            get(handlers::get_recipient)
                .patch(handlers::update_recipient)
                .delete(handlers::delete_recipient),
        )
}
