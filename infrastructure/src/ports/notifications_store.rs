use std::future::Future;
use std::pin::Pin;

use chrono::{DateTime, Utc};

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
}
