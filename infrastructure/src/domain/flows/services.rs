//! Project flow management: plain CRUD over stored definitions.
//!
//! Flows v2: no system flags. Any flow carrying a `notification.created`
//! trigger qualifies for sends; the send worker resolves candidates at
//! send time (oldest first).

use std::sync::Arc;

use crate::domain::auth::entities::UserId;
use crate::domain::error::AppError;
use crate::domain::flows::entities::Flow;
use crate::ports::auth_store::StoreError;
use crate::ports::flow_store::{FlowRecord, FlowStore};

pub struct FlowService {
    store: Arc<dyn FlowStore>,
}

impl FlowService {
    pub fn new(store: Arc<dyn FlowStore>) -> Self {
        Self { store }
    }

    fn map_store_error(e: StoreError) -> AppError {
        match e {
            StoreError::Conflict => AppError::Conflict("conflicting flow change".into()),
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

    fn to_flow(record: FlowRecord) -> Flow {
        Flow {
            id: record.id,
            project_id: record.project_id,
            name: record.name,
            definition: record.definition,
            version: record.version,
            created_at: record.created_at,
            updated_at: record.updated_at,
        }
    }

    fn check_definition(definition: &serde_json::Value) -> Result<(), AppError> {
        let is_doc =
            definition.is_object() && definition.get("nodes").is_some_and(|n| n.is_array());
        if !is_doc {
            return Err(AppError::Validation(
                "definition must be a flow document with a nodes array".into(),
            ));
        }
        Ok(())
    }

    /// All flows for a project, oldest first.
    pub async fn list(&self, actor: UserId, project_id: &str) -> Result<Vec<Flow>, AppError> {
        self.check_visible(actor, project_id).await?;
        Ok(self
            .store
            .list_by_project(project_id)
            .await
            .map_err(Self::map_store_error)?
            .into_iter()
            .map(Self::to_flow)
            .collect())
    }

    /// Project flows without a caller check — worker/system contexts only
    /// (campaign pipeline resolution at fire time). Never route this to
    /// HTTP handlers.
    pub async fn list_by_project(&self, project_id: &str) -> Result<Vec<Flow>, AppError> {
        Ok(self
            .store
            .list_by_project(project_id)
            .await
            .map_err(Self::map_store_error)?
            .into_iter()
            .map(Self::to_flow)
            .collect())
    }

    /// One flow by id. Project-scoped: foreign rows read as missing.
    pub async fn get(&self, actor: UserId, project_id: &str, id: &str) -> Result<Flow, AppError> {
        self.check_visible(actor, project_id).await?;
        self.store
            .get(id)
            .await
            .map_err(Self::map_store_error)?
            .filter(|r| r.project_id == project_id)
            .map(Self::to_flow)
            .ok_or_else(|| AppError::NotFound("flow not found".into()))
    }

    /// Insert a flow. Empty names fall back to "Untitled flow"; an omitted
    /// definition starts blank (the builder's validator asks for a trigger).
    pub async fn create(
        &self,
        actor: UserId,
        project_id: &str,
        name: &str,
        definition: Option<serde_json::Value>,
    ) -> Result<Flow, AppError> {
        self.check_visible(actor, project_id).await?;
        let name = name.trim();
        let name = if name.is_empty() {
            "Untitled flow"
        } else {
            name
        };
        let definition = match definition {
            Some(def) => {
                Self::check_definition(&def)?;
                def
            }
            None => serde_json::json!({ "version": 1, "nodes": [], "edges": [] }),
        };
        self.store
            .create(project_id, name, &definition)
            .await
            .map_err(Self::map_store_error)
            .map(Self::to_flow)
    }

    /// Replace a flow's name/definition (bumps version). Project-scoped.
    pub async fn update(
        &self,
        actor: UserId,
        project_id: &str,
        id: &str,
        name: &str,
        definition: &serde_json::Value,
    ) -> Result<Flow, AppError> {
        self.check_visible(actor, project_id).await?;
        let existing = self
            .store
            .get(id)
            .await
            .map_err(Self::map_store_error)?
            .filter(|r| r.project_id == project_id);
        if existing.is_none() {
            return Err(AppError::NotFound("flow not found".into()));
        }
        let name = name.trim();
        if name.is_empty() {
            return Err(AppError::Validation("flow name is required".into()));
        }
        Self::check_definition(definition)?;
        self.store
            .update(id, name, definition)
            .await
            .map_err(Self::map_store_error)?
            .map(Self::to_flow)
            .ok_or_else(|| AppError::NotFound("flow not found".into()))
    }

    /// Delete a flow. Returns false when there was nothing to delete.
    /// Project-scoped: foreign rows are untouched and read as missing.
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
        if existing.is_none() {
            return Ok(false);
        }
        self.store.delete(id).await.map_err(Self::map_store_error)
    }
}
