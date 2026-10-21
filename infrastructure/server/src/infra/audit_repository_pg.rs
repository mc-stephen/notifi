//! sqlx implementation of [`AuditStore`] (PostgreSQL).
//!
//! Append-only: entries are inserted and never updated or deleted.

use chrono::{DateTime, Utc};
use sqlx::PgPool;

use crate::domain::audit::entities::{ActorType, AuditEntry};
use crate::ports::audit_store::{AdminAuditFilters, AuditFilters, AuditStore, BoxFut};
use crate::ports::auth_store::StoreError;

pub struct PgAuditStore {
    pool: PgPool,
}

impl PgAuditStore {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

fn map_err(err: sqlx::Error) -> StoreError {
    StoreError::Storage(err.to_string())
}

#[derive(sqlx::FromRow)]
struct AuditRow {
    id: String,
    user_id: Option<String>,
    admin_id: Option<String>,
    actor_type: String,
    actor_name: Option<String>,
    event_type: String,
    message: String,
    project_id: Option<String>,
    metadata: Option<serde_json::Value>,
    occurred_at: DateTime<Utc>,
    total_count: Option<i64>,
}

impl From<AuditRow> for AuditEntry {
    fn from(row: AuditRow) -> Self {
        Self {
            id: row.id,
            user_id: row.user_id,
            admin_id: row.admin_id,
            actor_type: row.actor_type.parse::<ActorType>().unwrap_or_default(),
            actor_name: row.actor_name,
            event_type: row.event_type,
            message: row.message,
            project_id: row.project_id,
            metadata: row.metadata,
            occurred_at: row.occurred_at,
        }
    }
}

impl AuditStore for PgAuditStore {
    fn record(&self, entry: &AuditEntry) -> BoxFut<'_, Result<(), StoreError>> {
        let pool = self.pool.clone();
        let entry = entry.clone();
        Box::pin(async move {
            sqlx::query(
                "INSERT INTO audit_logs (id, user_id, admin_id, actor_type, actor_name, event_type,
                                         message, project_id, metadata, occurred_at)
                 VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)",
            )
            .bind(&entry.id)
            .bind(&entry.user_id)
            .bind(&entry.admin_id)
            .bind(entry.actor_type.as_str())
            .bind(&entry.actor_name)
            .bind(&entry.event_type)
            .bind(&entry.message)
            .bind(&entry.project_id)
            .bind(&entry.metadata)
            .bind(entry.occurred_at)
            .execute(&pool)
            .await
            .map_err(map_err)?;
            Ok(())
        })
    }

    fn list(
        &self,
        user_id: &str,
        filters: AuditFilters<'_>,
        limit: i64,
        before_id: Option<&str>,
    ) -> BoxFut<'_, Result<Vec<AuditEntry>, StoreError>> {
        let pool = self.pool.clone();
        let user_id = user_id.to_string();
        let project_id = filters.project_id.map(str::to_owned);
        let event_type = filters.event_type.map(str::to_owned);
        let before_id = before_id.map(str::to_owned);
        Box::pin(async move {
            let rows = sqlx::query_as::<_, AuditRow>(
                "SELECT id, user_id, admin_id, actor_type, actor_name, event_type, message,
                        project_id, metadata, occurred_at, NULL::bigint AS total_count
                 FROM audit_logs al
                 WHERE (al.user_id = $1
                        OR al.project_id IS NOT NULL
                           AND al.project_id IN (
                               SELECT pm.project_id
                               FROM platform_project_members pm
                               WHERE pm.user_id = $1 AND pm.deleted_at IS NULL
                           ))
                   AND ($2::text IS NULL OR al.event_type = $2)
                   AND ($3::text IS NULL OR al.project_id = $3)
                   AND ($4::text IS NULL OR al.id < $4)
                 ORDER BY al.occurred_at DESC, al.id DESC
                 LIMIT $5",
            )
            .bind(&user_id)
            .bind(&event_type)
            .bind(&project_id)
            .bind(&before_id)
            .bind(limit)
            .fetch_all(&pool)
            .await
            .map_err(map_err)?;

            Ok(rows.into_iter().map(AuditEntry::from).collect())
        })
    }

    fn list_all(
        &self,
        filters: AdminAuditFilters<'_>,
        limit: i64,
        offset: i64,
    ) -> BoxFut<'_, Result<(Vec<AuditEntry>, i64), StoreError>> {
        let pool = self.pool.clone();
        let event_type = filters.event_type.map(str::to_owned);
        let project_id = filters.project_id.map(str::to_owned);
        let actor_type = filters.actor_type.map(str::to_owned);
        let actor_id = filters.actor_id.map(str::to_owned);
        let search = filters.search.map(str::to_owned);
        Box::pin(async move {
            let rows = sqlx::query_as::<_, AuditRow>(
                "SELECT id, user_id, admin_id, actor_type, actor_name, event_type, message,
                        project_id, metadata, occurred_at, COUNT(*) OVER() AS total_count
                 FROM audit_logs
                 WHERE ($1::text IS NULL OR event_type = $1)
                   AND ($2::text IS NULL OR project_id = $2)
                   AND ($3::text IS NULL OR actor_type = $3)
                   AND ($4::text IS NULL OR user_id = $4 OR admin_id = $4)
                   AND ($5::text IS NULL
                        OR message ILIKE '%' || $5 || '%'
                        OR event_type ILIKE '%' || $5 || '%'
                        OR COALESCE(actor_name, '') ILIKE '%' || $5 || '%')
                 ORDER BY occurred_at DESC, id DESC
                 LIMIT $6 OFFSET $7",
            )
            .bind(&event_type)
            .bind(&project_id)
            .bind(&actor_type)
            .bind(&actor_id)
            .bind(&search)
            .bind(limit)
            .bind(offset)
            .fetch_all(&pool)
            .await
            .map_err(map_err)?;
            let total = rows.first().and_then(|r| r.total_count).unwrap_or(0);
            Ok((rows.into_iter().map(AuditEntry::from).collect(), total))
        })
    }
}
