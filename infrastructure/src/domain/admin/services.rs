//! Admin service — orchestrates admin creation, login, and TOTP management.

use argon2::password_hash::SaltString;
use argon2::{Argon2, PasswordHash, PasswordHasher, PasswordVerifier};
use chrono::Utc;

use crate::domain::admin::entities::{
    AdminPasswordResetToken, AdminPasswordResetTokenId, AdminSession, AdminSessionId, AdminUser,
    AdminUserId,
};
use crate::domain::auth::errors::AuthError;
use crate::domain::auth::value_objects::{Email, hash_token, new_token, validate_password};
use crate::ports::admin_store::AdminStore;

pub struct AdminService {
    store: Box<dyn AdminStore>,
    expose_dev_tokens: bool,
}

impl AdminService {
    pub fn new(store: Box<dyn AdminStore>, expose_dev_tokens: bool) -> Self {
        Self {
            store,
            expose_dev_tokens,
        }
    }

    /// Whether raw one-time tokens are surfaced in API responses (dev only).
    pub fn exposes_dev_tokens(&self) -> bool {
        self.expose_dev_tokens
    }

    /// Whether at least one admin user exists.
    pub async fn admin_exists(&self) -> Result<bool, AuthError> {
        Ok(self.store.admin_exists().await?)
    }

    /// Creates the first admin (bootstrap). Rejects if an admin already exists.
    pub async fn create_admin(
        &self,
        name: &str,
        email: &str,
        password: &str,
    ) -> Result<AdminUser, AuthError> {
        let email = Email::parse(email)?;
        validate_password(password)?;

        let name = name.trim();
        if name.is_empty() || name.len() > 100 {
            return Err(AuthError::Validation(
                "name must be between 1 and 100 characters".to_string(),
            ));
        }

        if self.store.admin_exists().await? {
            return Err(AuthError::Validation(
                "An admin account already exists. Use the login flow instead.".to_string(),
            ));
        }

        if self.store.find_admin_by_email(email.as_str()).await?.is_some() {
            return Err(AuthError::EmailAlreadyExists);
        }

        let now = Utc::now();
        let admin = AdminUser {
            id: AdminUserId::new(),
            name: name.to_string(),
            email,
            password_hash: hash_password(password)?,
            totp_secret: None,
            totp_enabled: false,
            created_at: now,
            last_login_at: None,
        };

        self.store.create_admin(&admin).await?;
        Ok(admin)
    }

    /// Finds an admin by email (for login).
    pub async fn find_admin_by_email(&self, email: &str) -> Result<Option<AdminUser>, AuthError> {
        Ok(self.store.find_admin_by_email(email).await?)
    }

    /// Finds an admin by id.
    pub async fn find_admin_by_id(&self, id: AdminUserId) -> Result<Option<AdminUser>, AuthError> {
        Ok(self.store.find_admin_by_id(id).await?)
    }

    /// Alias for find_admin_by_id — used by the CurrentAdmin extractor.
    pub async fn find_by_id(&self, id: AdminUserId) -> Result<Option<AdminUser>, AuthError> {
        self.find_admin_by_id(id).await
    }

    /// Verifies admin credentials and returns the admin if valid.
    pub async fn verify_credentials(
        &self,
        email: &str,
        password: &str,
    ) -> Result<AdminUser, AuthError> {
        let admin = self
            .store
            .find_admin_by_email(email)
            .await?
            .ok_or(AuthError::InvalidCredentials)?;

        if !verify_password(password, &admin.password_hash) {
            return Err(AuthError::InvalidCredentials);
        }

        // Touch last_login_at
        let now = Utc::now();
        self.store.touch_admin_last_login(admin.id, now).await?;

        Ok(admin)
    }

    /// Authenticates an admin from a session token.
    /// Session tokens are stored as SHA-256 hashes; we look up by the hash.
    pub async fn authenticate(&self, raw_token: &str) -> Result<AdminUser, AuthError> {
        let session = self
            .store
            .find_admin_session_by_hash(&hash_token(raw_token))
            .await?
            .ok_or(AuthError::Unauthorized)?;

        if !session.is_active(Utc::now()) {
            return Err(AuthError::Unauthorized);
        }

        let admin = self
            .store
            .find_admin_by_id(session.admin_id)
            .await?
            .ok_or(AuthError::Unauthorized)?;

        Ok(admin)
    }

