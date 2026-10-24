use super::handlers;
use axum::Router;
use axum::extract::DefaultBodyLimit;
use axum::routing::{get, post};

pub fn router<S>() -> Router<S>
where
    S: Clone + Send + Sync + 'static,
{
    Router::new()
        .route("/", get(handlers::list_projects))
        .route("/{project_id}", get(handlers::get_project))
        .route(
            "/import",
            post(handlers::import_project)
                .layer(DefaultBodyLimit::max(handlers::IMPORT_BODY_LIMIT)),
        )
}
