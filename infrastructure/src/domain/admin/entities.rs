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
    /// The first bootstrapped admin; approves everything, acts unilaterally,
    /// and cannot be removed or suspended.
    pub is_super_admin: bool,
    /// Account state: pending (awaiting approval), active, suspended.
    pub status: AdminStatus,
    pub created_at: DateTime<Utc>,
    pub last_login_at: Option<DateTime<Utc>>,
}

/// Account state for a platform admin.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AdminStatus {
    Pending,
    Active,
    Suspended,
}

impl AdminStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Pending => "pending",
            Self::Active => "active",
            Self::Suspended => "suspended",
        }
    }
}

impl std::str::FromStr for AdminStatus {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "pending" => Ok(Self::Pending),
            "active" => Ok(Self::Active),
            "suspended" => Ok(Self::Suspended),
            _ => Err(()),
        }
    }
}

impl std::fmt::Display for AdminStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

/// An approval request for a sensitive admin action.
#[derive(Debug, Clone)]
pub struct AdminApprovalRequest {
    pub id: String,
    pub kind: ApprovalKind,
    pub target_admin_id: AdminUserId,
    pub requested_by: AdminUserId,
    pub status: ApprovalStatus,
    pub decided_by: Option<AdminUserId>,
    pub decided_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ApprovalKind {
    Create,
    Remove,
}

impl ApprovalKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Create => "create",
            Self::Remove => "remove",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ApprovalStatus {
    Pending,
    Approved,
    Rejected,
}

impl ApprovalStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Pending => "pending",
            Self::Approved => "approved",
            Self::Rejected => "rejected",
        }
    }
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
