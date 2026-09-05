use chrono::{DateTime, Utc};
use sqlx::PgPool;
use ulid::Ulid;

use crate::domain::auth::entities::UserId;
use crate::domain::notifications::entities::{NotificationOrigin, NotificationType};
use crate::ports::auth_store::StoreError;
use crate::ports::notifications_store::{BoxFut, NotificationRecord, NotificationsStore};

pub struct PgNotificationsStore {
    pool: PgPool,
}

impl PgNotificationsStore {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

fn map_err(err: sqlx::Error) -> StoreError {
    if let sqlx::Error::Database(db) = &err
        && let Some(code) = db.code()
        && code == "23505"
    {
        return StoreError::Conflict;
    }
    StoreError::Storage(err.to_string())
}

#[derive(sqlx::FromRow)]
struct NotificationRow {
    id: String,
    user_id: String,
    r#type: String,
    origin: String,
    title: String,
    content: String,
    read_at: Option<DateTime<Utc>>,
    created_at: DateTime<Utc>,
    deleted_at: Option<DateTime<Utc>>,
}

impl From<NotificationRow> for NotificationRecord {
    fn from(row: NotificationRow) -> Self {
        let notification_type = row.r#type.parse::<NotificationType>().unwrap_or_default();
        let origin = row.origin.parse::<NotificationOrigin>().unwrap_or_default();
        Self {
            id: row.id,
            user_id: row.user_id,
            notification_type,
            origin,
            title: row.title,
            content: row.content,
            read_at: row.read_at,
            created_at: row.created_at,
            deleted_at: row.deleted_at,
        }
    }
}

impl NotificationsStore for PgNotificationsStore {
    fn create(
        &self,
        user_id: UserId,
        notification_type: NotificationType,
        origin: NotificationOrigin,
        title: &str,
        content: &str,
    ) -> BoxFut<'_, Result<NotificationRecord, StoreError>> {
        let pool = self.pool.clone();
        let user_str = user_id.to_string();
        let id = Ulid::new().to_string();
        let type_str = notification_type.to_string();
        let origin_str = origin.to_string();
        let title = title.to_string();
        let content = content.to_string();

        Box::pin(async move {
            let row = sqlx::query_as::<_, NotificationRow>(
                "INSERT INTO platform_in_app_notifications (id, user_id, type, origin, title, content)
                 VALUES ($1, $2, $3, $4, $5, $6)
                 RETURNING id, user_id, type, origin, title, content, read_at, created_at, deleted_at",
            )
            .bind(&id)
            .bind(&user_str)
            .bind(&type_str)
            .bind(&origin_str)
            .bind(&title)
            .bind(&content)
            .fetch_optional(&pool)
            .await
            .map_err(map_err)?;

            row.map(NotificationRecord::from)
                .ok_or_else(|| StoreError::Storage("insert returned no row".to_string()))
        })
    }

    fn list(
        &self,
        user_id: UserId,
        unread_only: bool,
        limit: i64,
        before: Option<&str>,
    ) -> BoxFut<'_, Result<Vec<NotificationRecord>, StoreError>> {
        let pool = self.pool.clone();
        let user_str = user_id.to_string();
        let before_owned = before.map(str::to_owned);

