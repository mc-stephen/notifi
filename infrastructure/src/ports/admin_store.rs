//! The persistence port for admin users.

use std::future::Future;
use std::pin::Pin;

use chrono::{DateTime, Utc};

use crate::domain::admin::entities::{
    AdminApprovalRequest, AdminPasswordResetToken, AdminSession, AdminSessionId, AdminStatus,
    AdminUser, AdminUserId,
};
use crate::ports::auth_store::StoreError;

/// Boxed future returned by every port method.
pub type BoxFut<'a, T> = Pin<Box<dyn Future<Output = T> + Send + 'a>>;

pub trait AdminStore: Send + Sync {
    /// Whether at least one admin user exists.
    fn admin_exists(&self) -> BoxFut<'_, Result<bool, StoreError>>;
    /// Creates a new admin user.
    fn create_admin(&self, admin: &AdminUser) -> BoxFut<'_, Result<(), StoreError>>;
    /// Finds an admin by email.
    fn find_admin_by_email(&self, email: &str) -> BoxFut<'_, Result<Option<AdminUser>, StoreError>>;
    /// Finds an admin by id.
    fn find_admin_by_id(&self, id: AdminUserId) -> BoxFut<'_, Result<Option<AdminUser>, StoreError>>;
    /// All non-deleted admins, oldest first, plus the total count.
    fn list_admins(
        &self,
        limit: i64,
        offset: i64,
    ) -> BoxFut<'_, Result<(Vec<AdminUser>, i64), StoreError>>;
    /// Sets an admin's account status. Returns false when unknown/deleted.
    fn set_admin_status(
        &self,
        id: AdminUserId,
        status: AdminStatus,
    ) -> BoxFut<'_, Result<bool, StoreError>>;
    /// Soft-deletes an admin. Returns false when unknown/already deleted.
    fn soft_delete_admin(&self, id: AdminUserId) -> BoxFut<'_, Result<bool, StoreError>>;
    /// Stores the TOTP secret for an admin.
    fn set_totp_secret(&self, id: AdminUserId, secret: String) -> BoxFut<'_, Result<(), StoreError>>;
    /// Enables TOTP for an admin.
    fn enable_totp(&self, id: AdminUserId) -> BoxFut<'_, Result<(), StoreError>>;
    /// Updates the admin's last login timestamp.
    fn touch_admin_last_login(&self, id: AdminUserId, at: DateTime<Utc>) -> BoxFut<'_, Result<(), StoreError>>;
    /// Creates a new admin session.
    fn create_admin_session(&self, session: &AdminSession) -> BoxFut<'_, Result<(), StoreError>>;
    /// Finds an admin session by its token hash.
    fn find_admin_session_by_hash(
        &self,
        hash: &str,
    ) -> BoxFut<'_, Result<Option<AdminSession>, StoreError>>;
    /// Revokes a single admin session.
    fn revoke_admin_session(&self, id: AdminSessionId) -> BoxFut<'_, Result<(), StoreError>>;
    /// Revokes all sessions for an admin.
    fn revoke_admin_sessions(&self, id: AdminUserId) -> BoxFut<'_, Result<(), StoreError>>;
    /// Updates an admin's password hash.
    fn update_admin_password(
        &self,
        id: AdminUserId,
        password_hash: String,
    ) -> BoxFut<'_, Result<(), StoreError>>;
    /// Creates a password-reset token.
    fn create_admin_reset_token(
        &self,
        token: &AdminPasswordResetToken,
    ) -> BoxFut<'_, Result<(), StoreError>>;
    /// Invalidates all outstanding reset tokens for an admin.
    fn consume_admin_reset_tokens_for_admin(
        &self,
        id: AdminUserId,
    ) -> BoxFut<'_, Result<(), StoreError>>;
    /// Finds a reset token by its hash.
    fn find_admin_reset_token_by_hash(
        &self,
        hash: &str,
    ) -> BoxFut<'_, Result<Option<AdminPasswordResetToken>, StoreError>>;
    /// Marks a reset token as consumed.
    fn consume_admin_reset_token(
        &self,
        id: crate::domain::admin::entities::AdminPasswordResetTokenId,
    ) -> BoxFut<'_, Result<(), StoreError>>;
    /// Creates an approval request.
    fn create_approval(
        &self,
        request: &AdminApprovalRequest,
    ) -> BoxFut<'_, Result<(), StoreError>>;
    /// Finds an approval request by id.
    fn find_approval(&self, id: &str) -> BoxFut<'_, Result<Option<AdminApprovalRequest>, StoreError>>;
    /// Lists approval requests, newest first, optionally filtered by status.
    fn list_approvals(
        &self,
        status: Option<&str>,
    ) -> BoxFut<'_, Result<Vec<AdminApprovalRequest>, StoreError>>;
    /// Decides a pending request. Returns false unless it was pending.
    fn decide_approval(
        &self,
        id: &str,
        approved: bool,
        decided_by: AdminUserId,
    ) -> BoxFut<'_, Result<bool, StoreError>>;
}
