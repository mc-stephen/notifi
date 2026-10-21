//! RabbitMQ transport for sweep ticks.
//!
//! One durable queue ([`SWEEP_QUEUE`]). The API publishes a tick every
//! [`SWEEP_INTERVAL_SECS`]; the `worker` binary consumes them and runs the
//! same sweeps the embedded loops run. Ticks are disposable triggers — the
//! database stays the source of truth through atomic `claim_due`, so a lost
//! or repeated tick can never double-send.

use lapin::message::BasicGetMessage;
use lapin::options::*;
use lapin::types::FieldTable;
use lapin::{BasicProperties, Channel, Connection, ConnectionProperties};

use crate::infra::config::AppConfig;

/// Durable queue carrying sweep ticks.
pub const SWEEP_QUEUE: &str = "notifi.sweep";
/// Tick cadence in seconds (mirrors the old embedded interval).
pub const SWEEP_INTERVAL_SECS: u64 = 30;

/// Tick body. The worker runs both sweeps whatever it says — the shape
/// exists so future tick kinds need no format change.
fn sweep_tick() -> Vec<u8> {
    format!(
        "{{\"sweep\":\"all\",\"issued_at\":\"{}\"}}",
        chrono::Utc::now().to_rfc3339()
    )
    .into_bytes()
}

pub struct RabbitHub {
    connection: Connection,
}

/// Tokio executor/reactor for lapin (v2 takes them explicitly).
fn tokio_properties() -> ConnectionProperties {
    ConnectionProperties::default()
        .with_executor(tokio_executor_trait::Tokio::current())
        .with_reactor(tokio_reactor_trait::Tokio)
}

/// Connects when `NOTIFI_RABBITMQ_URL` is set. Returns `None` (with a
/// warning, never an error) when unset or unreachable — callers fall back
/// to embedded behavior so nothing breaks.
pub async fn connect(config: &AppConfig) -> Option<RabbitHub> {
    let url = config.rabbitmq.url.as_deref()?;
    match Connection::connect(url, tokio_properties()).await {
        Ok(connection) => {
            tracing::info!("rabbitmq configured");
            Some(RabbitHub { connection })
        }
        Err(e) => {
            tracing::warn!(error = %e, "rabbitmq unreachable; continuing without it");
            None
        }
    }
}

impl RabbitHub {
    /// Declares the sweep queue (idempotent) and hands back a channel.
    pub async fn declare(&self) -> Result<Channel, lapin::Error> {
        let channel = self.connection.create_channel().await?;
        channel
            .queue_declare(
                SWEEP_QUEUE,
                QueueDeclareOptions {
                    durable: true,
                    ..Default::default()
                },
                FieldTable::default(),
            )
            .await?;
        Ok(channel)
    }

    /// Publishes one sweep tick (persistent). Opens a fresh channel per
    /// call — ticks are 1/30s, so the cost is noise and a dead channel can
    /// never wedge the publisher.
    pub async fn publish_sweep(&self) -> Result<(), String> {
        let channel = self.declare().await.map_err(|e| e.to_string())?;
        let confirm = channel
            .basic_publish(
                "",
                SWEEP_QUEUE,
                BasicPublishOptions::default(),
                &sweep_tick(),
                BasicProperties::default().with_delivery_mode(2),
            )
            .await
            .map_err(|e| e.to_string())?;
        confirm.await.map_err(|e| e.to_string())?;
        Ok(())
    }

    /// Polls one tick without blocking. `Ok(None)` means the queue is empty.
    /// Ack through `msg.delivery` after the sweep completes; an unacked
    /// tick from a crashed worker is redelivered, and the atomic claim
    /// keeps that a harmless repeat.
    pub async fn poll_tick(channel: &Channel) -> Result<Option<BasicGetMessage>, lapin::Error> {
        channel
            .basic_get(SWEEP_QUEUE, BasicGetOptions::default())
            .await
    }
}

impl crate::ports::QueuePublisher for RabbitHub {
    async fn publish(&self, topic: &str, payload: &[u8]) -> Result<(), crate::ports::QueueError> {
        use crate::ports::QueueError;
        let channel = self
            .connection
            .create_channel()
            .await
            .map_err(|e| QueueError::Unavailable(e.to_string()))?;
        channel
            .queue_declare(
                topic,
                QueueDeclareOptions {
                    durable: true,
                    ..Default::default()
                },
                FieldTable::default(),
            )
            .await
            .map_err(|e| QueueError::Publish {
                topic: topic.to_string(),
                message: e.to_string(),
            })?;
        let confirm = channel
            .basic_publish(
                "",
                topic,
                BasicPublishOptions::default(),
                payload,
                BasicProperties::default().with_delivery_mode(2),
            )
            .await
            .map_err(|e| QueueError::Publish {
                topic: topic.to_string(),
                message: e.to_string(),
            })?;
        confirm.await.map_err(|e| QueueError::Publish {
            topic: topic.to_string(),
            message: e.to_string(),
        })?;
        Ok(())
    }
}
