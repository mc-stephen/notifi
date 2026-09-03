//! The persistence port for the templates slice.
//!
//! Implemented by `infra` (Postgres) and `testing` (in-memory fakes), same
//! object-safe boxed-future pattern as `RecipientsStore`.

use std::future::Future;
use std::pin::Pin;

use crate::domain::auth::entities::UserId;
use crate::ports::auth_store::StoreError;

/// Boxed future returned by every port method.
pub type BoxFut<'a, T> = Pin<Box<dyn Future<Output = T> + Send + 'a>>;

/// An attachment as persisted by the store (metadata + URL only).
#[derive(Debug, Clone)]
pub struct AttachmentRecord {
    pub id: String,
    pub name: String,
    pub mime_type: String,
    pub size_bytes: i64,
    pub url: String,
}

/// A template as persisted by the store, with its attachments.
#[derive(Debug, Clone)]
pub struct TemplateRecord {
    pub id: String,
    pub project_id: String,
    pub name: String,
    pub description: Option<String>,
    pub channel: String,
    pub content: serde_json::Value,
    pub version: i32,
    pub attachments: Vec<AttachmentRecord>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

pub trait TemplatesStore: Send + Sync {
    /// Persists a new template in a project the caller belongs to.
    #[allow(clippy::too_many_arguments)]
    fn create(
        &self,
        actor: UserId,
        project_id: &str,
        name: &str,
        description: Option<&str>,
        channel: &str,
        content: serde_json::Value,
        attachments: Vec<AttachmentInput>,
    ) -> BoxFut<'_, Result<TemplateRecord, StoreError>>;

    /// Templates in a project the caller belongs to, newest first. `None`
    /// when the project isn't visible to `actor`.
    fn list(
        &self,
        actor: UserId,
        project_id: &str,
        search: Option<&str>,
        limit: i64,
        before: Option<&str>,
    ) -> BoxFut<'_, Result<Vec<TemplateRecord>, StoreError>>;

    /// A single visible template (with attachments); `None` when not found
    /// or the project isn't visible.
    fn get(
        &self,
        actor: UserId,
        project_id: &str,
        template_id: &str,
    ) -> BoxFut<'_, Result<Option<TemplateRecord>, StoreError>>;

    /// Updates a visible template (name/description/channel/content and its
    /// attachments, replaced wholesale). Returns `None` when not found.
    #[allow(clippy::too_many_arguments)]
    fn update(
        &self,
        actor: UserId,
        project_id: &str,
        template_id: &str,
        name: &str,
        description: Option<&str>,
        channel: &str,
        content: serde_json::Value,
        attachments: Vec<AttachmentInput>,
    ) -> BoxFut<'_, Result<Option<TemplateRecord>, StoreError>>;

    /// Soft-deletes a visible template (and its attachments). Returns whether
    /// anything was removed.
    fn remove(
        &self,
        actor: UserId,
        project_id: &str,
        template_id: &str,
    ) -> BoxFut<'_, Result<bool, StoreError>>;
}

/// Attachment input (metadata + URL) referenced by a create/update.
#[derive(Debug, Clone)]
pub struct AttachmentInput {
    pub name: String,
    pub mime_type: String,
    pub size_bytes: i64,
    pub url: String,
}
