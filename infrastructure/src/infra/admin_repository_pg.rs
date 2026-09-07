//! sqlx implementation of the [`AdminStore`] port (PostgreSQL).

use chrono::{DateTime, Utc};
use sqlx::PgPool;

use crate::domain::admin::entities::{AdminSession, AdminSessionId, AdminUser, AdminUserId};
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
        let created_at = admin.created_at;
        Box::pin(async move {
            sqlx::query(
                "INSERT INTO admin_users (id, name, email, password_hash, created_at, updated_at) \
                 VALUES ($1, $2, $3, $4, $5, $5)",
            )
            .bind(&id)
            .bind(&name)
            .bind(&email)
            .bind(&password_hash)
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
                "SELECT id, name, email, password_hash, totp_secret, totp_enabled, last_login_at, created_at \
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
                "SELECT id, name, email, password_hash, totp_secret, totp_enabled, last_login_at, created_at \
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
}
