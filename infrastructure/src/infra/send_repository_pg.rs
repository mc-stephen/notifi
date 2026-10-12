use chrono::{DateTime, Utc};
use sqlx::PgPool;
use ulid::Ulid;

use crate::domain::auth::entities::UserId;
use crate::ports::auth_store::StoreError;
use crate::ports::send_store::{BoxFut, SendRecord, SendStore};

pub struct PgSendStore {
    pool: PgPool,
}

impl PgSendStore {
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
struct SendRow {
    id: String,
    project_id: String,
    channel: String,
    recipients: serde_json::Value,
    title: String,
    body: String,
    send_at: DateTime<Utc>,
    status: String,
    attempts: i32,
    provider: Option<String>,
    external_id: Option<String>,
    last_error: Option<String>,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

impl From<SendRow> for SendRecord {
    fn from(r: SendRow) -> Self {
        Self {
            id: r.id,
            project_id: r.project_id,
            channel: r.channel,
            recipients: r.recipients,
            title: r.title,
            body: r.body,
            send_at: r.send_at,
            status: r.status,
            attempts: r.attempts,
            provider: r.provider,
            external_id: r.external_id,
            last_error: r.last_error,
            created_at: r.created_at,
            updated_at: r.updated_at,
        }
    }
}

const COLUMNS: &str = "id, project_id, channel, recipients, title, body, send_at,
    status, attempts, provider, external_id, last_error, created_at, updated_at";

impl SendStore for PgSendStore {
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

    fn create(
        &self,
        project_id: &str,
        channel: &str,
        recipients: &serde_json::Value,
        title: &str,
        body: &str,
        send_at: DateTime<Utc>,
    ) -> BoxFut<'_, Result<SendRecord, StoreError>> {
        let pool = self.pool.clone();
        let project_id = project_id.to_string();
        let channel = channel.to_string();
        let recipients = recipients.clone();
        let title = title.to_string();
        let body = body.to_string();
        Box::pin(async move {
            let row: SendRow = sqlx::query_as(&format!(
                "INSERT INTO notification_sends
                     (id, project_id, channel, recipients, title, body, send_at, status)
                 VALUES ($1, $2, $3, $4, $5, $6, $7, 'sending')
                 RETURNING {COLUMNS}"
            ))
            .bind(Ulid::new().to_string())
            .bind(&project_id)
            .bind(&channel)
            .bind(&recipients)
            .bind(&title)
            .bind(&body)
            .bind(send_at)
            .fetch_one(&pool)
            .await
            .map_err(map_err)?;
            Ok(SendRecord::from(row))
        })
    }

    fn mark_completed(
        &self,
        id: &str,
        provider: &str,
        external_id: Option<&str>,
    ) -> BoxFut<'_, Result<Option<SendRecord>, StoreError>> {
        let pool = self.pool.clone();
        let id = id.to_string();
        let provider = provider.to_string();
        let external_id = external_id.map(str::to_string);
        Box::pin(async move {
            let row: Option<SendRow> = sqlx::query_as(&format!(
                "UPDATE notification_sends
                 SET status = 'completed', provider = $2, external_id = $3,
                     updated_at = now()
                 WHERE id = $1
                 RETURNING {COLUMNS}"
            ))
            .bind(&id)
            .bind(&provider)
            .bind(&external_id)
            .fetch_optional(&pool)
            .await
            .map_err(map_err)?;
            Ok(row.map(SendRecord::from))
        })
    }

    fn mark_failed(
        &self,
        id: &str,
        last_error: &str,
    ) -> BoxFut<'_, Result<Option<SendRecord>, StoreError>> {
        let pool = self.pool.clone();
        let id = id.to_string();
        let last_error = last_error.to_string();
        Box::pin(async move {
            let row: Option<SendRow> = sqlx::query_as(&format!(
                "UPDATE notification_sends
                 SET status = 'failed', attempts = attempts + 1,
                     last_error = $2, updated_at = now()
                 WHERE id = $1
                 RETURNING {COLUMNS}"
            ))
            .bind(&id)
            .bind(&last_error)
            .fetch_optional(&pool)
            .await
            .map_err(map_err)?;
            Ok(row.map(SendRecord::from))
        })
    }

    fn list_by_project(
        &self,
        project_id: &str,
        limit: i64,
    ) -> BoxFut<'_, Result<Vec<SendRecord>, StoreError>> {
        let pool = self.pool.clone();
        let project_id = project_id.to_string();
        Box::pin(async move {
            let rows: Vec<SendRow> = sqlx::query_as(&format!(
                "SELECT {COLUMNS} FROM notification_sends
                 WHERE project_id = $1
                 ORDER BY created_at DESC
                 LIMIT $2"
            ))
            .bind(&project_id)
            .bind(limit)
            .fetch_all(&pool)
            .await
            .map_err(map_err)?;
            Ok(rows.into_iter().map(SendRecord::from).collect())
        })
    }
}
