//! Shared provider-chain delivery walker.
//!
//! One implementation serves ad-hoc sends, schedule runs, and inbound
//! webhook fires: walk the steps by priority with fallback, delivering
//! the same content to every recipient through the winning provider.
//! Alongside the outcome it reports a per-step [`ChainAttempt`] trace —
//! the flow run history folds it into the run's `steps` (node ids ride
//! the chain when the walk came from graph nodes).

use std::sync::Arc;
use std::time::{Duration, Instant};

use crate::domain::flows::entities::ProviderStep;

/// Per-attempt network budget before the chain falls through.
pub const ATTEMPT_TIMEOUT: Duration = Duration::from_secs(30);

/// One chain step's outcome — success, provider failure, timeout, or a
/// configuration miss that made the chain fall through. The flow run
/// history turns these into the run's `steps` trace entries.
#[derive(Debug, Clone)]
pub struct ChainAttempt {
    pub node_id: String,
    pub channel_id: String,
    pub provider_id: String,
    pub ok: bool,
    pub error: Option<String>,
    pub ms: u64,
}

/// Walks one flow's provider chain by priority with fallback. `Ok`
/// carries (provider, external id) on full delivery; `Err` carries the
/// terminal error otherwise. Both arms carry the per-step attempts so
/// callers can record what happened at each hop.
#[allow(clippy::too_many_arguments)]
pub async fn deliver_through_chain(
    providers: &Arc<dyn crate::ports::ChannelProviderStore>,
    adapters: &Arc<dyn crate::ports::AdapterFactory>,
    project_id: &str,
    chain: &[ProviderStep],
    channel: &str,
    recipients: &[String],
    title: &str,
    body: &str,
) -> (Result<(String, Option<String>), String>, Vec<ChainAttempt>) {
    let mut attempts: Vec<ChainAttempt> = Vec::new();
    let mut last_error = format!("no provider route for channel '{channel}'");
    for step in chain {
        let started = Instant::now();
        let attempt = |ok: bool, error: Option<String>| ChainAttempt {
            node_id: step.node_id.clone(),
            channel_id: step.channel_id.clone(),
            provider_id: step.provider_id.clone(),
            ok,
            error,
            ms: started.elapsed().as_millis() as u64,
        };
        let config = match providers
            .get_by_project_channel_provider(project_id, &step.channel_id, &step.provider_id)
            .await
        {
            Ok(Some(c)) if c.enabled => c.config,
            Ok(_) => {
                last_error = format!(
                    "provider '{}' is not configured for channel '{}'",
                    step.provider_id, step.channel_id
                );
                attempts.push(attempt(false, Some(last_error.clone())));
                continue;
            }
            Err(e) => {
                last_error = format!("provider lookup failed: {e}");
                attempts.push(attempt(false, Some(last_error.clone())));
                continue;
            }
        };
        let Some(adapter) =
            adapters.adapter(project_id, &step.channel_id, &step.provider_id, &config)
        else {
            last_error = format!(
                "provider '{}' has no sender yet (channel '{}')",
                step.provider_id, step.channel_id
            );
            attempts.push(attempt(false, Some(last_error.clone())));
            continue;
        };
        let mut step_ok = true;
        let mut step_receipt_external: Option<String> = None;
        for recipient in recipients {
            let message = crate::ports::DeliveryMessage {
                channel: step.channel_id.clone(),
                recipient: recipient.clone(),
                title: title.trim().to_string(),
                body: body.to_string(),
                metadata: None,
            };
            match tokio::time::timeout(ATTEMPT_TIMEOUT, adapter.send(&message)).await {
                Ok(Ok(receipt)) => {
                    step_receipt_external = receipt.provider_message_id;
                }
                Ok(Err(e)) => {
                    last_error = format!("provider '{}' failed: {e}", step.provider_id);
                    step_ok = false;
                    break;
                }
                Err(_) => {
                    last_error = format!("provider '{}' timed out", step.provider_id);
                    step_ok = false;
                    break;
                }
            }
        }
        if step_ok {
            attempts.push(attempt(true, None));
            return (
                Ok((step.provider_id.clone(), step_receipt_external)),
                attempts,
            );
        }
        attempts.push(attempt(false, Some(last_error.clone())));
    }
    (Err(last_error), attempts)
}
