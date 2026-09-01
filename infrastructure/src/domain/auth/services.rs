//! `AuthService` — the use cases of the auth slice.
//!
//! Orchestrates validation, password hashing, and one-time-token issuance on
//! top of the [`AuthStore`] port. Email delivery is NOT wired here: until M4
//! (providers + templates) raw tokens are surfaced via logs and, when
//! explicitly enabled, in responses.

use std::sync::Arc;

use argon2::password_hash::SaltString;
use argon2::{Argon2, PasswordHash, PasswordHasher, PasswordVerifier};
use chrono::{Duration, Utc};
use rand::RngCore;

use crate::domain::auth::entities::{AuthToken, Session, SessionId, TokenPurpose, User, UserId};
use crate::domain::auth::errors::AuthError;
use crate::ports::oauth::OAuthProfile;
use crate::ports::auth_store::{AuthStore, OnboardingInput};
use crate::domain::auth::value_objects::{Email, hash_token, new_token, validate_password};

/// How long a login cookie stays valid.
const SESSION_TTL_REMEMBER: Duration = Duration::days(30);
const SESSION_TTL_SHORT: Duration = Duration::days(1);

/// Outcome of a successful signup.
pub struct SignupOutcome {
    pub user: User,
    /// Session started at signup (`rememberMe = false` semantics) so the
    /// browser flows straight into onboarding and the dashboard.
    pub session: Session,
    /// Raw token for the `session_token` cookie (shown once, stored hashed).
    pub raw_token: String,
    /// Raw verification token; present only for local/dev flows.
    pub verification_token: String,
}

/// A freshly issued login session, including the one-time raw cookie value.
pub struct IssuedSession {
    pub user: User,
    pub session: Session,
    /// Raw token for the `session_token` cookie (shown once, stored hashed).
    pub raw_token: String,
}

pub struct AuthService {
    store: Arc<dyn AuthStore>,
    expose_dev_tokens: bool,
}

impl AuthService {
    pub fn new(store: Arc<dyn AuthStore>, expose_dev_tokens: bool) -> Self {
        Self {
            store,
            expose_dev_tokens,
        }
    }

    /// Whether raw one-time tokens may appear in API responses.
    pub fn exposes_dev_tokens(&self) -> bool {
        self.expose_dev_tokens
    }

    // ------------------------------------------------------------------
    // signup
    // ------------------------------------------------------------------

    pub async fn signup(
        &self,
        name: &str,
        email: &str,
        password: &str,
    ) -> Result<SignupOutcome, AuthError> {
        let email = Email::parse(email)?;
        validate_password(password)?;

        let name = name.trim();
        if name.is_empty() || name.len() > 100 {
            return Err(AuthError::Validation(
                "name must be between 1 and 100 characters".to_string(),
            ));
        }

        if self
            .store
            .find_user_by_email(email.as_str())
            .await?
            .is_some()
        {
            return Err(AuthError::EmailAlreadyExists);
        }

        let now = Utc::now();
        let user = User {
            id: UserId::new(),
            name: name.to_string(),
            email,
            password_hash: hash_password(password)?,
            avatar_url: None,
            email_verified_at: None,
            oauth_provider: None,
            oauth_subject: None,
            created_at: now,
            // signup counts as the first login
            last_login_at: Some(now),
        };
        self.store.create_user(&user).await?;

        let verification_token = self
            .issue_token(user.id, TokenPurpose::EmailVerification)
            .await?;

        let (session, raw_token) = self
            .issue_session(user.id, SESSION_TTL_SHORT)
            .await?;

        Ok(SignupOutcome {
            user,
            session,
            raw_token,
            verification_token,
        })
    }

    // ------------------------------------------------------------------
    // login / logout / session
    // ------------------------------------------------------------------

    pub async fn login(
        &self,
        email: &str,
        password: &str,
        remember_me: bool,
    ) -> Result<IssuedSession, AuthError> {
        // Parse failures map to invalid credentials: never reveal why.
        let email = Email::parse(email).map_err(|_| AuthError::InvalidCredentials)?;

        let user = self
            .store
            .find_user_by_email(email.as_str())
            .await?
            .ok_or(AuthError::InvalidCredentials)?;

        if !verify_password(password, &user.password_hash) {
            return Err(AuthError::InvalidCredentials);
        }

        let now = Utc::now();
        self.store.touch_last_login(user.id, now).await?;

        let ttl = if remember_me {
            SESSION_TTL_REMEMBER
        } else {
            SESSION_TTL_SHORT
        };
        let (session, raw_token) = self.issue_session(user.id, ttl).await?;

        Ok(IssuedSession {
            user,
            session,
            raw_token,
        })
    }

