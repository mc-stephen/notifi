//! Record-first API sends with inline default-flow execution (Phase A).
//!
//! `POST /v1/notifications` persists the send, resolves the project's
//! default flow, walks its provider chain by priority with fallback, and
//! marks the row completed/failed — all inside the request. Scheduled
//! sends, retries, and per-attempt history arrive in Phase B.

use std::sync::Arc;

use crate::domain::auth::entities::UserId;
use crate::domain::error::AppError;
use crate::domain::flows::entities::ProviderStep;
use crate::domain::flows::{FlowService, handles_event, provider_chain};
use crate::domain::sends::entities::{Send, SendInput, SendStatus};
use crate::ports::auth_store::StoreError;
use crate::ports::channel_provider_store::ChannelProviderStore;
use crate::ports::send_store::{SendRecord, SendStore};

/// What to deliver and where — the shared message bundle for direct API
/// sends and campaign-expanded rows alike (keeps `walk_candidates`'s
/// argument list honest).
struct Delivery<'a> {
    project_id: &'a str,
    channel: &'a str,
    recipients: &'a [String],
    title: &'a str,
    body: &'a str,
}

/// Max recipients per request (bounds inline execution time).
const MAX_RECIPIENTS: usize = 25;

pub(crate) fn to_send(record: SendRecord) -> Send {
    Send {
        id: record.id,
        project_id: record.project_id,
        channel: record.channel,
        recipients: record
            .recipients
            .as_array()
            .map(|a| {
                a.iter()
                    .filter_map(|v| v.as_str().map(str::to_string))
                    .collect()
            })
            .unwrap_or_default(),
        title: record.title,
        body: record.body,
        send_at: record.send_at,
        status: SendStatus::parse(&record.status).unwrap_or(SendStatus::Pending),
        attempts: record.attempts,
        provider: record.provider,
        external_id: record.external_id,
        last_error: record.last_error,
        campaign_id: record.campaign_id,
        created_at: record.created_at,
        updated_at: record.updated_at,
    }
}

pub struct SendService {
    store: Arc<dyn SendStore>,
    flows: Arc<FlowService>,
    providers: Arc<dyn ChannelProviderStore>,
    adapters: Arc<dyn crate::ports::AdapterFactory>,
    http: Arc<dyn crate::ports::HttpRequester>,
}

impl SendService {
    pub fn new(
        store: Arc<dyn SendStore>,
        flows: Arc<FlowService>,
        providers: Arc<dyn ChannelProviderStore>,
        adapters: Arc<dyn crate::ports::AdapterFactory>,
        http: Arc<dyn crate::ports::HttpRequester>,
    ) -> Self {
        Self {
            store,
            flows,
            providers,
            adapters,
            http,
        }
    }

    fn map_store_error(e: StoreError) -> AppError {
        match e {
            StoreError::Conflict => AppError::Conflict("conflicting send change".into()),
            StoreError::Storage(m) => AppError::Storage(m),
        }
    }

    async fn check_visible(&self, actor: UserId, project_id: &str) -> Result<(), AppError> {
        if !self
            .store
            .project_visible(actor, project_id)
            .await
            .map_err(Self::map_store_error)?
        {
            return Err(AppError::NotFound("project not found".into()));
        }
        Ok(())
    }

    /// Sends queued for the deliveries view, newest first.
    pub async fn list(
        &self,
        actor: UserId,
        project_id: &str,
        limit: i64,
    ) -> Result<Vec<Send>, AppError> {
        self.check_visible(actor, project_id).await?;
        Ok(self
            .store
            .list_by_project(project_id, limit.clamp(1, 100))
            .await
            .map_err(Self::map_store_error)?
            .into_iter()
            .map(to_send)
            .collect())
    }

