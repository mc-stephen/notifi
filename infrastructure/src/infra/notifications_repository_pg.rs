use chrono::{DateTime, Utc};
use sqlx::PgPool;
use ulid::Ulid;

use crate::domain::auth::entities::UserId;
use crate::domain::notifications::entities::{NotificationOrigin, NotificationType};
use crate::ports::auth_store::StoreError;
use crate::ports::notifications_store::{
    BoxFut, BroadcastRecipient, BroadcastRecord, BroadcastStats, BroadcastStatus,
    NotificationRecord, NotificationsStore,
};

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
    broadcast_id: Option<String>,
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
            broadcast_id: row.broadcast_id,
        }
    }
}

impl TryFrom<BroadcastRow> for BroadcastRecord {
    type Error = StoreError;

    fn try_from(row: BroadcastRow) -> Result<Self, Self::Error> {
        Ok(Self {
            id: row.id,
            admin_id: row.admin_id,
            admin_email: row.admin_email,
            notification_type: row.r#type.parse::<NotificationType>().unwrap_or_default(),
            title: row.title,
            content: row.content,
            audience: row.audience,
            channels: row.channels,
            status: row.status.parse::<BroadcastStatus>().map_err(|_| {
                StoreError::Storage(format!("invalid broadcast status: {}", row.status))
            })?,
            scheduled_for: row.scheduled_for,
            sent_at: row.sent_at,
            recipient_count: row.recipient_count,
            created_at: row.created_at,
        })
    }
}

#[derive(sqlx::FromRow)]
struct BroadcastRow {
    id: String,
    admin_id: String,
    admin_email: String,
    r#type: String,
    title: String,
    content: String,
    audience: serde_json::Value,
    channels: Vec<String>,
    status: String,
    scheduled_for: Option<DateTime<Utc>>,
    sent_at: Option<DateTime<Utc>>,
    recipient_count: i64,
    created_at: DateTime<Utc>,
    total_count: Option<i64>,
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
                 RETURNING id, user_id, type, origin, title, content, read_at, created_at, deleted_at, broadcast_id",
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
                "SELECT id, user_id, type, origin, title, content, read_at, created_at, deleted_at, broadcast_id
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

    fn count_unread(&self, user_id: UserId) -> BoxFut<'_, Result<i64, StoreError>> {
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

    fn count_all_for_user(&self, user_id: UserId) -> BoxFut<'_, Result<i64, StoreError>> {
        let pool = self.pool.clone();
        let user_str = user_id.to_string();

        Box::pin(async move {
            let row = sqlx::query_scalar::<_, i64>(
                "SELECT count(*) FROM platform_in_app_notifications
                 WHERE user_id = $1 AND deleted_at IS NULL",
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
                "SELECT id, user_id, type, origin, title, content, read_at, created_at, deleted_at, broadcast_id
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
                "SELECT id, user_id, type, origin, title, content, read_at, created_at, deleted_at, broadcast_id
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

    fn mark_all_read(&self, user_id: UserId) -> BoxFut<'_, Result<i64, StoreError>> {
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

    // === Admin broadcast methods ===========================================

    fn create_broadcast(&self, broadcast: &BroadcastRecord) -> BoxFut<'_, Result<(), StoreError>> {
        let pool = self.pool.clone();
        let broadcast = broadcast.clone();
        Box::pin(async move {
            sqlx::query(
                "INSERT INTO platform_notification_broadcasts
                     (id, admin_id, admin_email, type, title, content, audience, channels,
                      status, scheduled_for, sent_at, recipient_count, created_at)
                 VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13)",
            )
            .bind(&broadcast.id)
            .bind(&broadcast.admin_id)
            .bind(&broadcast.admin_email)
            .bind(broadcast.notification_type.to_string())
            .bind(&broadcast.title)
            .bind(&broadcast.content)
            .bind(&broadcast.audience)
            .bind(&broadcast.channels)
            .bind(broadcast.status.as_str())
            .bind(broadcast.scheduled_for)
            .bind(broadcast.sent_at)
            .bind(broadcast.recipient_count)
            .bind(broadcast.created_at)
            .execute(&pool)
            .await
            .map_err(map_err)?;
            Ok(())
        })
    }

    fn get_broadcast(&self, id: &str) -> BoxFut<'_, Result<Option<BroadcastRecord>, StoreError>> {
        let pool = self.pool.clone();
        let id = id.to_string();
        Box::pin(async move {
            let row = sqlx::query_as::<_, BroadcastRow>(
                "SELECT id, admin_id, admin_email, type, title, content, audience,
                        channels, status, scheduled_for, sent_at, recipient_count,
                        created_at, NULL::bigint AS total_count
                 FROM platform_notification_broadcasts
                 WHERE id = $1",
            )
            .bind(&id)
            .fetch_optional(&pool)
            .await
            .map_err(map_err)?;
            row.map(BroadcastRecord::try_from).transpose()
        })
    }