    /// Issues a fresh admin session. Returns the raw cookie value (shown once).
    pub async fn issue_session(&self, admin_id: AdminUserId) -> Result<String, AuthError> {
        let (raw, hash) = new_token();
        let now = Utc::now();
        let session = AdminSession {
            id: AdminSessionId::new(),
            admin_id,
            token_hash: hash,
            expires_at: now + chrono::Duration::days(1),
            revoked_at: None,
            created_at: now,
        };
        self.store.create_admin_session(&session).await?;
        Ok(raw)
    }

    /// Revokes the session behind `session_cookie` (idempotent).
    pub async fn logout(&self, session_cookie: &str) -> Result<(), AuthError> {
        if let Some(session) = self
            .store
            .find_admin_session_by_hash(&hash_token(session_cookie))
            .await?
        {
            self.store.revoke_admin_session(session.id).await?;
        }
        Ok(())
    }

    /// Stores a TOTP secret for an admin.
    pub async fn set_totp_secret(
        &self,
        admin_id: AdminUserId,
        secret: String,
    ) -> Result<(), AuthError> {
        self.store.set_totp_secret(admin_id, secret).await?;
        Ok(())
    }

    /// Enables TOTP for an admin after successful code verification.
    pub async fn enable_totp(&self, admin_id: AdminUserId) -> Result<(), AuthError> {
        self.store.enable_totp(admin_id).await?;
        Ok(())
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
        let Some(admin) = self.store.find_admin_by_email(email.as_str()).await? else {
            return Ok(None);
        };

        self.store
            .consume_admin_reset_tokens_for_admin(admin.id)
            .await?;
        let (raw, hash) = new_token();
        let now = Utc::now();
        self.store
            .create_admin_reset_token(&AdminPasswordResetToken {
                id: AdminPasswordResetTokenId::new(),
                admin_id: admin.id,
                token_hash: hash,
                expires_at: now + chrono::Duration::hours(1),
                consumed_at: None,
                created_at: now,
            })
            .await?;
        Ok(Some(raw))
    }

    pub async fn reset_password(
        &self,
        raw_token: &str,
        new_password: &str,
    ) -> Result<(), AuthError> {
        validate_password(new_password)?;

        let token = self
            .store
            .find_admin_reset_token_by_hash(&hash_token(raw_token))
            .await?
            .filter(|t| t.consumed_at.is_none())
            .ok_or_else(|| {
                AuthError::TokenInvalid("Invalid or expired reset token.".to_string())
            })?;
        if !token.is_usable(Utc::now()) {
            return Err(AuthError::TokenExpired(
                "This reset link has expired. Request a new one.".to_string(),
            ));
        }

        self.store.consume_admin_reset_token(token.id).await?;
        self.store
            .update_admin_password(token.admin_id, hash_password(new_password)?)
            .await?;
        // a reset invalidates every existing session for that account
        self.store.revoke_admin_sessions(token.admin_id).await?;
        Ok(())
    }
}

// ------------------------------------------------------------------
// password hashing (argon2id)
// ------------------------------------------------------------------

fn hash_password(password: &str) -> Result<String, AuthError> {
    use argon2::password_hash::rand_core::OsRng;
    let salt = SaltString::generate(&mut OsRng);
    Ok(Argon2::default()
        .hash_password(password.as_bytes(), &salt)
        .map_err(|e| AuthError::Storage(format!("password hashing failed: {e}")))?
        .to_string())
}

fn verify_password(password: &str, phc_hash: &str) -> bool {
    PasswordHash::new(phc_hash)
        .ok()
        .and_then(|parsed| {
            Argon2::default()
                .verify_password(password.as_bytes(), &parsed)
                .ok()
        })
        .is_some()
}
