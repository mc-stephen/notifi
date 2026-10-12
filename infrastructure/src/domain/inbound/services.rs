//! Inbound webhook fires: outside systems POST a trigger token to run
//! the owning flow.
//!
//! Resolution is caller-wins with flow fallback: each of channel,
//! recipients, title, body, and template comes from the call when
//! present, else the trigger node's stored config, else the picked
//! template's render. Anything still missing fails closed. Runs record
//! send rows, so deliveries show inbound fires like any other send.

use std::sync::Arc;

use crate::domain::auth::errors::AuthError;
use crate::domain::flows::{
    collect_http_steps, deliver_through_chain, fire_http_steps, provider_chain,
};
use crate::domain::inbound::entities::{InboundPayload, extract_trigger_tokens, webhook_node_data};
use crate::domain::schedules::entities::render_template_content;
use crate::domain::sends::entities::Send;
use crate::ports::auth_store::StoreError;
use crate::ports::send_store::SendStore;
use crate::ports::templates_store::TemplatesStore;
use crate::ports::trigger_token_store::TriggerTokenStore;

/// Max recipients per inbound fire (mirrors the ad-hoc send bound).
const MAX_RECIPIENTS: usize = 25;

pub struct InboundService {
    tokens: Arc<dyn TriggerTokenStore>,
    sends: Arc<dyn SendStore>,
    templates: Arc<dyn TemplatesStore>,
    providers: Arc<dyn crate::ports::ChannelProviderStore>,
    adapters: Arc<dyn notifi_domain_ports::AdapterFactory>,
    http: Arc<dyn crate::ports::HttpRequester>,
}

impl InboundService {
    pub fn new(
        tokens: Arc<dyn TriggerTokenStore>,
        sends: Arc<dyn SendStore>,
        templates: Arc<dyn TemplatesStore>,
        providers: Arc<dyn crate::ports::ChannelProviderStore>,
        adapters: Arc<dyn notifi_domain_ports::AdapterFactory>,
        http: Arc<dyn crate::ports::HttpRequester>,
    ) -> Self {
        Self {
            tokens,
            sends,
            templates,
            providers,
            adapters,
            http,
        }
    }

    fn map_store_error(e: StoreError) -> AuthError {
        match e {
            StoreError::Conflict => AuthError::Conflict("conflicting trigger change".into()),
            StoreError::Storage(m) => AuthError::Storage(m),
        }
    }

    /// Replaces a flow's token rows from its current definition. Called
    /// after every flow save (the definition is the source of truth).
    /// Sync failures must not fail the save — the next save retries.
    pub async fn sync_flow_tokens(
        &self,
        project_id: &str,
        flow_id: &str,
        definition: &serde_json::Value,
    ) -> usize {
        let pairs = extract_trigger_tokens(definition);
        let count = pairs.len();
        if let Err(e) = self
            .tokens
            .replace_flow_tokens(flow_id, project_id, definition, &pairs)
            .await
        {
            tracing::warn!(
                flow_id = %flow_id,
                error = %Self::map_store_error(e),
                "trigger token sync failed; retrying on next save"
            );
            return 0;
        }
        count
    }

    /// Fires the flow owning `token` with caller-wins/fallback resolution.
    /// Unknown tokens read as missing; delivery failures still return the
    /// recorded (failed) send, mirroring ad-hoc sends.
    pub async fn fire(&self, token: &str, payload: InboundPayload) -> Result<Send, AuthError> {
        let record = self
            .tokens
            .lookup(token)
            .await
            .map_err(Self::map_store_error)?
            .ok_or_else(|| AuthError::NotFound("unknown webhook trigger".into()))?;
        let node = webhook_node_data(&record.flow_definition, &record.node_id)
            .ok_or_else(|| AuthError::NotFound("webhook trigger no longer configured".into()))?;
        let project_id = record.project_id.clone();

        // Template: caller pick wins, else the node's pick, then render.
        let template_id = payload.template_id.or(node.template_id);
        let mut rendered: Option<(String, String, String)> = None;
        if let Some(tid) = template_id.as_deref() {
            let template = self
                .templates
                .get_by_id(tid)
                .await
                .map_err(Self::map_store_error)?
                .filter(|t| t.project_id == project_id)
                .ok_or_else(|| {
                    AuthError::Validation("template not found in this project".into())
                })?;
            let content = render_template_content(&template.content).ok_or_else(|| {
                AuthError::Validation("template covers no renderable channels".into())
            })?;
            rendered = Some((content.channel, content.title, content.body));
        }

        let channel = payload
            .channel
            .map(|c| c.trim().to_string())
            .filter(|c| !c.is_empty())
            .or(node.channel)
            .or_else(|| rendered.as_ref().map(|r| r.0.clone()))
            .ok_or_else(|| AuthError::Validation("channel is required".into()))?;
        let recipients = payload.recipients.unwrap_or(node.recipients);
        if recipients.is_empty() {
            return Err(AuthError::Validation(
                "at least one recipient is required".into(),
            ));
        }
        if recipients.len() > MAX_RECIPIENTS {
            return Err(AuthError::Validation(format!(
                "at most {MAX_RECIPIENTS} recipients per request"
            )));
        }
        let title = payload
            .title
            .map(|t| t.trim().to_string())
            .filter(|t| !t.is_empty())
            .or_else(|| rendered.as_ref().map(|r| r.1.clone()))
            .ok_or_else(|| AuthError::Validation("title is required".into()))?;
        let body = payload.body.unwrap_or_default().trim().to_string();
        let body = if body.is_empty() {
            rendered.as_ref().map(|r| r.2.clone()).unwrap_or_default()
        } else {
            body
        };

        let now = chrono::Utc::now();
        let definition = record.flow_definition.clone();
        let flow_id = record.flow_id.clone();
        let chain = provider_chain(&definition, &channel);
        let recipients_json = serde_json::Value::Array(
            recipients
                .iter()
                .map(|r| serde_json::Value::String(r.clone()))
                .collect(),
        );
        let record = self
            .sends
            .create(&project_id, &channel, &recipients_json, &title, &body, now)
            .await
            .map_err(Self::map_store_error)?;

        let mut delivered: Option<(String, Option<String>)> = None;
        let mut last_error = format!("no provider route for channel '{channel}'");
        if !chain.is_empty() {
            match deliver_through_chain(
                &self.providers,
                &self.adapters,
                &project_id,
                &chain,
                &channel,
                &recipients,
                &title,
                &body,
            )
            .await
            {
                Ok(done) => delivered = Some(done),
                Err(e) => last_error = e,
            }
        }
        for outcome in fire_http_steps(&self.http, &collect_http_steps(&definition)).await {
            if !outcome.ok {
                tracing::warn!(
                    flow_id = %flow_id,
                    node_id = %outcome.node_id,
                    status = ?outcome.status,
                    error = ?outcome.error,
                    "inbound HTTP step failed"
                );
            }
        }
        match delivered {
            Some((provider, external_id)) => {
                let done = self
                    .sends
                    .mark_completed(&record.id, &provider, external_id.as_deref())
                    .await
                    .map_err(Self::map_store_error)?;
                Ok(crate::domain::sends::services::to_send(
                    done.unwrap_or(record),
                ))
            }
            None => {
                let failed = self
                    .sends
                    .mark_failed(&record.id, &last_error)
                    .await
                    .map_err(Self::map_store_error)?;
                Ok(crate::domain::sends::services::to_send(
                    failed.unwrap_or(record),
                ))
            }
        }
    }
}
