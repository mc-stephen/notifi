//! sqlx implementation of the [`AdminStore`] port (PostgreSQL).

use chrono::{DateTime, Utc};
use sqlx::PgPool;

use crate::domain::admin::entities::{
    AdminApprovalRequest, AdminPasswordResetToken, AdminPasswordResetTokenId, AdminSession,
    AdminSessionId, AdminStatus, AdminUser, AdminUserId, ApprovalKind, ApprovalStatus,
};
use crate::domain::auth::value_objects::Email;
use crate::ports::admin_store::AdminStore;
use crate::ports::auth_store::{BoxFut, StoreError};

/// PostgreSQL-backed admin store.
pub struct PgAdminStore {
    pool: PgPool,
}

impl PgAdminStore {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[derive(sqlx::FromRow)]
struct AdminUserRow {
    id: String,
    name: String,
    email: String,
    password_hash: String,
    totp_secret: Option<String>,
    totp_enabled: bool,
    is_super_admin: bool,
    status: String,
    last_login_at: Option<DateTime<Utc>>,
    created_at: DateTime<Utc>,
}

impl TryFrom<AdminUserRow> for AdminUser {
    type Error = StoreError;

    fn try_from(row: AdminUserRow) -> Result<Self, Self::Error> {
        use std::str::FromStr;
        Ok(Self {
            id: AdminUserId::from_str(&row.id)
                .map_err(|e| StoreError::Storage(format!("invalid admin id in db: {e}")))?,
            name: row.name,
            email: Email::parse(&row.email)
                .map_err(|e| StoreError::Storage(format!("invalid email in db: {e}")))?,
            password_hash: row.password_hash,
            totp_secret: row.totp_secret,
            totp_enabled: row.totp_enabled,
            is_super_admin: row.is_super_admin,
            status: row
                .status
                .parse::<crate::domain::admin::entities::AdminStatus>()
                .unwrap_or(crate::domain::admin::entities::AdminStatus::Active),
            last_login_at: row.last_login_at,
            created_at: row.created_at,
        })
    }
}

#[derive(sqlx::FromRow)]
struct ApprovalRow {
    id: String,
    kind: String,
    target_admin_id: String,
    requested_by: String,
    status: String,
    decided_by: Option<String>,
    decided_at: Option<DateTime<Utc>>,
    created_at: DateTime<Utc>,
}

impl TryFrom<ApprovalRow> for AdminApprovalRequest {
    type Error = StoreError;

    fn try_from(row: ApprovalRow) -> Result<Self, Self::Error> {
        use std::str::FromStr;
        let parse_id = |raw: &str, what: &str| {
            AdminUserId::from_str(raw)
                .map_err(|e| StoreError::Storage(format!("invalid {what} in db: {e}")))
        };
        Ok(Self {
            id: row.id,
            kind: match row.kind.as_str() {
                "create" => ApprovalKind::Create,
                "remove" => ApprovalKind::Remove,
                other => {
                    return Err(StoreError::Storage(format!(
                        "invalid approval kind in db: {other}"
                    )))
                }
            },
            target_admin_id: parse_id(&row.target_admin_id, "target admin id")?,
            requested_by: parse_id(&row.requested_by, "requester admin id")?,
            status: match row.status.as_str() {
                "pending" => ApprovalStatus::Pending,
                "approved" => ApprovalStatus::Approved,
                "rejected" => ApprovalStatus::Rejected,
                other => {
                    return Err(StoreError::Storage(format!(
                        "invalid approval status in db: {other}"
                    )))
                }
            },
            decided_by: row
                .decided_by
                .map(|raw| parse_id(&raw, "decider admin id"))
                .transpose()?,
            decided_at: row.decided_at,
            created_at: row.created_at,
        })
    }
}

#[derive(sqlx::FromRow)]
struct AdminSessionRow {    id: String,
    admin_id: String,
    token_hash: String,
    expires_at: DateTime<Utc>,
    revoked_at: Option<DateTime<Utc>>,
    created_at: DateTime<Utc>,
}

impl TryFrom<AdminSessionRow> for AdminSession {
    type Error = StoreError;

