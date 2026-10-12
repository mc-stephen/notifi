use super::handlers;
use axum::Router;
use axum::routing::get;

pub fn router<S>() -> Router<S>
where
    S: Clone + Send + Sync + 'static,
{
    Router::new().route("/", get(handlers::list_logs))
}
