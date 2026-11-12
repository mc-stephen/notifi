use chrono::{DateTime, Utc};
use sqlx::PgPool;

use crate::ports::auth_store::StoreError;
use crate::ports::flow_run_store::{FlowRunRecord, FlowRunStore};
use crate::ports::trigger_token_store::BoxFut;

/// Postgres-backed run history. One row per completed flow run,
/// inserted at run end.
pub struct PgFlowRunStore {
    pool: PgPool,
}

impl PgFlowRunStore {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

fn map_err(err: sqlx::Error) -> StoreError {
    StoreError::Storage(err.to_string())
}

#[derive(sqlx::FromRow)]
struct RunRow {
    id: String,
    project_id: String,
    flow_id: String,
    mode: String,
    status: String,
    error: Option<String>,
    started_at: DateTime<Utc>,
    finished_at: Option<DateTime<Utc>>,
    send_id: Option<String>,
    steps: serde_json::Value,
}

impl From<RunRow> for FlowRunRecord {
    fn from(r: RunRow) -> Self {
        Self {
            id: r.id,
            project_id: r.project_id,
            flow_id: r.flow_id,
            mode: r.mode,
            status: r.status,
            error: r.error,
            started_at: r.started_at,
            finished_at: r.finished_at,
            send_id: r.send_id,
            steps: r.steps,
        }
    }
}

const COLS: &str =
    "id, project_id, flow_id, mode, status, error, started_at, finished_at, send_id, steps";

impl FlowRunStore for PgFlowRunStore {
    fn insert(&self, record: FlowRunRecord) -> BoxFut<'_, Result<(), StoreError>> {
        let pool = self.pool.clone();
        Box::pin(async move {
            sqlx::query(
                "INSERT INTO flow_runs
                    (id, project_id, flow_id, mode, status, error, started_at,
                     finished_at, send_id, steps)
                 VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)",
            )
            .bind(&record.id)
            .bind(&record.project_id)
            .bind(&record.flow_id)
            .bind(&record.mode)
            .bind(&record.status)
            .bind(&record.error)
            .bind(record.started_at)
            .bind(record.finished_at)
            .bind(&record.send_id)
            .bind(&record.steps)
            .execute(&pool)
            .await
            .map_err(map_err)?;
            Ok(())
        })
    }

    fn list(
        &self,
        project_id: &str,
        flow_id: &str,
        limit: i64,
    ) -> BoxFut<'_, Result<Vec<FlowRunRecord>, StoreError>> {
        let pool = self.pool.clone();
        let project_id = project_id.to_string();
        let flow_id = flow_id.to_string();
        Box::pin(async move {
            let sql = format!(
                "SELECT {COLS} FROM flow_runs
                 WHERE project_id = $1 AND flow_id = $2
                 ORDER BY started_at DESC LIMIT $3"
            );
            let rows: Vec<RunRow> = sqlx::query_as(&sql)
                .bind(&project_id)
                .bind(&flow_id)
                .bind(limit)
                .fetch_all(&pool)
                .await
                .map_err(map_err)?;
            Ok(rows.into_iter().map(FlowRunRecord::from).collect())
        })
    }

    fn get(
        &self,
        project_id: &str,
        flow_id: &str,
        run_id: &str,
    ) -> BoxFut<'_, Result<Option<FlowRunRecord>, StoreError>> {
        let pool = self.pool.clone();
        let project_id = project_id.to_string();
        let flow_id = flow_id.to_string();
        let run_id = run_id.to_string();
        Box::pin(async move {
            let sql = format!(
                "SELECT {COLS} FROM flow_runs WHERE project_id = $1 AND flow_id = $2 AND id = $3"
            );
            let row: Option<RunRow> = sqlx::query_as(&sql)
                .bind(&project_id)
                .bind(&flow_id)
                .bind(&run_id)
                .fetch_optional(&pool)
                .await
                .map_err(map_err)?;
            Ok(row.map(FlowRunRecord::from))
        })
    }
}
