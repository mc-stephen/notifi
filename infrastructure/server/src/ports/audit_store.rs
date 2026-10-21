//! The persistence port for the audit slice.
//!
//! Append-only by contract: entries are created, never updated/deleted.
//! Implemented by `infra` (Postgres) and `testing` (in-memory fakes), using
//! the same object-safe boxed-future pattern as [`crate::ports::AuthStore`].

use std::future::Future;
use std::pin::Pin;

use crate::domain::audit::entities::AuditEntry;
use crate::ports::auth_store::StoreError;

/// Boxed future returned by every port method.
pub type BoxFut<'a, T> = Pin<Box<dyn Future<Output = T> + Send + 'a>>;

/// Filters for listing audit entries.
#[derive(Debug, Clone, Copy, Default)]
pub struct AuditFilters<'a> {
    /// Restrict to a specific project.
    pub project_id: Option<&'a str>,
    /// Restrict to a single event type.
    pub event_type: Option<&'a str>,
}

/// Filters for the admin-wide listing (no visibility checks).
#[derive(Debug, Clone, Copy, Default)]
pub struct AdminAuditFilters<'a> {
    /// Restrict to a single event type.
    pub event_type: Option<&'a str>,
    /// Restrict to a project.
    pub project_id: Option<&'a str>,
    /// Restrict to an actor type (`user`, `admin`, `system`).
    pub actor_type: Option<&'a str>,
    /// Restrict to one actor (user or admin id).
    pub actor_id: Option<&'a str>,
    /// Case-insensitive substring over message and event type.
    pub search: Option<&'a str>,
}

pub trait AuditStore: Send + Sync {
    /// Appends an audit entry. Callers treat failures as non-fatal so audit
    /// never breaks the user-facing mutation that produced the action.
    fn record(&self, entry: &AuditEntry) -> BoxFut<'_, Result<(), StoreError>>;

    /// Lists audit entries a user may see (their own, plus project-scoped
    /// actions), newest first.
    fn list(
        &self,
        user_id: &str,
        filters: AuditFilters<'_>,
        limit: i64,
        before_id: Option<&str>,
    ) -> BoxFut<'_, Result<Vec<AuditEntry>, StoreError>>;

    /// Lists all audit entries (admin view, no visibility checks), newest
    /// first, plus the total matching count.
    fn list_all(
        &self,
        filters: AdminAuditFilters<'_>,
        limit: i64,
        offset: i64,
    ) -> BoxFut<'_, Result<(Vec<AuditEntry>, i64), StoreError>>;
}
