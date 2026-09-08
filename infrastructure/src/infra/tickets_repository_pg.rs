use chrono::{DateTime, Utc};
use sqlx::PgPool;
use ulid::Ulid;

use crate::domain::admin::entities::AdminUserId;
use crate::domain::auth::entities::UserId;
use crate::domain::support::entities::{MessageAuthor, TicketStatus};
use crate::ports::auth_store::StoreError;
use crate::ports::tickets_store::{
    AdminTicketMessageRecord, AdminTicketRecord, TicketCounts, TicketMessageRecord, TicketRecord,
    TicketsStore, BoxFut,
};

pub struct PgTicketsStore {
    pool: PgPool,
}

impl PgTicketsStore {
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
struct TicketRow {
    id: String,
    project_id: Option<String>,
    created_by: String,
    subject: String,
    category: String,
    priority: String,
    description: String,
    status: String,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
    deleted_at: Option<DateTime<Utc>>,
}

#[derive(sqlx::FromRow)]
struct AdminTicketRow {
    id: String,
    project_id: Option<String>,
    created_by: String,
    subject: String,
    category: String,
    priority: String,
    description: String,
    status: String,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
    deleted_at: Option<DateTime<Utc>>,
    customer_name: String,
    customer_email: String,
}

#[derive(sqlx::FromRow)]
struct AdminTicketMessageRow {
    id: String,
    ticket_id: String,
    author_type: String,
    author_id: Option<String>,
    body: String,
    created_at: DateTime<Utc>,
    author_name: Option<String>,
}

impl From<AdminTicketRow> for AdminTicketRecord {
    fn from(row: AdminTicketRow) -> Self {
        Self {
            customer_name: row.customer_name,
            customer_email: row.customer_email,
            ticket: TicketRecord {
                id: row.id,
                project_id: row.project_id,
                created_by: row.created_by,
                subject: row.subject,
                category: row.category,
                priority: row.priority,
                description: row.description,
                status: row.status.parse::<TicketStatus>().unwrap_or(TicketStatus::Open),
                created_at: row.created_at,
                updated_at: row.updated_at,
                deleted_at: row.deleted_at,
            },
        }
    }
}

impl From<AdminTicketMessageRow> for AdminTicketMessageRecord {
    fn from(row: AdminTicketMessageRow) -> Self {
        Self {
            author_name: row.author_name,
            message: TicketMessageRecord {
                id: row.id,
                ticket_id: row.ticket_id,
                author: row
                    .author_type
                    .parse::<MessageAuthor>()
                    .unwrap_or(MessageAuthor::Customer),
                author_id: row.author_id,
                body: row.body,
                created_at: row.created_at,
            },
        }
    }
}

#[derive(sqlx::FromRow)]
struct TicketMessageRow {
    id: String,
    ticket_id: String,
    author_type: String,
    author_id: Option<String>,
    body: String,
    created_at: DateTime<Utc>,
}

impl From<TicketMessageRow> for TicketMessageRecord {
    fn from(row: TicketMessageRow) -> Self {
        let author = row.author_type.parse::<MessageAuthor>().unwrap_or(MessageAuthor::Customer);
        Self {
            id: row.id,
            ticket_id: row.ticket_id,
            author,
            author_id: row.author_id,
            body: row.body,
            created_at: row.created_at,
        }
    }
}

impl From<TicketRow> for TicketRecord {
    fn from(row: TicketRow) -> Self {
        let status = row.status.parse::<TicketStatus>().unwrap_or(TicketStatus::Open);
        Self {
            id: row.id,
            project_id: row.project_id,
            created_by: row.created_by,
            subject: row.subject,
            category: row.category,
            priority: row.priority,
            description: row.description,
            status,
            created_at: row.created_at,
            updated_at: row.updated_at,
            deleted_at: row.deleted_at,
        }
    }
}

impl TicketsStore for PgTicketsStore {
    fn create(
        &self,
        actor: UserId,
        project_id: Option<&str>,
        subject: &str,
        category: &str,
        priority: &str,
        description: &str,
    ) -> BoxFut<'_, Result<TicketRecord, StoreError>> {
        let pool = self.pool.clone();
        let actor_str = actor.to_string();
        let ticket_id = Ulid::new().to_string();
        let project_id_owned = project_id.map(str::to_owned);
        let subject = subject.to_string();
        let category = category.to_string();
        let priority = priority.to_string();
        let description = description.to_string();

