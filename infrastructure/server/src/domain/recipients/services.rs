//! Recipient use cases: create, list, get, remove (soft delete).
//!
//! All operations are scoped to a project the caller belongs to; the store
//! returns a project not-found / not-visible as an indistinguishable `None`
//! (no resource enumeration), same contract as the projects slice.

use std::sync::Arc;

use serde_json::{Value, json};

use crate::domain::audit::AuditService;
use crate::domain::audit::entities::{AuditAction, AuditEvent};
use crate::domain::auth::entities::UserId;
use crate::domain::error::AppError;
use crate::domain::recipients::entities::Recipient;
use crate::ports::auth_store::StoreError;
use crate::ports::recipients_store::RecipientsStore;

/// Trimmed input bounds mirroring the documented contract.
const MAX_NAME: usize = 200;
const MAX_USER_ID: usize = 200;

/// The `devices` array doctrine (§25): device tokens are recipient data —
/// one entry per app install, `{destination, vendor?, token, label?}`.
/// Light on purpose: only the two load-bearing strings must be present
/// and non-empty; `vendor`/`label` are the project dev's own; unknown
/// keys anywhere in the blob stay allowed (future channels need no
/// migration).
fn validate_devices(contacts: &Value) -> Result<(), AppError> {
    let Some(devices) = contacts.get("devices") else {
        return Ok(());
    };
    let Some(list) = devices.as_array() else {
        return Err(AppError::Validation(
            "contacts.devices must be an array".to_string(),
        ));
    };
    for entry in list {
        let Some(entry) = entry.as_object() else {
            return Err(AppError::Validation(
                "each contacts.devices entry must be an object".to_string(),
            ));
        };
        for key in ["destination", "token"] {
            let value = entry
                .get(key)
                .and_then(|v| v.as_str())
                .unwrap_or_default()
                .trim();
            if value.is_empty() {
                return Err(AppError::Validation(format!(
                    "each contacts.devices entry needs a non-empty '{key}'"
                )));
            }
        }
        for key in ["vendor", "label"] {
            if let Some(v) = entry.get(key)
                && !v.is_string()
            {
                return Err(AppError::Validation(format!(
                    "contacts.devices entry '{key}' must be a string"
                )));
            }
        }
    }
    Ok(())
}

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
    ) -> Result<Recipient, AppError> {
        let name = name.trim();
        let user_id = user_id.trim();

        if name.is_empty() || name.len() > MAX_NAME {
            return Err(AppError::Validation(
                "name is required (200 characters max)".to_string(),
            ));
        }
        if user_id.is_empty() || user_id.len() > MAX_USER_ID {
            return Err(AppError::Validation(
                "user_id is required (200 characters max)".to_string(),
            ));
        }
        if !contacts.is_null() && !contacts.is_object() {
            return Err(AppError::Validation(
                "contacts must be a JSON object".to_string(),
            ));
        }
        let contacts = if contacts.is_null() {
            json!({})
        } else {
            contacts
        };
        validate_devices(&contacts)?;

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
    ) -> Result<Vec<Recipient>, AppError> {
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
    ) -> Result<Option<Recipient>, AppError> {
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
    /// is immutable (the project's targeting key).
    pub async fn update(
        &self,
        actor: UserId,
        project_id: &str,
        recipient_id: &str,
        name: Option<&str>,
        contacts: Option<Value>,
    ) -> Result<Recipient, AppError> {
        let current = self
            .store
            .get(actor, project_id, recipient_id)
            .await
            .map_err(map_store_error)?
            .ok_or_else(|| AppError::NotFound("recipient not found".to_string()))?;

        let new_name = match name {
            Some(name) => {
                let name = name.trim();
                if name.is_empty() || name.len() > MAX_NAME {
                    return Err(AppError::Validation(
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
                    return Err(AppError::Validation(
                        "contacts must be a JSON object".to_string(),
                    ));
                }
                let contacts = if value.is_null() { json!({}) } else { value };
                validate_devices(&contacts)?;
                contacts
            }
            None => current.contacts.clone(),
        };

        let record = self
            .store
            .update(actor, project_id, recipient_id, &new_name, new_contacts)
            .await
            .map_err(map_store_error)?
            .ok_or_else(|| AppError::NotFound("recipient not found".to_string()))?;

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
    ) -> Result<(), AppError> {
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

fn map_store_error(err: StoreError) -> AppError {
    match err {
        // Unlike the auth path, a conflict here is a duplicate recipient user_id,
        // not a duplicate email.
        StoreError::Conflict => AppError::Conflict(
            "A recipient with this user_id already exists in this project.".to_string(),
        ),
        StoreError::Storage(m) => AppError::Storage(m),
    }
}

#[cfg(test)]
mod device_blob_tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn blob_without_devices_key_passes_untouched() {
        // The blob stays shape-free: no devices key, nothing to check —
        // future contact kinds land without migrations.
        assert!(validate_devices(&json!({"email": "ada@example.com"})).is_ok());
        assert!(validate_devices(&json!({})).is_ok());
    }

    #[test]
    fn a_devices_entry_needs_destination_and_token() {
        let ok = json!({"devices": [
            {"destination": "android", "vendor": "fcm", "token": "tok-1", "label": "Pixel"}
        ]});
        assert!(validate_devices(&ok).is_ok());

        // Missing / empty / blank on the two load-bearing strings reject.
        for bad in [
            json!({"devices": [{"vendor": "fcm"}]}),
            json!({"devices": [{"destination": "android", "token": ""}]}),
            json!({"devices": [{"destination": "  ", "token": "t"}]}),
            json!({"devices": [{"destination": 7, "token": "t"}]}),
        ] {
            assert!(validate_devices(&bad).is_err(), "shape must reject: {bad}");
        }
    }

    #[test]
    fn devices_must_be_an_array_of_objects() {
        for bad in [
            json!({"devices": "tok-1"}),
            json!({"devices": ["tok-1"]}),
            json!({"devices": [{"destination": "android", "token": "t"}, "stray"]}),
        ] {
            assert!(validate_devices(&bad).is_err(), "reject: {bad}");
        }
    }

    #[test]
    fn vendor_and_label_may_be_absent_but_not_wrong_typed() {
        let minimal = json!({"devices": [{"destination": "apple", "token": "tok-9"}]});
        assert!(validate_devices(&minimal).is_ok());

        let bad = json!({"devices": [{"destination": "apple", "token": "t", "label": 7}]});
        assert!(validate_devices(&bad).is_err());
    }
}
