use std::sync::Arc;

use serde_json::json;

use crate::domain::admin::entities::AdminUser;
use crate::domain::audit::AuditService;
use crate::domain::audit::entities::{AuditAction, AuditEvent};
use crate::domain::auth::entities::UserId;
use crate::domain::auth::errors::AuthError;
use crate::domain::support::entities::TicketStatus;
use crate::domain::support::entities::{AdminTicket, AdminTicketMessage, Ticket, TicketMessage};
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

    pub async fn get(&self, actor: UserId, ticket_id: &str) -> Result<Option<Ticket>, AuthError> {
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

    // === Admin-scoped methods (callers enforce admin auth) =================

    pub async fn list_all_tickets(
        &self,
        status: Option<&str>,
        search: Option<&str>,
        limit: i64,
        offset: i64,
    ) -> Result<(Vec<AdminTicket>, i64), AuthError> {
        let (records, total) = self
            .store
            .list_all(status, search, limit, offset)
            .await
            .map_err(map_store_error)?;
        Ok((records.into_iter().map(AdminTicket::from).collect(), total))
    }

    pub async fn get_any_ticket(&self, ticket_id: &str) -> Result<Option<AdminTicket>, AuthError> {
        Ok(self
            .store
            .get_any(ticket_id)
            .await
            .map_err(map_store_error)?
            .map(AdminTicket::from))
    }

    pub async fn list_any_messages(
        &self,
        ticket_id: &str,
    ) -> Result<Vec<AdminTicketMessage>, AuthError> {
        Ok(self
            .store
            .list_messages_any(ticket_id)
            .await
            .map_err(map_store_error)?
            .into_iter()
            .map(AdminTicketMessage::from)
            .collect())
    }

    pub async fn add_admin_reply(
        &self,
        admin: &AdminUser,
        ticket_id: &str,
        body: &str,
    ) -> Result<TicketMessage, AuthError> {
        let ticket = self
            .store
            .get_any(ticket_id)
            .await
            .map_err(map_store_error)?
            .ok_or_else(|| AuthError::NotFound("ticket not found".into()))?;

        if ticket.ticket.status == TicketStatus::Closed {
            return Err(AuthError::Conflict(
                "This ticket is closed. Reopen it before replying.".to_string(),
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
            .add_support_message(admin.id, ticket_id, body)
            .await
            .map_err(map_store_error)?
            .ok_or_else(|| AuthError::NotFound("ticket not found".into()))?;

        // A support reply on a resolved ticket resumes work.
        if ticket.ticket.status == TicketStatus::Resolved {
            let _ = self
                .store
                .set_status(ticket_id, TicketStatus::InProgress)
                .await
                .map_err(map_store_error)?;
        }

        let message = TicketMessage::from(record);

        self.audit
            .record(
                chrono::Utc::now(),
                &AuditEvent::new_admin(
                    AuditAction::SupportTicketReplied,
                    &admin.id.to_string(),
                    Some(&admin.name),
                    ticket.ticket.project_id.as_deref(),
                    format!("ticket '{}' replied to by support", ticket.ticket.subject),
                    Some(json!({ "ticket_id": ticket.ticket.id, "message_id": message.id })),
                ),
            )
            .await;

        Ok(message)
    }

    pub async fn set_ticket_status(
        &self,
        admin: &AdminUser,
        ticket_id: &str,
        status: &str,
    ) -> Result<AdminTicket, AuthError> {
        use std::str::FromStr;
        let new_status = TicketStatus::from_str(status.trim()).map_err(|_| {
            AuthError::Validation(
                "invalid status (expected open, in_progress, resolved, or closed)".to_string(),
            )
        })?;

        let ticket = self
            .store
            .get_any(ticket_id)
            .await
            .map_err(map_store_error)?
            .ok_or_else(|| AuthError::NotFound("ticket not found".into()))?;

        if ticket.ticket.status != new_status {
            let updated = self
                .store
                .set_status(ticket_id, new_status)
                .await
                .map_err(map_store_error)?;
            if !updated {
                return Err(AuthError::NotFound("ticket not found".into()));
            }

            self.audit
                .record(
                    chrono::Utc::now(),
                    &AuditEvent::new_admin(
                        AuditAction::SupportTicketStatusChanged,
                        &admin.id.to_string(),
                        Some(&admin.name),
                        ticket.ticket.project_id.as_deref(),
                        format!(
                            "ticket '{}' status changed from {} to {}",
                            ticket.ticket.subject,
                            ticket.ticket.status.as_str(),
                            new_status.as_str(),
                        ),
                        Some(json!({
                            "ticket_id": ticket.ticket.id,
                            "from": ticket.ticket.status.as_str(),
                            "to": new_status.as_str(),
                        })),
                    ),
                )
                .await;
        }

        self.get_any_ticket(ticket_id)
            .await?
            .ok_or_else(|| AuthError::NotFound("ticket not found".into()))
    }
}

fn map_store_error(err: StoreError) -> AuthError {
    match err {
        StoreError::Conflict => {
            AuthError::Conflict("A ticket with this ID already exists.".to_string())
        }
        StoreError::Storage(m) => AuthError::Storage(m),
    }
}
