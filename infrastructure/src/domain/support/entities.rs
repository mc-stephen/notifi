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
