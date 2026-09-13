use axum::Router;
use axum::routing::{delete, get, post};

use super::handlers;

pub fn router<S>() -> Router<S>
where
    S: Clone + Send + Sync + 'static,
{
    Router::new()
        .route("/", post(handlers::create_key).get(handlers::list_keys))
        .route(
            "/{key_id}",
            get(handlers::get_key).patch(handlers::update_key),
        )
        .route("/{key_id}/revoke", post(handlers::revoke_key))
        .route(
            "/{key_id}/ips",
            get(handlers::list_ips).post(handlers::add_ip),
        )
        .route("/{key_id}/ips/{entry_id}", delete(handlers::remove_ip))
}
