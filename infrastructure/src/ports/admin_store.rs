//! The persistence port for admin users.

use std::future::Future;
use std::pin::Pin;

use chrono::{DateTime, Utc};

use crate::domain::admin::entities::{AdminSession, AdminSessionId, AdminUser, AdminUserId};
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
}
