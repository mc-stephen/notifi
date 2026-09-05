use std::sync::Arc;

use crate::domain::auth::entities::UserId;
use crate::domain::auth::errors::AuthError;
use crate::domain::notifications::entities::{InAppNotification, NotificationOrigin, NotificationType};
use crate::ports::auth_store::StoreError;
use crate::ports::notifications_store::NotificationsStore;

const MAX_TITLE: usize = 300;
const MAX_CONTENT: usize = 100_000;

pub struct NotificationService {
    store: Arc<dyn NotificationsStore>,
}

impl NotificationService {
    pub fn new(store: Arc<dyn NotificationsStore>) -> Self {
        Self { store }
    }

    pub async fn create_system(
        &self,
        user_id: UserId,
        notification_type: NotificationType,
        title: &str,
        content: &str,
    ) -> Result<InAppNotification, AuthError> {
        self.create_inner(user_id, notification_type, NotificationOrigin::System, title, content)
            .await
    }

    pub async fn create_admin(
        &self,
        user_id: UserId,
        notification_type: NotificationType,
        title: &str,
        content: &str,
    ) -> Result<InAppNotification, AuthError> {
        self.create_inner(user_id, notification_type, NotificationOrigin::Admin, title, content)
            .await
    }

    async fn create_inner(
        &self,
        user_id: UserId,
        notification_type: NotificationType,
        origin: NotificationOrigin,
        title: &str,
        content: &str,
    ) -> Result<InAppNotification, AuthError> {
        let title = title.trim();
        let content = content.trim();

        if title.is_empty() || title.len() > MAX_TITLE {
            return Err(AuthError::Validation(
                "title is required (300 characters max)".to_string(),
            ));
        }
        if content.is_empty() || content.len() > MAX_CONTENT {
            return Err(AuthError::Validation(
                "content is required (100,000 characters max)".to_string(),
            ));
        }

        let record = self
            .store
            .create(user_id, notification_type, origin, title, content)
            .await
            .map_err(map_store_error)?;

        Ok(InAppNotification::from(record))
    }

    pub async fn list(
        &self,
        user_id: UserId,
        unread_only: bool,
        limit: i64,
        before: Option<&str>,
    ) -> Result<Vec<InAppNotification>, AuthError> {
        Ok(self
            .store
            .list(user_id, unread_only, limit, before)
            .await
            .map_err(map_store_error)?
            .into_iter()
            .map(InAppNotification::from)
            .collect())
    }

    pub async fn count_unread(&self, user_id: UserId) -> Result<i64, AuthError> {
        self.store
            .count_unread(user_id)
            .await
            .map_err(map_store_error)
    }

    pub async fn get(
        &self,
        user_id: UserId,
        notification_id: &str,
    ) -> Result<Option<InAppNotification>, AuthError> {
        Ok(self
            .store
            .get(user_id, notification_id)
            .await
            .map_err(map_store_error)?
            .map(InAppNotification::from))
    }

    pub async fn set_read(
        &self,
        user_id: UserId,
        notification_id: &str,
        read: bool,
    ) -> Result<Option<InAppNotification>, AuthError> {
        Ok(self
            .store
            .set_read(user_id, notification_id, read)
            .await
            .map_err(map_store_error)?
            .map(InAppNotification::from))
    }

    pub async fn mark_all_read(
        &self,
        user_id: UserId,
    ) -> Result<i64, AuthError> {
        self.store
            .mark_all_read(user_id)
            .await
            .map_err(map_store_error)
    }

    pub async fn delete(
        &self,
        user_id: UserId,
        notification_id: &str,
    ) -> Result<bool, AuthError> {
        self.store
            .delete(user_id, notification_id)
            .await
            .map_err(map_store_error)
    }
}

fn map_store_error(err: StoreError) -> AuthError {
    match err {
        StoreError::Conflict => AuthError::Conflict(
            "A notification with this ID already exists.".to_string(),
        ),
        StoreError::Storage(m) => AuthError::Storage(m),
    }
}
