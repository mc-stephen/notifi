use chrono::{DateTime, Utc};
use sqlx::PgPool;
use ulid::Ulid;

use crate::domain::auth::entities::UserId;
use crate::ports::auth_store::StoreError;
use crate::ports::campaigns_store::{
    BoxFut, CampaignRecord, CampaignStore, CampaignUpdate, NewCampaign,
};

pub struct PgCampaignStore {
    pool: PgPool,
}

impl PgCampaignStore {
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
struct CampaignRow {
    id: String,
    project_id: String,
    name: String,
    description: String,
    created_by: String,
    channel: String,
    template_id: Option<String>,
    title: String,
    body: String,
    audience_mode: String,
    selected_user_ids: serde_json::Value,
    provider_id: String,
    status: String,
    send_at: Option<DateTime<Utc>>,
    started_at: Option<DateTime<Utc>>,
    completed_at: Option<DateTime<Utc>>,
    last_error: Option<String>,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

impl From<CampaignRow> for CampaignRecord {
    fn from(r: CampaignRow) -> Self {
        Self {
            id: r.id,
            project_id: r.project_id,
            name: r.name,
            description: r.description,
            created_by: r.created_by,
            channel: r.channel,
            template_id: r.template_id,
            title: r.title,
            body: r.body,
            audience_mode: r.audience_mode,
            selected_user_ids: r.selected_user_ids,
            provider_id: r.provider_id,
            status: r.status,
            send_at: r.send_at,
            started_at: r.started_at,
            completed_at: r.completed_at,
            last_error: r.last_error,
            created_at: r.created_at,
            updated_at: r.updated_at,
        }
    }
}

const COLUMNS: &str = "id, project_id, name, description, created_by, channel, template_id,
    title, body, audience_mode, selected_user_ids, provider_id, status, send_at, started_at, completed_at,
    last_error, created_at, updated_at";

impl CampaignStore for PgCampaignStore {
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
    ) -> BoxFut<'_, Result<Vec<CampaignRecord>, StoreError>> {
        let pool = self.pool.clone();
        let project_id = project_id.to_string();
        Box::pin(async move {
            let rows: Vec<CampaignRow> = sqlx::query_as(&format!(
                "SELECT {COLUMNS} FROM campaigns
                 WHERE project_id = $1
                 ORDER BY created_at ASC, id ASC"
            ))
            .bind(&project_id)
            .fetch_all(&pool)
            .await
            .map_err(map_err)?;
            Ok(rows.into_iter().map(CampaignRecord::from).collect())
        })
    }

