//! Admin user entity — platform administrators (separate from platform customers).

use chrono::{DateTime, Utc};
use notifi_core::define_id;

use crate::domain::auth::value_objects::Email;

define_id!(AdminUserId);
define_id!(AdminSessionId);

/// A platform admin (manages the Notifi platform, not a customer).
#[derive(Debug, Clone)]
pub struct AdminUser {
    pub id: AdminUserId,
    pub name: String,
    pub email: Email,
    /// argon2id PHC string; never leaves the domain boundary in responses.
    pub password_hash: String,
    /// TOTP secret (base32-encoded); set when 2FA is configured.
    pub totp_secret: Option<String>,
    /// Whether TOTP 2FA is enabled for this admin.
    pub totp_enabled: bool,
    pub created_at: DateTime<Utc>,
    pub last_login_at: Option<DateTime<Utc>>,
}

/// A signed-in admin browser session (cookie-backed).
#[derive(Debug, Clone)]
pub struct AdminSession {
    pub id: AdminSessionId,
    pub admin_id: AdminUserId,
    /// SHA-256 hex of the raw cookie value; raw value is never stored.
    pub token_hash: String,
    pub expires_at: DateTime<Utc>,
    pub revoked_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}

impl AdminSession {
    /// True when the session can still authenticate its owner.
    pub fn is_active(&self, now: DateTime<Utc>) -> bool {
        self.revoked_at.is_none() && now < self.expires_at
    }
}

define_id!(AdminPasswordResetTokenId);

/// A one-time password-reset token for an admin (1h TTL, single-use).
#[derive(Debug, Clone)]
pub struct AdminPasswordResetToken {
    pub id: AdminPasswordResetTokenId,
    pub admin_id: AdminUserId,
    /// SHA-256 hex of the raw token value; raw value is never stored.
    pub token_hash: String,
    pub expires_at: DateTime<Utc>,
    pub consumed_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}

impl AdminPasswordResetToken {
    /// True when the token can still be exchanged.
    pub fn is_usable(&self, now: DateTime<Utc>) -> bool {
        self.consumed_at.is_none() && now < self.expires_at
    }
}
