use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::domain::support::entities::{
    AdminTicket, AdminTicketMessage, MessageAuthor, TicketStatus,
};

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AdminTicketDto {
    pub id: String,
    pub project_id: Option<String>,
    pub subject: String,
    pub category: String,
    pub priority: String,
    pub description: String,
    pub status: TicketStatus,
    pub customer_id: String,
    pub customer_name: String,
    pub customer_email: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl From<AdminTicket> for AdminTicketDto {
    fn from(ticket: AdminTicket) -> Self {
        Self {
            id: ticket.id,
            project_id: ticket.project_id,
            subject: ticket.subject,
            category: ticket.category,
            priority: ticket.priority,
            description: ticket.description,
            status: ticket.status,
            customer_id: ticket.customer_id,
            customer_name: ticket.customer_name,
            customer_email: ticket.customer_email,
            created_at: ticket.created_at,
            updated_at: ticket.updated_at,
        }
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AdminTicketMessageDto {
    pub id: String,
    pub ticket_id: String,
    pub author: MessageAuthor,
    pub author_name: Option<String>,
    pub body: String,
    pub created_at: DateTime<Utc>,
}

impl From<AdminTicketMessage> for AdminTicketMessageDto {
    fn from(message: AdminTicketMessage) -> Self {
        Self {
            id: message.id,
            ticket_id: message.ticket_id,
            author: message.author,
            author_name: message.author_name,
            body: message.body,
            created_at: message.created_at,
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SetStatusRequest {
    pub status: String,
}
