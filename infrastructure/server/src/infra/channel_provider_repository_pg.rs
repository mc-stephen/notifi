use async_trait::async_trait;
use sqlx::PgPool;

use crate::domain::channels::{ProjectProviderConfig, ProjectProviderConfigInput};
use crate::ports::channel_provider_store::ChannelProviderStore;

pub struct PgChannelProviderStore {
    pool: PgPool,
}

impl PgChannelProviderStore {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl ChannelProviderStore for PgChannelProviderStore {
    async fn list_by_project(
        &self,
        project_id: &str,
    ) -> Result<Vec<ProjectProviderConfig>, String> {
        let rows: Vec<SqlxProjectProviderConfig> = sqlx::query_as(
            "SELECT id, project_id, channel_id, provider_id, config, smtp_fallback, enabled, created_at, updated_at
             FROM platform_project_provider_configs
             WHERE project_id = $1
             ORDER BY created_at DESC",
        )
        .bind(project_id)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| format!("Failed to list provider configs: {e}"))?;

        Ok(rows.into_iter().map(|r| r.into()).collect())
    }

    async fn get(&self, id: &str) -> Result<Option<ProjectProviderConfig>, String> {
        let row: Option<SqlxProjectProviderConfig> = sqlx::query_as(
            "SELECT id, project_id, channel_id, provider_id, config, smtp_fallback, enabled, created_at, updated_at
             FROM platform_project_provider_configs
             WHERE id = $1",
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| format!("Failed to get provider config: {e}"))?;

        Ok(row.map(|r| r.into()))
    }

    async fn get_by_project_channel_provider(
        &self,
        project_id: &str,
        channel_id: &str,
        provider_id: &str,
    ) -> Result<Option<ProjectProviderConfig>, String> {
        let row: Option<SqlxProjectProviderConfig> = sqlx::query_as(
            "SELECT id, project_id, channel_id, provider_id, config, smtp_fallback, enabled, created_at, updated_at
             FROM platform_project_provider_configs
             WHERE project_id = $1 AND channel_id = $2 AND provider_id = $3",
        )
        .bind(project_id)
        .bind(channel_id)
        .bind(provider_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| format!("Failed to get provider config: {e}"))?;

        Ok(row.map(|r| r.into()))
    }

    async fn create(
        &self,
        input: ProjectProviderConfigInput,
        project_id: &str,
    ) -> Result<ProjectProviderConfig, String> {
        let id = ulid::Ulid::new().to_string();
        let now = chrono::Utc::now();

        let row: SqlxProjectProviderConfig = sqlx::query_as(
            "INSERT INTO platform_project_provider_configs (id, project_id, channel_id, provider_id, config, smtp_fallback, enabled, created_at, updated_at)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
             RETURNING id, project_id, channel_id, provider_id, config, smtp_fallback, enabled, created_at, updated_at",
        )
        .bind(&id)
        .bind(project_id)
        .bind(&input.channel_id)
        .bind(&input.provider_id)
        .bind(&input.config)
        .bind(&input.smtp_fallback)
        .bind(input.enabled)
        .bind(now)
        .bind(now)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| format!("Failed to create provider config: {e}"))?;

        Ok(row.into())
    }

    async fn update(
        &self,
        id: &str,
        input: ProjectProviderConfigInput,
    ) -> Result<ProjectProviderConfig, String> {
        let now = chrono::Utc::now();

        let row: SqlxProjectProviderConfig = sqlx::query_as(
            "UPDATE platform_project_provider_configs
             SET channel_id = $2, provider_id = $3, config = $4, smtp_fallback = $5, enabled = $6, updated_at = $7
             WHERE id = $1
             RETURNING id, project_id, channel_id, provider_id, config, smtp_fallback, enabled, created_at, updated_at",
        )
        .bind(id)
        .bind(&input.channel_id)
        .bind(&input.provider_id)
        .bind(&input.config)
        .bind(&input.smtp_fallback)
        .bind(input.enabled)
        .bind(now)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| format!("Failed to update provider config: {e}"))?;

        Ok(row.into())
    }

    async fn delete(&self, id: &str) -> Result<(), String> {
        sqlx::query("DELETE FROM platform_project_provider_configs WHERE id = $1")
            .bind(id)
            .execute(&self.pool)
            .await
            .map_err(|e| format!("Failed to delete provider config: {e}"))?;

        Ok(())
    }

    async fn delete_by_project(&self, project_id: &str) -> Result<(), String> {
        sqlx::query("DELETE FROM platform_project_provider_configs WHERE project_id = $1")
            .bind(project_id)
            .execute(&self.pool)
            .await
            .map_err(|e| format!("Failed to delete provider configs: {e}"))?;

        Ok(())
    }
}

/// Internal sqlx row type for deserialization.
#[derive(Debug, sqlx::FromRow)]
struct SqlxProjectProviderConfig {
    id: String,
    project_id: String,
    channel_id: String,
    provider_id: String,
    config: serde_json::Value,
    smtp_fallback: Option<serde_json::Value>,
    enabled: bool,
    created_at: chrono::DateTime<chrono::Utc>,
    updated_at: chrono::DateTime<chrono::Utc>,
}

impl From<SqlxProjectProviderConfig> for ProjectProviderConfig {
    fn from(row: SqlxProjectProviderConfig) -> Self {
        Self {
            id: row.id,
            project_id: row.project_id,
            channel_id: row.channel_id,
            provider_id: row.provider_id,
            config: row.config,
            smtp_fallback: row.smtp_fallback,
            enabled: row.enabled,
            created_at: row.created_at,
            updated_at: row.updated_at,
        }
    }
}
