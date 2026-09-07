use std::str::FromStr;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TicketStatus {
    Open,
    InProgress,
    Resolved,
    Closed,
}

impl TicketStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Open => "open",
            Self::InProgress => "in_progress",
            Self::Resolved => "resolved",
            Self::Closed => "closed",
        }
    }
}

impl FromStr for TicketStatus {
    type Err = ();
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "open" => Ok(Self::Open),
            "in_progress" => Ok(Self::InProgress),
            "resolved" => Ok(Self::Resolved),
            "closed" => Ok(Self::Closed),
            _ => Err(()),
        }
    }
}

impl std::fmt::Display for TicketStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MessageAuthor {
    Customer,
    Support,
}

impl MessageAuthor {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Customer => "customer",
            Self::Support => "support",
        }
    }
}

impl FromStr for MessageAuthor {
    type Err = ();
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "customer" => Ok(Self::Customer),
            "support" => Ok(Self::Support),
            _ => Err(()),
        }
    }
}

impl std::fmt::Display for MessageAuthor {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

#[derive(Debug, Clone)]
pub struct Ticket {
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
}

impl From<crate::ports::tickets_store::TicketRecord> for Ticket {
    fn from(record: crate::ports::tickets_store::TicketRecord) -> Self {
        Self {
            id: record.id,
            project_id: record.project_id,
            created_by: record.created_by,
            subject: record.subject,
            category: record.category,
            priority: record.priority,
            description: record.description,
            status: record.status,
            created_at: record.created_at,
            updated_at: record.updated_at,
        }
    }
}

#[derive(Debug, Clone)]
pub struct TicketMessage {
    pub id: String,
    pub ticket_id: String,
    pub author: MessageAuthor,
    pub body: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

impl From<crate::ports::tickets_store::TicketMessageRecord> for TicketMessage {
    fn from(record: crate::ports::tickets_store::TicketMessageRecord) -> Self {
        Self {
            id: record.id,
            ticket_id: record.ticket_id,
            author: record.author,
            body: record.body,
            created_at: record.created_at,
        }
    }
}

/// A ticket plus its creator's identity, for admin views.
#[derive(Debug, Clone)]
pub struct AdminTicket {
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
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

impl From<crate::ports::tickets_store::AdminTicketRecord> for AdminTicket {
    fn from(record: crate::ports::tickets_store::AdminTicketRecord) -> Self {
        Self {
            id: record.ticket.id,
            project_id: record.ticket.project_id,
            subject: record.ticket.subject,
            category: record.ticket.category,
            priority: record.ticket.priority,
            description: record.ticket.description,
            status: record.ticket.status,
            customer_id: record.ticket.created_by,
            customer_name: record.customer_name,
            customer_email: record.customer_email,
            created_at: record.ticket.created_at,
            updated_at: record.ticket.updated_at,
        }
    }
}

/// A message plus its author's display name, for admin views.
#[derive(Debug, Clone)]
pub struct AdminTicketMessage {
    pub id: String,
    pub ticket_id: String,
    pub author: MessageAuthor,
    pub author_name: Option<String>,
    pub body: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

impl From<crate::ports::tickets_store::AdminTicketMessageRecord> for AdminTicketMessage {
    fn from(record: crate::ports::tickets_store::AdminTicketMessageRecord) -> Self {
        Self {
            id: record.message.id,
            ticket_id: record.message.ticket_id,
            author: record.message.author,
            author_name: record.author_name,
            body: record.message.body,
            created_at: record.message.created_at,
        }
    }
}