    fn try_from(row: AdminSessionRow) -> Result<Self, Self::Error> {
        use std::str::FromStr;
        Ok(Self {
            id: AdminSessionId::from_str(&row.id)
                .map_err(|e| StoreError::Storage(format!("invalid admin session id in db: {e}")))?,
            admin_id: AdminUserId::from_str(&row.admin_id)
                .map_err(|e| StoreError::Storage(format!("invalid admin id in db: {e}")))?,
            token_hash: row.token_hash,
            expires_at: row.expires_at,
            revoked_at: row.revoked_at,
            created_at: row.created_at,
        })
    }
}

#[derive(sqlx::FromRow)]
struct AdminPasswordResetTokenRow {
    id: String,
    admin_id: String,
    token_hash: String,
    expires_at: DateTime<Utc>,
    consumed_at: Option<DateTime<Utc>>,
    created_at: DateTime<Utc>,
}

impl TryFrom<AdminPasswordResetTokenRow> for AdminPasswordResetToken {
    type Error = StoreError;

    fn try_from(row: AdminPasswordResetTokenRow) -> Result<Self, Self::Error> {
        use std::str::FromStr;
        Ok(Self {
            id: AdminPasswordResetTokenId::from_str(&row.id)
                .map_err(|e| StoreError::Storage(format!("invalid reset token id in db: {e}")))?,
            admin_id: AdminUserId::from_str(&row.admin_id)
                .map_err(|e| StoreError::Storage(format!("invalid admin id in db: {e}")))?,
            token_hash: row.token_hash,
            expires_at: row.expires_at,
            consumed_at: row.consumed_at,
            created_at: row.created_at,
        })
    }
}

fn map_err(e: sqlx::Error) -> StoreError {
    StoreError::Storage(e.to_string())
}

impl AdminStore for PgAdminStore {
    fn admin_exists(&self) -> BoxFut<'_, Result<bool, StoreError>> {
        let pool = self.pool.clone();
        Box::pin(async move {
            let has = sqlx::query_scalar::<_, bool>(
                "SELECT EXISTS(SELECT 1 FROM admin_users WHERE deleted_at IS NULL)",
            )
            .fetch_one(&pool)
            .await
            .map_err(map_err)?;
            Ok(has)
        })
    }

