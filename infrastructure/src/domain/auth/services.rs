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
use totp_rs::{Rfc6238, Secret, TOTP};

use crate::domain::audit::AuditService;
use crate::domain::audit::entities::{AuditAction, AuditEvent};
use crate::domain::auth::entities::{AuthToken, Session, SessionId, TokenPurpose, User, UserId};
use crate::domain::auth::errors::AuthError;
use crate::domain::auth::value_objects::{Email, hash_token, new_token, validate_password};
use crate::domain::notifications::{NotificationService, NotificationType};
use crate::ports::auth_store::{AuthStore, OnboardingInput};
use crate::ports::oauth::OAuthProfile;

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
    audit: Arc<AuditService>,
    expose_dev_tokens: bool,
    notifications: Option<Arc<NotificationService>>,
    mailer: Option<Arc<dyn crate::ports::mailer::SmtpMailer>>,
    templates: Option<Arc<dyn crate::ports::mailer::SystemTemplates>>,
    dashboard_url: String,
}

impl AuthService {
    pub fn new(
        store: Arc<dyn AuthStore>,
        expose_dev_tokens: bool,
        audit: Arc<AuditService>,
    ) -> Self {
        Self {
            store,
            audit,
            expose_dev_tokens,
            notifications: None,
            mailer: None,
            templates: None,
            dashboard_url: String::new(),
        }
    }

    /// Wires the notification service after construction (avoids changing
    /// every call site that builds `AuthService`).
    pub fn with_notifications(mut self, svc: Arc<NotificationService>) -> Self {
        self.notifications = Some(svc);
        self
    }

    /// Wires system mail (transactional emails). All three pieces travel
    /// together: without any one of them, flows keep dev behavior.
    pub fn with_system_mail(
        mut self,
        mailer: Arc<dyn crate::ports::mailer::SmtpMailer>,
        templates: Arc<dyn crate::ports::mailer::SystemTemplates>,
        dashboard_url: String,
    ) -> Self {
        self.mailer = Some(mailer);
        self.templates = Some(templates);
        self.dashboard_url = dashboard_url;
        self
    }
    /// Sends a templated system email when mail is configured. Delivery
    /// failures are logged, never fatal to the flow that triggered them.
    /// All auth-service mail is automated, so it always sends as NoReply —
    /// human mail (e.g. from the admin dashboard) passes Support/Custom
    /// at its own call site instead.
    async fn send_system_mail(&self, to: &str, template: &str, vars: &[(&str, &str)]) {
        let (Some(mailer), Some(templates)) = (self.mailer.as_ref(), self.templates.as_ref())
        else {
            return;
        };
        let rendered = match templates.render(template, vars) {
            Ok(rendered) => rendered,
            Err(e) => {
                tracing::warn!(template, error = %e, "system template render failed");
                return;
            }
        };
        match mailer
            .send(
                crate::ports::mailer::SystemSender::NoReply,
                to,
                &rendered.subject,
                &rendered.text,
                Some(&rendered.html),
            )
            .await
        {
            Ok(message_id) => {
                tracing::info!(template, to, message_id = %message_id, "system email sent");
            }
            Err(e) => {
                tracing::warn!(template, to, error = %e, "system email delivery failed");
            }
        }
    }

    /// Emails a sign-in notice. Called from password + OAuth logins only —
    /// the TOTP second step is the same login, so it stays quiet.
    async fn send_login_notice(&self, user: &User) {
        let when = chrono::Utc::now()
            .format("%B %-d, %Y at %-I:%M %p UTC")
            .to_string();
        let link = format!("{}/profile", self.dashboard_url.trim_end_matches('/'));
        self.send_system_mail(
            user.email.as_str(),
            "login-notice",
            &[
                ("name", user.name.as_str()),
                ("when", &when),
                ("link", &link),
            ],
        )
        .await;
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
            totp_secret: None,
            totp_enabled: false,
            status: crate::domain::auth::entities::UserStatus::Active,
            created_at: now,
            // signup counts as the first login
            last_login_at: Some(now),
        };
        self.store.create_user(&user).await?;

        let verification_token = self
            .issue_token(user.id, TokenPurpose::EmailVerification)
            .await?;

        self.send_system_mail(
            user.email.as_str(),
            "verify-email",
            &[
                ("name", user.name.as_str()),
                (
                    "link",
                    &format!(
                        "{}/auth/verify-email?token={verification_token}",
                        self.dashboard_url.trim_end_matches('/')
                    ),
                ),
            ],
        )
        .await;