        Box::pin(async move {
            let row = sqlx::query_as::<_, TicketRow>(
                "INSERT INTO platform_support_tickets (id, project_id, created_by, subject, category, priority, description, status)
                 SELECT $2, $3, $1, $4, $5, $6, $7, 'open'
                 WHERE $3::varchar IS NULL
                    OR EXISTS (
                        SELECT 1 FROM platform_projects p
                        WHERE p.id = $3
                          AND p.deleted_at IS NULL
                          AND (
                               p.created_by = $1
                               OR EXISTS (
                                   SELECT 1 FROM platform_project_members pm
                                   WHERE pm.project_id = p.id
                                     AND pm.user_id = $1
                                     AND pm.deleted_at IS NULL
                               )
                          )
                    )
                 RETURNING id, project_id, created_by, subject, category, priority, description, status, created_at, updated_at, deleted_at",
            )
            .bind(&actor_str)
            .bind(&ticket_id)
            .bind(&project_id_owned)
            .bind(&subject)
            .bind(&category)
            .bind(&priority)
            .bind(&description)
            .fetch_optional(&pool)
            .await
            .map_err(map_err)?;

            match row {
                Some(row) => Ok(row.into()),
                None => Err(StoreError::Storage(
                    "project not found or not visible".to_string(),
                )),
            }
        })
    }

    fn list(
        &self,
        actor: UserId,
        project_id: Option<&str>,
        status: Option<&str>,
        limit: i64,
        before: Option<&str>,
    ) -> BoxFut<'_, Result<Vec<TicketRecord>, StoreError>> {
        let pool = self.pool.clone();
        let actor_str = actor.to_string();
        let project_id_owned = project_id.map(str::to_owned);
        let status_owned = status.map(str::to_owned);
        let before_owned = before.map(str::to_owned);

