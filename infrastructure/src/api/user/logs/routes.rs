//! Audit-log routes — mounted by the parent `v1_router` under `/logs`.

use axum::Router;
use axum::routing::get;
use super::handlers;

pub fn router<S>() -> Router<S>
where
    S: Clone + Send + Sync + 'static,
{
    Router::new().route("/", get(handlers::list_logs))
}
