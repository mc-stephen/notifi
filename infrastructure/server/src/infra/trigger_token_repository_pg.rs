use sqlx::PgPool;

use crate::ports::auth_store::StoreError;
use crate::ports::trigger_token_store::{BoxFut, TriggerTokenRecord, TriggerTokenStore};

pub struct PgTriggerTokenStore {
    pool: PgPool,
}

impl PgTriggerTokenStore {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

fn map_err(err: sqlx::Error) -> StoreError {
    StoreError::Storage(err.to_string())
}

#[derive(sqlx::FromRow)]
struct TokenRow {
    token: String,
    flow_id: String,
    node_id: String,
    project_id: String,
    flow_definition: serde_json::Value,
}

impl From<TokenRow> for TriggerTokenRecord {
    fn from(r: TokenRow) -> Self {
        Self {
            token: r.token,
            flow_id: r.flow_id,
            node_id: r.node_id,
            project_id: r.project_id,
            flow_definition: r.flow_definition,
        }
    }
}

impl TriggerTokenStore for PgTriggerTokenStore {
    fn replace_flow_tokens(
        &self,
        flow_id: &str,
        project_id: &str,
        _definition: &serde_json::Value,
        pairs: &[(String, String)],
    ) -> BoxFut<'_, Result<(), StoreError>> {
        let pool = self.pool.clone();
        let flow_id = flow_id.to_string();
        let project_id = project_id.to_string();
        let tokens: Vec<String> = pairs.iter().map(|(t, _)| t.clone()).collect();
        let nodes: Vec<String> = pairs.iter().map(|(_, n)| n.clone()).collect();
        Box::pin(async move {
            sqlx::query(
                "WITH del AS (DELETE FROM flow_trigger_tokens WHERE flow_id = $1)
                 INSERT INTO flow_trigger_tokens (token, flow_id, node_id, project_id)
                 SELECT u.token, $1, u.node_id, $4
                 FROM unnest($2::text[], $3::text[]) AS u(token, node_id)",
            )
            .bind(&flow_id)
            .bind(&tokens)
            .bind(&nodes)
            .bind(&project_id)
            .execute(&pool)
            .await
            .map_err(map_err)?;
            Ok(())
        })
    }

    fn lookup(&self, token: &str) -> BoxFut<'_, Result<Option<TriggerTokenRecord>, StoreError>> {
        let pool = self.pool.clone();
        let token = token.to_string();
        Box::pin(async move {
            let row: Option<TokenRow> = sqlx::query_as(
                "SELECT t.token, t.flow_id, t.node_id, t.project_id, f.definition AS flow_definition
                 FROM flow_trigger_tokens t JOIN flows f ON f.id = t.flow_id
                 WHERE t.token = $1",
            )
            .bind(&token)
            .fetch_optional(&pool)
            .await
            .map_err(map_err)?;
            Ok(row.map(TriggerTokenRecord::from))
        })
    }
}
