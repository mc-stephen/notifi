use std::future::Future;
use std::pin::Pin;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::domain::auth::entities::UserId;
use crate::domain::notifications::entities::{NotificationOrigin, NotificationType};
use crate::ports::auth_store::StoreError;

pub type BoxFut<'a, T> = Pin<Box<dyn Future<Output = T> + Send + 'a>>;

#[derive(Debug, Clone)]
pub struct NotificationRecord {
    pub id: String,
    pub user_id: String,
    pub notification_type: NotificationType,
    pub origin: NotificationOrigin,
    pub title: String,
    pub content: String,
    pub read_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub deleted_at: Option<DateTime<Utc>>,
    pub broadcast_id: Option<String>,
}

/// One row of an admin broadcast batch (materialized at send time).
#[derive(Debug, Clone)]
pub struct BroadcastRecipient {
    pub user_id: String,
    pub title: String,
    pub content: String,
}

/// An admin broadcast record (compose action + schedule state).
#[derive(Debug, Clone)]
pub struct BroadcastRecord {
    pub id: String,
    pub admin_id: String,
    pub admin_email: String,
    pub notification_type: NotificationType,
    pub title: String,
    pub content: String,
    pub audience: serde_json::Value,
    pub channels: Vec<String>,
    pub status: BroadcastStatus,
    pub scheduled_for: Option<DateTime<Utc>>,
    pub sent_at: Option<DateTime<Utc>>,
    pub recipient_count: i64,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BroadcastStatus {
    Scheduled,
    Sending,
    Sent,
    Cancelled,
}

impl BroadcastStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Scheduled => "scheduled",
            Self::Sending => "sending",
            Self::Sent => "sent",
            Self::Cancelled => "cancelled",
        }
    }
}

impl std::str::FromStr for BroadcastStatus {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "scheduled" => Ok(Self::Scheduled),
            "sending" => Ok(Self::Sending),
            "sent" => Ok(Self::Sent),
            "cancelled" => Ok(Self::Cancelled),
            _ => Err(()),
        }
    }
}

/// Read stats for one broadcast.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BroadcastStats {
    pub total: i64,
    pub read: i64,
}

pub trait NotificationsStore: Send + Sync {
    fn create(
        &self,
        user_id: UserId,
        notification_type: NotificationType,
        origin: NotificationOrigin,
        title: &str,
        content: &str,
    ) -> BoxFut<'_, Result<NotificationRecord, StoreError>>;

    fn list(
        &self,
        user_id: UserId,
        unread_only: bool,
        limit: i64,
        before: Option<&str>,
    ) -> BoxFut<'_, Result<Vec<NotificationRecord>, StoreError>>;

    fn count_unread(
        &self,
        user_id: UserId,
    ) -> BoxFut<'_, Result<i64, StoreError>>;

    /// Total non-deleted notifications for a user (admin stats).
    fn count_all_for_user(
        &self,
        user_id: UserId,
    ) -> BoxFut<'_, Result<i64, StoreError>>;

    fn get(
        &self,
        user_id: UserId,
        notification_id: &str,
    ) -> BoxFut<'_, Result<Option<NotificationRecord>, StoreError>>;

    fn set_read(
        &self,
        user_id: UserId,
        notification_id: &str,
        read: bool,
    ) -> BoxFut<'_, Result<Option<NotificationRecord>, StoreError>>;

    fn mark_all_read(
        &self,
        user_id: UserId,
    ) -> BoxFut<'_, Result<i64, StoreError>>;

    fn delete(
        &self,
        user_id: UserId,
        notification_id: &str,
    ) -> BoxFut<'_, Result<bool, StoreError>>;

    // === Admin broadcast methods ===========================================

    /// Persists a broadcast row (status scheduled or sent).
    fn create_broadcast(&self, broadcast: &BroadcastRecord) -> BoxFut<'_, Result<(), StoreError>>;

    /// Fetches one broadcast by id.
    fn get_broadcast(&self, id: &str) -> BoxFut<'_, Result<Option<BroadcastRecord>, StoreError>>;

    /// Broadcasts newest-first with optional status/type filters + totals.
    fn list_broadcasts(
        &self,
        status: Option<BroadcastStatus>,
        notification_type: Option<&str>,
        limit: i64,
        offset: i64,
    ) -> BoxFut<'_, Result<(Vec<BroadcastRecord>, i64), StoreError>>;

    /// Materializes one batch of recipient rows in a single statement.
    fn create_many(
        &self,
        broadcast_id: &str,
        notification_type: NotificationType,
        recipients: &[BroadcastRecipient],
    ) -> BoxFut<'_, Result<i64, StoreError>>;

    /// Atomically claims due scheduled broadcasts (scheduled -> sending)
    /// for the worker. Returns the claimed rows, oldest first.
    fn claim_due_scheduled(
        &self,
        now: DateTime<Utc>,
        limit: i64,
    ) -> BoxFut<'_, Result<Vec<BroadcastRecord>, StoreError>>;

    /// Marks a claimed broadcast sent (sending -> sent, sets sent_at and
    /// recipient count).
    fn mark_broadcast_sent(
        &self,
        id: &str,
        recipient_count: i64,
    ) -> BoxFut<'_, Result<bool, StoreError>>;

    /// Cancels a scheduled broadcast. Returns false unless it was scheduled.
    fn cancel_broadcast(&self, id: &str) -> BoxFut<'_, Result<bool, StoreError>>;

    /// Total vs read recipient rows for a broadcast.
    fn broadcast_read_stats(&self, id: &str) -> BoxFut<'_, Result<BroadcastStats, StoreError>>;
}
