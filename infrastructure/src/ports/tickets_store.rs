use std::future::Future;
use std::pin::Pin;

use crate::domain::admin::entities::AdminUserId;
use crate::domain::auth::entities::UserId;
use crate::domain::support::entities::{MessageAuthor, TicketStatus};
use crate::ports::auth_store::StoreError;

pub type BoxFut<'a, T> = Pin<Box<dyn Future<Output = T> + Send + 'a>>;

#[derive(Debug, Clone)]
pub struct TicketRecord {
    pub id: String,
    pub project_id: Option<String>,
    pub created_by: String,
    pub subject: String,
    pub category: String,
    pub priority: String,
    pub description: String,
    pub status: TicketStatus,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
    pub deleted_at: Option<chrono::DateTime<chrono::Utc>>,
}

#[derive(Debug, Clone)]
pub struct TicketMessageRecord {
    pub id: String,
    pub ticket_id: String,
    pub author: MessageAuthor,
    pub author_id: Option<String>,
    pub body: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

/// A ticket plus its creator's identity (admin views only).
#[derive(Debug, Clone)]
pub struct AdminTicketRecord {
    pub ticket: TicketRecord,
    pub customer_name: String,
    pub customer_email: String,
}

/// A message plus its author's display name (admin views only).
#[derive(Debug, Clone)]
pub struct AdminTicketMessageRecord {
    pub message: TicketMessageRecord,
    pub author_name: Option<String>,
}

/// Ticket totals for one user (created-by), for admin stats.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TicketCounts {
    pub total: i64,
    pub open: i64,
}

pub trait TicketsStore: Send + Sync {
    fn create(
        &self,
        actor: UserId,
        project_id: Option<&str>,
        subject: &str,
        category: &str,
        priority: &str,
        description: &str,
    ) -> BoxFut<'_, Result<TicketRecord, StoreError>>;

    fn list(
        &self,
        actor: UserId,
        project_id: Option<&str>,
        status: Option<&str>,
        limit: i64,
        before: Option<&str>,
    ) -> BoxFut<'_, Result<Vec<TicketRecord>, StoreError>>;

    fn get(
        &self,
        actor: UserId,
        ticket_id: &str,
    ) -> BoxFut<'_, Result<Option<TicketRecord>, StoreError>>;

    fn list_messages(
        &self,
        actor: UserId,
        ticket_id: &str,
    ) -> BoxFut<'_, Result<Vec<TicketMessageRecord>, StoreError>>;

    fn add_message(
        &self,
        actor: UserId,
        ticket_id: &str,
        body: &str,
    ) -> BoxFut<'_, Result<Option<TicketMessageRecord>, StoreError>>;

    fn reopen(&self, actor: UserId, ticket_id: &str) -> BoxFut<'_, Result<bool, StoreError>>;

    // === Admin-scoped methods (no actor visibility checks) =================

    /// Lists all tickets (newest first), with creator identity.
    /// `search` matches subject, customer name/email, or id
    /// (case-insensitive, substring). Returns the page plus the total
    /// matching count.
    fn list_all(
        &self,
        status: Option<&str>,
        search: Option<&str>,
        limit: i64,
        offset: i64,
    ) -> BoxFut<'_, Result<(Vec<AdminTicketRecord>, i64), StoreError>>;

    /// Fetches any ticket by id, with creator identity.
    fn get_any(&self, ticket_id: &str)
    -> BoxFut<'_, Result<Option<AdminTicketRecord>, StoreError>>;

    /// Lists messages for any ticket, with author display names.
    fn list_messages_any(
        &self,
        ticket_id: &str,
    ) -> BoxFut<'_, Result<Vec<AdminTicketMessageRecord>, StoreError>>;

    /// Adds a support-authored message to any ticket. Returns `None` when the
    /// ticket does not exist (or is soft-deleted).
    fn add_support_message(
        &self,
        admin_id: AdminUserId,
        ticket_id: &str,
        body: &str,
    ) -> BoxFut<'_, Result<Option<TicketMessageRecord>, StoreError>>;

    /// Sets a ticket's status. Returns `None` when the ticket does not exist
    /// (or is soft-deleted).
    fn set_status(
        &self,
        ticket_id: &str,
        status: TicketStatus,
    ) -> BoxFut<'_, Result<bool, StoreError>>;

    /// Counts non-deleted tickets created by a user (admin stats).
    fn count_tickets_for_user(&self, user_id: &str)
    -> BoxFut<'_, Result<TicketCounts, StoreError>>;
}
