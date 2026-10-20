//! Admin service — orchestrates admin creation, login, and TOTP management.

use std::sync::Arc;

use argon2::password_hash::SaltString;
use argon2::{Argon2, PasswordHash, PasswordHasher, PasswordVerifier};
use chrono::Utc;
use serde_json::json;

use crate::domain::admin::entities::{
    AdminPasswordResetToken, AdminPasswordResetTokenId, AdminSession, AdminSessionId, AdminStatus,
    AdminUser, AdminUserId,
};
use crate::domain::audit::AuditService;
use crate::domain::audit::entities::{AuditAction, AuditEvent};
use crate::domain::auth::value_objects::{Email, hash_token, new_token, validate_password};
use crate::domain::error::AppError;
use crate::ports::admin_store::AdminStore;

pub struct AdminService {
    store: Box<dyn AdminStore>,
    expose_dev_tokens: bool,
    audit: Arc<AuditService>,
    mailer: Option<Arc<dyn crate::ports::mailer::SmtpMailer>>,
    templates: Option<Arc<dyn crate::ports::mailer::SystemTemplates>>,
    admin_url: String,
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
            mailer: None,
            templates: None,
            admin_url: String::new(),
        }
    }

    /// Wires system mail (password-reset delivery). All three pieces travel
    /// together: without any one of them, the dev behavior is kept.
    pub fn with_system_mail(
        mut self,
        mailer: Arc<dyn crate::ports::mailer::SmtpMailer>,
        templates: Arc<dyn crate::ports::mailer::SystemTemplates>,
        admin_url: String,
    ) -> Self {
        self.mailer = Some(mailer);
        self.templates = Some(templates);
        self.admin_url = admin_url;
        self
    }

    /// Whether raw one-time tokens are surfaced in API responses (dev only).
    pub fn exposes_dev_tokens(&self) -> bool {
        self.expose_dev_tokens
    }

    /// Whether at least one admin user exists.
    pub async fn admin_exists(&self) -> Result<bool, AppError> {
        Ok(self.store.admin_exists().await?)
    }

    /// Creates the first admin (bootstrap). Rejects if an admin already exists.
    pub async fn create_admin(
        &self,
        name: &str,
        email: &str,
        password: &str,
    ) -> Result<AdminUser, AppError> {
        let email = Email::parse(email)?;
        validate_password(password)?;

        let name = name.trim();
        if name.is_empty() || name.len() > 100 {
            return Err(AppError::Validation(
                "name must be between 1 and 100 characters".to_string(),
            ));
        }

        if self.store.admin_exists().await? {
            return Err(AppError::Validation(
                "An admin account already exists. Use the login flow instead.".to_string(),
            ));
        }

        if self
            .store
            .find_admin_by_email(email.as_str())
            .await?
            .is_some()
        {
            return Err(AppError::EmailAlreadyExists);
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
        self.audit
            .record(
                now,
                &AuditEvent::new_admin(
                    AuditAction::AdminCreated,
                    &admin.id.to_string(),
                    Some(&admin.name),
                    None,
                    format!("admin '{}' bootstrapped as super admin", admin.email),
                    None,
                ),
            )
            .await;
        Ok(admin)
    }

    /// Finds an admin by email (for login).
    pub async fn find_admin_by_email(&self, email: &str) -> Result<Option<AdminUser>, AppError> {
        Ok(self.store.find_admin_by_email(email).await?)
    }

    /// Finds an admin by id.
    pub async fn find_admin_by_id(&self, id: AdminUserId) -> Result<Option<AdminUser>, AppError> {
        Ok(self.store.find_admin_by_id(id).await?)
    }

    /// Alias for find_admin_by_id — used by the CurrentAdmin extractor.
    pub async fn find_by_id(&self, id: AdminUserId) -> Result<Option<AdminUser>, AppError> {
        self.find_admin_by_id(id).await
    }

    /// Verifies admin credentials and returns the admin if valid.
    /// Pending, suspended, or removed accounts cannot sign in.
    pub async fn verify_credentials(
        &self,
        email: &str,
        password: &str,
    ) -> Result<AdminUser, AppError> {
        let admin = self
            .store
            .find_admin_by_email(email)
            .await?
            .ok_or(AppError::InvalidCredentials)?;

        if !verify_password(password, &admin.password_hash) {
            return Err(AppError::InvalidCredentials);
        }

        ensure_active_admin(&admin)?;

        // Touch last_login_at
        let now = Utc::now();
        self.store.touch_admin_last_login(admin.id, now).await?;

        self.audit
            .record(
                now,
                &AuditEvent::new_admin(
                    AuditAction::AdminLogin,
                    &admin.id.to_string(),
                    Some(&admin.name),
                    None,
                    format!("admin '{}' signed in", admin.email),
                    None,
                ),
            )
            .await;

        Ok(admin)
    }

    /// Authenticates an admin from a session token.
    /// Session tokens are stored as SHA-256 hashes; we look up by the hash.
    pub async fn authenticate(&self, raw_token: &str) -> Result<AdminUser, AppError> {
        let session = self
            .store
            .find_admin_session_by_hash(&hash_token(raw_token))
            .await?
            .ok_or(AppError::Unauthorized)?;

        if !session.is_active(Utc::now()) {
            return Err(AppError::Unauthorized);
        }

        let admin = self
            .store
            .find_admin_by_id(session.admin_id)
            .await?
            .ok_or(AppError::Unauthorized)?;

        ensure_active_admin(&admin)?;

        Ok(admin)
    }

    /// Issues a fresh admin session. Returns the raw cookie value (shown once).
    pub async fn issue_session(&self, admin_id: AdminUserId) -> Result<String, AppError> {
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
    pub async fn logout(&self, session_cookie: &str) -> Result<(), AppError> {
        if let Some(session) = self
            .store
            .find_admin_session_by_hash(&hash_token(session_cookie))
            .await?
        {
            self.store.revoke_admin_session(session.id).await?;
            if let Some(admin) = self.store.find_admin_by_id(session.admin_id).await? {
                self.audit
                    .record(
                        Utc::now(),
                        &AuditEvent::new_admin(
                            AuditAction::AdminLogout,
                            &admin.id.to_string(),
                            Some(&admin.name),
                            None,
                            format!("admin '{}' signed out", admin.email),
                            None,
                        ),
                    )
                    .await;
            }
        }
        Ok(())
    }

    /// Stores a TOTP secret for an admin.
    pub async fn set_totp_secret(
        &self,
        admin_id: AdminUserId,
        secret: String,
    ) -> Result<(), AppError> {
        self.store.set_totp_secret(admin_id, secret).await?;
        Ok(())
    }

    /// Enables TOTP for an admin after successful code verification.
    pub async fn enable_totp(&self, admin_id: AdminUserId) -> Result<(), AppError> {
        self.store.enable_totp(admin_id).await?;
        if let Some(admin) = self.store.find_admin_by_id(admin_id).await? {
            self.audit
                .record(
                    Utc::now(),
                    &AuditEvent::new_admin(
                        AuditAction::AdminTotpEnabled,
                        &admin.id.to_string(),
                        Some(&admin.name),
                        None,
                        format!("admin '{}' enabled two-factor authentication", admin.email),
                        None,
                    ),
                )
                .await;
        }
        Ok(())
    }

    // ------------------------------------------------------------------
    // password reset
    // ------------------------------------------------------------------

    /// Always succeeds from the caller's perspective (no enumeration):
    /// returns `Some(raw_token)` only when the account exists.
    pub async fn forgot_password(&self, email: &str) -> Result<Option<String>, AppError> {
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
        if let (Some(mailer), Some(templates)) = (self.mailer.as_ref(), self.templates.as_ref()) {
            let link = format!(
                "{}/password/reset?token={raw}",
                self.admin_url.trim_end_matches('/')
            );
            match templates.render("password-reset", &[("name", &admin.name), ("link", &link)]) {
                Ok(rendered) => {
                    if let Err(e) = mailer
                        .send(
                            crate::ports::mailer::SystemSender::NoReply,
                            admin.email.as_str(),
                            &rendered.subject,
                            &rendered.text,
                            Some(&rendered.html),
                        )
                        .await
                    {
                        tracing::warn!(error = %e, "admin reset email delivery failed");
                    }
                }
                Err(e) => tracing::warn!(error = %e, "admin reset template render failed"),
            }
        }
        Ok(Some(raw))
    }

    pub async fn reset_password(
        &self,
        raw_token: &str,
        new_password: &str,
    ) -> Result<(), AppError> {
        validate_password(new_password)?;

        let token = self
            .store
            .find_admin_reset_token_by_hash(&hash_token(raw_token))
            .await?
            .filter(|t| t.consumed_at.is_none())
            .ok_or_else(|| AppError::TokenInvalid("Invalid or expired reset token.".to_string()))?;
        if !token.is_usable(Utc::now()) {
            return Err(AppError::TokenExpired(
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
    ) -> Result<String, AppError> {
        if !verify_password(current_password, &admin.password_hash) {
            return Err(AppError::InvalidCredentials);
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
                &AuditEvent::new_admin(
                    AuditAction::AdminPasswordChanged,
                    &admin.id.to_string(),
                    Some(&admin.name),
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

    pub async fn list_admins(
        &self,
        limit: i64,
        offset: i64,
    ) -> Result<(Vec<AdminUser>, i64), AppError> {
        Ok(self.store.list_admins(limit, offset).await?)
    }

    /// Creates another admin. Super admins only; going active immediately.
    pub async fn invite_admin(
        &self,
        caller: &AdminUser,
        name: &str,
        email: &str,
        password: &str,
    ) -> Result<AdminUser, AppError> {
        require_super_admin(caller)?;

        let email = Email::parse(email)?;
        validate_password(password)?;

        let name = name.trim();
        if name.is_empty() || name.len() > 100 {
            return Err(AppError::Validation(
                "name must be between 1 and 100 characters".to_string(),
            ));
        }

        if self
            .store
            .find_admin_by_email(email.as_str())
            .await?
            .is_some()
        {
            return Err(AppError::EmailAlreadyExists);
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
            status: AdminStatus::Active,
            created_at: now,
            last_login_at: None,
        };
        self.store.create_admin(&admin).await?;

        self.audit
            .record(
                now,
                &AuditEvent::new_admin(
                    AuditAction::AdminCreated,
                    &caller.id.to_string(),
                    Some(&caller.name),
                    None,
                    format!(
                        "super admin '{}' created admin '{}'",
                        caller.email, admin.email
                    ),
                    Some(json!({ "admin_id": admin.id.to_string() })),
                ),
            )
            .await;
        Ok(admin)
    }

    /// Removes another admin. Super admins only; applies immediately.
    /// Refuses self-removal and any action on the super admin account.
    pub async fn request_removal(
        &self,
        caller: &AdminUser,
        target_id: AdminUserId,
    ) -> Result<(), AppError> {
        require_super_admin(caller)?;
        let target = self
            .store
            .find_admin_by_id(target_id)
            .await?
            .ok_or_else(|| AppError::NotFound("admin not found".into()))?;
        if target.is_super_admin {
            return Err(AppError::Validation(
                "the super admin account cannot be removed".to_string(),
            ));
        }
        if target.id == caller.id {
            return Err(AppError::Validation(
                "you cannot remove your own admin account".to_string(),
            ));
        }

        let now = Utc::now();
        self.apply_removal(&target).await?;
        self.audit
            .record(
                now,
                &AuditEvent::new_admin(
                    AuditAction::AdminApprovalDecided,
                    &caller.id.to_string(),
                    Some(&caller.name),
                    None,
                    format!(
                        "super admin '{}' removed admin '{}'",
                        caller.email, target.email
                    ),
                    Some(json!({ "admin_id": target.id.to_string(), "decision": "approved" })),
                ),
            )
            .await;
        Ok(())
    }

    /// Suspends or restores another admin. Super admins only; suspending
    /// revokes every session. Refuses self-suspension and any action on the
    /// super admin account.
    pub async fn set_admin_status(
        &self,
        caller: &AdminUser,
        target_id: AdminUserId,
        suspended: bool,
    ) -> Result<(), AppError> {
        require_super_admin(caller)?;
        let target = self
            .store
            .find_admin_by_id(target_id)
            .await?
            .ok_or_else(|| AppError::NotFound("admin not found".into()))?;
        if target.is_super_admin {
            return Err(AppError::Validation(
                "the super admin account cannot be suspended".to_string(),
            ));
        }
        if target.id == caller.id {
            return Err(AppError::Validation(
                "you cannot suspend your own admin account".to_string(),
            ));
        }

        let status = if suspended {
            AdminStatus::Suspended
        } else {
            AdminStatus::Active
        };
        if target.status == status {
            return Ok(());
        }
        self.store.set_admin_status(target.id, status).await?;
        if suspended {
            self.store.revoke_admin_sessions(target.id).await?;
        }
        self.audit
            .record(
                Utc::now(),
                &AuditEvent::new_admin(
                    AuditAction::AdminApprovalDecided,
                    &caller.id.to_string(),
                    Some(&caller.name),
                    None,
                    format!(
                        "super admin '{}' {} admin '{}'",
                        caller.email,
                        if suspended { "suspended" } else { "restored" },
                        target.email,
                    ),
                    Some(json!({
                        "admin_id": target.id.to_string(),
                        "decision": "approved",
                    })),
                ),
            )
            .await;
        Ok(())
    }

    /// Deactivates an admin: soft-delete + revoke every session.
    async fn apply_removal(&self, target: &AdminUser) -> Result<(), AppError> {
        self.store.soft_delete_admin(target.id).await?;
        self.store.revoke_admin_sessions(target.id).await?;
        Ok(())
    }
}

/// Rejects sign-in/session use for anything but active accounts.
fn ensure_active_admin(admin: &AdminUser) -> Result<(), AppError> {
    match admin.status {
        AdminStatus::Active => Ok(()),
        AdminStatus::Pending => Err(AppError::Forbidden(
            "this admin account is awaiting approval".to_string(),
        )),
        AdminStatus::Suspended => Err(AppError::Forbidden(
            "this admin account has been suspended".to_string(),
        )),
    }
}

fn require_super_admin(caller: &AdminUser) -> Result<(), AppError> {
    if caller.is_super_admin {
        Ok(())
    } else {
        Err(AppError::Forbidden(
            "only the super admin can perform this action".to_string(),
        ))
    }
}

// ------------------------------------------------------------------
// password hashing (argon2id)
// ------------------------------------------------------------------

fn hash_password(password: &str) -> Result<String, AppError> {
    use argon2::password_hash::rand_core::OsRng;
    let salt = SaltString::generate(&mut OsRng);
    Ok(Argon2::default()
        .hash_password(password.as_bytes(), &salt)
        .map_err(|e| AppError::Storage(format!("password hashing failed: {e}")))?
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
