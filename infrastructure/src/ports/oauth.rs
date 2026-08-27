//! OAuth sign-in port: consent-URL building and code exchange.
//!
//! The concrete implementation lives in `infrastructure/` (it speaks HTTPS
//! to the providers); the service stays framework-free behind this trait.

use std::sync::Arc;

use crate::ports::auth_store::BoxFut;

/// What a provider tells us about the signing-in person.
#[derive(Debug, Clone)]
pub struct OAuthProfile {
    /// Provider-stable identifier (`oauth_subject` on [`crate::domain::auth::entities::User`]).
    pub subject: String,
    pub email: String,
    /// The provider attests control of `email` — required for sign-in.
    pub email_verified: bool,
    pub name: Option<String>,
    pub avatar_url: Option<String>,
}

/// Result of building a provider consent redirect.
#[derive(Debug, Clone)]
pub struct AuthorizeStart {
    /// Fully-qualified consent URL to redirect the browser to.
    pub url: String,
    /// PKCE verifier; the handler parks it in a short-lived cookie until
    /// the callback exchanges the code (unused by GitHub, required by Google).
    pub pkce_verifier: String,
}

#[derive(Debug, thiserror::Error)]
pub enum OAuthError {
    #[error("provider '{0}' is not configured")]
    NotConfigured(String),
    #[error("oauth flow failed: {0}")]
    Flow(String),
}

/// Everything an OAuth handler needs at runtime — wired by the composition
/// root once at least one provider has credentials.
pub struct OAuthRuntime {
    pub provider: Arc<dyn OAuthIdentityProvider>,
    /// Dashboard origin for postMessage targets / redirect-mode returns.
    pub dashboard_url: String,
}

pub trait OAuthIdentityProvider: Send + Sync {
    /// Builds the consent redirect for `provider` (`github` | `google`).
    fn authorize_url(
        &self,
        provider: &str,
        csrf_state: &str,
    ) -> Result<AuthorizeStart, OAuthError>;

    /// Exchanges an authorization code for the signer-in's profile.
    fn exchange_code(
        &self,
        provider: &str,
        code: &str,
        pkce_verifier: &str,
    ) -> BoxFut<'_, Result<OAuthProfile, OAuthError>>;
}
