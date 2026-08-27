//! Auth aggregates and value types.

use chrono::{DateTime, Utc};
use notifi_core::define_id;

use crate::domain::auth::value_objects::Email;

define_id!(UserId);
define_id!(SessionId);
define_id!(AuthTokenId);

/// A platform user (someone who signs in to the dashboard).
#[derive(Debug, Clone)]
pub struct User {
    pub id: UserId,
    pub name: String,
    pub email: Email,
    /// argon2id PHC string; never leaves the domain boundary in responses.
    /// Empty for OAuth-only accounts (no password is known).
    pub password_hash: String,
    pub avatar_url: Option<String>,
    pub email_verified_at: Option<DateTime<Utc>>,
    /// Set when the account was created through (or later linked to) an
    /// OAuth provider; `(provider, subject)` is unique per user.
    pub oauth_provider: Option<String>,
    pub oauth_subject: Option<String>,
    pub created_at: DateTime<Utc>,
    pub last_login_at: Option<DateTime<Utc>>,
}

impl User {
    pub fn email_verified(&self) -> bool {
        self.email_verified_at.is_some()
    }
}

/// Kind of one-time email token.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TokenPurpose {
    EmailVerification,
    PasswordReset,
}

impl TokenPurpose {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::EmailVerification => "email_verification",
            Self::PasswordReset => "password_reset",
        }
    }

    /// How long a freshly issued token of this kind stays valid.
    pub fn ttl(self) -> chrono::Duration {
        match self {
            // verification links stay usable for a day; resets are tighter
            Self::EmailVerification => chrono::Duration::hours(24),
            Self::PasswordReset => chrono::Duration::hours(1),
        }
    }
}

/// A signed-in browser session (cookie-backed).
#[derive(Debug, Clone)]
pub struct Session {
    pub id: SessionId,
    pub user_id: UserId,
    /// SHA-256 hex of the raw cookie value; raw value is never stored.
    pub token_hash: String,
    pub expires_at: DateTime<Utc>,
    pub revoked_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}

impl Session {
    /// True when the session can still authenticate its owner.
    pub fn is_active(&self, now: DateTime<Utc>) -> bool {
        self.revoked_at.is_none() && now < self.expires_at
    }
}

/// A one-time token emailed to the user (verification / password reset).
#[derive(Debug, Clone)]
pub struct AuthToken {
    pub id: AuthTokenId,
    pub user_id: UserId,
    pub purpose: TokenPurpose,
    /// SHA-256 hex of the raw token; raw value is never stored.
    pub token_hash: String,
    pub expires_at: DateTime<Utc>,
    pub consumed_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}

impl AuthToken {
    /// True when the token can still be exchanged.
    pub fn is_usable(&self, now: DateTime<Utc>) -> bool {
        self.consumed_at.is_none() && now < self.expires_at
    }
}
