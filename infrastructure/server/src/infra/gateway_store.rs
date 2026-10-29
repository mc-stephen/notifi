//! Poll-and-delete gateway queue on the server database.
//!
//! `gateway_messages` holds a route's waiting messages; a fetch claims the
//! oldest rows (FOR UPDATE SKIP LOCKED), returns them and removes them in
//! the same transaction — a message leaves its route at first read. The
//! long-poll hold lives here: short re-checks until the wait budget is
//! spent, so rows queued mid-hold surface within one tick.
//!
//! Proven through the in-memory fake (`tests/common::FakeGatewayStore`) by
//! `tests/gateway_api.rs`; the SQL rides the next database-backed check.

use std::time::{Duration, Instant};

use sqlx::PgPool;

use crate::ports::gateway::{BoxGatewayFuture, GatewayError, GatewayMessage, GatewayStore};

/// One fetch never drains beyond this many rows.
const FETCH_LIMIT: i64 = 50;

/// How often the held poll re-checks the queue.
const POLL_TICK: Duration = Duration::from_millis(300);

/// Gateway queue over the server pool.
pub struct PgGatewayStore {
    pool: PgPool,
}

impl PgGatewayStore {
    /// Store over the server's database pool.
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

/// Claims and deletes one batch of the route's oldest rows.
async fn take_batch(
    pool: &PgPool,
    topic: &str,
    key: &str,
) -> Result<Vec<GatewayMessage>, GatewayError> {
    let mut tx = pool
        .begin()
        .await
        .map_err(|e| GatewayError::Storage(format!("gateway begin failed: {e}")))?;

    let rows: Vec<(i64, String, String, chrono::DateTime<chrono::Utc>)> = sqlx::query_as(
        "SELECT id, title, body, created_at FROM gateway_messages \
         WHERE topic = $1 AND secret_key = $2 \
         ORDER BY id LIMIT $3 FOR UPDATE SKIP LOCKED",
    )
    .bind(topic)
    .bind(key)
    .bind(FETCH_LIMIT)
    .fetch_all(&mut *tx)
    .await
    .map_err(|e| GatewayError::Storage(format!("gateway fetch failed: {e}")))?;

    let ids: Vec<i64> = rows.iter().map(|(id, _, _, _)| *id).collect();
    if !ids.is_empty() {
        sqlx::query(
            "DELETE FROM gateway_messages WHERE topic = $1 AND secret_key = $2 AND id = ANY($3)",
        )
        .bind(topic)
        .bind(key)
        .bind(&ids)
        .execute(&mut *tx)
        .await
        .map_err(|e| GatewayError::Storage(format!("gateway delete failed: {e}")))?;
    }

    tx.commit()
        .await
        .map_err(|e| GatewayError::Storage(format!("gateway commit failed: {e}")))?;

    Ok(rows
        .into_iter()
        .map(|(id, title, body, created_at)| GatewayMessage {
            id: id.to_string(),
            title,
            body,
            received_at: created_at.to_rfc3339(),
        })
        .collect())
}

impl GatewayStore for PgGatewayStore {
    fn push(&self, topic: &str, key: &str, title: &str, body: &str) -> BoxGatewayFuture<'_, ()> {
        // The boxed future must own everything (the `&str` borrows do not
        // outlive it) — clone before move.
        let topic = topic.to_string();
        let key = key.to_string();
        let title = title.to_string();
        let body = body.to_string();
        let pool = self.pool.clone();
        Box::pin(async move {
            sqlx::query("INSERT INTO gateway_messages (topic, secret_key, title, body) VALUES ($1, $2, $3, $4)")
                .bind(&topic)
                .bind(&key)
                .bind(&title)
                .bind(&body)
                .execute(&pool)
                .await
                .map(|_| ())
                .map_err(|e| GatewayError::Storage(format!("gateway push failed: {e}")))
        })
    }

    fn poll(
        &self,
        topic: &str,
        key: &str,
        wait_secs: u64,
    ) -> BoxGatewayFuture<'_, Vec<GatewayMessage>> {
        let topic = topic.to_string();
        let key = key.to_string();
        let pool = self.pool.clone();
        Box::pin(async move {
            let deadline = Instant::now() + Duration::from_secs(wait_secs);
            loop {
                let batch = take_batch(&pool, &topic, &key).await?;
                if !batch.is_empty() {
                    return Ok(batch);
                }
                if Instant::now() >= deadline {
                    return Ok(Vec::new());
                }
                tokio::time::sleep(POLL_TICK).await;
            }
        })
    }
}
