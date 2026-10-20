use chrono::{DateTime, Utc};
use sqlx::PgPool;
use ulid::Ulid;

use crate::domain::auth::entities::UserId;
use crate::ports::auth_store::StoreError;
use crate::ports::schedule_store::{
    BoxFut, NewSchedule, ScheduleRecord, ScheduleStore, ScheduleUpdate,
};

pub struct PgScheduleStore {
    pool: PgPool,
}

impl PgScheduleStore {
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
struct ScheduleRow {
    id: String,
    project_id: String,
    name: String,
    flow_id: String,
    flow_name: String,
    flow_definition: serde_json::Value,
    kind: String,
    run_at: Option<DateTime<Utc>>,
    interval_secs: Option<i64>,
    days: Option<serde_json::Value>,
    run_time: Option<String>,
    timezone: String,
    template_id: Option<String>,
    template_name: Option<String>,
    template_content: Option<serde_json::Value>,
    channel: String,
    title: String,
    body: String,
    recipients: serde_json::Value,
    active: bool,
    archived_at: Option<DateTime<Utc>>,
    next_run_at: Option<DateTime<Utc>>,
    last_run_at: Option<DateTime<Utc>>,
    last_status: Option<String>,
    last_error: Option<String>,
    run_count: i32,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

fn parse_days(value: Option<serde_json::Value>) -> Vec<i64> {
    value
        .and_then(|v| v.as_array().cloned())
        .map(|a| {
            a.iter()
                .filter_map(|v| v.as_i64())
                .filter(|d| (0..=6).contains(d))
                .collect::<Vec<_>>()
        })
        .unwrap_or_default()
}

fn days_json(days: &[i64]) -> serde_json::Value {
    serde_json::Value::Array(days.iter().map(|d| serde_json::Value::from(*d)).collect())
}

impl From<ScheduleRow> for ScheduleRecord {
    fn from(r: ScheduleRow) -> Self {
        Self {
            id: r.id,
            project_id: r.project_id,
            name: r.name,
            flow_id: r.flow_id,
            flow_name: r.flow_name,
            flow_definition: r.flow_definition,
            kind: r.kind,
            run_at: r.run_at,
            interval_secs: r.interval_secs,
            days: parse_days(r.days),
            run_time: r.run_time,
            timezone: r.timezone,
            template_id: r.template_id,
            template_name: r.template_name,
            template_content: r.template_content,
            channel: r.channel,
            title: r.title,
            body: r.body,
            recipients: r.recipients,
            active: r.active,
            archived_at: r.archived_at,
            next_run_at: r.next_run_at,
            last_run_at: r.last_run_at,
            last_status: r.last_status,
            last_error: r.last_error,
            run_count: r.run_count,
            created_at: r.created_at,
            updated_at: r.updated_at,
        }
    }
}

const COLUMNS: &str = "s.id, s.project_id, s.name, s.flow_id, f.name AS flow_name, \
    f.definition AS flow_definition, s.kind, s.run_at, s.interval_secs, s.days, \
    s.run_time, s.timezone, s.template_id, t.name AS template_name, \
    t.content AS template_content, s.channel, s.title, s.body, s.recipients, \
    s.active, s.archived_at, s.next_run_at, s.last_run_at, s.last_status, s.last_error, \
    s.run_count, s.created_at, s.updated_at";
const JOINS: &str = "FROM schedules s JOIN flows f ON f.id = s.flow_id \
    LEFT JOIN platform_templates t ON t.id = s.template_id";

impl ScheduleStore for PgScheduleStore {
    fn project_visible(
        &self,
        actor: UserId,
        project_id: &str,
    ) -> BoxFut<'_, Result<bool, StoreError>> {
        let pool = self.pool.clone();
        let actor = actor.to_string();
        let project_id = project_id.to_string();
        Box::pin(async move {
            let row: (bool,) = sqlx::query_as(
                "SELECT EXISTS(
                    SELECT 1 FROM platform_projects p
                    WHERE p.id = $1 AND p.deleted_at IS NULL
                      AND (p.created_by = $2
                           OR EXISTS(SELECT 1 FROM platform_project_members pm
                                     WHERE pm.project_id = p.id AND pm.user_id = $2
                                       AND pm.deleted_at IS NULL))
                 )",
            )
            .bind(&project_id)
            .bind(&actor)
            .fetch_one(&pool)
            .await
            .map_err(map_err)?;
            Ok(row.0)
        })
    }

