//! Admin routes — mounted by the user API under `/v1/admin`.

use axum::Router;
use axum::routing::{get, post};
use super::handlers;
use super::support;

pub fn router<S>() -> Router<S>
where
    S: Clone + Send + Sync + 'static,
{
    Router::new()
        .route("/status", get(handlers::admin_status))
        .route("/bootstrap", post(handlers::bootstrap))
        .route("/login", post(handlers::login))
        .route("/logout", post(handlers::logout))
        .route("/totp/setup", post(handlers::totp_setup))
        .route("/totp/verify", post(handlers::totp_verify))
        .nest("/support", support::routes::router())
}
