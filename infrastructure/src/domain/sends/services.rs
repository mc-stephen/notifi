//! Record-first API sends with inline default-flow execution (Phase A).
//!
//! `POST /v1/notifications` persists the send, resolves the project's
//! default flow, walks its provider chain by priority with fallback, and
//! marks the row completed/failed — all inside the request. Scheduled
//! sends, retries, and per-attempt history arrive in Phase B.

use std::sync::Arc;

use crate::domain::auth::entities::UserId;
use crate::domain::auth::errors::AuthError;
use crate::domain::flows::{FlowService, handles_event, provider_chain};
use crate::domain::sends::entities::{Send, SendInput, SendStatus};
use crate::ports::auth_store::StoreError;
use crate::ports::channel_provider_store::ChannelProviderStore;
use crate::ports::send_store::{SendRecord, SendStore};

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

    fn map_store_error(e: StoreError) -> AuthError {
        match e {
            StoreError::Conflict => AuthError::Conflict("conflicting send change".into()),
            StoreError::Storage(m) => AuthError::Storage(m),
        }
    }

    async fn check_visible(&self, actor: UserId, project_id: &str) -> Result<(), AuthError> {
        if !self
            .store
            .project_visible(actor, project_id)
            .await
            .map_err(Self::map_store_error)?
        {
            return Err(AuthError::NotFound("project not found".into()));
        }
        Ok(())
    }

    /// Sends queued for the deliveries view, newest first.
    pub async fn list(
        &self,
        actor: UserId,
        project_id: &str,
        limit: i64,
    ) -> Result<Vec<Send>, AuthError> {
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
    ) -> Result<Send, AuthError> {
        self.check_visible(actor, project_id).await?;

        let channel = input.channel.trim();
        if channel.is_empty() {
            return Err(AuthError::Validation("channel is required".into()));
        }
        if input.recipients.is_empty() {
            return Err(AuthError::Validation(
                "at least one recipient is required".into(),
            ));
        }
        if input.recipients.len() > MAX_RECIPIENTS {
            return Err(AuthError::Validation(format!(
                "at most {MAX_RECIPIENTS} recipients per request"
            )));
        }
        if input.title.trim().is_empty() {
            return Err(AuthError::Validation("title is required".into()));
        }
        let now = chrono::Utc::now();
        let send_at = input.send_at.unwrap_or(now);
        if send_at > now + chrono::Duration::seconds(60) {
            return Err(AuthError::Validation(
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
            return Err(AuthError::Conflict(
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

        // Run every qualifying flow, oldest first. The send completes if
        // at least one flow delivers fully, else fails with the last error.
        let mut delivered: Option<(String, Option<String>)> = None;
        let mut last_error = format!("no provider route for channel '{channel}'");
        for flow in &candidates {
            let chain = provider_chain(&flow.definition, channel);
            if chain.is_empty() {
                continue;
            }
            match crate::domain::flows::deliver_through_chain(
                &self.providers,
                &self.adapters,
                project_id,
                &chain,
                channel,
                &input.recipients,
                &input.title,
                &input.body,
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
        }

        match delivered {
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
                    .mark_failed(&record.id, &last_error)
                    .await
                    .map_err(Self::map_store_error)?;
                Ok(to_send(failed.unwrap_or(record)))
            }
        }
    }
}