    fn list_by_project(
        &self,
        project_id: &str,
    ) -> BoxFut<'_, Result<Vec<ScheduleRecord>, StoreError>> {
        let pool = self.pool.clone();
        let project_id = project_id.to_string();
        Box::pin(async move {
            let rows: Vec<ScheduleRow> = sqlx::query_as(&format!(
                "SELECT {COLUMNS} {JOINS}
                 WHERE s.project_id = $1 ORDER BY s.created_at ASC, s.id ASC"
            ))
            .bind(&project_id)
            .fetch_all(&pool)
            .await
            .map_err(map_err)?;
            Ok(rows.into_iter().map(ScheduleRecord::from).collect())
        })
    }

    fn get(&self, id: &str) -> BoxFut<'_, Result<Option<ScheduleRecord>, StoreError>> {
        let pool = self.pool.clone();
        let id = id.to_string();
        Box::pin(async move {
            let row: Option<ScheduleRow> = sqlx::query_as(&format!(
                "SELECT {COLUMNS} {JOINS}
                 WHERE s.id = $1"
            ))
            .bind(&id)
            .fetch_optional(&pool)
            .await
            .map_err(map_err)?;
            Ok(row.map(ScheduleRecord::from))
        })
    }

    fn scheduled_flow_ids(&self, project_id: &str) -> BoxFut<'_, Result<Vec<String>, StoreError>> {
        let pool = self.pool.clone();
        let project_id = project_id.to_string();
        Box::pin(async move {
            let rows: Vec<(String,)> =
                sqlx::query_as("SELECT DISTINCT flow_id FROM schedules WHERE project_id = $1")
                    .bind(&project_id)
                    .fetch_all(&pool)
                    .await
                    .map_err(map_err)?;
            Ok(rows.into_iter().map(|r| r.0).collect())
        })
    }

    fn create(&self, draft: &NewSchedule) -> BoxFut<'_, Result<ScheduleRecord, StoreError>> {
        let pool = self.pool.clone();
        let draft = draft.clone();
        Box::pin(async move {
            let row: ScheduleRow = sqlx::query_as(
                "WITH ins AS (
                   INSERT INTO schedules (id, project_id, name, flow_id, kind, run_at,
                       interval_secs, days, run_time, timezone, template_id,
                       channel, title, body, recipients, active, archived_at, next_run_at)
                   VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16, $17, $18)
                   RETURNING *
                 )
                 SELECT ins.id, ins.project_id, ins.name, ins.flow_id,
                     f.name AS flow_name, f.definition AS flow_definition,
                     ins.kind, ins.run_at, ins.interval_secs, ins.days, ins.run_time,
                     ins.timezone, ins.template_id,
                     t.name AS template_name, t.content AS template_content,
                     ins.channel, ins.title, ins.body, ins.recipients, ins.active,
                     ins.archived_at, ins.next_run_at, ins.last_run_at, ins.last_status, ins.last_error,
                     ins.run_count, ins.created_at, ins.updated_at
                 FROM ins JOIN flows f ON f.id = ins.flow_id
                 LEFT JOIN platform_templates t ON t.id = ins.template_id",
            )
            .bind(Ulid::new().to_string())
            .bind(&draft.project_id)
            .bind(&draft.name)
            .bind(&draft.flow_id)
            .bind(&draft.kind)
            .bind(draft.run_at)
            .bind(draft.interval_secs)
            .bind(days_json(&draft.days))
            .bind(&draft.run_time)
            .bind(&draft.timezone)
            .bind(&draft.template_id)
            .bind(&draft.channel)
            .bind(&draft.title)
            .bind(&draft.body)
            .bind(&draft.recipients)
            .bind(draft.active)
            .bind(draft.archived_at)
            .bind(draft.next_run_at)
            .fetch_one(&pool)
            .await
            .map_err(map_err)?;
            Ok(ScheduleRecord::from(row))
        })
    }

    fn update(
        &self,
        id: &str,
        patch: &ScheduleUpdate,
    ) -> BoxFut<'_, Result<Option<ScheduleRecord>, StoreError>> {
        let pool = self.pool.clone();
        let id = id.to_string();
        let patch = patch.clone();
        Box::pin(async move {
            let row: Option<ScheduleRow> = sqlx::query_as(
                "WITH upd AS (
                   UPDATE schedules SET name = $2, flow_id = $3, kind = $4,
                       run_at = $5, interval_secs = $6, timezone = $7,
                       channel = $8, title = $9, body = $10, recipients = $11,
                       active = $12, next_run_at = $13, days = $14, run_time = $15,
                       template_id = $16, archived_at = $17, updated_at = now()
                   WHERE id = $1
                   RETURNING *
                 )
                 SELECT upd.id, upd.project_id, upd.name, upd.flow_id,
                     f.name AS flow_name, f.definition AS flow_definition,
                     upd.kind, upd.run_at, upd.interval_secs, upd.days, upd.run_time,
                     upd.timezone, upd.template_id,
                     t.name AS template_name, t.content AS template_content,
                     upd.channel, upd.title, upd.body, upd.recipients, upd.active,
                     upd.archived_at, upd.next_run_at, upd.last_run_at, upd.last_status, upd.last_error,
                     upd.run_count, upd.created_at, upd.updated_at
                 FROM upd JOIN flows f ON f.id = upd.flow_id
                 LEFT JOIN platform_templates t ON t.id = upd.template_id",
            )
            .bind(&id)
            .bind(&patch.name)
            .bind(&patch.flow_id)
            .bind(&patch.kind)
            .bind(patch.run_at)
            .bind(patch.interval_secs)
            .bind(&patch.timezone)
            .bind(&patch.channel)
            .bind(&patch.title)
            .bind(&patch.body)
            .bind(&patch.recipients)
            .bind(patch.active)
            .bind(patch.next_run_at)
            .bind(days_json(&patch.days))
            .bind(&patch.run_time)
            .bind(&patch.template_id)
            .bind(patch.archived_at)
            .fetch_optional(&pool)
            .await
            .map_err(map_err)?;
            Ok(row.map(ScheduleRecord::from))
        })
    }

    fn delete(&self, id: &str) -> BoxFut<'_, Result<bool, StoreError>> {
        let pool = self.pool.clone();
        let id = id.to_string();
        Box::pin(async move {
            let result = sqlx::query("DELETE FROM schedules WHERE id = $1")
                .bind(&id)
                .execute(&pool)
                .await
                .map_err(map_err)?;
            Ok(result.rows_affected() > 0)
        })
    }

    fn claim_due(
        &self,
        now: DateTime<Utc>,
        limit: i64,
    ) -> BoxFut<'_, Result<Vec<ScheduleRecord>, StoreError>> {
        let pool = self.pool.clone();
        Box::pin(async move {
            let rows: Vec<ScheduleRow> = sqlx::query_as(
                "WITH due AS (
                   SELECT id FROM schedules
                   WHERE active AND archived_at IS NULL AND next_run_at <= $1
                   ORDER BY next_run_at ASC, id ASC
                   LIMIT $2
                   FOR UPDATE SKIP LOCKED
                 ),
                 claimed AS (
                   UPDATE schedules s SET last_run_at = $1, updated_at = now()
                   FROM due WHERE s.id = due.id
                   RETURNING s.*
                 )
                 SELECT claimed.id, claimed.project_id, claimed.name, claimed.flow_id,
                     f.name AS flow_name, f.definition AS flow_definition,
                     claimed.kind, claimed.run_at, claimed.interval_secs,
                     claimed.days, claimed.run_time, claimed.timezone,
                     claimed.template_id,
                     t.name AS template_name, t.content AS template_content,
                     claimed.channel, claimed.title, claimed.body, claimed.recipients,
                     claimed.active, claimed.archived_at, claimed.next_run_at, claimed.last_run_at,
                     claimed.last_status, claimed.last_error, claimed.run_count,
                     claimed.created_at, claimed.updated_at
                 FROM claimed JOIN flows f ON f.id = claimed.flow_id
                 LEFT JOIN platform_templates t ON t.id = claimed.template_id
                 ORDER BY claimed.next_run_at ASC, claimed.id ASC",
            )
            .bind(now)
            .bind(limit)
            .fetch_all(&pool)
            .await
            .map_err(map_err)?;
            Ok(rows.into_iter().map(ScheduleRecord::from).collect())
        })
    }

    fn mark_run(
        &self,
        id: &str,
        last_status: &str,
        last_error: Option<&str>,
        next_run_at: Option<DateTime<Utc>>,
        active: bool,
    ) -> BoxFut<'_, Result<Option<ScheduleRecord>, StoreError>> {
        let pool = self.pool.clone();
        let id = id.to_string();
        let last_status = last_status.to_string();
        let last_error = last_error.map(str::to_string);
        Box::pin(async move {
            let row: Option<ScheduleRow> = sqlx::query_as(
                "WITH upd AS (
                   UPDATE schedules SET last_status = $2, last_error = $3,
                       next_run_at = $4, active = $5,
                       run_count = run_count + 1,
                       last_run_at = COALESCE(last_run_at, now()),
                       updated_at = now()
                   WHERE id = $1
                   RETURNING *
                 )
                 SELECT upd.id, upd.project_id, upd.name, upd.flow_id,
                     f.name AS flow_name, f.definition AS flow_definition,
                     upd.kind, upd.run_at, upd.interval_secs, upd.days, upd.run_time,
                     upd.timezone, upd.template_id,
                     t.name AS template_name, t.content AS template_content,
                     upd.channel, upd.title, upd.body, upd.recipients, upd.active,
                     upd.archived_at, upd.next_run_at, upd.last_run_at, upd.last_status, upd.last_error,
                     upd.run_count, upd.created_at, upd.updated_at
                 FROM upd JOIN flows f ON f.id = upd.flow_id
                 LEFT JOIN platform_templates t ON t.id = upd.template_id",
            )
            .bind(&id)
            .bind(&last_status)
            .bind(last_error)
            .bind(next_run_at)
            .bind(active)
            .fetch_optional(&pool)
            .await
            .map_err(map_err)?;
            Ok(row.map(ScheduleRecord::from))
        })
    }
}