    /// Resolves the owner of an active session cookie value.
    pub async fn authenticate(&self, session_cookie: &str) -> Result<User, AuthError> {
        let now = Utc::now();
        let session = self
            .store
            .find_session_by_hash(&hash_token(session_cookie))
            .await?
            .filter(|s| s.is_active(now))
            .ok_or(AuthError::Unauthorized)?;

        self.store
            .find_user_by_id(session.user_id)
            .await?
            .ok_or(AuthError::Unauthorized)
    }

    /// Revokes the session behind `session_cookie` (idempotent).
    pub async fn logout(&self, session_cookie: &str) -> Result<(), AuthError> {
        if let Some(session) = self
            .store
            .find_session_by_hash(&hash_token(session_cookie))
            .await?
        {
            self.store.revoke_session(session.id).await?;
        }
        Ok(())
    }

    // ------------------------------------------------------------------
    // email verification
    // ------------------------------------------------------------------

    pub async fn verify_email(&self, raw_token: &str) -> Result<(), AuthError> {
        let token = self
            .store
            .find_token_by_hash(&hash_token(raw_token), TokenPurpose::EmailVerification)
            .await?
            .ok_or_else(|| {
                AuthError::TokenInvalid(
                    "This verification link is invalid or has already been used.".to_string(),
                )
            })?;

        let now = Utc::now();
        if !token.is_usable(now) {
            return if token.consumed_at.is_some() {
                Err(AuthError::TokenInvalid(
                    "This verification link is invalid or has already been used.".to_string(),
                ))
            } else {
                // message wording is load-bearing: the dashboard checks for "expired"
                Err(AuthError::TokenExpired(
                    "This verification link has expired.".to_string(),
                ))
            };
        }

        self.store.consume_token(token.id).await?;
        self.store.set_email_verified(token.user_id, now).await?;
        Ok(())
    }

    /// Re-issues the verification email token (invalidating previous ones).
    /// Returns `None` for unknown emails — response stays 200 either way.
    pub async fn resend_verification(&self, email: &str) -> Result<Option<String>, AuthError> {
        let email = match Email::parse(email) {
            Ok(email) => email,
            Err(_) => return Ok(None),
        };
        let Some(user) = self.store.find_user_by_email(email.as_str()).await? else {
            return Ok(None);
        };

        let token = self
            .issue_token(user.id, TokenPurpose::EmailVerification)
            .await?;
        Ok(Some(token))
    }

    // ------------------------------------------------------------------
    // password reset
    // ------------------------------------------------------------------

    /// Always succeeds from the caller's perspective (no enumeration):
    /// returns `Some(raw_token)` only when the account exists.
    pub async fn forgot_password(&self, email: &str) -> Result<Option<String>, AuthError> {
        let email = match Email::parse(email) {
            Ok(email) => email,
            // malformed input behaves like an unknown address
            Err(_) => return Ok(None),
        };
        let Some(user) = self.store.find_user_by_email(email.as_str()).await? else {
            return Ok(None);
        };

        let token = self
            .issue_token(user.id, TokenPurpose::PasswordReset)
            .await?;
        Ok(Some(token))
    }

    pub async fn reset_password(
        &self,
        raw_token: &str,
        new_password: &str,
    ) -> Result<(), AuthError> {
        validate_password(new_password)?;

        let token = self
            .store
            .find_token_by_hash(&hash_token(raw_token), TokenPurpose::PasswordReset)
            .await?
            .filter(|t| t.is_usable(Utc::now()))
            .ok_or_else(|| {
                AuthError::TokenInvalid("Invalid or expired reset token.".to_string())
            })?;

        self.store.consume_token(token.id).await?;
        self.store
            .update_password(token.user_id, hash_password(new_password)?)
            .await?;
        // a reset invalidates every existing session for that account
        self.store
            .revoke_all_sessions_for_user(token.user_id)
            .await?;
        Ok(())
    }

    // ------------------------------------------------------------------
    // oauth
    // ------------------------------------------------------------------