    fn create_admin(&self, admin: &AdminUser) -> BoxFut<'_, Result<(), StoreError>> {
        let pool = self.pool.clone();
        let id = admin.id.to_string();
        let name = admin.name.clone();
        let email = admin.email.as_str().to_string();
        let password_hash = admin.password_hash.clone();
        let is_super_admin = admin.is_super_admin;
        let status = admin.status.as_str().to_string();
        let created_at = admin.created_at;
        Box::pin(async move {
            sqlx::query(
                "INSERT INTO admin_users (id, name, email, password_hash, is_super_admin, status, created_at, updated_at) \
                 VALUES ($1, $2, $3, $4, $5, $6, $7, $7)",
            )
            .bind(&id)
            .bind(&name)
            .bind(&email)
            .bind(&password_hash)
            .bind(is_super_admin)
            .bind(&status)
            .bind(created_at)
            .execute(&pool)
            .await
            .map_err(map_err)?;
            Ok(())
        })
    }

    fn find_admin_by_email(&self, email: &str) -> BoxFut<'_, Result<Option<AdminUser>, StoreError>> {
        let pool = self.pool.clone();
        let email = email.to_string();
        Box::pin(async move {
            let row = sqlx::query_as::<_, AdminUserRow>(
                "SELECT id, name, email, password_hash, totp_secret, totp_enabled, is_super_admin, status, last_login_at, created_at \
                 FROM admin_users WHERE email = $1 AND deleted_at IS NULL",
            )
            .bind(&email)
            .fetch_optional(&pool)
            .await
            .map_err(map_err)?;
            row.map(AdminUser::try_from).transpose()
        })
    }

    fn find_admin_by_id(&self, id: AdminUserId) -> BoxFut<'_, Result<Option<AdminUser>, StoreError>> {
        let pool = self.pool.clone();
        let id_str = id.to_string();
        Box::pin(async move {
            let row = sqlx::query_as::<_, AdminUserRow>(
                "SELECT id, name, email, password_hash, totp_secret, totp_enabled, is_super_admin, status, last_login_at, created_at \
                 FROM admin_users WHERE id = $1 AND deleted_at IS NULL",
            )
            .bind(&id_str)
            .fetch_optional(&pool)
            .await
            .map_err(map_err)?;
            row.map(AdminUser::try_from).transpose()
        })
    }

    fn set_totp_secret(&self, id: AdminUserId, secret: String) -> BoxFut<'_, Result<(), StoreError>> {
        let pool = self.pool.clone();
        let id_str = id.to_string();
        Box::pin(async move {
            sqlx::query(
                "UPDATE admin_users SET totp_secret = $1, updated_at = now() \
                 WHERE id = $2 AND deleted_at IS NULL",
            )
            .bind(&secret)
            .bind(&id_str)
            .execute(&pool)
            .await
            .map_err(map_err)?;
            Ok(())
        })
    }

    fn enable_totp(&self, id: AdminUserId) -> BoxFut<'_, Result<(), StoreError>> {
        let pool = self.pool.clone();
        let id_str = id.to_string();
        Box::pin(async move {
            sqlx::query(
                "UPDATE admin_users SET totp_enabled = TRUE, updated_at = now() \
                 WHERE id = $1 AND deleted_at IS NULL",
            )
            .bind(&id_str)
            .execute(&pool)
            .await
            .map_err(map_err)?;
            Ok(())
        })
    }

    fn touch_admin_last_login(&self, id: AdminUserId, at: DateTime<Utc>) -> BoxFut<'_, Result<(), StoreError>> {
        let pool = self.pool.clone();
        let id_str = id.to_string();
        Box::pin(async move {
            sqlx::query(
                "UPDATE admin_users SET last_login_at = $1, updated_at = now() \
                 WHERE id = $2 AND deleted_at IS NULL",
            )
            .bind(at)
            .bind(&id_str)
            .execute(&pool)
            .await
            .map_err(map_err)?;
            Ok(())
        })
    }

    fn create_admin_session(&self, session: &AdminSession) -> BoxFut<'_, Result<(), StoreError>> {
        let pool = self.pool.clone();
        let id = session.id.to_string();
        let admin_id = session.admin_id.to_string();
        let token_hash = session.token_hash.clone();
        let expires_at = session.expires_at;
        let created_at = session.created_at;
        Box::pin(async move {
            sqlx::query(
                "INSERT INTO admin_sessions (id, admin_id, token_hash, expires_at, created_at) \
                 VALUES ($1, $2, $3, $4, $5)",
            )
            .bind(&id)
            .bind(&admin_id)
            .bind(&token_hash)
            .bind(expires_at)
            .bind(created_at)
            .execute(&pool)
            .await
            .map_err(map_err)?;
            Ok(())
        })
    }

    fn find_admin_session_by_hash(
        &self,
        hash: &str,
    ) -> BoxFut<'_, Result<Option<AdminSession>, StoreError>> {
        let pool = self.pool.clone();
        let hash = hash.to_string();
        Box::pin(async move {
            let row = sqlx::query_as::<_, AdminSessionRow>(
                "SELECT id, admin_id, token_hash, expires_at, revoked_at, created_at \
                 FROM admin_sessions WHERE token_hash = $1",
            )
            .bind(&hash)
            .fetch_optional(&pool)
            .await
            .map_err(map_err)?;
            row.map(AdminSession::try_from).transpose()
        })
    }

    fn revoke_admin_sessions(&self, id: AdminUserId) -> BoxFut<'_, Result<(), StoreError>> {
        let pool = self.pool.clone();
        let id_str = id.to_string();
        Box::pin(async move {
            sqlx::query(
                "UPDATE admin_sessions SET revoked_at = now() \
                 WHERE admin_id = $1 AND revoked_at IS NULL",
            )
            .bind(&id_str)
            .execute(&pool)
            .await
            .map_err(map_err)?;
            Ok(())
        })
    }

    fn revoke_admin_session(&self, id: AdminSessionId) -> BoxFut<'_, Result<(), StoreError>> {
        let pool = self.pool.clone();
        let id_str = id.to_string();
        Box::pin(async move {
            sqlx::query(
                "UPDATE admin_sessions SET revoked_at = now() \
                 WHERE id = $1 AND revoked_at IS NULL",
            )
            .bind(&id_str)
            .execute(&pool)
            .await
            .map_err(map_err)?;
            Ok(())
        })
    }

    fn update_admin_password(
        &self,
        id: AdminUserId,
        password_hash: String,
    ) -> BoxFut<'_, Result<(), StoreError>> {
        let pool = self.pool.clone();
        let id_str = id.to_string();
        Box::pin(async move {
            sqlx::query(
                "UPDATE admin_users SET password_hash = $1, updated_at = now() \
                 WHERE id = $2 AND deleted_at IS NULL",
            )
            .bind(&password_hash)
            .bind(&id_str)
            .execute(&pool)
            .await
            .map_err(map_err)?;
            Ok(())
        })
    }

    fn create_admin_reset_token(
        &self,
        token: &AdminPasswordResetToken,
    ) -> BoxFut<'_, Result<(), StoreError>> {
        let pool = self.pool.clone();
        let id = token.id.to_string();
        let admin_id = token.admin_id.to_string();
        let token_hash = token.token_hash.clone();
        let expires_at = token.expires_at;
        let created_at = token.created_at;
        Box::pin(async move {
            sqlx::query(
                "INSERT INTO admin_password_reset_tokens (id, admin_id, token_hash, expires_at, created_at) \
                 VALUES ($1, $2, $3, $4, $5)",
            )
            .bind(&id)
            .bind(&admin_id)
            .bind(&token_hash)
            .bind(expires_at)
            .bind(created_at)
            .execute(&pool)
            .await
            .map_err(map_err)?;
            Ok(())
        })
    }

    fn consume_admin_reset_tokens_for_admin(
        &self,
        id: AdminUserId,
    ) -> BoxFut<'_, Result<(), StoreError>> {
        let pool = self.pool.clone();
        let id_str = id.to_string();
        Box::pin(async move {
            sqlx::query(
                "UPDATE admin_password_reset_tokens SET consumed_at = now() \
                 WHERE admin_id = $1 AND consumed_at IS NULL",
            )
            .bind(&id_str)
            .execute(&pool)
            .await
            .map_err(map_err)?;
            Ok(())
        })
    }

    fn find_admin_reset_token_by_hash(
        &self,
        hash: &str,
    ) -> BoxFut<'_, Result<Option<AdminPasswordResetToken>, StoreError>> {
        let pool = self.pool.clone();
        let hash = hash.to_string();
        Box::pin(async move {
            let row = sqlx::query_as::<_, AdminPasswordResetTokenRow>(
                "SELECT id, admin_id, token_hash, expires_at, consumed_at, created_at \
                 FROM admin_password_reset_tokens WHERE token_hash = $1",
            )
            .bind(&hash)
            .fetch_optional(&pool)
            .await
            .map_err(map_err)?;
            row.map(AdminPasswordResetToken::try_from).transpose()
        })
    }

    fn consume_admin_reset_token(
        &self,
        id: AdminPasswordResetTokenId,
    ) -> BoxFut<'_, Result<(), StoreError>> {
        let pool = self.pool.clone();
        let id_str = id.to_string();
        Box::pin(async move {
            sqlx::query(
                "UPDATE admin_password_reset_tokens SET consumed_at = now() \
                 WHERE id = $1 AND consumed_at IS NULL",
            )
            .bind(&id_str)
            .execute(&pool)
            .await
            .map_err(map_err)?;
            Ok(())
        })
    }

    fn list_admins(&self) -> BoxFut<'_, Result<Vec<AdminUser>, StoreError>> {
        let pool = self.pool.clone();
        Box::pin(async move {
            let rows = sqlx::query_as::<_, AdminUserRow>(
                "SELECT id, name, email, password_hash, totp_secret, totp_enabled, is_super_admin, status, last_login_at, created_at \
                 FROM admin_users WHERE deleted_at IS NULL \
                 ORDER BY created_at ASC, id ASC",
            )
            .fetch_all(&pool)
            .await
            .map_err(map_err)?;
            rows.into_iter().map(AdminUser::try_from).collect()
        })
    }

    fn set_admin_status(
        &self,
        id: AdminUserId,
        status: AdminStatus,
    ) -> BoxFut<'_, Result<bool, StoreError>> {
        let pool = self.pool.clone();
        let id_str = id.to_string();
        let status_str = status.as_str().to_string();
        Box::pin(async move {
            let result = sqlx::query(
                "UPDATE admin_users SET status = $2, updated_at = now() \
                 WHERE id = $1 AND deleted_at IS NULL",
            )
            .bind(&id_str)
            .bind(&status_str)
            .execute(&pool)
            .await
            .map_err(map_err)?;
            Ok(result.rows_affected() > 0)
        })
    }

    fn soft_delete_admin(&self, id: AdminUserId) -> BoxFut<'_, Result<bool, StoreError>> {
        let pool = self.pool.clone();
        let id_str = id.to_string();
        Box::pin(async move {
            let result = sqlx::query(
                "UPDATE admin_users SET deleted_at = now(), updated_at = now() \
                 WHERE id = $1 AND deleted_at IS NULL",
            )
            .bind(&id_str)
            .execute(&pool)
            .await
            .map_err(map_err)?;
            Ok(result.rows_affected() > 0)
        })
    }

    fn create_approval(
        &self,
        request: &AdminApprovalRequest,
    ) -> BoxFut<'_, Result<(), StoreError>> {
        let pool = self.pool.clone();
        let id = request.id.clone();
        let kind = request.kind.as_str().to_string();
        let target = request.target_admin_id.to_string();
        let requested_by = request.requested_by.to_string();
        let status = request.status.as_str().to_string();
        let created_at = request.created_at;
        Box::pin(async move {
            sqlx::query(
                "INSERT INTO admin_approval_requests (id, kind, target_admin_id, requested_by, status, created_at) \
                 VALUES ($1, $2, $3, $4, $5, $6)",
            )
            .bind(&id)
            .bind(&kind)
            .bind(&target)
            .bind(&requested_by)
            .bind(&status)
            .bind(created_at)
            .execute(&pool)
            .await
            .map_err(map_err)?;
            Ok(())
        })
    }

    fn find_approval(&self, id: &str) -> BoxFut<'_, Result<Option<AdminApprovalRequest>, StoreError>> {
        let pool = self.pool.clone();
        let id = id.to_string();
        Box::pin(async move {
            let row = sqlx::query_as::<_, ApprovalRow>(
                "SELECT id, kind, target_admin_id, requested_by, status, decided_by, decided_at, created_at \
                 FROM admin_approval_requests WHERE id = $1",
            )
            .bind(&id)
            .fetch_optional(&pool)
            .await
            .map_err(map_err)?;
            row.map(AdminApprovalRequest::try_from).transpose()
        })
    }

    fn list_approvals(
        &self,
        status: Option<&str>,
    ) -> BoxFut<'_, Result<Vec<AdminApprovalRequest>, StoreError>> {
        let pool = self.pool.clone();
        let status_owned = status.map(str::to_owned);
        Box::pin(async move {
            let rows = sqlx::query_as::<_, ApprovalRow>(
                "SELECT id, kind, target_admin_id, requested_by, status, decided_by, decided_at, created_at \
                 FROM admin_approval_requests \
                 WHERE ($1::text IS NULL OR status = $1) \
                 ORDER BY created_at DESC, id DESC",
            )
            .bind(&status_owned)
            .fetch_all(&pool)
            .await
            .map_err(map_err)?;
            rows.into_iter()
                .map(AdminApprovalRequest::try_from)
                .collect()
        })
    }

    fn decide_approval(
        &self,
        id: &str,
        approved: bool,
        decided_by: AdminUserId,
    ) -> BoxFut<'_, Result<bool, StoreError>> {
        let pool = self.pool.clone();
        let id = id.to_string();
        let decided_by_str = decided_by.to_string();
        let status = if approved { "approved" } else { "rejected" };
        Box::pin(async move {
            let result = sqlx::query(
                "UPDATE admin_approval_requests \
                 SET status = $2, decided_by = $3, decided_at = now() \
                 WHERE id = $1 AND status = 'pending'",
            )
            .bind(&id)
            .bind(status)
            .bind(&decided_by_str)
            .execute(&pool)
            .await
            .map_err(map_err)?;
            Ok(result.rows_affected() > 0)
        })
    }
}
