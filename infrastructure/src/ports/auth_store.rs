//! The persistence port for the auth slice.
//!
//! One aggregate port (`AuthStore`) keeps the service simple; the sqlx
//! implementation lives in `infrastructure/`, tests use in-memory fakes.
//! Object-safe by design (boxed futures) so `Arc<dyn AuthStore>` works.

use std::future::Future;
use std::pin::Pin;

use chrono::{DateTime, Utc};

use crate::domain::auth::entities::{
    AuthToken, AuthTokenId, Session, SessionId, TokenPurpose, User, UserId,
};

/// Boxed future returned by every port method (keeps the trait object-safe
/// without an async-trait dependency).
pub type BoxFut<'a, T> = Pin<Box<dyn Future<Output = T> + Send + 'a>>;

#[derive(Debug, thiserror::Error)]
pub enum StoreError {
    /// Unique constraint violated (e.g. duplicate email).
    #[error("already exists")]
    Conflict,
    #[error("storage failure: {0}")]
    Storage(String),
}

impl From<StoreError> for crate::domain::auth::errors::AuthError {
    fn from(err: StoreError) -> Self {
        match err {
            StoreError::Conflict => crate::domain::auth::errors::AuthError::EmailAlreadyExists,
            StoreError::Storage(m) => crate::domain::auth::errors::AuthError::Storage(m),
        }
    }
}

/// Data collected by the dashboard onboarding flow (first org + project).
#[derive(Debug, Clone)]
pub struct OnboardingInput {
    pub org_name: String,
    pub org_logo_url: Option<String>,
    pub org_region: Option<String>,
    pub org_timezone: Option<String>,
    pub project_name: String,
    pub project_description: Option<String>,
    /// One of `development | staging | production`.
    pub environment: String,
}

pub trait AuthStore: Send + Sync {
    // -- users ------------------------------------------------------------
    fn create_user(&self, user: &User) -> BoxFut<'_, Result<(), StoreError>>;
    fn find_user_by_email(&self, email: &str) -> BoxFut<'_, Result<Option<User>, StoreError>>;
    fn find_user_by_id(&self, id: UserId) -> BoxFut<'_, Result<Option<User>, StoreError>>;
    fn set_email_verified(
        &self,
        user_id: UserId,
        verified_at: DateTime<Utc>,
    ) -> BoxFut<'_, Result<(), StoreError>>;
    fn update_password(
        &self,
        user_id: UserId,
        password_hash: String,
    ) -> BoxFut<'_, Result<(), StoreError>>;
    fn touch_last_login(
        &self,
        user_id: UserId,
        at: DateTime<Utc>,
    ) -> BoxFut<'_, Result<(), StoreError>>;

    // -- sessions ---------------------------------------------------------
    fn create_session(&self, session: &Session) -> BoxFut<'_, Result<(), StoreError>>;
    fn find_session_by_hash(
        &self,
        token_hash: &str,
    ) -> BoxFut<'_, Result<Option<Session>, StoreError>>;
    fn revoke_session(&self, id: SessionId) -> BoxFut<'_, Result<(), StoreError>>;
    fn revoke_all_sessions_for_user(&self, user_id: UserId) -> BoxFut<'_, Result<(), StoreError>>;

    // -- one-time tokens ----------------------------------------------------
    fn create_token(&self, token: &AuthToken) -> BoxFut<'_, Result<(), StoreError>>;
    fn find_token_by_hash(
        &self,
        token_hash: &str,
        purpose: TokenPurpose,
    ) -> BoxFut<'_, Result<Option<AuthToken>, StoreError>>;
    fn consume_token(&self, id: AuthTokenId) -> BoxFut<'_, Result<(), StoreError>>;
    /// Marks all unconsumed tokens of `purpose` for `user_id` as consumed —
    /// used when re-issuing so only the newest link works.
    fn consume_tokens_for_user(
        &self,
        user_id: UserId,
        purpose: TokenPurpose,
    ) -> BoxFut<'_, Result<(), StoreError>>;

    // -- oauth ---------------------------------------------------------------
    /// The account previously created through / linked to `(provider, subject)`.
    fn find_user_by_oauth(
        &self,
        provider: &str,
        subject: &str,
    ) -> BoxFut<'_, Result<Option<User>, StoreError>>;
    /// Attaches an OAuth identity to an existing (password) account.
    fn link_oauth_to_user(
        &self,
        user_id: UserId,
        provider: &str,
        subject: &str,
    ) -> BoxFut<'_, Result<(), StoreError>>;

    // -- onboarding ---------------------------------------------------------
    /// Whether `user_id` belongs to at least one organization that owns at
    /// least one project — the "can skip onboarding" test.
    fn has_org_and_project(&self, user_id: UserId) -> BoxFut<'_, Result<bool, StoreError>>;
    /// Creates the first organization (+ owner membership), project, and
    /// default environment atomically.
    fn complete_onboarding(
        &self,
        user_id: UserId,
        input: OnboardingInput,
    ) -> BoxFut<'_, Result<(), StoreError>>;
}
