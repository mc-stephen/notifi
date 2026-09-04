use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::domain::support::entities::{MessageAuthor, Ticket, TicketMessage, TicketStatus};

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateTicketRequest {
    #[serde(default)]
    pub project_id: Option<String>,
    pub subject: String,
    pub category: String,
    pub priority: String,
    pub description: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TicketDto {
    pub id: String,
    pub project_id: Option<String>,
    pub subject: String,
    pub category: String,
    pub priority: String,
    pub description: String,
    pub status: TicketStatus,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl From<Ticket> for TicketDto {
    fn from(ticket: Ticket) -> Self {
        Self {
            id: ticket.id,
            project_id: ticket.project_id,
            subject: ticket.subject,
            category: ticket.category,
            priority: ticket.priority,
            description: ticket.description,
            status: ticket.status,
            created_at: ticket.created_at,
            updated_at: ticket.updated_at,
        }
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TicketMessageDto {
    pub id: String,
    pub ticket_id: String,
    pub author: MessageAuthor,
    pub body: String,
    pub created_at: DateTime<Utc>,
}

impl From<TicketMessage> for TicketMessageDto {
    fn from(msg: TicketMessage) -> Self {
        Self {
            id: msg.id,
            ticket_id: msg.ticket_id,
            author: msg.author,
            body: msg.body,
            created_at: msg.created_at,
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SendReplyRequest {
    pub body: String,
}