    /// Record-first send: persist, resolve the default flow, walk its
    /// provider chain by priority with fallback, mark completed/failed.
    /// Future `send_at` is refused (scheduled sends land in Phase B).
    /// Missing default flow fails closed (409) per product decision.
    pub async fn send(
        &self,
        actor: UserId,
        project_id: &str,
        input: SendInput,
    ) -> Result<Send, AppError> {
        self.check_visible(actor, project_id).await?;

        let channel = input.channel.trim();
        if channel.is_empty() {
            return Err(AppError::Validation("channel is required".into()));
        }
        if input.recipients.is_empty() {
            return Err(AppError::Validation(
                "at least one recipient is required".into(),
            ));
        }
        if input.recipients.len() > MAX_RECIPIENTS {
            return Err(AppError::Validation(format!(
                "at most {MAX_RECIPIENTS} recipients per request"
            )));
        }
        if input.title.trim().is_empty() {
            return Err(AppError::Validation("title is required".into()));
        }
        let now = chrono::Utc::now();
        let send_at = input.send_at.unwrap_or(now);
        if send_at > now + chrono::Duration::seconds(60) {
            return Err(AppError::Validation(
                "scheduled sends arrive in Phase B — send now or omit send_at".into(),
            ));
        }

        let flows = self.flows.list(actor, project_id).await?;
        let candidates: Vec<_> = flows
            .into_iter()
            .filter(|f| handles_event(&f.definition, "notification.created"))
            .collect();
        if candidates.is_empty() {
            // No stored flag: "default" is emergent — at least one flow
            // must carry a notification.created trigger.
            return Err(AppError::Conflict(
                "no flow handles notification.created — create one on the Flows page first".into(),
            ));
        }

        let recipients_json = serde_json::Value::Array(
            input
                .recipients
                .iter()
                .map(|r| serde_json::Value::String(r.clone()))
                .collect(),
        );
        let record = self
            .store
            .create(
                project_id,
                channel,
                &recipients_json,
                input.title.trim(),
                &input.body,
                send_at,
            )
            .await
            .map_err(Self::map_store_error)?;

        let delivery = Delivery {
            project_id,
            channel,
            recipients: &input.recipients,
            title: input.title.trim(),
            body: &input.body,
        };
        let (provider, external_id) = self.walk_candidates(&candidates, &delivery, false).await;

        match provider {
            Some((provider, external_id)) => {
                let done = self
                    .store
                    .mark_completed(&record.id, &provider, external_id.as_deref())
                    .await
                    .map_err(Self::map_store_error)?;
                Ok(to_send(done.unwrap_or(record)))
            }
            None => {
                let failed = self
                    .store
                    .mark_failed(&record.id, &external_id.unwrap_or_default())
                    .await
                    .map_err(Self::map_store_error)?;
                Ok(to_send(failed.unwrap_or(record)))
            }
        }
    }

    /// Flow candidates for a project handling `notification.created`,
    /// without a caller check — worker/system contexts only (campaign
    /// pipeline resolution). Never route this to HTTP handlers.
    pub async fn default_flow_candidates(
        &self,
        project_id: &str,
    ) -> Result<Vec<crate::domain::flows::Flow>, AppError> {
        Ok(self
            .flows
            .list_by_project(project_id)
            .await?
            .into_iter()
            .filter(|f| handles_event(&f.definition, "notification.created"))
            .collect())
    }

