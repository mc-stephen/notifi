use std::sync::Arc;

use serde_json::json;

use crate::domain::auth::entities::UserId;
use crate::domain::auth::errors::AuthError;
use crate::domain::audit::entities::{AuditAction, AuditEvent};
use crate::domain::audit::AuditService;
use crate::domain::support::entities::{Ticket, TicketMessage};
use crate::ports::auth_store::StoreError;
use crate::ports::tickets_store::TicketsStore;

const MAX_SUBJECT: usize = 300;
const MAX_CATEGORY: usize = 100;
const MAX_PRIORITY: usize = 50;
const MAX_DESCRIPTION: usize = 10_000;
const MAX_MESSAGE: usize = 10_000;

pub struct TicketService {
    store: Arc<dyn TicketsStore>,
    audit: Arc<AuditService>,
}

impl TicketService {
    pub fn new(store: Arc<dyn TicketsStore>, audit: Arc<AuditService>) -> Self {
        Self { store, audit }
    }

    pub async fn create(
        &self,
        actor: UserId,
        project_id: Option<&str>,
        subject: &str,
        category: &str,
        priority: &str,
        description: &str,
    ) -> Result<Ticket, AuthError> {
        let subject = subject.trim();
        let category = category.trim();
        let priority = priority.trim();
        let description = description.trim();

        if subject.is_empty() || subject.len() > MAX_SUBJECT {
            return Err(AuthError::Validation(
                "subject is required (300 characters max)".to_string(),
            ));
        }
        if category.is_empty() || category.len() > MAX_CATEGORY {
            return Err(AuthError::Validation(
                "category is required (100 characters max)".to_string(),
            ));
        }
        if priority.is_empty() || priority.len() > MAX_PRIORITY {
            return Err(AuthError::Validation(
                "priority is required (50 characters max)".to_string(),
            ));
        }
        if description.is_empty() || description.len() > MAX_DESCRIPTION {
            return Err(AuthError::Validation(
                "description is required (10,000 characters max)".to_string(),
            ));
        }

        let record = self
            .store
            .create(actor, project_id, subject, category, priority, description)
            .await
            .map_err(map_store_error)?;

        let ticket = Ticket::from(record);

        self.audit
            .record(
                chrono::Utc::now(),
                &AuditEvent::new(
                    AuditAction::SupportTicketCreated,
                    Some(&actor.to_string()),
                    None,
                    ticket.project_id.as_deref(),
                    format!("support ticket '{}' created", ticket.subject),
                    Some(json!({ "ticket_id": ticket.id })),
                ),
            )
            .await;

        Ok(ticket)
    }

    pub async fn list(
        &self,
        actor: UserId,
        project_id: Option<&str>,
        status: Option<&str>,
        limit: i64,
        before: Option<&str>,
    ) -> Result<Vec<Ticket>, AuthError> {
        Ok(self
            .store
            .list(actor, project_id, status, limit, before)
            .await
            .map_err(map_store_error)?
            .into_iter()
            .map(Ticket::from)
            .collect())
    }

    pub async fn get(
        &self,
        actor: UserId,
        ticket_id: &str,
    ) -> Result<Option<Ticket>, AuthError> {
        Ok(self
            .store
            .get(actor, ticket_id)
            .await
            .map_err(map_store_error)?
            .map(Ticket::from))
    }

    pub async fn list_messages(
        &self,
        actor: UserId,
        ticket_id: &str,
    ) -> Result<Vec<TicketMessage>, AuthError> {
        Ok(self
            .store
            .list_messages(actor, ticket_id)
            .await
            .map_err(map_store_error)?
            .into_iter()
            .map(TicketMessage::from)
            .collect())
    }

    pub async fn add_reply(
        &self,
        actor: UserId,
        ticket_id: &str,
        body: &str,
    ) -> Result<TicketMessage, AuthError> {
        let ticket = self
            .store
            .get(actor, ticket_id)
            .await
            .map_err(map_store_error)?
            .ok_or_else(|| AuthError::NotFound("ticket not found".into()))?;

        if ticket.status == crate::domain::support::entities::TicketStatus::Closed {
            return Err(AuthError::Conflict(
                "This ticket is closed. Please open a new ticket.".to_string(),
            ));
        }

        let body = body.trim();
        if body.is_empty() || body.len() > MAX_MESSAGE {
            return Err(AuthError::Validation(
                "body is required (10,000 characters max)".to_string(),
            ));
        }

        let record = self
            .store
            .add_message(actor, ticket_id, body)
            .await
            .map_err(map_store_error)?
            .ok_or_else(|| AuthError::NotFound("ticket not found".into()))?;

        if ticket.status == crate::domain::support::entities::TicketStatus::Resolved {
            let _ = self
                .store
                .reopen(actor, ticket_id)
                .await
                .map_err(map_store_error)?;
        }

        let message = TicketMessage::from(record);

        self.audit
            .record(
                chrono::Utc::now(),
                &AuditEvent::new(
                    AuditAction::SupportTicketReplied,
                    Some(&actor.to_string()),
                    None,
                    ticket.project_id.as_deref(),
                    format!("ticket '{}' replied to", ticket.subject),
                    Some(json!({ "ticket_id": ticket.id, "message_id": message.id })),
                ),
            )
            .await;

        Ok(message)
    }
}

fn map_store_error(err: StoreError) -> AuthError {
    match err {
        StoreError::Conflict => AuthError::Conflict(
            "A ticket with this ID already exists.".to_string(),
        ),
        StoreError::Storage(m) => AuthError::Storage(m),
    }
}
