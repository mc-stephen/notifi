//! Inbound webhook fires: outside systems POST a trigger token to run
//! the owning flow.
//!
//! Resolution is caller-wins with flow fallback: each of channel,
//! recipients, title, body, and template comes from the call when
//! present, else the trigger node's stored config, else the picked
//! template's render. The fallback template's declared `{{refs}}` render
//! from the trigger's stored `values` presets (the config panel's
//! dynamic-data inputs; caller-side variable plumbing rides on top the
//! day inbound carries it — proven by
//! `template_fallback_presets_render_declared_refs` in
//! `server/tests/inbound_api.rs`). Anything still missing fails closed.
//! Runs record send rows, so deliveries show inbound fires like any
//! other send.

use std::sync::Arc;

use crate::domain::error::AppError;
use crate::domain::flows::{
    collect_http_steps, deliver_through_chain, fire_http_steps, provider_chain,
};
use crate::domain::inbound::entities::{
    InboundPayload, extract_trigger_tokens, ip_allowed, webhook_node_data,
};
use crate::domain::schedules::entities::render_template_content;
use crate::domain::sends::entities::Send;
use crate::ports::auth_store::StoreError;
use crate::ports::inbound_probe_store::{InboundProbeRecord, InboundProbeStore};
use crate::ports::send_store::SendStore;
use crate::ports::templates_store::TemplatesStore;
use crate::ports::trigger_token_store::TriggerTokenStore;

/// Max recipients per inbound fire (mirrors the ad-hoc send bound).
const MAX_RECIPIENTS: usize = 25;

pub struct InboundService {
    tokens: Arc<dyn TriggerTokenStore>,
    probes: Arc<dyn InboundProbeStore>,
    sends: Arc<dyn SendStore>,
    templates: Arc<dyn TemplatesStore>,
    providers: Arc<dyn crate::ports::ChannelProviderStore>,
    adapters: Arc<dyn crate::ports::AdapterFactory>,
    http: Arc<dyn crate::ports::HttpRequester>,
}

impl InboundService {
    pub fn new(
        tokens: Arc<dyn TriggerTokenStore>,
        probes: Arc<dyn InboundProbeStore>,
        sends: Arc<dyn SendStore>,
        templates: Arc<dyn TemplatesStore>,
        providers: Arc<dyn crate::ports::ChannelProviderStore>,
        adapters: Arc<dyn crate::ports::AdapterFactory>,
        http: Arc<dyn crate::ports::HttpRequester>,
    ) -> Self {
        Self {
            tokens,
            probes,
            sends,
            templates,
            providers,
            adapters,
            http,
        }
    }

