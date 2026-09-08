//! Admin routes — mounted by the API under `/admin`.

use axum::Router;
use axum::routing::{get, post};
use super::handlers;
use super::support;
use super::users;

pub fn router<S>() -> Router<S>
where
    S: Clone + Send + Sync + 'static,
{
    Router::new()
        .route("/status", get(handlers::admin_status))
        .route("/bootstrap", post(handlers::bootstrap))
        .route("/login", post(handlers::login))
        .route("/logout", post(handlers::logout))
        .route("/me", get(handlers::me))
        .route("/password/forgot", post(handlers::forgot_password))
        .route("/password/reset", post(handlers::reset_password))
        .route("/totp/setup", post(handlers::totp_setup))
        .route("/totp/verify", post(handlers::totp_verify))
        .nest("/support", support::routes::router())
        .nest("/users", users::routes::router())
}
