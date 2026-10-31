//! Auth routes — mounted by the api binary's central registry under
//! `/app/auth`.
//!
//! Generic over the parent state: handlers pull their dependencies from
//! request extensions (`Arc<AuthService>`), so no `AppState` coupling.

use super::avatar;
use super::handlers;
use axum::Router;
use axum::routing::{get, post};

/// Builds the `/app/auth` router. `config_store` backs the avatar
/// routes (the `users/` folder's files) — absent without a store, in
/// which case they answer 503 like every other stored endpoint.
pub fn router<S>(
    config_store: Option<std::sync::Arc<dyn crate::ports::ConfigStore + Send + Sync>>,
) -> Router<S>
where
    S: Clone + Send + Sync + 'static,
{
    let router = Router::new()
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
        .route(
            "/profile/avatar",
            post(avatar::upload)
                .get(avatar::fetch)
                .delete(avatar::remove),
        );
    match config_store {
        Some(store) => router.layer(axum::Extension(store)),
        None => router,
    }
}
