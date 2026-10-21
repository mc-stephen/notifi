use sqlx::PgPool;
use ulid::Ulid;

use crate::domain::auth::entities::UserId;
use crate::ports::auth_store::StoreError;
use crate::ports::webhook_store::{BoxFut, NewWebhook, WebhookRecord, WebhookStore, WebhookUpdate};

pub struct PgWebhookStore {
    pool: PgPool,
}

impl PgWebhookStore {
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
struct WebhookRow {
    id: String,
    project_id: String,
    name: String,
    description: String,
    flow_id: String,
    flow_name: String,
    events: serde_json::Value,
    active: bool,
    archived_at: Option<chrono::DateTime<chrono::Utc>>,
    created_at: chrono::DateTime<chrono::Utc>,
    updated_at: chrono::DateTime<chrono::Utc>,
}

impl From<WebhookRow> for WebhookRecord {
    fn from(r: WebhookRow) -> Self {
        Self {
            id: r.id,
            project_id: r.project_id,
            name: r.name,
            description: r.description,
            flow_id: r.flow_id,
            flow_name: r.flow_name,
            events: r.events,
            active: r.active,
            archived_at: r.archived_at,
            created_at: r.created_at,
            updated_at: r.updated_at,
        }
    }
}

const COLUMNS: &str = "w.id, w.project_id, w.name, w.description, w.flow_id, \
    f.name AS flow_name, w.events, w.active, w.archived_at, w.created_at, w.updated_at";
const JOINS: &str = "FROM webhooks w JOIN flows f ON f.id = w.flow_id";

impl WebhookStore for PgWebhookStore {
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

    fn list_by_project(
        &self,
        project_id: &str,
    ) -> BoxFut<'_, Result<Vec<WebhookRecord>, StoreError>> {
        let pool = self.pool.clone();
        let project_id = project_id.to_string();
        Box::pin(async move {
            let rows: Vec<WebhookRow> = sqlx::query_as(&format!(
                "SELECT {COLUMNS} {JOINS}
                 WHERE w.project_id = $1 ORDER BY w.created_at ASC, w.id ASC"
            ))
            .bind(&project_id)
            .fetch_all(&pool)
            .await
            .map_err(map_err)?;
            Ok(rows.into_iter().map(WebhookRecord::from).collect())
        })
    }

    fn get(&self, id: &str) -> BoxFut<'_, Result<Option<WebhookRecord>, StoreError>> {
        let pool = self.pool.clone();
        let id = id.to_string();
        Box::pin(async move {
            let row: Option<WebhookRow> = sqlx::query_as(&format!(
                "SELECT {COLUMNS} {JOINS}
                 WHERE w.id = $1"
            ))
            .bind(&id)
            .fetch_optional(&pool)
            .await
            .map_err(map_err)?;
            Ok(row.map(WebhookRecord::from))
        })
    }

    fn owned_flow_ids(&self, project_id: &str) -> BoxFut<'_, Result<Vec<String>, StoreError>> {
        let pool = self.pool.clone();
        let project_id = project_id.to_string();
        Box::pin(async move {
            let rows: Vec<(String,)> =
                sqlx::query_as("SELECT DISTINCT flow_id FROM webhooks WHERE project_id = $1")
                    .bind(&project_id)
                    .fetch_all(&pool)
                    .await
                    .map_err(map_err)?;
            Ok(rows.into_iter().map(|r| r.0).collect())
        })
    }

    fn create(&self, draft: &NewWebhook) -> BoxFut<'_, Result<WebhookRecord, StoreError>> {
        let pool = self.pool.clone();
        let draft = draft.clone();
        Box::pin(async move {
            let row: WebhookRow = sqlx::query_as(
                "WITH ins AS (
                   INSERT INTO webhooks (id, project_id, name, description, flow_id,
                       events, active, created_by, archived_at)
                   VALUES ($1, $2, $3, $4, $5, $6, $7, $8, NULL)
                   RETURNING *
                 )
                 SELECT ins.id, ins.project_id, ins.name, ins.description, ins.flow_id,
                     f.name AS flow_name, ins.events, ins.active, ins.archived_at,
                     ins.created_at, ins.updated_at
                 FROM ins JOIN flows f ON f.id = ins.flow_id",
            )
            .bind(Ulid::new().to_string())
            .bind(&draft.project_id)
            .bind(&draft.name)
            .bind(&draft.description)
            .bind(&draft.flow_id)
            .bind(&draft.events)
            .bind(draft.active)
            .bind(draft.created_by.to_string())
            .fetch_one(&pool)
            .await
            .map_err(map_err)?;
            Ok(WebhookRecord::from(row))
        })
    }

    fn update(
        &self,
        id: &str,
        patch: &WebhookUpdate,
    ) -> BoxFut<'_, Result<Option<WebhookRecord>, StoreError>> {
        let pool = self.pool.clone();
        let id = id.to_string();
        let patch = patch.clone();
        Box::pin(async move {
            let row: Option<WebhookRow> = sqlx::query_as(
                "WITH upd AS (
                   UPDATE webhooks SET name = $2, description = $3, events = $4,
                       active = $5, archived_at = $6, updated_at = now()
                   WHERE id = $1
                   RETURNING *
                 )
                 SELECT upd.id, upd.project_id, upd.name, upd.description, upd.flow_id,
                     f.name AS flow_name, upd.events, upd.active, upd.archived_at,
                     upd.created_at, upd.updated_at
                 FROM upd JOIN flows f ON f.id = upd.flow_id",
            )
            .bind(&id)
            .bind(&patch.name)
            .bind(&patch.description)
            .bind(&patch.events)
            .bind(patch.active)
            .bind(patch.archived_at)
            .fetch_optional(&pool)
            .await
            .map_err(map_err)?;
            Ok(row.map(WebhookRecord::from))
        })
    }

    fn delete(&self, id: &str) -> BoxFut<'_, Result<bool, StoreError>> {
        let pool = self.pool.clone();
        let id = id.to_string();
        Box::pin(async move {
            let result = sqlx::query("DELETE FROM webhooks WHERE id = $1")
                .bind(&id)
                .execute(&pool)
                .await
                .map_err(map_err)?;
            Ok(result.rows_affected() > 0)
        })
    }
}
