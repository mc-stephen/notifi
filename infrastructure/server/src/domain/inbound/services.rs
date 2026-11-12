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
//! other send — and every authorized fire records a run row (mode
//! `webhook`) in `flow_runs` for the builder's History tab (see
//! `domain::flows::runs`); recording is best-effort, and door-rejected
//! pings (401/403) record no run.

use std::sync::Arc;
use std::time::Instant;

use crate::domain::error::AppError;
use crate::domain::flows::runs::{MODE_WEBHOOK, RUN_FAILED, RUN_SUCCEEDED, RunRecorder, RunStep};
use crate::domain::flows::{
    collect_http_steps, deliver_through_chain, fire_http_steps, provider_chain,
};
use crate::domain::inbound::entities::{
    InboundPayload, WebhookNodeConfig, extract_trigger_tokens, ip_allowed, webhook_node_data,
};
use crate::domain::schedules::entities::render_template_content;
use crate::domain::sends::entities::{Send, SendStatus};
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
    /// Optional run-history recorder (unset by fakes; compose wires it).
    runs: Option<Arc<RunRecorder>>,
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
            runs: None,
        }
    }

    /// Attaches the run-history recorder (builder History tab).
    pub fn with_run_recorder(mut self, runs: Arc<RunRecorder>) -> Self {
        self.runs = Some(runs);
        self
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
        let started = Instant::now();
        let started_at = chrono::Utc::now();
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
        // pings aren't probed — and they never record a run either.
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
        let flow_id = record.flow_id.clone();
        let node_id = record.node_id.clone();
        let definition = record.flow_definition.clone();
        let (result, steps) = self
            .run_flow(&project_id, &node_id, &definition, node, payload, started)
            .await;

        // Run history — the delivery already happened (or failed
        // closed); the history write itself never fails the fire.
        if let Some(runs) = self.runs.as_ref() {
            let send_id = result.as_ref().ok().map(|s| s.id.clone());
            let (status, error) = match &result {
                Ok(send) => match send.status {
                    SendStatus::Completed => (RUN_SUCCEEDED, None),
                    _ => (RUN_FAILED, send.last_error.clone()),
                },
                Err(e) => (RUN_FAILED, Some(e.to_string())),
            };
            runs.record(
                MODE_WEBHOOK,
                &project_id,
                &flow_id,
                status,
                error,
                &steps,
                send_id.as_deref(),
                started_at,
            )
            .await;
        }
        result
    }

    /// The delivery half of a fire: template render → fallback
    /// resolution → provider chain → HTTP side steps → send record.
    /// Returns the send (or the closed validation failure) plus the run
    /// trace; steps ride graph node ids so the History tab can pin
    /// outcomes.
    #[allow(clippy::too_many_arguments)]
    async fn run_flow(
        &self,
        project_id: &str,
        node_id: &str,
        definition: &serde_json::Value,
        node: WebhookNodeConfig,
        payload: InboundPayload,
        started: Instant,
    ) -> (Result<Send, AppError>, Vec<RunStep>) {
        let mut steps: Vec<RunStep> = Vec::new();
        steps.push(RunStep {
            node_id: node_id.to_string(),
            kind: "trigger".into(),
            label: "webhook door".into(),
            status: "ok".into(),
            detail: None,
            ms: started.elapsed().as_millis() as u64,
        });

        // Template: caller pick wins, else the node's pick, then render.
        let template_id = payload.template_id.or(node.template_id);
        let mut rendered: Option<(String, String, String)> = None;
        if let Some(tid) = template_id.as_deref() {
            let template = match self.templates.get_by_id(tid).await {
                Ok(Some(t)) if t.project_id == project_id => t,
                Ok(_) => {
                    steps.push(RunStep {
                        node_id: node_id.to_string(),
                        kind: "template".into(),
                        label: tid.to_string(),
                        status: "failed".into(),
                        detail: Some("template not found in this project".into()),
                        ms: started.elapsed().as_millis() as u64,
                    });
                    return (
                        Err(AppError::Validation(
                            "template not found in this project".into(),
                        )),
                        steps,
                    );
                }
                Err(e) => return (Err(Self::map_store_error(e)), steps),
            };
            match render_template_content(&template.content, &node.values) {
                Some(content) => {
                    steps.push(RunStep {
                        node_id: node_id.to_string(),
                        kind: "template".into(),
                        label: template.name.clone(),
                        status: "ok".into(),
                        detail: None,
                        ms: started.elapsed().as_millis() as u64,
                    });
                    rendered = Some((content.channel, content.title, content.body));
                }
                None => {
                    steps.push(RunStep {
                        node_id: node_id.to_string(),
                        kind: "template".into(),
                        label: template.name.clone(),
                        status: "failed".into(),
                        detail: Some("template covers no renderable channels".into()),
                        ms: started.elapsed().as_millis() as u64,
                    });
                    return (
                        Err(AppError::Validation(
                            "template covers no renderable channels".into(),
                        )),
                        steps,
                    );
                }
            }
        }

        // Fallback resolution — anything still missing fails closed.
        let resolve_failed = |steps: &mut Vec<RunStep>, detail: String, started: Instant| {
            steps.push(RunStep {
                node_id: node_id.to_string(),
                kind: "resolve".into(),
                label: "resolve".into(),
                status: "failed".into(),
                detail: Some(detail),
                ms: started.elapsed().as_millis() as u64,
            });
        };
        let channel = match payload
            .channel
            .map(|c| c.trim().to_string())
            .filter(|c| !c.is_empty())
            .or(node.channel)
            .or_else(|| rendered.as_ref().map(|r| r.0.clone()))
        {
            Some(c) => c,
            None => {
                let e = "channel is required".to_string();
                resolve_failed(&mut steps, e.clone(), started);
                return (Err(AppError::Validation(e)), steps);
            }
        };
        let recipients = payload.recipients.unwrap_or(node.recipients);
        if recipients.is_empty() {
            let e = "at least one recipient is required".to_string();
            resolve_failed(&mut steps, e.clone(), started);
            return (Err(AppError::Validation(e)), steps);
        }
        if recipients.len() > MAX_RECIPIENTS {
            let e = format!("at most {MAX_RECIPIENTS} recipients per request");
            resolve_failed(&mut steps, e.clone(), started);
            return (Err(AppError::Validation(e)), steps);
        }
        let title = match payload
            .title
            .map(|t| t.trim().to_string())
            .filter(|t| !t.is_empty())
            .or_else(|| rendered.as_ref().map(|r| r.1.clone()))
        {
            Some(t) => t,
            None => {
                let e = "title is required".to_string();
                resolve_failed(&mut steps, e.clone(), started);
                return (Err(AppError::Validation(e)), steps);
            }
        };
        let body = payload.body.unwrap_or_default().trim().to_string();
        let body = if body.is_empty() {
            rendered.as_ref().map(|r| r.2.clone()).unwrap_or_default()
        } else {
            body
        };

        let now = chrono::Utc::now();
        let recipients_json = serde_json::Value::Array(
            recipients
                .iter()
                .map(|r| serde_json::Value::String(r.clone()))
                .collect(),
        );
        let send_row = match self
            .sends
            .create(project_id, &channel, &recipients_json, &title, &body, now)
            .await
        {
            Ok(row) => row,
            Err(e) => return (Err(Self::map_store_error(e)), steps),
        };

        // The provider chain walk (empty chain fails with the terminal
        // message from the walker itself) + the HTTP side steps.
        let chain = provider_chain(definition, &channel);
        let (delivery, attempts) = deliver_through_chain(
            &self.providers,
            &self.adapters,
            project_id,
            &chain,
            &channel,
            &recipients,
            &title,
            &body,
        )
        .await;
        for a in attempts {
            steps.push(RunStep {
                node_id: a.node_id,
                kind: "provider".into(),
                label: a.provider_id,
                status: if a.ok { "ok" } else { "failed" }.into(),
                detail: a.error,
                ms: a.ms,
            });
        }
        let http_steps = collect_http_steps(definition);
        for outcome in fire_http_steps(&self.http, &http_steps).await {
            let label = http_steps
                .iter()
                .find(|s| s.node_id == outcome.node_id)
                .map(|s| format!("{} {}", s.method, s.url))
                .unwrap_or_else(|| "http step".into());
            let detail = if !outcome.ok {
                tracing::warn!(
                    node_id = %outcome.node_id,
                    status = ?outcome.status,
                    error = ?outcome.error,
                    "inbound HTTP step failed"
                );
                outcome
                    .error
                    .or_else(|| outcome.status.map(|s| format!("status {s}")))
            } else {
                outcome.status.map(|s| format!("status {s}"))
            };
            steps.push(RunStep {
                node_id: outcome.node_id,
                kind: "http-request".into(),
                label,
                status: if outcome.ok { "ok" } else { "failed" }.into(),
                detail,
                ms: 0,
            });
        }
        match delivery {
            Ok((provider, external_id)) => {
                match self
                    .sends
                    .mark_completed(&send_row.id, &provider, external_id.as_deref())
                    .await
                {
                    Ok(done) => (
                        Ok(crate::domain::sends::services::to_send(
                            done.unwrap_or(send_row),
                        )),
                        steps,
                    ),
                    Err(e) => (Err(Self::map_store_error(e)), steps),
                }
            }
            Err(last_error) => match self.sends.mark_failed(&send_row.id, &last_error).await {
                Ok(failed) => (
                    Ok(crate::domain::sends::services::to_send(
                        failed.unwrap_or(send_row),
                    )),
                    steps,
                ),
                Err(e) => (Err(Self::map_store_error(e)), steps),
            },
        }
    }
}
