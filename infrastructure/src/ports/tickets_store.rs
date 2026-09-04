use std::future::Future;
use std::pin::Pin;

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

    fn reopen(
        &self,
        actor: UserId,
        ticket_id: &str,
    ) -> BoxFut<'_, Result<bool, StoreError>>;
}
