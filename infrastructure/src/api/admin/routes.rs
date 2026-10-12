//! Admin routes — mounted by the API under `/admin`.

use super::billing;
use super::handlers;
use super::logs;
use super::notifications;
use super::projects;
use super::support;
use super::users;
use axum::Router;
use axum::routing::{get, patch, post};

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
        .route("/password/change", post(handlers::change_password))
        .route(
            "/admins",
            get(handlers::list_admins).post(handlers::create_admin),
        )
        .route("/admins/{admin_id}/remove", post(handlers::remove_admin))
        .route(
            "/admins/{admin_id}/status",
            patch(handlers::set_admin_status),
        )
        .route("/totp/setup", post(handlers::totp_setup))
        .route("/totp/verify", post(handlers::totp_verify))
        .nest("/support", support::routes::router())
        .nest("/users", users::routes::router())
        .nest("/projects", projects::routes::router())
        .nest("/notifications", notifications::routes::router())
        .nest("/billing", billing::routes::router())
        .nest("/logs", logs::routes::router())
}
