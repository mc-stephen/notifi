//! Postgres backing for project API keys + per-key IP allowlists.
//!
//! Visibility mirrors the recipients slice: the caller owns the project or
//! is an active member, otherwise rows read as missing (no enumeration).

use chrono::{DateTime, Utc};
use sqlx::PgPool;
use ulid::Ulid;

use crate::domain::auth::entities::UserId;
use crate::ports::api_keys_store::{ApiKeyRecord, ApiKeysStore, BoxFut, IpAllowEntry};
use crate::ports::auth_store::StoreError;

/// Maps driver errors; unique violations become [`StoreError::Conflict`].
fn map_err(err: sqlx::Error) -> StoreError {
    if let sqlx::Error::Database(db) = &err
        && let Some(code) = db.code()
        && code == "23505"
    {
        return StoreError::Conflict;
    }
    StoreError::Storage(err.to_string())
}

pub struct PgApiKeysStore {
    pool: PgPool,
}

impl PgApiKeysStore {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[derive(sqlx::FromRow)]
struct ApiKeyRow {
    id: String,
    project_id: String,
    name: String,
    prefix: String,
    key_hash: String,
    environment: String,
    permissions: Vec<String>,
    expires_at: Option<DateTime<Utc>>,
    last_used_at: Option<DateTime<Utc>>,
    enabled: bool,
    revoked_at: Option<DateTime<Utc>>,
    created_at: DateTime<Utc>,
}

impl From<ApiKeyRow> for ApiKeyRecord {
    fn from(row: ApiKeyRow) -> Self {
        Self {
            id: row.id,
            project_id: row.project_id,
            name: row.name,
            prefix: row.prefix,
            key_hash: row.key_hash,
            environment: row.environment,
            permissions: row.permissions,
            expires_at: row.expires_at,
            last_used_at: row.last_used_at,
            enabled: row.enabled,
            revoked_at: row.revoked_at,
            created_at: row.created_at,
        }
    }
}

#[derive(sqlx::FromRow)]
struct IpAllowRow {
    id: String,
    key_id: String,
    cidr: String,
    label: String,
    created_at: DateTime<Utc>,
}

impl From<IpAllowRow> for IpAllowEntry {
    fn from(row: IpAllowRow) -> Self {
        Self {
            id: row.id,
            key_id: row.key_id,
            cidr: row.cidr,
            label: row.label,
            created_at: row.created_at,
        }
    }
}

const KEY_COLUMNS: &str = "id, project_id, name, prefix, key_hash, environment, permissions, expires_at, last_used_at, enabled, revoked_at, created_at";

/// Scopes a key row to projects the actor can see; invisible rows read as missing.
fn visible_key(actor: &str, project_id: &str, key_id: Option<&str>) -> (String, Vec<String>) {
    let mut sql = format!(
        "SELECT {KEY_COLUMNS} FROM auth_api_keys k
         WHERE k.project_id = $2 AND k.deleted_at IS NULL
           AND EXISTS (
               SELECT 1 FROM platform_projects p
               WHERE p.id = k.project_id AND p.deleted_at IS NULL
                 AND (p.created_by = $1 OR EXISTS (
                     SELECT 1 FROM platform_project_members pm
                     WHERE pm.project_id = p.id AND pm.user_id = $1 AND pm.deleted_at IS NULL
                 ))
           )"
    );
    let mut binds = vec![actor.to_string(), project_id.to_string()];
    if let Some(key_id) = key_id {
        sql.push_str(" AND k.id = $3");
        binds.push(key_id.to_string());
    }
    (sql, binds)
}

impl ApiKeysStore for PgApiKeysStore {
    #[allow(clippy::too_many_arguments)]
    fn create_key(
        &self,
        actor: UserId,
        project_id: &str,
        name: &str,
        environment: &str,
        permissions: &[String],
        expires_at: Option<DateTime<Utc>>,
        prefix: &str,
        key_hash: &str,
    ) -> BoxFut<'_, Result<ApiKeyRecord, StoreError>> {
        let pool = self.pool.clone();
        let actor = actor.to_string();
        let project_id = project_id.to_string();
        let name = name.to_string();
        let environment = environment.to_string();
        let permissions = permissions.to_vec();
        let prefix = prefix.to_string();
        let key_hash = key_hash.to_string();
        Box::pin(async move {
            let row = sqlx::query_as::<_, ApiKeyRow>(&format!(
                "INSERT INTO auth_api_keys (id, project_id, name, prefix, key_hash, environment, permissions, expires_at)
                 SELECT $2, $3, $4, $5, $6, $7, $8, $9
                 WHERE EXISTS (
                     SELECT 1 FROM platform_projects p
                     WHERE p.id = $3 AND p.deleted_at IS NULL
                       AND (p.created_by = $1 OR EXISTS (
                           SELECT 1 FROM platform_project_members pm
                           WHERE pm.project_id = p.id AND pm.user_id = $1 AND pm.deleted_at IS NULL
                       ))
                 )
                 RETURNING {KEY_COLUMNS}"
            ))
            .bind(&actor)
            .bind(Ulid::new().to_string())
            .bind(&project_id)
            .bind(&name)
            .bind(&prefix)
            .bind(&key_hash)
            .bind(&environment)
            .bind(&permissions)
            .bind(expires_at)
            .fetch_optional(&pool)
            .await
            .map_err(map_err)?;
            row.map(ApiKeyRecord::from).ok_or_else(|| {
                // Ambiguous by design is wrong here: conflicts surface as
                // 23505 above, so a missing row means invisibility.
                StoreError::Storage("project not found or not visible".to_string())
            })
        })
    }

