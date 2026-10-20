//! Webhook management: owner-side CRUD for flow-owning webhooks.
//!
//! Doctrine (mirroring schedules): the webhook **creates** its flow —
//! named after it, seeded with one locked webhook trigger node carrying
//! a fresh 128-bit token plus the configured event names — and deletes
//! it with itself. Flows are never assigned to webhooks, so owned flows
//! never reappear in `/flows` on delete ([`WebhookService::owned_flow_ids`]
//! keeps them off the list). Renames sync down to the flow. `archived`
//! is the non-destructive alternative: archived webhooks keep their flow
//! bound and restorable.
//!
//! Trigger tokens sync best-effort on create + every definition-affecting
//! update (a failed sync retries on next save).

use std::sync::Arc;

use rand::RngCore;

use crate::domain::auth::entities::UserId;
use crate::domain::error::AppError;
use crate::domain::inbound::InboundService;
use crate::domain::webhooks::entities::{Webhook, WebhookInput, WebhookPatch, normalize_events};
use crate::ports::auth_store::StoreError;
use crate::ports::flow_store::FlowStore;
use crate::ports::webhook_store::{NewWebhook, WebhookRecord, WebhookStore, WebhookUpdate};

pub struct WebhookService {
    store: Arc<dyn WebhookStore>,
    flows: Arc<dyn FlowStore>,
    inbound: Arc<InboundService>,
}

impl WebhookService {
    pub fn new(
        store: Arc<dyn WebhookStore>,
        flows: Arc<dyn FlowStore>,
        inbound: Arc<InboundService>,
    ) -> Self {
        Self {
            store,
            flows,
            inbound,
        }
    }

    fn map_store_error(e: StoreError) -> AppError {
        match e {
            StoreError::Conflict => AppError::Conflict("conflicting webhook change".into()),
            StoreError::Storage(m) => AppError::Storage(m),
        }
    }

    async fn check_visible(&self, actor: UserId, project_id: &str) -> Result<(), AppError> {
        if !self
            .store
            .project_visible(actor, project_id)
            .await
            .map_err(Self::map_store_error)?
        {
            return Err(AppError::NotFound("project not found".into()));
        }
        Ok(())
    }

    fn to_webhook(record: WebhookRecord) -> Webhook {
        Webhook {
            id: record.id,
            project_id: record.project_id,
            name: record.name,
            description: record.description,
            flow_id: record.flow_id,
            flow_name: record.flow_name,
            events: record
                .events
                .as_array()
                .map(|a| {
                    a.iter()
                        .filter_map(|v| v.as_str().map(str::to_string))
                        .collect()
                })
                .unwrap_or_default(),
            active: record.active,
            archived_at: record.archived_at,
            created_at: record.created_at,
            updated_at: record.updated_at,
        }
    }

    fn check_name(name: &str) -> Result<String, AppError> {
        let name = name.trim();
        if name.is_empty() {
            return Err(AppError::Validation("webhook name is required".into()));
        }
        Ok(name.to_string())
    }

    /// A fresh inbound token (128-bit hex — mirrors `newTriggerToken` on
    /// the frontend and `valid_trigger_token` in the inbound entities).
    fn new_trigger_token() -> String {
        let mut bytes = [0u8; 16];
        rand::rng().fill_bytes(&mut bytes);
        bytes.iter().map(|b| format!("{b:02x}")).collect()
    }

    /// The definition born with a webhook's flow: one locked webhook
    /// trigger node with the token + event names, nothing else. The
    /// builder wires steps onto it; the trigger itself is owned data
    /// (never deletable, not in the palette).
    fn seed_definition(token: &str, events: &[String]) -> serde_json::Value {
        serde_json::json!({
            "version": 1,
            "nodes": [
                {
                    "id": "trigger",
                    "type": "trigger",
                    "position": {"x": 0, "y": 0},
                    "data": {"kind": "webhook", "token": token, "events": events}
                }
            ],
            "edges": []
        })
    }

    /// All webhooks for a project, oldest first.
    pub async fn list(&self, actor: UserId, project_id: &str) -> Result<Vec<Webhook>, AppError> {
        self.check_visible(actor, project_id).await?;
        Ok(self
            .store
            .list_by_project(project_id)
            .await
            .map_err(Self::map_store_error)?
            .into_iter()
            .map(Self::to_webhook)
            .collect())
    }

    /// Flow ids owned by webhooks (the `/flows` list hides these — they
    /// are managed from their webhook's page).
    pub async fn owned_flow_ids(
        &self,
        actor: UserId,
        project_id: &str,
    ) -> Result<Vec<String>, AppError> {
        self.check_visible(actor, project_id).await?;
        self.store
            .owned_flow_ids(project_id)
            .await
            .map_err(Self::map_store_error)
    }

    /// One webhook by id. Project-scoped: foreign rows read as missing.
    pub async fn get(
        &self,
        actor: UserId,
        project_id: &str,
        id: &str,
    ) -> Result<Webhook, AppError> {
        self.check_visible(actor, project_id).await?;
        self.store
            .get(id)
            .await
            .map_err(Self::map_store_error)?
            .filter(|r| r.project_id == project_id)
            .map(Self::to_webhook)
            .ok_or_else(|| AppError::NotFound("webhook not found".into()))
    }

