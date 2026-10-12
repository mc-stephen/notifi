//! Auth routes — mounted by the api binary's central registry under
//! `/app/auth`.
//!
//! Generic over the parent state: handlers pull their dependencies from
//! request extensions (`Arc<AuthService>`), so no `AppState` coupling.

use super::handlers;
use axum::Router;
use axum::routing::{get, post};

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
        .route("/totp/setup", post(handlers::totp_setup))
        .route("/totp/verify", post(handlers::totp_verify))
        .route("/totp/disable", post(handlers::totp_disable))
        .route("/totp/challenge", post(handlers::totp_challenge))
        .route("/oauth/{provider}", get(handlers::oauth_start))
        .route("/oauth/{provider}/callback", get(handlers::oauth_callback))
}
