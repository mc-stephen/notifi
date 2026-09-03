//! sqlx implementation of [`TemplatesStore`] (PostgreSQL).
//!
//! Templates live in `platform_templates` with their attachments in
//! `platform_template_attachments` (see `0004_templates.sql`). Every query is
//! scoped to a project the caller owns or belongs to, and soft-deleted rows
//! are invisible everywhere. Attachments are read/rewritten alongside the
//! template (the list is small).

use chrono::{DateTime, Utc};
use sqlx::PgPool;
use ulid::Ulid;

use crate::domain::auth::entities::UserId;
use crate::ports::auth_store::StoreError;
use crate::ports::templates_store::{
    AttachmentInput, AttachmentRecord, BoxFut, TemplateRecord, TemplatesStore,
};

pub struct PgTemplatesStore {
    pool: PgPool,
}

impl PgTemplatesStore {
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
/// active member. Reused across all template queries.
const VISIBLE_PROJECT: &str = r#"
    EXISTS (
        SELECT 1 FROM platform_projects p
        WHERE p.id = platform_templates.project_id
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
struct TemplateRow {
    id: String,
    project_id: String,
    name: String,
    description: Option<String>,
    channel: String,
    content: serde_json::Value,
    version: i32,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

#[derive(sqlx::FromRow)]
struct AttachmentRow {
    id: String,
    name: String,
    mime_type: String,
    size_bytes: i64,
    url: String,
}

impl From<AttachmentRow> for AttachmentRecord {
    fn from(row: AttachmentRow) -> Self {
        Self {
            id: row.id,
            name: row.name,
            mime_type: row.mime_type,
            size_bytes: row.size_bytes,
            url: row.url,
        }
    }
}

impl From<(TemplateRow, Vec<AttachmentRecord>)> for TemplateRecord {
    fn from((row, attachments): (TemplateRow, Vec<AttachmentRecord>)) -> Self {
        Self {
            id: row.id,
            project_id: row.project_id,
            name: row.name,
            description: row.description,
            channel: row.channel,
            content: row.content,
            version: row.version,
            attachments,
            created_at: row.created_at,
            updated_at: row.updated_at,
        }
    }
}

impl PgTemplatesStore {
    async fn load_attachments(
        pool: &PgPool,
        template_id: &str,
    ) -> Result<Vec<AttachmentRecord>, sqlx::Error> {
        let rows = sqlx::query_as::<_, AttachmentRow>(
            "SELECT id, name, mime_type, size_bytes, url
             FROM platform_template_attachments
             WHERE template_id = $1
             ORDER BY created_at, id",
        )
        .bind(template_id)
        .fetch_all(pool)
        .await?;
        Ok(rows.into_iter().map(AttachmentRecord::from).collect())
    }

    async fn replace_attachments(
        pool: &PgPool,
        template_id: &str,
        attachments: Vec<AttachmentInput>,
    ) -> Result<Vec<AttachmentRecord>, sqlx::Error> {
        // Rewrite the whole attachment list for the template.
        sqlx::query("DELETE FROM platform_template_attachments WHERE template_id = $1")
            .bind(template_id)
            .execute(pool)
            .await?;

        let mut inserted = Vec::with_capacity(attachments.len());
        for att in attachments {
            let id = Ulid::new().to_string();
            let row = sqlx::query_as::<_, AttachmentRow>(
                "INSERT INTO platform_template_attachments
                   (id, template_id, name, mime_type, size_bytes, url)
                 VALUES ($1, $2, $3, $4, $5, $6)
                 RETURNING id, name, mime_type, size_bytes, url",
            )
            .bind(&id)
            .bind(template_id)
            .bind(att.name.trim())
            .bind(att.mime_type.trim())
            .bind(att.size_bytes)
            .bind(att.url.trim())
            .fetch_one(pool)
            .await?;
            inserted.push(AttachmentRecord::from(row));
        }
        Ok(inserted)
    }
}

impl TemplatesStore for PgTemplatesStore {
    #[allow(clippy::too_many_arguments)]
    fn create(
        &self,
        actor: UserId,
        project_id: &str,
        name: &str,
        description: Option<&str>,
        channel: &str,
        content: serde_json::Value,
        attachments: Vec<AttachmentInput>,
    ) -> BoxFut<'_, Result<TemplateRecord, StoreError>> {
        let pool = self.pool.clone();
        let actor = actor.to_string();
        let project_id = project_id.to_string();
        let name = name.to_string();
        let description = description.map(str::to_owned);
        let channel = channel.to_string();
        Box::pin(async move {
            let template_id = Ulid::new().to_string();
            let row = sqlx::query_as::<_, TemplateRow>(
                "INSERT INTO platform_templates (id, project_id, name, description, channel, content, created_by)
                 SELECT $2, $3, $4, $5, $6, $7, $1
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
                 RETURNING id, project_id, name, description, channel, content, version, created_at, updated_at",
            )
            .bind(&actor)
            .bind(&template_id)
            .bind(&project_id)
            .bind(&name)
            .bind(&description)
            .bind(&channel)
            .bind(&content)
            .fetch_optional(&pool)
            .await
            .map_err(map_err)?;

            match row {
                Some(row) => {
                    let inserted =
                        PgTemplatesStore::replace_attachments(&pool, &template_id, attachments)
                            .await
                            .map_err(map_err)?;
                    Ok(TemplateRecord::from((row, inserted)))
                }
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
    ) -> BoxFut<'_, Result<Vec<TemplateRecord>, StoreError>> {
        let pool = self.pool.clone();
        let actor = actor.to_string();
        let project_id = project_id.to_string();
        let search = search.map(str::to_owned);
        let before = before.map(str::to_owned);
        Box::pin(async move {
            let rows = sqlx::query_as::<_, TemplateRow>(
                &format!(
                    "SELECT id, project_id, name, description, channel, content, version, created_at, updated_at
                     FROM platform_templates
                     WHERE project_id = $2
                       AND deleted_at IS NULL
                       AND {VISIBLE_PROJECT}
                       AND ($3::text IS NULL OR name ILIKE '%' || $3 || '%')
                       AND ($4::text IS NULL OR id < $4)
                     ORDER BY created_at DESC, id DESC
                     LIMIT $5"
                ),
            )
            .bind(&actor)
            .bind(&project_id)
            .bind(&search)
            .bind(&before)
            .bind(limit)
            .fetch_all(&pool)
            .await
            .map_err(map_err)?;

            let mut out = Vec::with_capacity(rows.len());
            for row in rows {
                let attachments = PgTemplatesStore::load_attachments(&pool, &row.id)
                    .await
                    .map_err(map_err)?;
                out.push(TemplateRecord::from((row, attachments)));
            }
            Ok(out)
        })
    }

    fn get(
        &self,
        actor: UserId,
        project_id: &str,
        template_id: &str,
    ) -> BoxFut<'_, Result<Option<TemplateRecord>, StoreError>> {
        let pool = self.pool.clone();
        let actor = actor.to_string();
        let project_id = project_id.to_string();
        let template_id = template_id.to_string();
        Box::pin(async move {
            let row = sqlx::query_as::<_, TemplateRow>(
                &format!(
                    "SELECT id, project_id, name, description, channel, content, version, created_at, updated_at
                     FROM platform_templates
                     WHERE id = $2
                       AND project_id = $3
                       AND deleted_at IS NULL
                       AND {VISIBLE_PROJECT}"
                ),
            )
            .bind(&actor)
            .bind(&template_id)
            .bind(&project_id)
            .fetch_optional(&pool)
            .await
            .map_err(map_err)?;

            match row {
                Some(row) => {
                    let attachments = PgTemplatesStore::load_attachments(&pool, &row.id)
                        .await
                        .map_err(map_err)?;
                    Ok(Some(TemplateRecord::from((row, attachments))))
                }
                None => Ok(None),
            }
        })
    }

    #[allow(clippy::too_many_arguments)]
    fn update(
        &self,
        actor: UserId,
        project_id: &str,
        template_id: &str,
        name: &str,
        description: Option<&str>,
        channel: &str,
        content: serde_json::Value,
        attachments: Vec<AttachmentInput>,
    ) -> BoxFut<'_, Result<Option<TemplateRecord>, StoreError>> {
        let pool = self.pool.clone();
        let actor = actor.to_string();
        let project_id = project_id.to_string();
        let template_id = template_id.to_string();
        let name = name.to_string();
        let description = description.map(str::to_owned);
        let channel = channel.to_string();
        Box::pin(async move {
            let row = sqlx::query_as::<_, TemplateRow>(
                &format!(
                    "UPDATE platform_templates
                     SET name = $4, description = $5, channel = $6, content = $7,
                         version = version + 1, updated_at = now()
                     WHERE id = $2
                       AND project_id = $3
                       AND deleted_at IS NULL
                       AND {VISIBLE_PROJECT}
                     RETURNING id, project_id, name, description, channel, content, version, created_at, updated_at"
                ),
            )
            .bind(&actor)
            .bind(&template_id)
            .bind(&project_id)
            .bind(&name)
            .bind(&description)
            .bind(&channel)
            .bind(&content)
            .fetch_optional(&pool)
            .await
            .map_err(map_err)?;

            match row {
                Some(row) => {
                    let inserted =
                        PgTemplatesStore::replace_attachments(&pool, &template_id, attachments)
                            .await
                            .map_err(map_err)?;
                    Ok(Some(TemplateRecord::from((row, inserted))))
                }
                None => Ok(None),
            }
        })
    }

    fn remove(
        &self,
        actor: UserId,
        project_id: &str,
        template_id: &str,
    ) -> BoxFut<'_, Result<bool, StoreError>> {
        let pool = self.pool.clone();
        let actor = actor.to_string();
        let project_id = project_id.to_string();
        let template_id = template_id.to_string();
        Box::pin(async move {
            let result = sqlx::query(
                &format!(
                    "UPDATE platform_templates
                     SET deleted_at = now(), updated_at = now()
                     WHERE id = $2
                       AND project_id = $3
                       AND deleted_at IS NULL
                       AND {VISIBLE_PROJECT}"
                ),
            )
            .bind(&actor)
            .bind(&template_id)
            .bind(&project_id)
            .execute(&pool)
            .await
            .map_err(map_err)?;

            Ok(result.rows_affected() > 0)
        })
    }
}
