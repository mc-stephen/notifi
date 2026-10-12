//! Product send records: one row per `POST /v1/notifications` call.
//!
//! Instant sends execute inline and land completed/failed. Scheduled rows,
//! retries, and per-attempt history arrive in Phase B. Framework-free.

/// Send lifecycle. `pending` rows are claimed by a future worker;
/// instant sends move straight through `sending`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SendStatus {
    Pending,
    Sending,
    Completed,
    Failed,
}

impl SendStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Pending => "pending",
            Self::Sending => "sending",
            Self::Completed => "completed",
            Self::Failed => "failed",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "pending" => Some(Self::Pending),
            "sending" => Some(Self::Sending),
            "completed" => Some(Self::Completed),
            "failed" => Some(Self::Failed),
            _ => None,
        }
    }
}

/// One API-triggered send.
#[derive(Debug, Clone)]
pub struct Send {
    pub id: String,
    pub project_id: String,
    pub channel: String,
    pub recipients: Vec<String>,
    pub title: String,
    pub body: String,
    pub send_at: chrono::DateTime<chrono::Utc>,
    pub status: SendStatus,
    pub attempts: i32,
    pub provider: Option<String>,
    pub external_id: Option<String>,
    pub last_error: Option<String>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

/// Input for a new send request.
#[derive(Debug, Clone)]
pub struct SendInput {
    pub channel: String,
    pub recipients: Vec<String>,
    pub title: String,
    pub body: String,
    pub send_at: Option<chrono::DateTime<chrono::Utc>>,
}
