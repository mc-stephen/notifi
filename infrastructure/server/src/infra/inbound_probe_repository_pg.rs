use chrono::{DateTime, Utc};
use sqlx::PgPool;

use crate::ports::auth_store::StoreError;
use crate::ports::inbound_probe_store::{InboundProbeRecord, InboundProbeStore};
use crate::ports::trigger_token_store::BoxFut;

/// Postgres-backed last-ping probes for webhook doors. One row per
/// trigger token, upserted on every authorized fire.
pub struct PgInboundProbeStore {
    pool: PgPool,
}

impl PgInboundProbeStore {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

fn map_err(err: sqlx::Error) -> StoreError {
    StoreError::Storage(err.to_string())
}

#[derive(sqlx::FromRow)]
struct ProbeRow {
    token: String,
    flow_id: String,
    project_id: String,
    client_ip: Option<String>,
    content_type: Option<String>,
    user_agent: Option<String>,
    forwarded_for: Option<String>,
    body: Option<String>,
    fired_at: DateTime<Utc>,
}

impl From<ProbeRow> for InboundProbeRecord {
    fn from(r: ProbeRow) -> Self {
        Self {
            token: r.token,
            flow_id: r.flow_id,
            project_id: r.project_id,
            client_ip: r.client_ip,
            content_type: r.content_type,
            user_agent: r.user_agent,
            forwarded_for: r.forwarded_for,
            body: r.body,
            fired_at: r.fired_at,
        }
    }
}

impl InboundProbeStore for PgInboundProbeStore {
    fn record(&self, record: InboundProbeRecord) -> BoxFut<'_, Result<(), StoreError>> {
        let pool = self.pool.clone();
        Box::pin(async move {
            sqlx::query(
                "INSERT INTO flow_inbound_probes
                    (token, flow_id, project_id, client_ip, content_type,
                     user_agent, forwarded_for, body, fired_at)
                 VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
                 ON CONFLICT (token) DO UPDATE SET
                    flow_id = EXCLUDED.flow_id,
                    project_id = EXCLUDED.project_id,
                    client_ip = EXCLUDED.client_ip,
                    content_type = EXCLUDED.content_type,
                    user_agent = EXCLUDED.user_agent,
                    forwarded_for = EXCLUDED.forwarded_for,
                    body = EXCLUDED.body,
                    fired_at = EXCLUDED.fired_at",
            )
            .bind(&record.token)
            .bind(&record.flow_id)
            .bind(&record.project_id)
            .bind(&record.client_ip)
            .bind(&record.content_type)
            .bind(&record.user_agent)
            .bind(&record.forwarded_for)
            .bind(&record.body)
            .bind(record.fired_at)
            .execute(&pool)
            .await
            .map_err(map_err)?;
            Ok(())
        })
    }

    fn last_for_token(
        &self,
        token: &str,
    ) -> BoxFut<'_, Result<Option<InboundProbeRecord>, StoreError>> {
        let pool = self.pool.clone();
        let token = token.to_string();
        Box::pin(async move {
            let row: Option<ProbeRow> =
                sqlx::query_as("SELECT * FROM flow_inbound_probes WHERE token = $1")
                    .bind(&token)
                    .fetch_optional(&pool)
                    .await
                    .map_err(map_err)?;
            Ok(row.map(InboundProbeRecord::from))
        })
    }

    fn last_for_flow(
        &self,
        project_id: &str,
        flow_id: &str,
    ) -> BoxFut<'_, Result<Option<InboundProbeRecord>, StoreError>> {
        let pool = self.pool.clone();
        let project_id = project_id.to_string();
        let flow_id = flow_id.to_string();
        Box::pin(async move {
            let row: Option<ProbeRow> = sqlx::query_as(
                "SELECT * FROM flow_inbound_probes
                 WHERE project_id = $1 AND flow_id = $2
                 ORDER BY fired_at DESC LIMIT 1",
            )
            .bind(&project_id)
            .bind(&flow_id)
            .fetch_optional(&pool)
            .await
            .map_err(map_err)?;
            Ok(row.map(InboundProbeRecord::from))
        })
    }
}
