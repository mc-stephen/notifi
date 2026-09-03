//! Template routes — mounted by the parent `v1_router` under
//! `/projects/{project_id}/templates`.

use axum::Router;
use axum::routing::{get, post};

use super::handlers;

pub fn router<S>() -> Router<S>
where
    S: Clone + Send + Sync + 'static,
{
    Router::new()
        .route("/", post(handlers::create_template).get(handlers::list_templates))
        .route(
            "/{template_id}",
            get(handlers::get_template)
                .patch(handlers::update_template)
                .delete(handlers::delete_template),
        )
}