    fn get(&self, id: &str) -> BoxFut<'_, Result<Option<CampaignRecord>, StoreError>> {
        let pool = self.pool.clone();
        let id = id.to_string();
        Box::pin(async move {
            let row: Option<CampaignRow> =
                sqlx::query_as(&format!("SELECT {COLUMNS} FROM campaigns WHERE id = $1"))
                    .bind(&id)
                    .fetch_optional(&pool)
                    .await
                    .map_err(map_err)?;
            Ok(row.map(CampaignRecord::from))
        })
    }

    fn create(&self, draft: &NewCampaign) -> BoxFut<'_, Result<CampaignRecord, StoreError>> {
        let pool = self.pool.clone();
        let draft = draft.clone();
        Box::pin(async move {
            let row: CampaignRow = sqlx::query_as(&format!(
                "INSERT INTO campaigns
                     (id, project_id, name, description, created_by, channel, template_id,
                      title, body, audience_mode, selected_user_ids, provider_id, status, send_at)
                 VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14)
                 RETURNING {COLUMNS}"
            ))
            .bind(Ulid::new().to_string())
            .bind(&draft.project_id)
            .bind(&draft.name)
            .bind(&draft.description)
            .bind(&draft.created_by)
            .bind(&draft.channel)
            .bind(&draft.template_id)
            .bind(&draft.title)
            .bind(&draft.body)
            .bind(&draft.audience_mode)
            .bind(&draft.selected_user_ids)
            .bind(&draft.provider_id)
            .bind(&draft.status)
            .bind(draft.send_at)
            .fetch_one(&pool)
            .await
            .map_err(map_err)?;
            Ok(CampaignRecord::from(row))
        })
    }

    fn update(
        &self,
        id: &str,
        patch: &CampaignUpdate,
    ) -> BoxFut<'_, Result<Option<CampaignRecord>, StoreError>> {
        let pool = self.pool.clone();
        let id = id.to_string();
        let patch = patch.clone();
        Box::pin(async move {
            let row: Option<CampaignRow> = sqlx::query_as(&format!(
                "UPDATE campaigns SET
                     name = $2, description = $3, channel = $4, template_id = $5,
                     title = $6, body = $7, audience_mode = $8, selected_user_ids = $9,
                     provider_id = $10, send_at = $11, updated_at = now()
                 WHERE id = $1
                 RETURNING {COLUMNS}"
            ))
            .bind(&id)
            .bind(&patch.name)
            .bind(&patch.description)
            .bind(&patch.channel)
            .bind(&patch.template_id)
            .bind(&patch.title)
            .bind(&patch.body)
            .bind(&patch.audience_mode)
            .bind(&patch.selected_user_ids)
            .bind(&patch.provider_id)
            .bind(patch.send_at)
            .fetch_optional(&pool)
            .await
            .map_err(map_err)?;
            Ok(row.map(CampaignRecord::from))
        })
    }

    fn delete(&self, id: &str) -> BoxFut<'_, Result<bool, StoreError>> {
        let pool = self.pool.clone();
        let id = id.to_string();
        Box::pin(async move {
            let removed: Option<i64> =
                sqlx::query_scalar("DELETE FROM campaigns WHERE id = $1 RETURNING 1")
                    .bind(&id)
                    .fetch_optional(&pool)
                    .await
                    .map_err(map_err)?;
            Ok(removed.is_some())
        })
    }

    fn claim_due(
        &self,
        now: DateTime<Utc>,
        limit: i64,
    ) -> BoxFut<'_, Result<Vec<CampaignRecord>, StoreError>> {
        let pool = self.pool.clone();
        Box::pin(async move {
            let rows: Vec<CampaignRow> = sqlx::query_as(&format!(
                "UPDATE campaigns SET status = 'running', started_at = $1, updated_at = now()
                 WHERE id IN (
                     SELECT id FROM campaigns
                     WHERE status = 'scheduled'
                       AND send_at IS NOT NULL
                       AND send_at <= $1
                     ORDER BY send_at ASC, id ASC
                     LIMIT $2
                 )
                 RETURNING {COLUMNS}"
            ))
            .bind(now)
            .bind(limit)
            .fetch_all(&pool)
            .await
            .map_err(map_err)?;
            Ok(rows.into_iter().map(CampaignRecord::from).collect())
        })
    }

    fn list_running(&self, limit: i64) -> BoxFut<'_, Result<Vec<CampaignRecord>, StoreError>> {
        let pool = self.pool.clone();
        Box::pin(async move {
            let rows: Vec<CampaignRow> = sqlx::query_as(&format!(
                "SELECT {COLUMNS} FROM campaigns
                 WHERE status = 'running'
                 ORDER BY created_at ASC, id ASC
                 LIMIT $1"
            ))
            .bind(limit)
            .fetch_all(&pool)
            .await
            .map_err(map_err)?;
            Ok(rows.into_iter().map(CampaignRecord::from).collect())
        })
    }

    fn transition(
        &self,
        id: &str,
        status: &str,
        started_at: Option<DateTime<Utc>>,
        completed_at: Option<DateTime<Utc>>,
        last_error: Option<&str>,
    ) -> BoxFut<'_, Result<Option<CampaignRecord>, StoreError>> {
        let pool = self.pool.clone();
        let id = id.to_string();
        let status = status.to_string();
        let last_error = last_error.map(str::to_string);
        Box::pin(async move {
            let row: Option<CampaignRow> = sqlx::query_as(&format!(
                "UPDATE campaigns SET
                     status = $2,
                     started_at = COALESCE($3, started_at),
                     completed_at = COALESCE($4, completed_at),
                     last_error = $5,
                     updated_at = now()
                 WHERE id = $1
                 RETURNING {COLUMNS}"
            ))
            .bind(&id)
            .bind(&status)
            .bind(started_at)
            .bind(completed_at)
            .bind(last_error)
            .fetch_optional(&pool)
            .await
            .map_err(map_err)?;
            Ok(row.map(CampaignRecord::from))
        })
    }
}
