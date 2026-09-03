use async_trait::async_trait;
use crate::domain::channels::{ProjectProviderConfig, ProjectProviderConfigInput};

/// Port for storing and retrieving project-level provider configurations.
#[async_trait]
pub trait ChannelProviderStore: Send + Sync {
    /// Get all provider configs for a project.
    async fn list_by_project(
        &self,
        project_id: &str,
    ) -> Result<Vec<ProjectProviderConfig>, String>;

    /// Get a single provider config by ID.
    async fn get(
        &self,
        id: &str,
    ) -> Result<Option<ProjectProviderConfig>, String>;

    /// Get a provider config by project + channel + provider.
    async fn get_by_project_channel_provider(
        &self,
        project_id: &str,
        channel_id: &str,
        provider_id: &str,
    ) -> Result<Option<ProjectProviderConfig>, String>;

    /// Create a new provider config.
    async fn create(
        &self,
        input: ProjectProviderConfigInput,
        project_id: &str,
    ) -> Result<ProjectProviderConfig, String>;

    /// Update an existing provider config.
    async fn update(
        &self,
        id: &str,
        input: ProjectProviderConfigInput,
    ) -> Result<ProjectProviderConfig, String>;

    /// Delete a provider config.
    async fn delete(&self, id: &str) -> Result<(), String>;

    /// Delete all provider configs for a project.
    async fn delete_by_project(&self, project_id: &str) -> Result<(), String>;
}