    fn map_store_error(e: StoreError) -> AppError {
        match e {
            StoreError::Conflict => AppError::Conflict("conflicting trigger change".into()),
            StoreError::Storage(m) => AppError::Storage(m),
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

    /// The builder's "Incoming data": the last authorized ping a webhook
    /// door received. A `token` scopes to that door (verified to belong
    /// to the flow); without one, the flow's most recent ping across its
    /// doors.
    pub async fn last_probe(
        &self,
        project_id: &str,
        flow_id: &str,
        token: Option<&str>,
    ) -> Result<Option<InboundProbeRecord>, AppError> {
        let probe = match token {
            Some(token) => {
                let probe = self
                    .probes
                    .last_for_token(token)
                    .await
                    .map_err(Self::map_store_error)?;
                match probe {
                    Some(p) if p.flow_id == flow_id && p.project_id == project_id => Some(p),
                    _ => None,
                }
            }
            None => self
                .probes
                .last_for_flow(project_id, flow_id)
                .await
                .map_err(Self::map_store_error)?,
        };
        Ok(probe)
    }

    /// Fires the flow owning `token` with caller-wins/fallback resolution.
    /// Unknown tokens read as missing; delivery failures still return the
    /// recorded (failed) send, mirroring ad-hoc sends.
    ///
    /// Door auth lives on the node: any configured secret demands a
    /// matching `x-notifi-secret` header (401 when missing/wrong) and
    /// any configured IP rule demands the caller inside it (403
    /// otherwise). Empty lists keep the door open — the pre-auth
    /// default, unchanged behavior. Proofs:
    /// `webhook_fire_secret_*` + `webhook_fire_ip_allowlist_*` +
    /// `webhook_fire_open_door_without_config` in `tests/inbound_api.rs`.
    pub async fn fire(
        &self,
        token: &str,
        secret: Option<String>,
        client_ip: Option<std::net::IpAddr>,
        payload: InboundPayload,
        probe: crate::domain::inbound::entities::InboundProbe,
    ) -> Result<Send, AppError> {
        let record = self
            .tokens
            .lookup(token)
            .await
            .map_err(Self::map_store_error)?
            .ok_or_else(|| AppError::NotFound("unknown webhook trigger".into()))?;
        let node = webhook_node_data(&record.flow_definition, &record.node_id)
            .ok_or_else(|| AppError::NotFound("webhook trigger no longer configured".into()))?;

        // The door's own checks run before any template/recipients work.
        if !node.secrets.is_empty() {
            let presented = secret.as_deref().unwrap_or("");
            let ok = node.secrets.iter().any(|key| {
                // Full-entropy generated keys; a timing leak can't walk
                // 128-bit space.
                key == presented
            });
            if !ok {
                return Err(AppError::Unauthorized);
            }
        }
        if !node.allowed_ips.is_empty() {
            let ip = client_ip.ok_or_else(|| {
                AppError::Forbidden("caller address unavailable for this webhook's IP rules".into())
            })?;
            if !ip_allowed(&node.allowed_ips, ip) {
                return Err(AppError::Forbidden(
                    "caller address outside this webhook's IP allow-list".into(),
                ));
            }
        }

        // The door's checks passed — capture what the caller sent for
        // the builder's "Incoming data" tab. Best-effort: a probe-store
        // failure never fails the fire (this is debug data, not the
        // delivery). Door-auth failures never get here, so rejected
        // pings aren't probed.
        let probe_record = InboundProbeRecord {
            token: token.to_string(),
            flow_id: record.flow_id.clone(),
            project_id: record.project_id.clone(),
            fired_at: chrono::Utc::now(),
            client_ip: probe.client_ip,
            content_type: probe.content_type,
            user_agent: probe.user_agent,
            forwarded_for: probe.forwarded_for,
            body: probe.body,
        };
        let _ = self.probes.record(probe_record).await;

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
                .ok_or_else(|| AppError::Validation("template not found in this project".into()))?;
            // Declared refs render from the trigger's stored presets
            // (the node's `values` map — one entry per declared ref, see
            // the webhook fallback panel). Caller-side variable plumbing
            // is still on the road: a payload `data` map will override
            // these the day inbound carries it (proven by
            // `template_fallback_presets_render_declared_refs` in
            // `server/tests/inbound_api.rs`).
            let content =
                render_template_content(&template.content, &node.values).ok_or_else(|| {
                    AppError::Validation("template covers no renderable channels".into())
                })?;
            rendered = Some((content.channel, content.title, content.body));
        }

        let channel = payload
            .channel
            .map(|c| c.trim().to_string())
            .filter(|c| !c.is_empty())
            .or(node.channel)
            .or_else(|| rendered.as_ref().map(|r| r.0.clone()))
            .ok_or_else(|| AppError::Validation("channel is required".into()))?;
        let recipients = payload.recipients.unwrap_or(node.recipients);
        if recipients.is_empty() {
            return Err(AppError::Validation(
                "at least one recipient is required".into(),
            ));
        }
        if recipients.len() > MAX_RECIPIENTS {
            return Err(AppError::Validation(format!(
                "at most {MAX_RECIPIENTS} recipients per request"
            )));
        }
        let title = payload
            .title
            .map(|t| t.trim().to_string())
            .filter(|t| !t.is_empty())
            .or_else(|| rendered.as_ref().map(|r| r.1.clone()))
            .ok_or_else(|| AppError::Validation("title is required".into()))?;
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
