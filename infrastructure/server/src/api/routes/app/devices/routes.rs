//! Device routes — mounted by the app router under
//! `/projects/{project_id}/devices`.

use super::handlers;
use axum::Router;
use axum::routing::{get, post};

pub fn router<S>() -> Router<S>
where
    S: Clone + Send + Sync + 'static,
{
    Router::new()
        .route(
            "/",
            post(handlers::register_device).get(handlers::list_devices),
        )
        .route(
            "/{device_id}",
            get(handlers::get_device).delete(handlers::revoke_device),
        )
}