    /// Signs a person in through an OAuth identity, upserting as needed:
    ///
    /// 1. returning OAuth user (matched by `(provider, subject)`), or
    /// 2. existing account with the same email — auto-linked, because a
    ///    provider-verified address proves control of that inbox, or
    /// 3. brand-new verified account.
    pub async fn login_with_oauth(
        &self,
        provider: &str,
        profile: OAuthProfile,
    ) -> Result<IssuedSession, AuthError> {
        if !matches!(provider, "github" | "google") {
            return Err(AuthError::Validation(format!(
                "unsupported OAuth provider '{provider}'"
            )));
        }
        if !profile.email_verified {
            return Err(AuthError::Validation(
                "the provider did not verify this email address".to_string(),
            ));
        }
        let email = Email::parse(&profile.email)?;
        if profile.subject.trim().is_empty() {
            return Err(AuthError::Validation("missing provider subject".to_string()));
        }

        let user = match self.store.find_user_by_oauth(provider, &profile.subject).await? {
            Some(user) => user,
            None => match self.store.find_user_by_email(email.as_str()).await? {
                Some(existing) => {
                    self.store
                        .link_oauth_to_user(existing.id, provider, &profile.subject)
                        .await?;
                    existing
                }
                None => {
                    let now = Utc::now();
                    let new_user = User {
                        id: UserId::new(),
                        name: profile.name.clone().unwrap_or_else(|| {
                            email.as_str().split('@').next().unwrap_or("user").to_string()
                        }),
                        email,
                        // OAuth-only account: no password exists to verify against.
                        password_hash: String::new(),
                        avatar_url: profile.avatar_url.clone(),
                        email_verified_at: Some(now),
                        oauth_provider: Some(provider.to_string()),
                        oauth_subject: Some(profile.subject.clone()),
                        created_at: now,
                        last_login_at: Some(now),
                    };
                    self.store.create_user(&new_user).await?;
                    new_user
                }
            },
        };

        self.store.touch_last_login(user.id, Utc::now()).await?;
        let (session, raw_token) = self.issue_session(user.id, SESSION_TTL_SHORT).await?;

        Ok(IssuedSession {
            user,
            session,
            raw_token,
        })
    }

    // ------------------------------------------------------------------
    // onboarding
    // ------------------------------------------------------------------

    /// Whether the account can skip onboarding — true when the user already
    /// owns or belongs to at least one project (e.g. an invited member
    /// signing up for the first time).
    pub async fn onboarding_completed(&self, user_id: UserId) -> Result<bool, AuthError> {
        Ok(self.store.has_project(user_id).await?)
    }

    /// Persists the first project collected by the dashboard onboarding
    /// flow. New projects start in `development` mode (the project-level
    /// environment gate, defaulted by the schema).
    pub async fn complete_onboarding(
        &self,
        user_id: UserId,
        input: OnboardingInput,
    ) -> Result<(), AuthError> {
        let project_name = input.project_name.trim();
        if project_name.is_empty() || project_name.len() > 100 {
            return Err(AuthError::Validation(
                "project name must be between 1 and 100 characters".to_string(),
            ));
        }

        self.store.complete_onboarding(user_id, input).await?;
        Ok(())
    }

    // ------------------------------------------------------------------
    // internals
    // ------------------------------------------------------------------

    /// Issues a fresh login session; returns the row plus the one-time raw
    /// cookie value (only the hash is persisted).
    async fn issue_session(
        &self,
        user_id: UserId,
        ttl: Duration,
    ) -> Result<(Session, String), AuthError> {
        let now = Utc::now();
        let (raw_token, token_hash) = new_token();
        let session = Session {
            id: SessionId::new(),
            user_id,
            token_hash,
            expires_at: now + ttl,
            revoked_at: None,
            created_at: now,
        };
        self.store.create_session(&session).await?;
        Ok((session, raw_token))
    }

    /// Issues a fresh one-time token, invalidating any unconsumed ones of the
    /// same purpose. Returns the raw token (emailed exactly once).
    async fn issue_token(
        &self,
        user_id: UserId,
        purpose: TokenPurpose,
    ) -> Result<String, AuthError> {
        self.store.consume_tokens_for_user(user_id, purpose).await?;

        let (raw, hash) = new_token();
        let now = Utc::now();
        let token = AuthToken {
            id: crate::domain::auth::entities::AuthTokenId::new(),
            user_id,
            purpose,
            token_hash: hash,
            expires_at: now + purpose.ttl(),
            consumed_at: None,
            created_at: now,
        };
        self.store.create_token(&token).await?;
        Ok(raw)
    }
}

// ----------------------------------------------------------------------
// password hashing (argon2id)
// ----------------------------------------------------------------------

fn hash_password(password: &str) -> Result<String, AuthError> {
    let mut salt_bytes = [0u8; 16];
    rand::rng().fill_bytes(&mut salt_bytes);
    let salt = SaltString::encode_b64(&salt_bytes)
        .map_err(|e| AuthError::Storage(format!("salt generation failed: {e}")))?;
    Argon2::default()
        .hash_password(password.as_bytes(), &salt)
        .map(|hash| hash.to_string())
        .map_err(|e| AuthError::Storage(format!("password hashing failed: {e}")))
}

fn verify_password(password: &str, phc_hash: &str) -> bool {
    PasswordHash::new(phc_hash)
        .map(|parsed| {
            Argon2::default()
                .verify_password(password.as_bytes(), &parsed)
                .is_ok()
        })
        .unwrap_or(false)
}