        let (session, raw_token) = self.issue_session(user.id, SESSION_TTL_SHORT).await?;

        self.audit
            .record(
                now,
                &AuditEvent::new(
                    AuditAction::UserSignup,
                    Some(&user.id.to_string()),
                    Some(&user.name),
                    None,
                    "account created".to_string(),
                    Some(serde_json::json!({ "email": user.email.as_str() })),
                ),
            )
            .await;

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
        totp_code: Option<&str>,
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

        ensure_active(&user)?;

        // TOTP second step: enabled accounts must present a code. Without
        // one the caller gets TotpRequired (no session) and completes via
        // totp_challenge or a second login call carrying the code.
        if user.totp_enabled {
            let code = totp_code
                .map(str::trim)
                .filter(|c| !c.is_empty())
                .ok_or(AuthError::TotpRequired)?;
            self.check_totp(&user, code)?;
        }

        let now = Utc::now();
        self.store.touch_last_login(user.id, now).await?;

        let ttl = if remember_me {
            SESSION_TTL_REMEMBER
        } else {
            SESSION_TTL_SHORT
        };
        let (session, raw_token) = self.issue_session(user.id, ttl).await?;

        self.audit
            .record(
                Utc::now(),
                &AuditEvent::new(
                    AuditAction::UserLogin,
                    Some(&user.id.to_string()),
                    Some(&user.name),
                    None,
                    format!("{} signed in", user.name),
                    None,
                ),
            )
            .await;

        if let Some(ref svc) = self.notifications {
            let _ = svc
                .create_system(
                    user.id,
                    NotificationType::NewLogin,
                    "New login",
                    "A new sign-in to your account was detected.",
                )
                .await;
        }

        self.send_login_notice(&user).await;

