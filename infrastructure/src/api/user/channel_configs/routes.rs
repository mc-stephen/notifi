use axum::Router;
use super::handlers;

pub fn routes<S>() -> Router<S>
where
    S: Clone + Send + Sync + 'static,
{
    Router::new()
        .route("/", axum::routing::get(handlers::list_configs).post(handlers::create_config))
        .route("/{config_id}", axum::routing::patch(handlers::update_config).delete(handlers::delete_config))
}
