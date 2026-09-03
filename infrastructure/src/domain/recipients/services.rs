//! Recipient use cases: create, list, get, remove (soft delete).
//!
//! All operations are scoped to a project the caller belongs to; the store
//! returns a project not-found / not-visible as an indistinguishable `None`
//! (no resource enumeration), same contract as the projects slice.

use std::sync::Arc;

use serde_json::{json, Value};

use crate::domain::auth::entities::UserId;
use crate::domain::auth::errors::AuthError;
use crate::domain::audit::entities::{AuditAction, AuditEvent};
use crate::domain::audit::AuditService;
use crate::domain::recipients::entities::Recipient;
use crate::ports::auth_store::StoreError;
use crate::ports::recipients_store::RecipientsStore;

/// Trimmed input bounds mirroring the documented contract.
const MAX_NAME: usize = 200;
const MAX_USER_ID: usize = 200;

pub struct RecipientService {
    store: Arc<dyn RecipientsStore>,
    audit: Arc<AuditService>,
}

impl RecipientService {
    pub fn new(store: Arc<dyn RecipientsStore>, audit: Arc<AuditService>) -> Self {
        Self { store, audit }
    }

    /// Creates a recipient in `project_id` on behalf of `actor` (project owner/member).
    pub async fn create(
        &self,
        actor: UserId,
        project_id: &str,
        user_id: &str,
        name: &str,
        contacts: Value,
    ) -> Result<Recipient, AuthError> {
        let name = name.trim();
        let user_id = user_id.trim();

        if name.is_empty() || name.len() > MAX_NAME {
            return Err(AuthError::Validation(
                "name is required (200 characters max)".to_string(),
            ));
        }
        if user_id.is_empty() || user_id.len() > MAX_USER_ID {
            return Err(AuthError::Validation(
                "user_id is required (200 characters max)".to_string(),
            ));
        }
        if !contacts.is_null() && !contacts.is_object() {
            return Err(AuthError::Validation(
                "contacts must be a JSON object".to_string(),
            ));
        }
        let contacts = if contacts.is_null() { json!({}) } else { contacts };

        let record = self
            .store
            .create(actor, project_id, user_id, name, contacts)
            .await
            .map_err(map_store_error)?;

        let recipient = Recipient::from(record);

        self.audit
            .record(
                chrono::Utc::now(),
                &AuditEvent::new(
                    AuditAction::RecipientCreated,
                    Some(&actor.to_string()),
                    None,
                    Some(project_id),
                    format!("recipient '{}' created", recipient.name),
                    Some(json!({ "recipient_id": recipient.id })),
                ),
            )
            .await;

        Ok(recipient)
    }

    /// Most recently created first, scoped to a visible project.
    pub async fn list<P: AsRef<str>>(
        &self,
        actor: UserId,
        project_id: P,
        search: Option<&str>,
        limit: i64,
        before: Option<&str>,
    ) -> Result<Vec<Recipient>, AuthError> {
        Ok(self
            .store
            .list(actor, project_id.as_ref(), search, limit, before)
            .await
            .map_err(map_store_error)?
            .into_iter()
            .map(Recipient::from)
            .collect())
    }

    /// A single recipient; `None` when the project or recipient isn't visible.
    pub async fn get(
        &self,
        actor: UserId,
        project_id: &str,
        recipient_id: &str,
    ) -> Result<Option<Recipient>, AuthError> {
        Ok(self
            .store
            .get(actor, project_id, recipient_id)
            .await
            .map_err(map_store_error)?
            .map(Recipient::from))
    }

    /// Updates a recipient's display details (name and/or contacts).
    ///
    /// `name` and `contacts` are optional: `None` keeps the current value.
    /// When supplied, `contacts` **replaces** the whole contact blob. `user_id`
    /// is immutable (the brand's targeting key).
    pub async fn update(
        &self,
        actor: UserId,
        project_id: &str,
        recipient_id: &str,
        name: Option<&str>,
        contacts: Option<Value>,
    ) -> Result<Recipient, AuthError> {
        let current = self
            .store
            .get(actor, project_id, recipient_id)
            .await
            .map_err(map_store_error)?
            .ok_or_else(|| AuthError::NotFound("recipient not found".to_string()))?;

        let new_name = match name {
            Some(name) => {
                let name = name.trim();
                if name.is_empty() || name.len() > MAX_NAME {
                    return Err(AuthError::Validation(
                        "name is required (200 characters max)".to_string(),
                    ));
                }
                name.to_string()
            }
            None => current.name.clone(),
        };

        let new_contacts = match contacts {
            Some(value) => {
                if !value.is_null() && !value.is_object() {
                    return Err(AuthError::Validation(
                        "contacts must be a JSON object".to_string(),
                    ));
                }
                if value.is_null() { json!({}) } else { value }
            }
            None => current.contacts.clone(),
        };

        let record = self
            .store
            .update(actor, project_id, recipient_id, &new_name, new_contacts)
            .await
            .map_err(map_store_error)?
            .ok_or_else(|| AuthError::NotFound("recipient not found".to_string()))?;

        let recipient = Recipient::from(record);

        self.audit
            .record(
                chrono::Utc::now(),
                &AuditEvent::new(
                    AuditAction::RecipientUpdated,
                    Some(&actor.to_string()),
                    None,
                    Some(project_id),
                    format!("recipient '{}' updated", recipient.name),
                    Some(json!({ "recipient_id": recipient.id })),
                ),
            )
            .await;

        Ok(recipient)
    }

    /// Soft-deletes a recipient (idempotent).
    pub async fn remove(
        &self,
        actor: UserId,
        project_id: &str,
        recipient_id: &str,
    ) -> Result<(), AuthError> {
        let removed = self
            .store
            .remove(actor, project_id, recipient_id)
            .await
            .map_err(map_store_error)?;

        if removed {
            self.audit
                .record(
                    chrono::Utc::now(),
                    &AuditEvent::new(
                        AuditAction::RecipientDeleted,
                        Some(&actor.to_string()),
                        None,
                        Some(project_id),
                        format!("recipient {recipient_id} deleted"),
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
        // Unlike the auth path, a conflict here is a duplicate brand user_id,
        // not a duplicate email.
        StoreError::Conflict => AuthError::Conflict(
            "A recipient with this user_id already exists in this project.".to_string(),
        ),
        StoreError::Storage(m) => AuthError::Storage(m),
    }
}