    /// Worker step: processes up to `limit` pending rows expanded from one
    /// campaign, oldest first. Campaign rows deliver **directly** through
    /// the project's connected providers for the row's channel — flows
    /// play no part in campaign delivery. Connected, enabled configs
    /// become the chain in store order with fallback; the campaign's
    /// explicit provider pick, when given, is tried first (it moves to
    /// the front of the chain). Nothing connected → rows fail closed
    /// with the Providers-page guidance.
    /// Returns how many rows were processed.
    pub async fn process_campaign_batch(
        &self,
        campaign_id: &str,
        pick_provider: Option<&str>,
        limit: usize,
    ) -> Result<usize, AppError> {
        let rows = self
            .store
            .list_by_campaign(campaign_id)
            .await
            .map_err(Self::map_store_error)?;
        let pending = rows
            .iter()
            .filter(|r| r.status == SendStatus::Pending.as_str())
            .take(limit);
        let mut processed = 0usize;
        for row in pending {
            let recipients: Vec<String> = row
                .recipients
                .as_array()
                .map(|a| {
                    a.iter()
                        .filter_map(|v| v.as_str().map(str::to_string))
                        .collect()
                })
                .unwrap_or_default();
            let configs = self
                .providers
                .list_by_project(&row.project_id)
                .await
                .map_err(AppError::Storage)?;
            let mut chain: Vec<ProviderStep> = configs
                .iter()
                .enumerate()
                .filter(|(_, c)| c.channel_id == row.channel && c.enabled)
                .map(|(priority, c)| ProviderStep {
                    channel_id: row.channel.clone(),
                    provider_id: c.provider_id.clone(),
                    priority: priority as i64,
                })
                .collect();
            // The user's named provider goes first; the rest keep their
            // store order as fallbacks. A pick that no longer matches a
            // connected config simply stays out of the chain.
            if let Some(pick) = pick_provider.map(str::trim).filter(|p| !p.is_empty())
                && let Some(pos) = chain.iter().position(|s| s.provider_id == pick)
            {
                let step = chain.remove(pos);
                chain.insert(0, step);
            }
            if chain.is_empty() {
                let _ = self
                    .store
                    .mark_failed(
                        &row.id,
                        &format!(
                            "no provider connected for channel '{}' — connect one on the Providers page",
                            row.channel
                        ),
                    )
                    .await;
                processed += 1;
                continue;
            }
            match crate::domain::flows::deliver_through_chain(
                &self.providers,
                &self.adapters,
                &row.project_id,
                &chain,
                &row.channel,
                &recipients,
                &row.title,
                &row.body,
            )
            .await
            {
                Ok((provider, external_id)) => {
                    let _ = self
                        .store
                        .mark_completed(&row.id, &provider, external_id.as_deref())
                        .await;
                }
                Err(e) => {
                    let _ = self.store.mark_failed(&row.id, &e).await;
                }
            }
            processed += 1;
        }
        Ok(processed)
    }

    /// Walks the given flows — each handling `notification.created` —
    /// delivering the message through each flow's provider chain for the
    /// channel with fallback. HTTP Request steps fire once per flow,
    /// whatever the outcome; failures log server-side only. Returns
    /// `Some((provider, external id))` of the first flow that delivered
    /// fully, else `None` plus the last error message.
    ///
    /// `stop_on_first` short-circuits after the first flow delivers —
    /// used by campaign rows where one row is one message. Direct sends
    /// keep walking every candidate flow (pre-campaign behavior).
    async fn walk_candidates(
        &self,
        flows: &[crate::domain::flows::Flow],
        delivery: &Delivery<'_>,
        stop_on_first: bool,
    ) -> (Option<(String, Option<String>)>, Option<String>) {
        let mut delivered: Option<(String, Option<String>)> = None;
        let mut last_error = format!("no provider route for channel '{}'", delivery.channel);
        for flow in flows {
            let chain = provider_chain(&flow.definition, delivery.channel);
            if chain.is_empty() {
                continue;
            }
            match crate::domain::flows::deliver_through_chain(
                &self.providers,
                &self.adapters,
                delivery.project_id,
                &chain,
                delivery.channel,
                delivery.recipients,
                delivery.title,
                delivery.body,
            )
            .await
            {
                Ok(done) => {
                    if delivered.is_none() {
                        delivered = Some(done);
                    }
                }
                Err(e) => {
                    last_error = e;
                }
            }
            // HTTP Request steps fire once per flow run, whatever the
            // delivery outcome. Failures log server-side only: they never
            // fail the send and never appear in deliveries.
            for outcome in crate::domain::flows::fire_http_steps(
                &self.http,
                &crate::domain::flows::collect_http_steps(&flow.definition),
            )
            .await
            {
                if !outcome.ok {
                    tracing::warn!(
                        flow_id = %flow.id,
                        node_id = %outcome.node_id,
                        status = ?outcome.status,
                        error = ?outcome.error,
                        "flow HTTP step failed"
                    );
                }
            }
            if stop_on_first && delivered.is_some() {
                break;
            }
        }
        (delivered, Some(last_error))
    }
}
