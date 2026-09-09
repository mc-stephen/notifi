//! Admin service — orchestrates admin creation, login, and TOTP management.

use std::sync::Arc;

use argon2::password_hash::SaltString;
use argon2::{Argon2, PasswordHash, PasswordHasher, PasswordVerifier};
use chrono::Utc;
use serde_json::json;

use crate::domain::admin::entities::{
    AdminApprovalRequest, AdminPasswordResetToken, AdminPasswordResetTokenId, AdminSession,
    AdminSessionId, AdminStatus, AdminUser, AdminUserId, ApprovalKind, ApprovalStatus,
};
use crate::domain::auth::errors::AuthError;
use crate::domain::auth::value_objects::{Email, hash_token, new_token, validate_password};
use crate::domain::audit::entities::{AuditAction, AuditEvent};
use crate::domain::audit::AuditService;
use crate::ports::admin_store::AdminStore;

pub struct AdminService {
    store: Box<dyn AdminStore>,
    expose_dev_tokens: bool,
    audit: Arc<AuditService>,
}

impl AdminService {
    pub fn new(
        store: Box<dyn AdminStore>,
        expose_dev_tokens: bool,
        audit: Arc<AuditService>,
    ) -> Self {
        Self {
            store,
            expose_dev_tokens,
            audit,
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
            // Bootstrap creates the one and only super admin.
            is_super_admin: true,
            status: AdminStatus::Active,
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
    /// Pending, suspended, or removed accounts cannot sign in.
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

        ensure_active_admin(&admin)?;

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

        ensure_active_admin(&admin)?;

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

    // ------------------------------------------------------------------
    // logged-in password change
    // ------------------------------------------------------------------

    /// Changes the caller's own password. Verifies the current password,
    /// rotates the hash, revokes every session, and returns a fresh raw
    /// session token so the caller stays signed in.
    pub async fn change_password(
        &self,
        admin: &AdminUser,
        current_password: &str,
        new_password: &str,
    ) -> Result<String, AuthError> {
        if !verify_password(current_password, &admin.password_hash) {
            return Err(AuthError::InvalidCredentials);
        }
        validate_password(new_password)?;

        self.store
            .update_admin_password(admin.id, hash_password(new_password)?)
            .await?;
        self.store.revoke_admin_sessions(admin.id).await?;
        let raw = self.issue_session(admin.id).await?;

        self.audit
            .record(
                Utc::now(),
                &AuditEvent::new(
                    AuditAction::AdminPasswordChanged,
                    Some(&admin.id.to_string()),
                    None,
                    None,
                    format!("admin '{}' changed their password", admin.email),
                    None,
                ),
            )
            .await;
        Ok(raw)
    }

    // ------------------------------------------------------------------
    // admin governance (super admin supervision)
    // ------------------------------------------------------------------

    pub async fn list_admins(&self) -> Result<Vec<AdminUser>, AuthError> {
        Ok(self.store.list_admins().await?)
    }

    pub async fn list_approvals(
        &self,
        caller: &AdminUser,
        status: Option<&str>,
    ) -> Result<Vec<AdminApprovalRequest>, AuthError> {
        require_super_admin(caller)?;
        if let Some(s) = status {
            match s {
                "pending" | "approved" | "rejected" => {}
                _ => {
                    return Err(AuthError::Validation(
                        "invalid status (expected pending, approved, or rejected)".to_string(),
                    ))
                }
            }
        }
        Ok(self.store.list_approvals(status).await?)
    }

    /// Pending/decided requests with target + requester display names.
    pub async fn list_approval_views(
        &self,
        caller: &AdminUser,
        status: Option<&str>,
    ) -> Result<Vec<AdminApprovalView>, AuthError> {
        let requests = self.list_approvals(caller, status).await?;
        let mut views = Vec::with_capacity(requests.len());
        for request in requests {
            let target = self.store.find_admin_by_id(request.target_admin_id).await?;
            let requester = self
                .store
                .find_admin_by_id(request.requested_by)
                .await?;
            views.push(AdminApprovalView {
                request,
                target_name: target.as_ref().map(|a| a.name.clone()),
                target_email: target.as_ref().map(|a| a.email.to_string()),
                requester_name: requester.as_ref().map(|a| a.name.clone()),
            });
        }
        Ok(views)
    }

    /// Creates another admin. Super-admin creations go active immediately;
    /// everyone else's land `pending` with an approval request and cannot
    /// sign in until approved.
    pub async fn invite_admin(
        &self,
        caller: &AdminUser,
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

        if self
            .store
            .find_admin_by_email(email.as_str())
            .await?
            .is_some()
        {
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
            is_super_admin: false,
            status: if caller.is_super_admin {
                AdminStatus::Active
            } else {
                AdminStatus::Pending
            },
            created_at: now,
            last_login_at: None,
        };
        self.store.create_admin(&admin).await?;

        if caller.is_super_admin {
            self.audit
                .record(
                    now,
                    &AuditEvent::new(
                        AuditAction::AdminCreated,
                        Some(&caller.id.to_string()),
                        None,
                        None,
                        format!(
                            "super admin '{}' created admin '{}'",
                            caller.email, admin.email
                        ),
                        Some(json!({ "admin_id": admin.id.to_string() })),
                    ),
                )
                .await;
        } else {
            self.store
                .create_approval(&AdminApprovalRequest {
                    id: notifi_core::Ulid::new().to_string(),
                    kind: ApprovalKind::Create,
                    target_admin_id: admin.id,
                    requested_by: caller.id,
                    status: ApprovalStatus::Pending,
                    decided_by: None,
                    decided_at: None,
                    created_at: now,
                })
                .await?;
            self.audit
                .record(
                    now,
                    &AuditEvent::new(
                        AuditAction::AdminCreated,
                        Some(&caller.id.to_string()),
                        None,
                        None,
                        format!(
                            "admin '{}' requested creation of admin '{}' (pending approval)",
                            caller.email, admin.email
                        ),
                        Some(json!({ "admin_id": admin.id.to_string() })),
                    ),
                )
                .await;
        }
        Ok(admin)
    }

    /// Requests (or, for the super admin, immediately applies) the removal
    /// of another admin. Refuses self-removal and any action on the super
    /// admin account.
    pub async fn request_removal(
        &self,
        caller: &AdminUser,
        target_id: AdminUserId,
    ) -> Result<bool, AuthError> {
        let target = self
            .store
            .find_admin_by_id(target_id)
            .await?
            .ok_or_else(|| AuthError::NotFound("admin not found".into()))?;
        if target.is_super_admin {
            return Err(AuthError::Validation(
                "the super admin account cannot be removed".to_string(),
            ));
        }
        if target.id == caller.id {
            return Err(AuthError::Validation(
                "you cannot remove your own admin account".to_string(),
            ));
        }

        let now = Utc::now();
        if caller.is_super_admin {
            self.apply_removal(&target).await?;
            self.audit
                .record(
                    now,
                    &AuditEvent::new(
                        AuditAction::AdminApprovalDecided,
                        Some(&caller.id.to_string()),
                        None,
                        None,
                        format!(
                            "super admin '{}' removed admin '{}'",
                            caller.email, target.email
                        ),
                        Some(
                            json!({ "admin_id": target.id.to_string(), "decision": "approved" }),
                        ),
                    ),
                )
                .await;
            return Ok(true);
        }

        self.store
            .create_approval(&AdminApprovalRequest {
                id: notifi_core::Ulid::new().to_string(),
                kind: ApprovalKind::Remove,
                target_admin_id: target.id,
                requested_by: caller.id,
                status: ApprovalStatus::Pending,
                decided_by: None,
                decided_at: None,
                created_at: now,
            })
            .await?;
        Ok(false)
    }

    /// Decides a pending approval request. Super admins only.
    pub async fn decide_approval(
        &self,
        caller: &AdminUser,
        request_id: &str,
        approve: bool,
    ) -> Result<(), AuthError> {
        require_super_admin(caller)?;

        let request = self
            .store
            .find_approval(request_id)
            .await?
            .ok_or_else(|| AuthError::NotFound("approval request not found".into()))?;
        if request.status != ApprovalStatus::Pending {
            return Err(AuthError::Conflict(
                "this request has already been decided".to_string(),
            ));
        }

        let target = self
            .store
            .find_admin_by_id(request.target_admin_id)
            .await?
            .ok_or_else(|| AuthError::NotFound("target admin not found".into()))?;
        if target.is_super_admin {
            return Err(AuthError::Validation(
                "the super admin account cannot be removed or suspended".to_string(),
            ));
        }

        if approve {
            match request.kind {
                ApprovalKind::Create => {
                    self.store
                        .set_admin_status(target.id, AdminStatus::Active)
                        .await?;
                }
                ApprovalKind::Remove => {
                    self.apply_removal(&target).await?;
                }
            }
        } else if request.kind == ApprovalKind::Create {
            // Rejected invites vanish entirely.
            self.store.soft_delete_admin(target.id).await?;
        }

        let decided = self
            .store
            .decide_approval(request_id, approve, caller.id)
            .await?;
        if !decided {
            return Err(AuthError::Conflict(
                "this request has already been decided".to_string(),
            ));
        }

        let decision = if approve { "approved" } else { "rejected" };
        self.audit
            .record(
                Utc::now(),
                &AuditEvent::new(
                    AuditAction::AdminApprovalDecided,
                    Some(&caller.id.to_string()),
                    None,
                    None,
                    format!(
                        "super admin '{}' {decision} {} request for admin '{}'",
                        caller.email,
                        request.kind.as_str(),
                        target.email,
                    ),
                    Some(json!({
                        "request_id": request_id,
                        "kind": request.kind.as_str(),
                        "decision": decision,
                        "admin_id": target.id.to_string(),
                    })),
                ),
            )
            .await;
        Ok(())
    }

    /// Deactivates an admin: soft-delete + revoke every session.
    async fn apply_removal(&self, target: &AdminUser) -> Result<(), AuthError> {
        self.store.soft_delete_admin(target.id).await?;
        self.store.revoke_admin_sessions(target.id).await?;
        Ok(())
    }
}

/// An approval request bundled with display names for the UI.
#[derive(Debug, Clone)]
pub struct AdminApprovalView {
    pub request: AdminApprovalRequest,
    pub target_name: Option<String>,
    pub target_email: Option<String>,
    pub requester_name: Option<String>,
}

/// Rejects sign-in/session use for anything but active accounts.
fn ensure_active_admin(admin: &AdminUser) -> Result<(), AuthError> {
    match admin.status {
        AdminStatus::Active => Ok(()),
        AdminStatus::Pending => Err(AuthError::Forbidden(
            "this admin account is awaiting approval".to_string(),
        )),
        AdminStatus::Suspended => Err(AuthError::Forbidden(
            "this admin account has been suspended".to_string(),
        )),
    }
}

fn require_super_admin(caller: &AdminUser) -> Result<(), AuthError> {
    if caller.is_super_admin {
        Ok(())
    } else {
        Err(AuthError::Forbidden(
            "only the super admin can perform this action".to_string(),
        ))
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
