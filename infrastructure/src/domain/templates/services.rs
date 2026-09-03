//! Template use cases: create, list, get, update, remove (soft delete).
//!
//! All operations are scoped to a project the caller belongs to; the store
//! returns a project not-found / not-visible as an indistinguishable `None`
//! (no resource enumeration), same contract as the recipients slice.

use std::sync::Arc;

use serde_json::{json, Value};

use crate::domain::audit::entities::{AuditAction, AuditEvent};
use crate::domain::audit::AuditService;
use crate::domain::auth::entities::UserId;
use crate::domain::auth::errors::AuthError;
use crate::domain::templates::entities::Template;
use crate::ports::auth_store::StoreError;
use crate::ports::templates_store::{AttachmentInput, TemplatesStore};

/// Trimmed input bounds mirroring the documented contract.
const MAX_NAME: usize = 200;
const MAX_DESCRIPTION: usize = 2000;

/// Primary channels we recognise so the dashboard can validate `content`.
pub const CHANNELS: [&str; 8] = [
    "email", "sms", "in_app", "push", "webhook", "slack", "whatsapp", "telegram",
];

pub struct TemplateService {
    store: Arc<dyn TemplatesStore>,
    audit: Arc<AuditService>,
}

impl TemplateService {
    pub fn new(store: Arc<dyn TemplatesStore>, audit: Arc<AuditService>) -> Self {
        Self { store, audit }
    }

    /// Creates a template in `project_id` on behalf of `actor`.
    #[allow(clippy::too_many_arguments)]
    pub async fn create(
        &self,
        actor: UserId,
        project_id: &str,
        name: &str,
        description: Option<&str>,
        channel: &str,
        content: Value,
        attachments: Vec<AttachmentInput>,
    ) -> Result<Template, AuthError> {
        let name = name.trim();
        if name.is_empty() || name.len() > MAX_NAME {
            return Err(AuthError::Validation(
                "name is required (200 characters max)".to_string(),
            ));
        }
        let description = match description {
            Some(d) if d.len() > MAX_DESCRIPTION => {
                return Err(AuthError::Validation(
                    "description too long (2000 characters max)".to_string(),
                ))
            }
            other => other.map(str::trim).filter(|s| !s.is_empty()),
        };
        let channel = channel.trim();
        if channel.is_empty() {
            return Err(AuthError::Validation(
                "channel is required".to_string(),
            ));
        }
        if !content.is_null() && !content.is_object() {
            return Err(AuthError::Validation(
                "content must be a JSON object".to_string(),
            ));
        }
        let content = if content.is_null() { json!({}) } else { content };

        let record = self
            .store
            .create(
                actor, project_id, name, description, channel, content, attachments,
            )
            .await
            .map_err(map_store_error)?;

        let template = Template::from(record);

        self.audit
            .record(
                chrono::Utc::now(),
                &AuditEvent::new(
                    AuditAction::TemplateCreated,
                    Some(&actor.to_string()),
                    None,
                    Some(project_id),
                    format!("template '{}' created", template.name),
                    Some(json!({ "template_id": template.id })),
                ),
            )
            .await;

        Ok(template)
    }

    /// Most recently created first, scoped to a visible project.
    pub async fn list<P: AsRef<str>>(
        &self,
        actor: UserId,
        project_id: P,
        search: Option<&str>,
        limit: i64,
        before: Option<&str>,
    ) -> Result<Vec<Template>, AuthError> {
        Ok(self
            .store
            .list(actor, project_id.as_ref(), search, limit, before)
            .await
            .map_err(map_store_error)?
            .into_iter()
            .map(Template::from)
            .collect())
    }

    /// A single template; `None` when the project or template isn't visible.
    pub async fn get(
        &self,
        actor: UserId,
        project_id: &str,
        template_id: &str,
    ) -> Result<Option<Template>, AuthError> {
        Ok(self
            .store
            .get(actor, project_id, template_id)
            .await
            .map_err(map_store_error)?
            .map(Template::from))
    }

    /// Updates a template's editable fields and replaces its content. When
    /// `attachments` is `Some`, they replace the existing list wholesale; when
    /// `None`, the current attachments are kept.
    #[allow(clippy::too_many_arguments)]
    pub async fn update(
        &self,
        actor: UserId,
        project_id: &str,
        template_id: &str,
        name: &str,
        description: Option<&str>,
        channel: &str,
        content: Value,
        attachments: Option<Vec<AttachmentInput>>,
    ) -> Result<Template, AuthError> {
        let name = name.trim();
        if name.is_empty() || name.len() > MAX_NAME {
            return Err(AuthError::Validation(
                "name is required (200 characters max)".to_string(),
            ));
        }
        let description = match description {
            Some(d) if d.len() > MAX_DESCRIPTION => {
                return Err(AuthError::Validation(
                    "description too long (2000 characters max)".to_string(),
                ))
            }
            other => other.map(str::trim).filter(|s| !s.is_empty()),
        };
        let channel = channel.trim();
        if channel.is_empty() {
            return Err(AuthError::Validation(
                "channel is required".to_string(),
            ));
        }
        if !content.is_null() && !content.is_object() {
            return Err(AuthError::Validation(
                "content must be a JSON object".to_string(),
            ));
        }
        let content = if content.is_null() { json!({}) } else { content };

        // Preserve existing attachments when the caller didn't supply any.
        let attachments = match attachments {
            Some(list) => list,
            None => {
                let current = self
                    .store
                    .get(actor, project_id, template_id)
                    .await
                    .map_err(map_store_error)?
                    .ok_or_else(|| AuthError::NotFound("template not found".to_string()))?;
                current
                    .attachments
                    .into_iter()
                    .map(|a| AttachmentInput {
                        name: a.name,
                        mime_type: a.mime_type,
                        size_bytes: a.size_bytes,
                        url: a.url,
                    })
                    .collect()
            }
        };

        let record = self
            .store
            .update(
                actor, project_id, template_id, name, description, channel, content,
                attachments,
            )
            .await
            .map_err(map_store_error)?
            .ok_or_else(|| AuthError::NotFound("template not found".to_string()))?;

        let template = Template::from(record);

        self.audit
            .record(
                chrono::Utc::now(),
                &AuditEvent::new(
                    AuditAction::TemplateUpdated,
                    Some(&actor.to_string()),
                    None,
                    Some(project_id),
                    format!("template '{}' updated", template.name),
                    Some(json!({ "template_id": template.id })),
                ),
            )
            .await;

        Ok(template)
    }

    /// Soft-deletes a template (idempotent).
    pub async fn remove(
        &self,
        actor: UserId,
        project_id: &str,
        template_id: &str,
    ) -> Result<(), AuthError> {
        let removed = self
            .store
            .remove(actor, project_id, template_id)
            .await
            .map_err(map_store_error)?;

        if removed {
            self.audit
                .record(
                    chrono::Utc::now(),
                    &AuditEvent::new(
                        AuditAction::TemplateDeleted,
                        Some(&actor.to_string()),
                        None,
                        Some(project_id),
                        format!("template {template_id} deleted"),
                        None,
                    ),
                )
                .await;
        }

        Ok(())
    }
}

fn map_store_error(err: StoreError) -> AuthError {
    match err {
        StoreError::Conflict => AuthError::Conflict(
            "A template with this id already exists.".to_string(),
        ),
        StoreError::Storage(m) => AuthError::Storage(m),
    }
}
