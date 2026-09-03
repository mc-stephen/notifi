use axum::extract::{Extension, Path};
use axum::Json;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use crate::domain::channels::{ProjectProviderConfig, ProjectProviderConfigInput};
use crate::domain::auth::errors::AuthError;
use crate::ports::channel_provider_store::ChannelProviderStore;

use super::super::auth::{CurrentUser, Problem};

/// Request body for creating/updating a provider config.
#[derive(Debug, Deserialize)]
pub struct CreateProviderConfigRequest {
    pub channel_id: String,
    pub provider_id: String,
    pub config: serde_json::Value,
    #[serde(default)]
    pub smtp_fallback: Option<serde_json::Value>,
    #[serde(default = "default_true")]
    pub enabled: bool,
}

fn default_true() -> bool {
    true
}

/// Response type for provider config endpoints.
#[derive(Debug, Serialize)]
pub struct ProviderConfigResponse {
    pub id: String,
    pub project_id: String,
    pub channel_id: String,
    pub provider_id: String,
    pub config: serde_json::Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub smtp_fallback: Option<serde_json::Value>,
    pub enabled: bool,
    pub created_at: String,
    pub updated_at: String,
}

/// GET /v1/projects/:project_id/channel-configs
pub async fn list_configs(
    CurrentUser(_user): CurrentUser,
    Extension(store): Extension<Arc<dyn ChannelProviderStore + Send + Sync>>,
    Path(project_id): Path<String>,
) -> Result<Json<Vec<ProviderConfigResponse>>, Problem> {
    let configs = store
        .list_by_project(&project_id)
        .await
        .map_err(|e| Problem::from(AuthError::Storage(e)))?;

    Ok(Json(configs.into_iter().map(|c| c.into()).collect()))
}

/// POST /v1/projects/:project_id/channel-configs
pub async fn create_config(
    CurrentUser(_user): CurrentUser,
    Extension(store): Extension<Arc<dyn ChannelProviderStore + Send + Sync>>,
    Path(project_id): Path<String>,
    Json(req): Json<CreateProviderConfigRequest>,
) -> Result<Json<ProviderConfigResponse>, Problem> {
    let input = ProjectProviderConfigInput {
        channel_id: req.channel_id,
        provider_id: req.provider_id,
        config: req.config,
        smtp_fallback: req.smtp_fallback,
        enabled: req.enabled,
    };

    let config = store
        .create(input, &project_id)
        .await
        .map_err(|e| Problem::from(AuthError::Storage(e)))?;

    Ok(Json(config.into()))
}

/// PATCH /v1/projects/:project_id/channel-configs/:config_id
pub async fn update_config(
    CurrentUser(_user): CurrentUser,
    Extension(store): Extension<Arc<dyn ChannelProviderStore + Send + Sync>>,
    Path((_project_id, config_id)): Path<(String, String)>,
    Json(req): Json<CreateProviderConfigRequest>,
) -> Result<Json<ProviderConfigResponse>, Problem> {
    let input = ProjectProviderConfigInput {
        channel_id: req.channel_id,
        provider_id: req.provider_id,
        config: req.config,
        smtp_fallback: req.smtp_fallback,
        enabled: req.enabled,
    };

    let config = store
        .update(&config_id, input)
        .await
        .map_err(|e| Problem::from(AuthError::Storage(e)))?;

    Ok(Json(config.into()))
}

/// DELETE /v1/projects/:project_id/channel-configs/:config_id
pub async fn delete_config(
    CurrentUser(_user): CurrentUser,
    Extension(store): Extension<Arc<dyn ChannelProviderStore + Send + Sync>>,
    Path((_project_id, config_id)): Path<(String, String)>,
) -> Result<(), Problem> {
    store
        .delete(&config_id)
        .await
        .map_err(|e| Problem::from(AuthError::Storage(e)))?;

    Ok(())
}

impl From<ProjectProviderConfig> for ProviderConfigResponse {
    fn from(config: ProjectProviderConfig) -> Self {
        Self {
            id: config.id,
            project_id: config.project_id,
            channel_id: config.channel_id,
            provider_id: config.provider_id,
            config: config.config,
            smtp_fallback: config.smtp_fallback,
            enabled: config.enabled,
            created_at: config.created_at.to_rfc3339(),
            updated_at: config.updated_at.to_rfc3339(),
        }
    }
}
