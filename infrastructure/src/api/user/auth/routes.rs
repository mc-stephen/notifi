//! Auth routes — mounted by the api binary's central registry under
//! `/v1/auth`.
//!
//! Generic over the parent state: handlers pull their dependencies from
//! request extensions (`Arc<AuthService>`), so no `AppState` coupling.

use axum::Router;
use axum::routing::{get, post};
use super::handlers;

pub fn router<S>() -> Router<S>
where
    S: Clone + Send + Sync + 'static,
{
    Router::new()
        .route("/signup", post(handlers::signup))
        .route("/login", post(handlers::login))
        .route("/logout", post(handlers::logout))
        .route("/me", get(handlers::me))
        .route("/onboarding/complete", post(handlers::complete_onboarding))
        .route("/password/forgot", post(handlers::forgot_password))
        .route("/password/reset", post(handlers::reset_password))
        .route("/verify-email", post(handlers::verify_email))
        .route("/verify-email/resend", post(handlers::resend_verification))
        .route("/oauth/{provider}", get(handlers::oauth_start))
        .route("/oauth/{provider}/callback", get(handlers::oauth_callback))
}
