use axum::Router;
use super::handlers;

pub fn routes<S>() -> Router<S>
where
    S: Clone + Send + Sync + 'static,
{
    Router::new().route("/", axum::routing::get(handlers::get_providers))
}