        Ok(IssuedSession {
            user,
            session,
            raw_token,
        })
    }

    // ------------------------------------------------------------------
    // TOTP two-factor authentication
    // ------------------------------------------------------------------

    /// Starts (or resumes) 2FA setup: stores a secret unless one is
    /// already enabled, and returns it with the authenticator URI.
    /// Refused while 2FA is enabled — disable first, then re-enroll.
    pub async fn totp_setup(&self, user_id: UserId) -> Result<(String, String), AuthError> {
        let user = self
            .store
            .find_user_by_id(user_id)
            .await?
            .ok_or(AuthError::Unauthorized)?;
        ensure_active(&user)?;
        if user.totp_enabled {
            return Err(AuthError::Validation(
                "two-factor authentication is already enabled".to_string(),
            ));
        }
        let secret = match user.totp_secret {
            Some(secret) => secret,
            None => {
                let secret = generate_totp_secret()?;
                self.store.set_totp_secret(user_id, secret.clone()).await?;
                secret
            }
        };
        let uri = totp_uri(&secret, user.email.as_str())?;
        Ok((secret, uri))
    }

    /// Verifies a code against the pending secret and enables 2FA.
    pub async fn totp_verify(&self, user_id: UserId, code: &str) -> Result<(), AuthError> {
        let user = self
            .store
            .find_user_by_id(user_id)
            .await?
            .ok_or(AuthError::Unauthorized)?;
        ensure_active(&user)?;
        if user.totp_enabled {
            return Err(AuthError::Validation(
                "two-factor authentication is already enabled".to_string(),
            ));
        }
        self.check_totp(&user, code)?;
        self.store.enable_totp(user_id).await?;

        self.audit
            .record(
                Utc::now(),
                &AuditEvent::new(
                    AuditAction::UserTotpEnabled,
                    Some(&user.id.to_string()),
                    Some(&user.name),
                    None,
                    format!("{} enabled two-factor authentication", user.name),
                    None,
                ),
            )
            .await;
        Ok(())
    }

    /// Disables 2FA. Requires the current password when the account has
    /// one, plus a valid TOTP code in all cases.
    pub async fn totp_disable(
        &self,
        user_id: UserId,
        password: Option<&str>,
        code: &str,
    ) -> Result<(), AuthError> {
        let user = self
            .store
            .find_user_by_id(user_id)
            .await?
            .ok_or(AuthError::Unauthorized)?;
        ensure_active(&user)?;
        if !user.totp_enabled {
            return Err(AuthError::Validation(
                "two-factor authentication is not enabled".to_string(),
            ));
        }
        if !user.password_hash.is_empty() {
            let password = password.unwrap_or("");
            if !verify_password(password, &user.password_hash) {
                return Err(AuthError::InvalidCredentials);
            }
        }
        self.check_totp(&user, code)?;
        self.store.disable_totp(user_id).await?;

        self.audit
            .record(
                Utc::now(),
                &AuditEvent::new(
                    AuditAction::UserTotpDisabled,
                    Some(&user.id.to_string()),
                    Some(&user.name),
                    None,
                    format!("{} disabled two-factor authentication", user.name),
                    None,
                ),
            )
            .await;
        Ok(())
    }

    /// Completes login for a TOTP-enabled account with email + code only
    /// (used after the OAuth detour, where no password exists to resend).
    /// Issues a short session on success.
    pub async fn totp_challenge(
        &self,
        email: &str,
        code: &str,
    ) -> Result<IssuedSession, AuthError> {
        let email = Email::parse(email).map_err(|_| AuthError::InvalidCredentials)?;
        let user = self
            .store
            .find_user_by_email(email.as_str())
            .await?
            .ok_or(AuthError::InvalidCredentials)?;
        ensure_active(&user)?;
        if !user.totp_enabled {
            return Err(AuthError::Validation(
                "two-factor authentication is not enabled for this account".to_string(),
            ));
        }
        self.check_totp(&user, code)?;

        let now = Utc::now();
        self.store.touch_last_login(user.id, now).await?;
        let (session, raw_token) = self.issue_session(user.id, SESSION_TTL_SHORT).await?;

        self.audit
            .record(
                Utc::now(),
                &AuditEvent::new(
                    AuditAction::UserLogin,
                    Some(&user.id.to_string()),
                    Some(&user.name),
                    None,
                    format!("{} signed in (two-factor challenge)", user.name),
                    None,
                ),
            )
            .await;

        Ok(IssuedSession {
            user,
            session,
            raw_token,
        })
    }

    fn check_totp(&self, user: &User, code: &str) -> Result<(), AuthError> {
        let secret = user
            .totp_secret
            .as_deref()
            .ok_or_else(|| AuthError::Validation("TOTP has not been set up yet".to_string()))?;
        let totp = build_totp(secret)?;
        if !totp
            .check_current(code.trim())
            .map_err(|e| AuthError::Storage(format!("TOTP verification failed: {e}")))?
        {
            return Err(AuthError::TotpInvalid);
        }
        Ok(())
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

        let user = self
            .store
            .find_user_by_id(session.user_id)
            .await?
            .ok_or(AuthError::Unauthorized)?;
        ensure_active(&user)?;
        Ok(user)
    }

    /// Revokes the session behind `session_cookie` (idempotent).
    pub async fn logout(&self, session_cookie: &str) -> Result<(), AuthError> {
        if let Some(session) = self
            .store
            .find_session_by_hash(&hash_token(session_cookie))
            .await?
        {
            self.store.revoke_session(session.id).await?;
            self.audit
                .record(
                    Utc::now(),
                    &AuditEvent::new(
                        AuditAction::UserLogout,
                        Some(&session.user_id.to_string()),
                        None,
                        None,
                        "signed out".to_string(),
                        None,
                    ),
                )
                .await;
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

        self.audit
            .record(
                now,
                &AuditEvent::new(
                    AuditAction::UserEmailVerified,
                    Some(&token.user_id.to_string()),
                    None,
                    None,
                    "email address verified".to_string(),
                    None,
                ),
            )
            .await;
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
        self.send_system_mail(
            user.email.as_str(),
            "verify-email",
            &[
                ("name", user.name.as_str()),
                (
                    "link",
                    &format!(
                        "{}/auth/verify-email?token={token}",
                        self.dashboard_url.trim_end_matches('/')
                    ),
                ),
            ],
        )
        .await;
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
        self.send_system_mail(
            user.email.as_str(),
            "password-reset",
            &[
                ("name", user.name.as_str()),
                (
                    "link",
                    &format!(
                        "{}/auth/password/reset?token={token}",
                        self.dashboard_url.trim_end_matches('/')
                    ),
                ),
            ],
        )
        .await;
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

        self.audit
            .record(
                Utc::now(),
                &AuditEvent::new(
                    AuditAction::UserPasswordReset,
                    Some(&token.user_id.to_string()),
                    None,
                    None,
                    "password reset".to_string(),
                    None,
                ),
            )
            .await;

        // Compromise signal: the reset flow never changes the email, so
        // this reliably reaches the account owner even when someone else
        // performed the reset. Best-effort — never fails the reset itself.
        if let Ok(Some(user)) = self.store.find_user_by_id(token.user_id).await {
            self.send_system_mail(
                user.email.as_str(),
                "password-changed",
                &[
                    ("name", user.name.as_str()),
                    (
                        "link",
                        &format!("{}/auth/login", self.dashboard_url.trim_end_matches('/')),
                    ),
                ],
            )
            .await;
        }
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
            return Err(AuthError::Validation(
                "missing provider subject".to_string(),
            ));
        }

        let mut user = match self
            .store
            .find_user_by_oauth(provider, &profile.subject)
            .await?
        {
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
                            email
                                .as_str()
                                .split('@')
                                .next()
                                .unwrap_or("user")
                                .to_string()
                        }),
                        email,
                        // OAuth-only account: no password exists to verify against.
                        password_hash: String::new(),
                        avatar_url: profile.avatar_url.clone(),
                        email_verified_at: Some(now),
                        oauth_provider: Some(provider.to_string()),
                        oauth_subject: Some(profile.subject.clone()),
                        totp_secret: None,
                        totp_enabled: false,
                        status: crate::domain::auth::entities::UserStatus::Active,
                        created_at: now,
                        last_login_at: Some(now),
                    };
                    self.store.create_user(&new_user).await?;
                    new_user
                }
            },
        };

        // The provider re-asserts inbox control on every sign-in
        // (provider-unverified addresses never reach here), so any row
        // still unverified — a pre-fix OAuth row, a linked password
        // account — heals itself here. Single choke point: no branch
        // can forget it again.
        if user.email_verified_at.is_none() {
            let now = Utc::now();
            self.store.set_email_verified(user.id, now).await?;
            user.email_verified_at = Some(now);
        }

        self.store.touch_last_login(user.id, Utc::now()).await?;
        ensure_active(&user)?;
        if user.totp_enabled {
            // Second step happens on the login page via totp_challenge —
            // no session is issued here.
            return Err(AuthError::TotpRequired);
        }
        let (session, raw_token) = self.issue_session(user.id, SESSION_TTL_SHORT).await?;

        self.audit
            .record(
                Utc::now(),
                &AuditEvent::new(
                    AuditAction::UserLogin,
                    Some(&user.id.to_string()),
                    Some(&user.name),
                    None,
                    format!("{} signed in with {provider}", user.name),
                    None,
                ),
            )
            .await;

        if let Some(ref svc) = self.notifications {
            let _ = svc
                .create_system(
                    user.id,
                    NotificationType::NewLogin,
                    "New login",
                    &format!("A new sign-in via {provider} was detected."),
                )
                .await;
        }

        self.send_login_notice(&user).await;

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

        self.audit
            .record(
                Utc::now(),
                &AuditEvent::new(
                    AuditAction::UserOnboardingCompleted,
                    Some(&user_id.to_string()),
                    None,
                    None,
                    "completed onboarding".to_string(),
                    None,
                ),
            )
            .await;
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

