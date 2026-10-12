use chrono::{DateTime, Utc};
use sqlx::PgPool;
use ulid::Ulid;

use crate::domain::auth::entities::UserId;
use crate::ports::auth_store::StoreError;
use crate::ports::flow_store::{BoxFut, FlowRecord, FlowStore};

pub struct PgFlowStore {
    pool: PgPool,
}

impl PgFlowStore {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

fn map_err(err: sqlx::Error) -> StoreError {
    if let sqlx::Error::Database(db) = &err
        && let Some(code) = db.code()
        && code == "23505"
    {
        return StoreError::Conflict;
    }
    StoreError::Storage(err.to_string())
}

#[derive(sqlx::FromRow)]
struct FlowRow {
    id: String,
    project_id: String,
    name: String,
    definition: serde_json::Value,
    version: i32,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

impl From<FlowRow> for FlowRecord {
    fn from(r: FlowRow) -> Self {
        Self {
            id: r.id,
            project_id: r.project_id,
            name: r.name,
            definition: r.definition,
            version: r.version,
            created_at: r.created_at,
            updated_at: r.updated_at,
        }
    }
}

const COLUMNS: &str = "id, project_id, name, definition, version, created_at, updated_at";

impl FlowStore for PgFlowStore {
    fn project_visible(
        &self,
        actor: UserId,
        project_id: &str,
    ) -> BoxFut<'_, Result<bool, StoreError>> {
        let pool = self.pool.clone();
        let actor = actor.to_string();
        let project_id = project_id.to_string();
        Box::pin(async move {
            let row: (bool,) = sqlx::query_as(
                "SELECT EXISTS(
                    SELECT 1 FROM platform_projects p
                    WHERE p.id = $1 AND p.deleted_at IS NULL
                      AND (p.created_by = $2
                           OR EXISTS(SELECT 1 FROM platform_project_members pm
                                     WHERE pm.project_id = p.id AND pm.user_id = $2
                                       AND pm.deleted_at IS NULL))
                 )",
            )
            .bind(&project_id)
            .bind(&actor)
            .fetch_one(&pool)
            .await
            .map_err(map_err)?;
            Ok(row.0)
        })
    }

    fn list_by_project(&self, project_id: &str) -> BoxFut<'_, Result<Vec<FlowRecord>, StoreError>> {
        let pool = self.pool.clone();
        let project_id = project_id.to_string();
        Box::pin(async move {
            let rows: Vec<FlowRow> = sqlx::query_as(&format!(
                "SELECT {COLUMNS} FROM flows WHERE project_id = $1 ORDER BY created_at ASC, id ASC"
            ))
            .bind(&project_id)
            .fetch_all(&pool)
            .await
            .map_err(map_err)?;
            Ok(rows.into_iter().map(FlowRecord::from).collect())
        })
    }

    fn get(&self, id: &str) -> BoxFut<'_, Result<Option<FlowRecord>, StoreError>> {
        let pool = self.pool.clone();
        let id = id.to_string();
        Box::pin(async move {
            let row: Option<FlowRow> =
                sqlx::query_as(&format!("SELECT {COLUMNS} FROM flows WHERE id = $1"))
                    .bind(&id)
                    .fetch_optional(&pool)
                    .await
                    .map_err(map_err)?;
            Ok(row.map(FlowRecord::from))
        })
    }

    fn create(
        &self,
        project_id: &str,
        name: &str,
        definition: &serde_json::Value,
    ) -> BoxFut<'_, Result<FlowRecord, StoreError>> {
        let pool = self.pool.clone();
        let project_id = project_id.to_string();
        let name = name.to_string();
        let definition = definition.clone();
        Box::pin(async move {
            let row: FlowRow = sqlx::query_as(&format!(
                "INSERT INTO flows (id, project_id, name, definition)
                 VALUES ($1, $2, $3, $4)
                 RETURNING {COLUMNS}"
            ))
            .bind(Ulid::new().to_string())
            .bind(&project_id)
            .bind(&name)
            .bind(&definition)
            .fetch_one(&pool)
            .await
            .map_err(map_err)?;
            Ok(FlowRecord::from(row))
        })
    }

    fn update(
        &self,
        id: &str,
        name: &str,
        definition: &serde_json::Value,
    ) -> BoxFut<'_, Result<Option<FlowRecord>, StoreError>> {
        let pool = self.pool.clone();
        let id = id.to_string();
        let name = name.to_string();
        let definition = definition.clone();
        Box::pin(async move {
            let row: Option<FlowRow> = sqlx::query_as(&format!(
                "UPDATE flows SET name = $2, definition = $3, version = version + 1,
                        updated_at = now()
                 WHERE id = $1
                 RETURNING {COLUMNS}"
            ))
            .bind(&id)
            .bind(&name)
            .bind(&definition)
            .fetch_optional(&pool)
            .await
            .map_err(map_err)?;
            Ok(row.map(FlowRecord::from))
        })
    }

    fn delete(&self, id: &str) -> BoxFut<'_, Result<bool, StoreError>> {
        let pool = self.pool.clone();
        let id = id.to_string();
        Box::pin(async move {
            let result = sqlx::query("DELETE FROM flows WHERE id = $1")
                .bind(&id)
                .execute(&pool)
                .await
                .map_err(map_err)?;
            Ok(result.rows_affected() > 0)
        })
    }
}
