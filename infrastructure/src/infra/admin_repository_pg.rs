//! sqlx implementation of the [`AdminStore`] port (PostgreSQL).

use chrono::{DateTime, Utc};
use sqlx::PgPool;

use crate::domain::admin::entities::{
    AdminPasswordResetToken, AdminPasswordResetTokenId, AdminSession, AdminSessionId, AdminStatus,
    AdminUser, AdminUserId,
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
    total_count: Option<i64>,
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
struct AdminSessionRow {
    id: String,
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

    fn find_admin_by_email(
        &self,
        email: &str,
    ) -> BoxFut<'_, Result<Option<AdminUser>, StoreError>> {
        let pool = self.pool.clone();
        let email = email.to_string();
        Box::pin(async move {
            let row = sqlx::query_as::<_, AdminUserRow>(
                "SELECT id, name, email, password_hash, totp_secret, totp_enabled, is_super_admin, status, last_login_at, created_at, \
                        NULL::bigint AS total_count \
                 FROM admin_users WHERE email = $1 AND deleted_at IS NULL",
            )
            .bind(&email)
            .fetch_optional(&pool)
            .await
            .map_err(map_err)?;
            row.map(AdminUser::try_from).transpose()
        })
    }

    fn find_admin_by_id(
        &self,
        id: AdminUserId,
    ) -> BoxFut<'_, Result<Option<AdminUser>, StoreError>> {
        let pool = self.pool.clone();
        let id_str = id.to_string();
        Box::pin(async move {
            let row = sqlx::query_as::<_, AdminUserRow>(
                "SELECT id, name, email, password_hash, totp_secret, totp_enabled, is_super_admin, status, last_login_at, created_at, \
                        NULL::bigint AS total_count \
                 FROM admin_users WHERE id = $1 AND deleted_at IS NULL",
            )
            .bind(&id_str)
            .fetch_optional(&pool)
            .await
            .map_err(map_err)?;
            row.map(AdminUser::try_from).transpose()
        })
    }

    fn set_totp_secret(
        &self,
        id: AdminUserId,
        secret: String,
    ) -> BoxFut<'_, Result<(), StoreError>> {
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

    fn touch_admin_last_login(
        &self,
        id: AdminUserId,
        at: DateTime<Utc>,
    ) -> BoxFut<'_, Result<(), StoreError>> {
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

    fn list_admins(
        &self,
        limit: i64,
        offset: i64,
    ) -> BoxFut<'_, Result<(Vec<AdminUser>, i64), StoreError>> {
        let pool = self.pool.clone();
        Box::pin(async move {
            let rows = sqlx::query_as::<_, AdminUserRow>(
                "SELECT id, name, email, password_hash, totp_secret, totp_enabled, is_super_admin, status, last_login_at, created_at, \
                        COUNT(*) OVER() AS total_count \
                 FROM admin_users WHERE deleted_at IS NULL \
                 ORDER BY created_at ASC, id ASC \
                 LIMIT $1 OFFSET $2",
            )
            .bind(limit)
            .bind(offset)
            .fetch_all(&pool)
            .await
            .map_err(map_err)?;
            let total = rows.first().and_then(|r| r.total_count).unwrap_or(0);
            let admins = rows
                .into_iter()
                .map(AdminUser::try_from)
                .collect::<Result<Vec<_>, _>>()?;
            Ok((admins, total))
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
}