    /// Insert a webhook — creating its flow first. The flow is born
    /// named after the webhook with a locked webhook trigger node.
    pub async fn create(
        &self,
        actor: UserId,
        project_id: &str,
        input: WebhookInput,
    ) -> Result<Webhook, AppError> {
        self.check_visible(actor, project_id).await?;
        let name = Self::check_name(&input.name)?;
        let description = input.description.trim().to_string();
        let events = normalize_events(&input.events);
        let token = Self::new_trigger_token();
        let definition = Self::seed_definition(&token, &events);

        // The flow is owned data: created here, deleted with the webhook.
        let flow = self
            .flows
            .create(project_id, &name, &definition)
            .await
            .map_err(Self::map_store_error)?;
        let draft = NewWebhook {
            project_id: project_id.to_string(),
            name: name.clone(),
            description,
            flow_id: flow.id.clone(),
            events: serde_json::Value::Array(
                events
                    .iter()
                    .map(|e| serde_json::Value::String(e.clone()))
                    .collect(),
            ),
            active: true,
            created_by: actor,
        };
        match self.store.create(&draft).await {
            Ok(record) => {
                self.inbound
                    .sync_flow_tokens(project_id, &flow.id, &definition)
                    .await;
                Ok(Self::to_webhook(record))
            }
            Err(e) => {
                // Leave no owner-less flow behind (it would never surface
                // in the flows list — the owner row is the only door in).
                let _ = self.flows.delete(&flow.id).await;
                Err(Self::map_store_error(e))
            }
        }
    }

    /// Patch a webhook. Renames sync down to the flow; event changes
    /// rewrite the locked trigger node in place (the token survives, so
    /// callers of the inbound URL don't break). Archived webhooks stay
    /// editable — restoring is a click away.
    pub async fn update(
        &self,
        actor: UserId,
        project_id: &str,
        id: &str,
        patch: WebhookPatch,
    ) -> Result<Webhook, AppError> {
        self.check_visible(actor, project_id).await?;
        let current = self
            .store
            .get(id)
            .await
            .map_err(Self::map_store_error)?
            .filter(|r| r.project_id == project_id)
            .ok_or_else(|| AppError::NotFound("webhook not found".into()))?;

        let name = match patch.name {
            Some(n) => Self::check_name(&n)?,
            None => current.name.clone(),
        };
        let description = patch.description.unwrap_or(current.description.clone());
        let events = match patch.events {
            Some(e) => normalize_events(&e),
            None => Self::to_webhook(current.clone()).events,
        };
        let active = patch.active.unwrap_or(current.active);
        let now = chrono::Utc::now();
        let archived_at = match patch.archived {
            Some(true) => current.archived_at.or(Some(now)),
            Some(false) => None,
            None => current.archived_at,
        };

        // Rename sync: the owner owns the flow's identity too.
        if name != current.name
            && let Some(flow) = self
                .flows
                .get(&current.flow_id)
                .await
                .map_err(Self::map_store_error)?
        {
            let _ = self
                .flows
                .update(&current.flow_id, &name, &flow.definition)
                .await;
        }

        // Event changes rewrite the trigger node in the definition (same
        // node, same token). Anything the builder added stays untouched.
        let events_value = serde_json::Value::Array(
            events
                .iter()
                .map(|e| serde_json::Value::String(e.clone()))
                .collect(),
        );
        if events_value != current.events
            && let Some(flow) = self
                .flows
                .get(&current.flow_id)
                .await
                .map_err(Self::map_store_error)?
        {
            let mut definition = flow.definition.clone();
            if let Some(nodes) = definition.get_mut("nodes").and_then(|n| n.as_array_mut())
                && let Some(node) = nodes
                    .iter_mut()
                    .find(|n| n.get("type").and_then(|t| t.as_str()) == Some("trigger"))
                && let Some(data) = node.get_mut("data")
            {
                data["events"] = events_value.clone();
                let _ = self
                    .flows
                    .update(&current.flow_id, &flow.name, &definition)
                    .await;
                self.inbound
                    .sync_flow_tokens(project_id, &current.flow_id, &definition)
                    .await;
            }
        }

        self.store
            .update(
                id,
                &WebhookUpdate {
                    name,
                    description,
                    events: events_value,
                    active,
                    archived_at,
                },
            )
            .await
            .map_err(Self::map_store_error)?
            .map(Self::to_webhook)
            .ok_or_else(|| AppError::NotFound("webhook not found".into()))
    }

    /// Delete a webhook — and its flow with it. Owned flows never show
    /// in `/flows`, so releasing them would only resurrect an orphan the
    /// user can't find; the archive path exists for that instead.
    pub async fn delete(
        &self,
        actor: UserId,
        project_id: &str,
        id: &str,
    ) -> Result<bool, AppError> {
        self.check_visible(actor, project_id).await?;
        let existing = self
            .store
            .get(id)
            .await
            .map_err(Self::map_store_error)?
            .filter(|r| r.project_id == project_id);
        let Some(existing) = existing else {
            return Ok(false);
        };
        let deleted = self.store.delete(id).await.map_err(Self::map_store_error)?;
        if deleted {
            // The flow is owned data; it dies with its owner. Failure
            // here leaves a stray flow that no longer lists anywhere —
            // logged so it is never silent.
            if let Err(e) = self.flows.delete(&existing.flow_id).await {
                tracing::warn!(webhook_id = %id, error = ?e, "failed to delete owned flow");
            }
        }
        Ok(deleted)
    }
}
