//! sqlx implementation of [`RecipientsStore`] (PostgreSQL).
//!
//! Recipients live in `platform_recipients` (see `0003_recipients.sql`). Every
//! query is scoped to a project the caller owns or belongs to, and soft-deleted
//! rows are invisible everywhere.

use chrono::{DateTime, Utc};
use sqlx::PgPool;
use ulid::Ulid;

use crate::domain::auth::entities::UserId;
use crate::ports::auth_store::StoreError;
use crate::ports::recipients_store::{RecipientRecord, RecipientsStore, BoxFut};

pub struct PgRecipientsStore {
    pool: PgPool,
}

impl PgRecipientsStore {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

/// Maps driver errors; unique violations become [`StoreError::Conflict`].
fn map_err(err: sqlx::Error) -> StoreError {
    if let sqlx::Error::Database(db) = &err
        && let Some(code) = db.code()
        && code == "23505"
    {
        return StoreError::Conflict;
    }
    StoreError::Storage(err.to_string())
}

/// Project-scoped visibility predicate: the caller owns the project or is an
/// active member. Reused across all recipient queries.
const VISIBLE_PROJECT: &str = r#"
    EXISTS (
        SELECT 1 FROM platform_projects p
        WHERE p.id = platform_recipients.project_id
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
"#;

#[derive(sqlx::FromRow)]
struct RecipientRow {
    id: String,
    project_id: String,
    user_id: String,
    name: String,
    contacts: serde_json::Value,
    created_at: DateTime<Utc>,
}

impl From<RecipientRow> for RecipientRecord {
    fn from(row: RecipientRow) -> Self {
        Self {
            id: row.id,
            project_id: row.project_id,
            user_id: row.user_id,
            name: row.name,
            contacts: row.contacts,
            created_at: row.created_at,
        }
    }
}

impl RecipientsStore for PgRecipientsStore {
    fn create(
        &self,
        actor: UserId,
        project_id: &str,
        user_id: &str,
        name: &str,
        contacts: serde_json::Value,
    ) -> BoxFut<'_, Result<RecipientRecord, StoreError>> {
        let pool = self.pool.clone();
        let project_id = project_id.to_string();
        let user_id = user_id.to_string();
        let name = name.to_string();
        Box::pin(async move {
            let recipient_id = Ulid::new().to_string();
            let row = sqlx::query_as::<_, RecipientRow>(
                "INSERT INTO platform_recipients (id, project_id, user_id, name, contacts, created_by)
                 SELECT $2, $3, $4, $5, $6, $1
                 WHERE EXISTS (
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
                 RETURNING id, project_id, user_id, name, contacts, created_at",
            )
            .bind(actor.to_string())
            .bind(&recipient_id)
            .bind(&project_id)
            .bind(&user_id)
            .bind(&name)
            .bind(&contacts)
            .fetch_optional(&pool)
            .await
            .map_err(map_err)?;

            match row {
                Some(row) => Ok(row.into()),
                // The project isn't visible to the actor.
                None => Err(StoreError::Storage(
                    "project not found or not visible".to_string(),
                )),
            }
        })
    }

    fn list(
        &self,
        actor: UserId,
        project_id: &str,
        search: Option<&str>,
        limit: i64,
        before: Option<&str>,
    ) -> BoxFut<'_, Result<Vec<RecipientRecord>, StoreError>> {
        let pool = self.pool.clone();
        let project_id = project_id.to_string();
        let search = search.map(str::to_owned);
        let before = before.map(str::to_owned);
        Box::pin(async move {
            let rows = sqlx::query_as::<_, RecipientRow>(
                &format!(
                    "SELECT id, project_id, user_id, name, contacts, created_at
                     FROM platform_recipients
                     WHERE project_id = $2
                       AND deleted_at IS NULL
                       AND {VISIBLE_PROJECT}
                       AND ($3::text IS NULL OR name ILIKE '%' || $3 || '%'
                            OR user_id ILIKE '%' || $3 || '%')
                       AND ($4::text IS NULL OR id < $4)
                     ORDER BY created_at DESC, id DESC
                     LIMIT $5"
                ),
            )
            .bind(actor.to_string())
            .bind(&project_id)
            .bind(&search)
            .bind(&before)
            .bind(limit)
            .fetch_all(&pool)
            .await
            .map_err(map_err)?;

            Ok(rows.into_iter().map(RecipientRecord::from).collect())
        })
    }

    fn get(
        &self,
        actor: UserId,
        project_id: &str,
        recipient_id: &str,
    ) -> BoxFut<'_, Result<Option<RecipientRecord>, StoreError>> {
        let pool = self.pool.clone();
        let project_id = project_id.to_string();
        let recipient_id = recipient_id.to_string();
        Box::pin(async move {
            let row = sqlx::query_as::<_, RecipientRow>(
                &format!(
                    "SELECT id, project_id, user_id, name, contacts, created_at
                     FROM platform_recipients
                     WHERE id = $2
                       AND project_id = $3
                       AND deleted_at IS NULL
                       AND {VISIBLE_PROJECT}"
                ),
            )
            .bind(actor.to_string())
            .bind(&recipient_id)
            .bind(&project_id)
            .fetch_optional(&pool)
            .await
            .map_err(map_err)?;

            Ok(row.map(RecipientRecord::from))
        })
    }

    fn update(
        &self,
        actor: UserId,
        project_id: &str,
        recipient_id: &str,
        name: &str,
        contacts: serde_json::Value,
    ) -> BoxFut<'_, Result<Option<RecipientRecord>, StoreError>> {
        let pool = self.pool.clone();
        let project_id = project_id.to_string();
        let recipient_id = recipient_id.to_string();
        let name = name.to_string();
        Box::pin(async move {
            let row = sqlx::query_as::<_, RecipientRow>(
                &format!(
                    "UPDATE platform_recipients
                     SET name = $4, contacts = $5, updated_at = now()
                     WHERE id = $2
                       AND project_id = $3
                       AND deleted_at IS NULL
                       AND {VISIBLE_PROJECT}
                     RETURNING id, project_id, user_id, name, contacts, created_at"
                ),
            )
            .bind(actor.to_string())
            .bind(&recipient_id)
            .bind(&project_id)
            .bind(&name)
            .bind(&contacts)
            .fetch_optional(&pool)
            .await
            .map_err(map_err)?;

            Ok(row.map(RecipientRecord::from))
        })
    }

    fn remove(
        &self,
        actor: UserId,
        project_id: &str,
        recipient_id: &str,
    ) -> BoxFut<'_, Result<bool, StoreError>> {
        let pool = self.pool.clone();
        let project_id = project_id.to_string();
        let recipient_id = recipient_id.to_string();
        Box::pin(async move {
            let result = sqlx::query(
                &format!(
                    "UPDATE platform_recipients
                     SET deleted_at = now(), updated_at = now()
                     WHERE id = $2
                       AND project_id = $3
                       AND deleted_at IS NULL
                       AND {VISIBLE_PROJECT}"
                ),
            )
            .bind(actor.to_string())
            .bind(&recipient_id)
            .bind(&project_id)
            .execute(&pool)
            .await
            .map_err(map_err)?;

            Ok(result.rows_affected() > 0)
        })
    }
}