        Box::pin(async move {
            let rows = sqlx::query_as::<_, NotificationRow>(
                "SELECT id, user_id, type, origin, title, content, read_at, created_at, deleted_at
                 FROM platform_in_app_notifications
                 WHERE user_id = $1
                   AND deleted_at IS NULL
                   AND ($2 = false OR read_at IS NULL)
                   AND ($3 IS NULL OR id < $3)
                 ORDER BY created_at DESC, id DESC
                 LIMIT $4",
            )
            .bind(&user_str)
            .bind(unread_only)
            .bind(&before_owned)
            .bind(limit)
            .fetch_all(&pool)
            .await
            .map_err(map_err)?;

            Ok(rows.into_iter().map(NotificationRecord::from).collect())
        })
    }

    fn count_unread(
        &self,
        user_id: UserId,
    ) -> BoxFut<'_, Result<i64, StoreError>> {
        let pool = self.pool.clone();
        let user_str = user_id.to_string();

        Box::pin(async move {
            let row = sqlx::query_scalar::<_, i64>(
                "SELECT count(*) FROM platform_in_app_notifications
                 WHERE user_id = $1 AND read_at IS NULL AND deleted_at IS NULL",
            )
            .bind(&user_str)
            .fetch_optional(&pool)
            .await
            .map_err(map_err)?;

            Ok(row.unwrap_or(0))
        })
    }

    fn get(
        &self,
        user_id: UserId,
        notification_id: &str,
    ) -> BoxFut<'_, Result<Option<NotificationRecord>, StoreError>> {
        let pool = self.pool.clone();
        let user_str = user_id.to_string();
        let id = notification_id.to_string();

        Box::pin(async move {
            let row = sqlx::query_as::<_, NotificationRow>(
                "SELECT id, user_id, type, origin, title, content, read_at, created_at, deleted_at
                 FROM platform_in_app_notifications
                 WHERE id = $2 AND user_id = $1 AND deleted_at IS NULL",
            )
            .bind(&user_str)
            .bind(&id)
            .fetch_optional(&pool)
            .await
            .map_err(map_err)?;

            Ok(row.map(NotificationRecord::from))
        })
    }

    fn set_read(
        &self,
        user_id: UserId,
        notification_id: &str,
        read: bool,
    ) -> BoxFut<'_, Result<Option<NotificationRecord>, StoreError>> {
        let pool = self.pool.clone();
        let user_str = user_id.to_string();
        let id = notification_id.to_string();

        Box::pin(async move {
            sqlx::query(
                "UPDATE platform_in_app_notifications
                 SET read_at = CASE WHEN $3 THEN now() ELSE NULL END
                 WHERE id = $2 AND user_id = $1 AND deleted_at IS NULL",
            )
            .bind(&user_str)
            .bind(&id)
            .bind(read)
            .execute(&pool)
            .await
            .map_err(map_err)?;

            let row = sqlx::query_as::<_, NotificationRow>(
                "SELECT id, user_id, type, origin, title, content, read_at, created_at, deleted_at
                 FROM platform_in_app_notifications
                 WHERE id = $2 AND user_id = $1 AND deleted_at IS NULL",
            )
            .bind(&user_str)
            .bind(&id)
            .fetch_optional(&pool)
            .await
            .map_err(map_err)?;

            Ok(row.map(NotificationRecord::from))
        })
    }

    fn mark_all_read(
        &self,
        user_id: UserId,
    ) -> BoxFut<'_, Result<i64, StoreError>> {
        let pool = self.pool.clone();
        let user_str = user_id.to_string();

        Box::pin(async move {
            let result = sqlx::query(
                "UPDATE platform_in_app_notifications
                 SET read_at = now()
                 WHERE user_id = $1 AND read_at IS NULL AND deleted_at IS NULL",
            )
            .bind(&user_str)
            .execute(&pool)
            .await
            .map_err(map_err)?;

            Ok(result.rows_affected() as i64)
        })
    }

    fn delete(
        &self,
        user_id: UserId,
        notification_id: &str,
    ) -> BoxFut<'_, Result<bool, StoreError>> {
        let pool = self.pool.clone();
        let user_str = user_id.to_string();
        let id = notification_id.to_string();

        Box::pin(async move {
            let result = sqlx::query(
                "UPDATE platform_in_app_notifications
                 SET deleted_at = now()
                 WHERE id = $2 AND user_id = $1 AND deleted_at IS NULL",
            )
            .bind(&user_str)
            .bind(&id)
            .execute(&pool)
            .await
            .map_err(map_err)?;

            Ok(result.rows_affected() > 0)
        })
    }
}