fn ensure_active(user: &User) -> Result<(), AuthError> {
    if user.status == crate::domain::auth::entities::UserStatus::Suspended {
        return Err(AuthError::AccountSuspended);
    }
    Ok(())
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

/// Generates a random TOTP secret, returned base32-encoded.
fn generate_totp_secret() -> Result<String, AuthError> {
    let mut bytes = vec![0u8; 20];
    rand::rng().fill_bytes(&mut bytes);
    Ok(Secret::Raw(bytes).to_encoded().to_string())
}

/// Builds a TOTP instance from a base32 secret string.
fn build_totp(base32: &str) -> Result<TOTP, AuthError> {
    let secret = Secret::Encoded(base32.to_string());
    let secret_raw = secret
        .to_raw()
        .map_err(|e| AuthError::Storage(format!("invalid TOTP secret: {e}")))?;
    let rfc = Rfc6238::with_defaults(
        secret_raw
            .to_bytes()
            .map_err(|e| AuthError::Storage(format!("TOTP secret conversion failed: {e}")))?,
    )
    .map_err(|e| AuthError::Storage(format!("TOTP config failed: {e}")))?;
    TOTP::from_rfc6238(rfc).map_err(|e| AuthError::Storage(format!("TOTP creation failed: {e}")))
}

fn totp_uri(secret_base32: &str, account_email: &str) -> Result<String, AuthError> {
    let mut totp = build_totp(secret_base32)?;
    totp.issuer = Some("Notifi".to_string());
    totp.account_name = account_email.to_string();
    Ok(totp.get_url().to_string())
}