        Box::pin(async move {
            let rows = sqlx::query_as::<_, TicketRow>(
                "SELECT id, project_id, created_by, subject, category, priority, description, status, created_at, updated_at, deleted_at
                 FROM platform_support_tickets
                 WHERE deleted_at IS NULL
                   AND ($2::text IS NULL OR status = $2)
                   AND ($3::text IS NULL OR id < $3)
                   AND ($5::text IS NULL OR project_id = $5)
                   AND (
                        project_id IS NULL AND created_by = $1
                        OR (
                            project_id IS NOT NULL
                            AND EXISTS (
                                SELECT 1 FROM platform_projects p
                                WHERE p.id = platform_support_tickets.project_id
                                  AND p.deleted_at IS NULL
                                  AND (
                                       p.created_by = $1
                                       OR EXISTS (
                                           SELECT 1 FROM platform_project_members pm
                                           WHERE pm.project_id = p.id
                                             AND pm.user_id = $1
                                             AND pm.deleted_at IS NULL
                                       )
                                  )
                            )
                        )
                   )
                 ORDER BY created_at DESC, id DESC
                 LIMIT $4",
            )
            .bind(&actor_str)
            .bind(&status_owned)
            .bind(&before_owned)
            .bind(limit)
            .bind(&project_id_owned)
            .fetch_all(&pool)
            .await
            .map_err(map_err)?;

            Ok(rows.into_iter().map(TicketRecord::from).collect())
        })
    }

    fn get(
        &self,
        actor: UserId,
        ticket_id: &str,
    ) -> BoxFut<'_, Result<Option<TicketRecord>, StoreError>> {
        let pool = self.pool.clone();
        let actor_str = actor.to_string();
        let ticket_id_owned = ticket_id.to_string();

        Box::pin(async move {
            let row = sqlx::query_as::<_, TicketRow>(
                "SELECT id, project_id, created_by, subject, category, priority, description, status, created_at, updated_at, deleted_at
                 FROM platform_support_tickets
                 WHERE id = $2
                   AND deleted_at IS NULL
                   AND (
                        project_id IS NULL AND created_by = $1
                        OR (
                            project_id IS NOT NULL
                            AND EXISTS (
                                SELECT 1 FROM platform_projects p
                                WHERE p.id = platform_support_tickets.project_id
                                  AND p.deleted_at IS NULL
                                  AND (
                                       p.created_by = $1
                                       OR EXISTS (
                                           SELECT 1 FROM platform_project_members pm
                                           WHERE pm.project_id = p.id
                                             AND pm.user_id = $1
                                             AND pm.deleted_at IS NULL
                                       )
                                  )
                            )
                        )
                   )",
            )
            .bind(&actor_str)
            .bind(&ticket_id_owned)
            .fetch_optional(&pool)
            .await
            .map_err(map_err)?;

            Ok(row.map(TicketRecord::from))
        })
    }

    fn list_messages(
        &self,
        actor: UserId,
        ticket_id: &str,
    ) -> BoxFut<'_, Result<Vec<TicketMessageRecord>, StoreError>> {
        let pool = self.pool.clone();
        let actor_str = actor.to_string();
        let ticket_id_owned = ticket_id.to_string();

        Box::pin(async move {
            let rows = sqlx::query_as::<_, TicketMessageRow>(
                "SELECT m.id, m.ticket_id, m.author_type, m.author_id, m.body, m.created_at
                 FROM platform_support_ticket_messages m
                 JOIN platform_support_tickets t ON t.id = m.ticket_id
                 WHERE m.ticket_id = $2
                   AND t.deleted_at IS NULL
                   AND (
                        t.project_id IS NULL AND t.created_by = $1
                        OR (
                            t.project_id IS NOT NULL
                            AND EXISTS (
                                SELECT 1 FROM platform_projects p
                                WHERE p.id = t.project_id
                                  AND p.deleted_at IS NULL
                                  AND (
                                       p.created_by = $1
                                       OR EXISTS (
                                           SELECT 1 FROM platform_project_members pm
                                           WHERE pm.project_id = p.id
                                             AND pm.user_id = $1
                                             AND pm.deleted_at IS NULL
                                       )
                                  )
                            )
                        )
                   )
                 ORDER BY m.created_at ASC, m.id ASC",
            )
            .bind(&actor_str)
            .bind(&ticket_id_owned)
            .fetch_all(&pool)
            .await
            .map_err(map_err)?;

            Ok(rows.into_iter().map(TicketMessageRecord::from).collect())
        })
    }

    fn add_message(
        &self,
        actor: UserId,
        ticket_id: &str,
        body: &str,
    ) -> BoxFut<'_, Result<Option<TicketMessageRecord>, StoreError>> {
        let pool = self.pool.clone();
        let actor_str = actor.to_string();
        let ticket_id_owned = ticket_id.to_string();
        let message_id = Ulid::new().to_string();
        let body_owned = body.to_string();

        Box::pin(async move {
            let row = sqlx::query_as::<_, TicketMessageRow>(
                "WITH ticket_check AS (
                    SELECT id, status FROM platform_support_tickets
                    WHERE id = $2
                      AND deleted_at IS NULL
                      AND (
                           project_id IS NULL AND created_by = $1
                           OR (
                               project_id IS NOT NULL
                               AND EXISTS (
                                   SELECT 1 FROM platform_projects p
                                   WHERE p.id = platform_support_tickets.project_id
                                     AND p.deleted_at IS NULL
                                     AND (
                                          p.created_by = $1
                                          OR EXISTS (
                                              SELECT 1 FROM platform_project_members pm
                                              WHERE pm.project_id = p.id
                                                AND pm.user_id = $1
                                                AND pm.deleted_at IS NULL
                                          )
                                     )
                               )
                           )
                      )
                ),
                ins AS (
                    INSERT INTO platform_support_ticket_messages (id, ticket_id, author_type, author_id, body)
                    SELECT $3, $2, 'customer', $1, $4
                    FROM ticket_check
                    RETURNING id, ticket_id, author_type, author_id, body, created_at
                )
                UPDATE platform_support_tickets t
                SET updated_at = now()
                FROM ins
                WHERE t.id = ins.ticket_id
                RETURNING ins.id, ins.ticket_id, ins.author_type, ins.author_id, ins.body, ins.created_at",
            )
            .bind(&actor_str)
            .bind(&ticket_id_owned)
            .bind(&message_id)
            .bind(&body_owned)
            .fetch_optional(&pool)
            .await
            .map_err(map_err)?;

            Ok(row.map(TicketMessageRecord::from))
        })
    }

    fn reopen(
        &self,
        actor: UserId,
        ticket_id: &str,
    ) -> BoxFut<'_, Result<bool, StoreError>> {
        let pool = self.pool.clone();
        let actor_str = actor.to_string();
        let ticket_id_owned = ticket_id.to_string();

        Box::pin(async move {
            let result = sqlx::query(
                "UPDATE platform_support_tickets
                 SET status = 'open', updated_at = now()
                 WHERE id = $2
                   AND status = 'resolved'
                   AND deleted_at IS NULL
                   AND (
                        project_id IS NULL AND created_by = $1
                        OR (
                            project_id IS NOT NULL
                            AND EXISTS (
                                SELECT 1 FROM platform_projects p
                                WHERE p.id = platform_support_tickets.project_id
                                  AND p.deleted_at IS NULL
                                  AND (
                                       p.created_by = $1
                                       OR EXISTS (
                                           SELECT 1 FROM platform_project_members pm
                                           WHERE pm.project_id = p.id
                                             AND pm.user_id = $1
                                             AND pm.deleted_at IS NULL
                                       )
                                  )
                            )
                        )
                   )",
            )
            .bind(&actor_str)
            .bind(&ticket_id_owned)
            .execute(&pool)
            .await
            .map_err(map_err)?;

            Ok(result.rows_affected() > 0)
        })
    }

    // === Admin-scoped methods (no actor visibility checks) =================

    fn list_all(
        &self,
        status: Option<&str>,
        limit: i64,
        before: Option<&str>,
    ) -> BoxFut<'_, Result<Vec<AdminTicketRecord>, StoreError>> {
        let pool = self.pool.clone();
        let status_owned = status.map(str::to_owned);
        let before_owned = before.map(str::to_owned);

        Box::pin(async move {
            let rows = sqlx::query_as::<_, AdminTicketRow>(
                "SELECT t.id, t.project_id, t.created_by, t.subject, t.category, t.priority,
                        t.description, t.status, t.created_at, t.updated_at, t.deleted_at,
                        u.name AS customer_name, u.email AS customer_email
                 FROM platform_support_tickets t
                 JOIN auth_users u ON u.id = t.created_by
                 WHERE t.deleted_at IS NULL
                   AND ($1::text IS NULL OR t.status = $1)
                   AND ($2::text IS NULL OR t.id < $2)
                 ORDER BY t.created_at DESC, t.id DESC
                 LIMIT $3",
            )
            .bind(&status_owned)
            .bind(&before_owned)
            .bind(limit)
            .fetch_all(&pool)
            .await
            .map_err(map_err)?;

            Ok(rows.into_iter().map(AdminTicketRecord::from).collect())
        })
    }

    fn get_any(&self, ticket_id: &str) -> BoxFut<'_, Result<Option<AdminTicketRecord>, StoreError>> {
        let pool = self.pool.clone();
        let ticket_id_owned = ticket_id.to_string();

        Box::pin(async move {
            let row = sqlx::query_as::<_, AdminTicketRow>(
                "SELECT t.id, t.project_id, t.created_by, t.subject, t.category, t.priority,
                        t.description, t.status, t.created_at, t.updated_at, t.deleted_at,
                        u.name AS customer_name, u.email AS customer_email
                 FROM platform_support_tickets t
                 JOIN auth_users u ON u.id = t.created_by
                 WHERE t.id = $1
                   AND t.deleted_at IS NULL",
            )
            .bind(&ticket_id_owned)
            .fetch_optional(&pool)
            .await
            .map_err(map_err)?;

            Ok(row.map(AdminTicketRecord::from))
        })
    }

    fn list_messages_any(
        &self,
        ticket_id: &str,
    ) -> BoxFut<'_, Result<Vec<AdminTicketMessageRecord>, StoreError>> {
        let pool = self.pool.clone();
        let ticket_id_owned = ticket_id.to_string();

        Box::pin(async move {
            let rows = sqlx::query_as::<_, AdminTicketMessageRow>(
                "SELECT m.id, m.ticket_id, m.author_type, m.author_id, m.body, m.created_at,
                        COALESCE(u.name, a.name) AS author_name
                 FROM platform_support_ticket_messages m
                 JOIN platform_support_tickets t ON t.id = m.ticket_id
                 LEFT JOIN auth_users u
                   ON m.author_type = 'customer' AND m.author_id = u.id
                 LEFT JOIN admin_users a
                   ON m.author_type = 'support' AND m.author_id = a.id
                 WHERE m.ticket_id = $1
                   AND t.deleted_at IS NULL
                 ORDER BY m.created_at ASC, m.id ASC",
            )
            .bind(&ticket_id_owned)
            .fetch_all(&pool)
            .await
            .map_err(map_err)?;

            Ok(rows.into_iter().map(AdminTicketMessageRecord::from).collect())
        })
    }

    fn add_support_message(
        &self,
        admin_id: AdminUserId,
        ticket_id: &str,
        body: &str,
    ) -> BoxFut<'_, Result<Option<TicketMessageRecord>, StoreError>> {
        let pool = self.pool.clone();
        let admin_id_str = admin_id.to_string();
        let ticket_id_owned = ticket_id.to_string();
        let message_id = Ulid::new().to_string();
        let body_owned = body.to_string();

        Box::pin(async move {
            let row = sqlx::query_as::<_, TicketMessageRow>(
                "WITH ticket_check AS (
                     SELECT id FROM platform_support_tickets
                     WHERE id = $2 AND deleted_at IS NULL
                 ),
                 ins AS (
                     INSERT INTO platform_support_ticket_messages (id, ticket_id, author_type, author_id, body)
                     SELECT $3, $2, 'support', $1, $4
                     FROM ticket_check
                     RETURNING id, ticket_id, author_type, author_id, body, created_at
                 )
                 UPDATE platform_support_tickets t
                 SET updated_at = now()
                 FROM ins
                 WHERE t.id = ins.ticket_id
                 RETURNING ins.id, ins.ticket_id, ins.author_type, ins.author_id, ins.body, ins.created_at",
            )
            .bind(&admin_id_str)
            .bind(&ticket_id_owned)
            .bind(&message_id)
            .bind(&body_owned)
            .fetch_optional(&pool)
            .await
            .map_err(map_err)?;

            Ok(row.map(TicketMessageRecord::from))
        })
    }

    fn set_status(
        &self,
        ticket_id: &str,
        status: TicketStatus,
    ) -> BoxFut<'_, Result<bool, StoreError>> {
        let pool = self.pool.clone();
        let ticket_id_owned = ticket_id.to_string();
        let status_str = status.as_str().to_string();

        Box::pin(async move {
            let result = sqlx::query(
                "UPDATE platform_support_tickets
                 SET status = $2, updated_at = now()
                 WHERE id = $1 AND deleted_at IS NULL",
            )
            .bind(&ticket_id_owned)
            .bind(&status_str)
            .execute(&pool)
            .await
            .map_err(map_err)?;

            Ok(result.rows_affected() > 0)
        })
    }

    fn count_tickets_for_user(
        &self,
        user_id: &str,
    ) -> BoxFut<'_, Result<TicketCounts, StoreError>> {
        let pool = self.pool.clone();
        let user_id_owned = user_id.to_string();

        Box::pin(async move {
            let row = sqlx::query_as::<_, (i64, i64)>(
                "SELECT count(*),
                        count(*) FILTER (WHERE status IN ('open', 'in_progress'))
                 FROM platform_support_tickets
                 WHERE created_by = $1 AND deleted_at IS NULL",
            )
            .bind(&user_id_owned)
            .fetch_optional(&pool)
            .await
            .map_err(map_err)?;

            let (total, open) = row.unwrap_or((0, 0));
            Ok(TicketCounts { total, open })
        })
    }
}