    fn list_broadcasts(
        &self,
        status: Option<BroadcastStatus>,
        notification_type: Option<&str>,
        limit: i64,
        offset: i64,
    ) -> BoxFut<'_, Result<(Vec<BroadcastRecord>, i64), StoreError>> {
        let pool = self.pool.clone();
        let status_owned = status.map(|s| s.as_str().to_string());
        let type_owned = notification_type.map(str::to_owned);
        Box::pin(async move {
            let rows = sqlx::query_as::<_, BroadcastRow>(
                "SELECT id, admin_id, admin_email, type, title, content, audience,
                        channels, status, scheduled_for, sent_at, recipient_count,
                        created_at, COUNT(*) OVER() AS total_count
                 FROM platform_notification_broadcasts
                 WHERE ($1::text IS NULL OR status = $1)
                   AND ($2::text IS NULL OR type = $2)
                 ORDER BY created_at DESC, id DESC
                 LIMIT $3 OFFSET $4",
            )
            .bind(&status_owned)
            .bind(&type_owned)
            .bind(limit)
            .bind(offset)
            .fetch_all(&pool)
            .await
            .map_err(map_err)?;
            let total = rows.first().and_then(|r| r.total_count).unwrap_or(0);
            let records = rows
                .into_iter()
                .map(BroadcastRecord::try_from)
                .collect::<Result<Vec<_>, _>>()?;
            Ok((records, total))
        })
    }

    fn create_many(
        &self,
        broadcast_id: &str,
        notification_type: NotificationType,
        recipients: &[BroadcastRecipient],
    ) -> BoxFut<'_, Result<i64, StoreError>> {
        let pool = self.pool.clone();
        let broadcast_id = broadcast_id.to_string();
        let type_str = notification_type.to_string();
        let ids: Vec<String> = recipients.iter().map(|_| Ulid::new().to_string()).collect();
        let user_ids: Vec<String> = recipients.iter().map(|r| r.user_id.clone()).collect();
        let titles: Vec<String> = recipients.iter().map(|r| r.title.clone()).collect();
        let contents: Vec<String> = recipients.iter().map(|r| r.content.clone()).collect();
        Box::pin(async move {
            if user_ids.is_empty() {
                return Ok(0);
            }
            let result = sqlx::query(
                "INSERT INTO platform_in_app_notifications
                     (id, user_id, type, origin, title, content, broadcast_id)
                 SELECT u_id, u_user, $4, 'admin', u_title, u_content, $5
                 FROM UNNEST($1::varchar[], $2::varchar[], $3::text[], $6::text[])
                   AS t(u_id, u_user, u_title, u_content)",
            )
            .bind(&ids)
            .bind(&user_ids)
            .bind(&titles)
            .bind(&type_str)
            .bind(&broadcast_id)
            .bind(&contents)
            .execute(&pool)
            .await
            .map_err(map_err)?;
            Ok(result.rows_affected() as i64)
        })
    }

    fn claim_due_scheduled(
        &self,
        now: DateTime<Utc>,
        limit: i64,
    ) -> BoxFut<'_, Result<Vec<BroadcastRecord>, StoreError>> {
        let pool = self.pool.clone();
        Box::pin(async move {
            let rows = sqlx::query_as::<_, BroadcastRow>(
                "UPDATE platform_notification_broadcasts
                 SET status = 'sending'
                 WHERE id IN (
                     SELECT id FROM platform_notification_broadcasts
                     WHERE status = 'scheduled' AND scheduled_for <= $1
                     ORDER BY scheduled_for ASC, id ASC
                     LIMIT $2
                     FOR UPDATE SKIP LOCKED
                 )
                 RETURNING id, admin_id, admin_email, type, title, content, audience,
                           channels, status, scheduled_for, sent_at, recipient_count,
                           created_at, NULL::bigint AS total_count",
            )
            .bind(now)
            .bind(limit)
            .fetch_all(&pool)
            .await
            .map_err(map_err)?;
            rows.into_iter()
                .map(BroadcastRecord::try_from)
                .collect::<Result<Vec<_>, _>>()
        })
    }

    fn mark_broadcast_sent(
        &self,
        id: &str,
        recipient_count: i64,
    ) -> BoxFut<'_, Result<bool, StoreError>> {
        let pool = self.pool.clone();
        let id = id.to_string();
        Box::pin(async move {
            let result = sqlx::query(
                "UPDATE platform_notification_broadcasts
                 SET status = 'sent', sent_at = now(), recipient_count = $2
                 WHERE id = $1 AND status = 'sending'",
            )
            .bind(&id)
            .bind(recipient_count)
            .execute(&pool)
            .await
            .map_err(map_err)?;
            Ok(result.rows_affected() > 0)
        })
    }

    fn cancel_broadcast(&self, id: &str) -> BoxFut<'_, Result<bool, StoreError>> {
        let pool = self.pool.clone();
        let id = id.to_string();
        Box::pin(async move {
            let result = sqlx::query(
                "UPDATE platform_notification_broadcasts
                 SET status = 'cancelled'
                 WHERE id = $1 AND status = 'scheduled'",
            )
            .bind(&id)
            .execute(&pool)
            .await
            .map_err(map_err)?;
            Ok(result.rows_affected() > 0)
        })
    }

    fn broadcast_read_stats(&self, id: &str) -> BoxFut<'_, Result<BroadcastStats, StoreError>> {
        let pool = self.pool.clone();
        let id = id.to_string();
        Box::pin(async move {
            let row = sqlx::query_as::<_, (i64, i64)>(
                "SELECT COUNT(*),
                        COUNT(*) FILTER (WHERE read_at IS NOT NULL)
                 FROM platform_in_app_notifications
                 WHERE broadcast_id = $1 AND deleted_at IS NULL",
            )
            .bind(&id)
            .fetch_optional(&pool)
            .await
            .map_err(map_err)?;
            let (total, read) = row.unwrap_or((0, 0));
            Ok(BroadcastStats { total, read })
        })
    }
}