    fn list_keys(
        &self,
        actor: UserId,
        project_id: &str,
    ) -> BoxFut<'_, Result<Vec<ApiKeyRecord>, StoreError>> {
        let pool = self.pool.clone();
        let (sql, binds) = visible_key(&actor.to_string(), project_id, None);
        Box::pin(async move {
            let sql = format!("{sql} ORDER BY k.created_at DESC");
            let mut query = sqlx::query_as::<_, ApiKeyRow>(&sql);
            for bind in &binds {
                query = query.bind(bind);
            }
            Ok(query
                .fetch_all(&pool)
                .await
                .map_err(map_err)?
                .into_iter()
                .map(ApiKeyRecord::from)
                .collect())
        })
    }

    fn get_key(
        &self,
        actor: UserId,
        project_id: &str,
        key_id: &str,
    ) -> BoxFut<'_, Result<Option<ApiKeyRecord>, StoreError>> {
        let pool = self.pool.clone();
        let (sql, binds) = visible_key(&actor.to_string(), project_id, Some(key_id));
        Box::pin(async move {
            let mut query = sqlx::query_as::<_, ApiKeyRow>(&sql);
            for bind in &binds {
                query = query.bind(bind);
            }
            Ok(query
                .fetch_optional(&pool)
                .await
                .map_err(map_err)?
                .map(ApiKeyRecord::from))
        })
    }

    fn rename_key(
        &self,
        actor: UserId,
        project_id: &str,
        key_id: &str,
        name: &str,
    ) -> BoxFut<'_, Result<bool, StoreError>> {
        let pool = self.pool.clone();
        let actor = actor.to_string();
        let project_id = project_id.to_string();
        let key_id = key_id.to_string();
        let name = name.to_string();
        Box::pin(async move {
            let result = sqlx::query(
                "UPDATE auth_api_keys k SET name = $4, updated_at = now()
                 WHERE k.id = $3 AND k.project_id = $2 AND k.deleted_at IS NULL
                   AND EXISTS (
                       SELECT 1 FROM platform_projects p
                       WHERE p.id = k.project_id AND p.deleted_at IS NULL
                         AND (p.created_by = $1 OR EXISTS (
                             SELECT 1 FROM platform_project_members pm
                             WHERE pm.project_id = p.id AND pm.user_id = $1 AND pm.deleted_at IS NULL
                         ))
                   )",
            )
            .bind(&actor)
            .bind(&project_id)
            .bind(&key_id)
            .bind(&name)
            .execute(&pool)
            .await
            .map_err(map_err)?;
            Ok(result.rows_affected() > 0)
        })
    }

    fn set_key_enabled(
        &self,
        actor: UserId,
        project_id: &str,
        key_id: &str,
        enabled: bool,
    ) -> BoxFut<'_, Result<bool, StoreError>> {
        let pool = self.pool.clone();
        let actor = actor.to_string();
        let project_id = project_id.to_string();
        let key_id = key_id.to_string();
        Box::pin(async move {
            let result = sqlx::query(
                "UPDATE auth_api_keys k SET enabled = $4, updated_at = now()
                 WHERE k.id = $3 AND k.project_id = $2 AND k.deleted_at IS NULL
                   AND EXISTS (
                       SELECT 1 FROM platform_projects p
                       WHERE p.id = k.project_id AND p.deleted_at IS NULL
                         AND (p.created_by = $1 OR EXISTS (
                             SELECT 1 FROM platform_project_members pm
                             WHERE pm.project_id = p.id AND pm.user_id = $1 AND pm.deleted_at IS NULL
                         ))
                   )",
            )
            .bind(&actor)
            .bind(&project_id)
            .bind(&key_id)
            .bind(enabled)
            .execute(&pool)
            .await
            .map_err(map_err)?;
            Ok(result.rows_affected() > 0)
        })
    }

    fn revoke_key(
        &self,
        actor: UserId,
        project_id: &str,
        key_id: &str,
    ) -> BoxFut<'_, Result<bool, StoreError>> {
        let pool = self.pool.clone();
        let actor = actor.to_string();
        let project_id = project_id.to_string();
        let key_id = key_id.to_string();
        Box::pin(async move {
            let result = sqlx::query(
                "UPDATE auth_api_keys k SET revoked_at = now(), updated_at = now()
                 WHERE k.id = $3 AND k.project_id = $2 AND k.deleted_at IS NULL
                   AND k.revoked_at IS NULL
                   AND EXISTS (
                       SELECT 1 FROM platform_projects p
                       WHERE p.id = k.project_id AND p.deleted_at IS NULL
                         AND (p.created_by = $1 OR EXISTS (
                             SELECT 1 FROM platform_project_members pm
                             WHERE pm.project_id = p.id AND pm.user_id = $1 AND pm.deleted_at IS NULL
                         ))
                   )",
            )
            .bind(&actor)
            .bind(&project_id)
            .bind(&key_id)
            .execute(&pool)
            .await
            .map_err(map_err)?;
            Ok(result.rows_affected() > 0)
        })
    }

    fn find_key_by_hash(
        &self,
        key_hash: &str,
    ) -> BoxFut<'_, Result<Option<ApiKeyRecord>, StoreError>> {
        let pool = self.pool.clone();
        let key_hash = key_hash.to_string();
        Box::pin(async move {
            Ok(sqlx::query_as::<_, ApiKeyRow>(&format!(
                "SELECT {KEY_COLUMNS} FROM auth_api_keys k WHERE k.key_hash = $1 AND k.deleted_at IS NULL"
            ))
            .bind(&key_hash)
            .fetch_optional(&pool)
            .await
            .map_err(map_err)?
            .map(ApiKeyRecord::from))
        })
    }

    fn touch_key_last_used(&self, key_id: &str) -> BoxFut<'_, Result<(), StoreError>> {
        let pool = self.pool.clone();
        let key_id = key_id.to_string();
        Box::pin(async move {
            sqlx::query("UPDATE auth_api_keys SET last_used_at = now() WHERE id = $1")
                .bind(&key_id)
                .execute(&pool)
                .await
                .map_err(map_err)?;
            Ok(())
        })
    }

    fn list_ips(
        &self,
        actor: UserId,
        project_id: &str,
        key_id: &str,
    ) -> BoxFut<'_, Result<Vec<IpAllowEntry>, StoreError>> {
        let pool = self.pool.clone();
        let actor = actor.to_string();
        let project_id = project_id.to_string();
        let key_id = key_id.to_string();
        Box::pin(async move {
            Ok(sqlx::query_as::<_, IpAllowRow>(
                "SELECT i.id, i.key_id, i.cidr, i.label, i.created_at
                 FROM key_ip_allowlist i
                 JOIN auth_api_keys k ON k.id = i.key_id
                 WHERE i.key_id = $3 AND k.project_id = $2 AND k.deleted_at IS NULL
                   AND EXISTS (
                       SELECT 1 FROM platform_projects p
                       WHERE p.id = k.project_id AND p.deleted_at IS NULL
                         AND (p.created_by = $1 OR EXISTS (
                             SELECT 1 FROM platform_project_members pm
                             WHERE pm.project_id = p.id AND pm.user_id = $1 AND pm.deleted_at IS NULL
                         ))
                   )
                 ORDER BY i.created_at ASC",
            )
            .bind(&actor)
            .bind(&project_id)
            .bind(&key_id)
            .fetch_all(&pool)
            .await
            .map_err(map_err)?
            .into_iter()
            .map(IpAllowEntry::from)
            .collect())
        })
    }

    fn list_ips_for_key(&self, key_id: &str) -> BoxFut<'_, Result<Vec<IpAllowEntry>, StoreError>> {
        let pool = self.pool.clone();
        let key_id = key_id.to_string();
        Box::pin(async move {
            Ok(sqlx::query_as::<_, IpAllowRow>(
                "SELECT id, key_id, cidr, label, created_at FROM key_ip_allowlist WHERE key_id = $1 ORDER BY created_at ASC",
            )
            .bind(&key_id)
            .fetch_all(&pool)
            .await
            .map_err(map_err)?
            .into_iter()
            .map(IpAllowEntry::from)
            .collect())
        })
    }

    fn add_ip(
        &self,
        actor: UserId,
        project_id: &str,
        key_id: &str,
        cidr: &str,
        label: &str,
    ) -> BoxFut<'_, Result<IpAllowEntry, StoreError>> {
        let pool = self.pool.clone();
        let actor = actor.to_string();
        let project_id = project_id.to_string();
        let key_id = key_id.to_string();
        let cidr = cidr.to_string();
        let label = label.to_string();
        Box::pin(async move {
            let row = sqlx::query_as::<_, IpAllowRow>(
                "INSERT INTO key_ip_allowlist (id, key_id, cidr, label)
                 SELECT $4, $3, $5, $6
                 WHERE EXISTS (
                     SELECT 1 FROM auth_api_keys k
                     WHERE k.id = $3 AND k.project_id = $2 AND k.deleted_at IS NULL
                       AND EXISTS (
                           SELECT 1 FROM platform_projects p
                           WHERE p.id = k.project_id AND p.deleted_at IS NULL
                             AND (p.created_by = $1 OR EXISTS (
                                 SELECT 1 FROM platform_project_members pm
                                 WHERE pm.project_id = p.id AND pm.user_id = $1 AND pm.deleted_at IS NULL
                             ))
                       )
                 )
                 RETURNING id, key_id, cidr, label, created_at",
            )
            .bind(&actor)
            .bind(&project_id)
            .bind(&key_id)
            .bind(Ulid::new().to_string())
            .bind(&cidr)
            .bind(&label)
            .fetch_optional(&pool)
            .await
            .map_err(map_err)?;
            row.map(IpAllowEntry::from)
                .ok_or_else(|| StoreError::Storage("API key not found".to_string()))
        })
    }

    fn remove_ip(
        &self,
        actor: UserId,
        project_id: &str,
        key_id: &str,
        entry_id: &str,
    ) -> BoxFut<'_, Result<bool, StoreError>> {
        let pool = self.pool.clone();
        let actor = actor.to_string();
        let project_id = project_id.to_string();
        let key_id = key_id.to_string();
        let entry_id = entry_id.to_string();
        Box::pin(async move {
            let result = sqlx::query(
                "DELETE FROM key_ip_allowlist i
                 USING auth_api_keys k
                 WHERE i.id = $4 AND i.key_id = $3 AND k.id = i.key_id
                   AND k.project_id = $2 AND k.deleted_at IS NULL
                   AND EXISTS (
                       SELECT 1 FROM platform_projects p
                       WHERE p.id = k.project_id AND p.deleted_at IS NULL
                         AND (p.created_by = $1 OR EXISTS (
                             SELECT 1 FROM platform_project_members pm
                             WHERE pm.project_id = p.id AND pm.user_id = $1 AND pm.deleted_at IS NULL
                         ))
                   )",
            )
            .bind(&actor)
            .bind(&project_id)
            .bind(&key_id)
            .bind(&entry_id)
            .execute(&pool)
            .await
            .map_err(map_err)?;
            Ok(result.rows_affected() > 0)
        })
    }
}
