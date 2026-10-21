//! Shared provider-chain delivery walker.
//!
//! One implementation serves ad-hoc sends, schedule runs, and inbound
//! webhook fires: walk the steps by priority with fallback, delivering
//! the same content to every recipient through the winning provider.

use std::sync::Arc;
use std::time::Duration;

use crate::domain::flows::entities::ProviderStep;

/// Per-attempt network budget before the chain falls through.
pub const ATTEMPT_TIMEOUT: Duration = Duration::from_secs(30);

/// Walks one flow's provider chain by priority with fallback. `Ok`
/// carries (provider, external id) on full delivery; `Err` carries the
/// terminal error otherwise.
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
) -> Result<(String, Option<String>), String> {
    let mut last_error = format!("no provider route for channel '{channel}'");
    for step in chain {
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
                continue;
            }
            Err(e) => {
                last_error = format!("provider lookup failed: {e}");
                continue;
            }
        };
        let Some(adapter) = adapters.adapter(&step.channel_id, &step.provider_id, &config) else {
            last_error = format!(
                "provider '{}' has no sender yet (channel '{}')",
                step.provider_id, step.channel_id
            );
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
            return Ok((step.provider_id.clone(), step_receipt_external));
        }
    }
    Err(last_error)
}
