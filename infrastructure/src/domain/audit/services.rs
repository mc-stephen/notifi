//! Audit service: the in-process listener that records system actions.
//!
//! Mutating domains call [`AuditService::record`] after a successful
//! mutation; the entry is persisted append-only. Writes are best-effort — a
//! failed record is logged, never propagated to the caller's error path.

use std::sync::Arc;

use crate::domain::audit::entities::{AuditEntry, AuditEvent};
use crate::domain::auth::errors::AuthError;
use crate::ports::audit_store::{AdminAuditFilters, AuditFilters, AuditStore};
use notifi_core::Ulid;

pub struct AuditService {
    store: Arc<dyn AuditStore>,
}

impl AuditService {
    pub fn new(store: Arc<dyn AuditStore>) -> Self {
        Self { store }
    }

    /// Records a system action. Non-fatal: failures surface as a warning so
    /// the producing mutation always completes.
    pub async fn record(&self, now: chrono::DateTime<chrono::Utc>, event: &AuditEvent) {
        let entry = AuditEntry::new(Ulid::new().to_string(), event, now);
        if let Err(e) = self.store.record(&entry).await {
            tracing::warn!(event_type = %entry.event_type, error = %e, "failed to record audit entry");
        }
    }

    /// Lists audit entries visible to a user, newest first.
    pub async fn list(
        &self,
        user_id: &str,
        filters: AuditFilters<'_>,
        limit: i64,
        before_id: Option<&str>,
    ) -> Result<Vec<AuditEntry>, AuthError> {
        self.store
            .list(user_id, filters, limit, before_id)
            .await
            .map_err(Into::into)
    }

    /// Lists all audit entries (admin view), newest first, plus the total
    /// matching count.
    pub async fn list_all(
        &self,
        filters: AdminAuditFilters<'_>,
        limit: i64,
        offset: i64,
    ) -> Result<(Vec<AuditEntry>, i64), AuthError> {
        self.store
            .list_all(filters, limit, offset)
            .await
            .map_err(Into::into)
    }
}
