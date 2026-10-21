//! Campaign lifecycle: compose, schedule, fire, drain, cancel.
//!
//! Doctrine recap: a campaign is fully flow-less. It expands its audience
//! into `pending` send rows (one per recipient, `campaign_id` set) and
//! those rows deliver **directly** through the project's connected
//! providers for the campaign's channel — the Providers page is the only
//! delivery prerequisite, and a missing connection fails rows with
//! guidance rather than blocking the send. Expansion happens at fire
//! time so newly-registered recipients are included; stats always derive
//! from the send rows.
//!
//! Batching: `run_now` expands everything, then processes only the first
//! batch inline; the worker sweep drains the rest per tick. That is what
//! makes pause (in-flight batches finish, pending rows wait) and cancel
//! (pending rows dropped) real operations. One worker is expected;
//! concurrent sweeps race on rows between fetch and mark, mirroring the
//! broadcast sweep semantics (see ARCHITECTURE.md).

use std::collections::HashSet;
use std::sync::Arc;

use crate::domain::auth::entities::UserId;
use crate::domain::campaigns::entities::{
    AudienceMode, Campaign, CampaignInput, CampaignPatch, CampaignStats, CampaignStatus,
};
use crate::domain::error::AppError;
use crate::domain::sends::{SendService, entities::SendStatus};
use crate::ports::auth_store::StoreError;
use crate::ports::campaigns_store::{CampaignRecord, CampaignStore, CampaignUpdate, NewCampaign};
use crate::ports::recipients_store::RecipientsStore;
use crate::ports::send_store::{SendRecord, SendStore};
use crate::ports::templates_store::TemplatesStore;

/// Expansion safety bound: this round targets mail-merge scale, not
/// streaming-scale audiences (inline expansion, not per-row queue jobs).
pub const AUDIENCE_MAX: usize = 10_000;

/// Batch processed inline at fire time; the sweep drains the rest.
pub const BATCH_LIMIT: usize = 25;

/// Max campaigns touched per sweep tick.
const CLAIM_LIMIT: i64 = 10;

/// The contact key a channel maps to in a recipient's `contacts` blob.
/// Text channels share "phone"; every other channel owns a key named
/// after itself (e.g. `"email"`, `"android"`).
fn contact_key(channel: &str) -> &str {
    match channel {
        "sms" | "whatsapp" => "phone",
        other => other,
    }
}

/// The contact value a recipient can be reached at on `channel` — the
/// actual address/token (e.g. `"ada@example.com"`), since send rows and
/// provider chains speak addresses, not the brand's targeting keys.
fn contact_value<'a>(channel: &str, contacts: &'a serde_json::Value) -> Option<&'a str> {
    contacts
        .get(contact_key(channel))
        .and_then(|v| v.as_str())
        .map(str::trim)
        .filter(|v| !v.is_empty())
}

fn to_keys(value: &serde_json::Value) -> Vec<String> {
    value
        .as_array()
        .map(|a| {
            a.iter()
                .filter_map(|v| v.as_str().map(str::to_string))
                .collect()
        })
        .unwrap_or_default()
}

pub struct CampaignService {
    store: Arc<dyn CampaignStore>,
    send_store: Arc<dyn SendStore>,
    sends: Arc<SendService>,
    recipients: Arc<dyn RecipientsStore>,
    templates: Arc<dyn TemplatesStore>,
}

impl CampaignService {
    pub fn new(
        store: Arc<dyn CampaignStore>,
        send_store: Arc<dyn SendStore>,
        sends: Arc<SendService>,
        recipients: Arc<dyn RecipientsStore>,
        templates: Arc<dyn TemplatesStore>,
    ) -> Self {
        Self {
            store,
            send_store,
            sends,
            recipients,
            templates,
        }
    }

    fn map_store_error(e: StoreError) -> AppError {
        match e {
            StoreError::Conflict => AppError::Conflict("conflicting campaign change".into()),
            StoreError::Storage(m) => AppError::Storage(m),
        }
    }

    /// The human detail of an error (feeds `last_error`, not the wire).
    fn error_detail(e: &AppError) -> String {
        match e {
            AppError::Validation(m)
            | AppError::TokenInvalid(m)
            | AppError::TokenExpired(m)
            | AppError::Forbidden(m)
            | AppError::NotFound(m)
            | AppError::Conflict(m)
            | AppError::PlanRetired(m)
            | AppError::NotImplemented(m)
            | AppError::Storage(m) => m.clone(),
            _ => e.to_string(),
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

    fn to_campaign(record: CampaignRecord) -> Campaign {
        Campaign {
            id: record.id,
            project_id: record.project_id,
            name: record.name,
            description: record.description,
            created_by: record.created_by,
            channel: record.channel,
            template_id: record.template_id,
            title: record.title,
            body: record.body,
            audience_mode: AudienceMode::parse(&record.audience_mode).unwrap_or_default(),
            selected_user_ids: to_keys(&record.selected_user_ids),
            provider_id: record.provider_id,
            status: CampaignStatus::parse(&record.status).unwrap_or(CampaignStatus::Draft),
            send_at: record.send_at,
            started_at: record.started_at,
            completed_at: record.completed_at,
            last_error: record.last_error,
            created_at: record.created_at,
            updated_at: record.updated_at,
        }
    }

    fn selected_json(keys: &[String]) -> serde_json::Value {
        serde_json::Value::Array(
            keys.iter()
                .map(|k| serde_json::Value::String(k.clone()))
                .collect(),
        )
    }

    fn check_name(name: &str) -> Result<String, AppError> {
        let name = name.trim();
        if name.is_empty() {
            return Err(AppError::Validation("campaign name is required".into()));
        }
        if name.chars().count() > 100 {
            return Err(AppError::Validation(
                "campaign name is too long (max 100 characters)".into(),
            ));
        }
        Ok(name.to_string())
    }

    fn check_selected(keys: &[String]) -> Result<Vec<String>, AppError> {
        let mut seen = HashSet::new();
        let mut selected = Vec::new();
        for raw in keys {
            let key = raw.trim();
            if key.is_empty() {
                return Err(AppError::Validation(
                    "selected recipients must not be blank".into(),
                ));
            }
            if key.len() > 500 {
                return Err(AppError::Validation(
                    "selected recipient key is too long".into(),
                ));
            }
            if seen.insert(key.to_string()) {
                selected.push(key.to_string());
            }
        }
        if selected.len() > AUDIENCE_MAX {
            return Err(AppError::Validation(format!(
                "at most {AUDIENCE_MAX} recipients can be selected"
            )));
        }
        Ok(selected)
    }

    fn check_send_at(
        send_at: Option<chrono::DateTime<chrono::Utc>>,
    ) -> Result<Option<chrono::DateTime<chrono::Utc>>, AppError> {
        match send_at {
            Some(ts) if ts <= chrono::Utc::now() => Err(AppError::Validation(
                "schedule time must be in the future".into(),
            )),
            other => Ok(other),
        }
    }

    /// Trimmed provider pick. Lenient by design: the user names the
    /// connected provider to try first; if it later disappears from the
    /// connections, the drain skips it and the fallbacks deliver.
    fn check_provider(provider_id: &str) -> Result<String, AppError> {
        let trimmed = provider_id.trim();
        if trimmed.len() > 100 {
            return Err(AppError::Validation("provider pick is too long".into()));
        }
        Ok(trimmed.to_string())
    }

    /// Resolves the message at compose time. Campaigns compose from
    /// templates by design: both the template and its channel version
    /// are explicit (no default preference — the sender names the exact
    /// version they intend to deliver). The chosen render is snapshotted
    /// forever — later template edits never mutate a stored campaign.
    async fn resolve_message(
        &self,
        project_id: &str,
        template_id: &str,
        channel: &str,
    ) -> Result<(String, String, String), AppError> {
        let channel = channel.trim();
        if channel.is_empty() {
            return Err(AppError::Validation(
                "pick which version of the template to send (its channel)".into(),
            ));
        }
        let record = self
            .templates
            .get_by_id(template_id)
            .await
            .map_err(Self::map_store_error)?
            .filter(|r| r.project_id == project_id);
        let Some(record) = record else {
            return Err(AppError::Validation(
                "template not found in this project".into(),
            ));
        };
        let rendered =
            // Per-recipient personalization lands the day campaigns carry it;
            // today's campaigns compose a fixed body per launch.
            crate::domain::schedules::entities::render_channel_content(
                &record.content,
                channel,
                &std::collections::HashMap::new(),
            )
            .ok_or_else(|| {
                    AppError::Validation(format!(
                        "template has no content for {channel} — pick one it covers"
                    ))
                })?;
        Ok((rendered.channel, rendered.title, rendered.body))
    }

    /// Message stats for one campaign, derived from its send rows.
    pub async fn stats_for(&self, campaign_id: &str) -> Result<CampaignStats, AppError> {
        let rows = self
            .send_store
            .list_by_campaign(campaign_id)
            .await
            .map_err(Self::map_store_error)?;
        Ok(Self::stats_from_rows(&rows))
    }

    fn stats_from_rows(rows: &[SendRecord]) -> CampaignStats {
        let mut stats = CampaignStats::default();
        for row in rows {
            match SendStatus::parse(&row.status) {
                Some(SendStatus::Completed) => stats.delivered += 1,
                Some(SendStatus::Failed) => stats.failed += 1,
                Some(SendStatus::Pending | SendStatus::Sending) => stats.pending += 1,
                None => {}
            }
        }
        stats.total = rows.len();
        stats
    }

    /// All campaigns for a project, oldest first.
    pub async fn list(&self, actor: UserId, project_id: &str) -> Result<Vec<Campaign>, AppError> {
        self.check_visible(actor, project_id).await?;
        Ok(self
            .store
            .list_by_project(project_id)
            .await
            .map_err(Self::map_store_error)?
            .into_iter()
            .map(Self::to_campaign)
            .collect())
    }

    /// One campaign by id. Project-scoped: foreign rows read as missing.
    pub async fn get(
        &self,
        actor: UserId,
        project_id: &str,
        id: &str,
    ) -> Result<Campaign, AppError> {
        self.check_visible(actor, project_id).await?;
        self.store
            .get(id)
            .await
            .map_err(Self::map_store_error)?
            .filter(|r| r.project_id == project_id)
            .map(Self::to_campaign)
            .ok_or_else(|| AppError::NotFound("campaign not found".into()))
    }

    /// Compose a campaign. Creates never auto-send: a `send_at` parks the
    /// campaign as scheduled (the worker fires it), otherwise it lands as
    /// a plain draft. Both the template and its channel version are
    /// explicit; delivery resolves per-row against connected providers.
    pub async fn create(
        &self,
        actor: UserId,
        project_id: &str,
        input: CampaignInput,
    ) -> Result<Campaign, AppError> {
        self.check_visible(actor, project_id).await?;
        let name = Self::check_name(&input.name)?;
        if input.template_id.trim().is_empty() {
            return Err(AppError::Validation(
                "campaigns compose from a template — pick one".into(),
            ));
        }
        let (channel, title, body) = self
            .resolve_message(project_id, &input.template_id, &input.channel)
            .await?;
        let provider_pick = match &input.provider_id {
            Some(p) => Self::check_provider(p)?,
            None => String::new(),
        };
        let selected = Self::check_selected(&input.selected_user_ids)?;
        if input.audience_mode == AudienceMode::Custom && selected.is_empty() {
            return Err(AppError::Validation(
                "custom audiences need at least one selected recipient".into(),
            ));
        }
        let send_at = Self::check_send_at(input.send_at)?;
        let status = if send_at.is_some() {
            CampaignStatus::Scheduled
        } else {
            CampaignStatus::Draft
        };

        let record = self
            .store
            .create(&NewCampaign {
                project_id: project_id.to_string(),
                name,
                description: input.description.unwrap_or_default().trim().to_string(),
                created_by: actor.to_string(),
                channel,
                template_id: Some(input.template_id.trim().to_string()),
                title,
                body,
                audience_mode: input.audience_mode.as_str().to_string(),
                selected_user_ids: Self::selected_json(&selected),
                provider_id: provider_pick,
                status: status.as_str().to_string(),
                send_at,
            })
            .await
            .map_err(Self::map_store_error)?;
        Ok(Self::to_campaign(record))
    }

    /// Patch a draft/scheduled campaign. A different `template_id`
    /// re-renders (copy is only ever re-rendered, never hand-edited); a
    /// `channel` re-picks which template channel to snapshot; removing
    /// `send_at` turns a scheduled campaign back into a draft.
    pub async fn update(
        &self,
        actor: UserId,
        project_id: &str,
        id: &str,
        patch: CampaignPatch,
    ) -> Result<Campaign, AppError> {
        self.check_visible(actor, project_id).await?;
        let current = self
            .store
            .get(id)
            .await
            .map_err(Self::map_store_error)?
            .filter(|r| r.project_id == project_id);
        let current = current.ok_or_else(|| AppError::NotFound("campaign not found".into()))?;
        if !CampaignStatus::parse(&current.status)
            .unwrap_or(CampaignStatus::Draft)
            .editable()
        {
            return Err(AppError::Conflict(
                "only draft or scheduled campaigns can be edited".into(),
            ));
        }

        let name = match patch.name {
            Some(n) => Self::check_name(&n)?,
            None => current.name.clone(),
        };
        let description = match patch.description {
            Some(d) => d.unwrap_or_default().trim().to_string(),
            None => current.description.clone(),
        };

        // Message mode: a template patch (or channel re-pick) re-renders
        // the snapshot; nothing given keeps the stored one untouched. A
        // template patch without a channel re-pick falls back to the
        // stored channel and fails if the new template does not cover it.
        let current_template = current.template_id.clone().unwrap_or_default();
        let (template_id, channel, title, body) = match (&patch.template_id, &patch.channel) {
            (Some(tid), _) => {
                let effective = patch
                    .channel
                    .as_deref()
                    .map(str::trim)
                    .filter(|c| !c.is_empty())
                    .unwrap_or(current.channel.as_str());
                let (channel, title, body) =
                    self.resolve_message(project_id, tid, effective).await?;
                (Some(tid.clone()), channel, title, body)
            }
            (None, Some(channel)) => {
                // Re-picking the channel re-snapshots from the bound
                // template; template-less rows cannot exist in v2, but
                // fail honest if one ever does.
                if current_template.is_empty() {
                    return Err(AppError::Conflict(
                        "campaign has no template to re-render from".into(),
                    ));
                }
                let (channel, title, body) = self
                    .resolve_message(project_id, &current_template, channel)
                    .await?;
                (Some(current_template), channel, title, body)
            }
            (None, None) => (
                Some(current_template.clone()),
                current.channel.clone(),
                current.title.clone(),
                current.body.clone(),
            ),
        };

        let audience_mode = patch
            .audience_mode
            .unwrap_or(AudienceMode::parse(&current.audience_mode).unwrap_or_default());
        let selected = match &patch.selected_user_ids {
            Some(s) => Self::check_selected(s)?,
            None => to_keys(&current.selected_user_ids),
        };
        if audience_mode == AudienceMode::Custom && selected.is_empty() {
            return Err(AppError::Validation(
                "custom audiences need at least one selected recipient".into(),
            ));
        }

        let send_at = match patch.send_at {
            Some(next) => Self::check_send_at(next)?,
            None => current.send_at,
        };
        // The primary provider pick re-points leniently: Some("") clears
        // it (server order decides with fallbacks), None leaves it. A
        // picked provider that vanished from the connections still
        // stores fine — the drain just skips it at fire time.
        let provider_id = match &patch.provider_id {
            Some(p) => Self::check_provider(p)?,
            None => current.provider_id.clone(),
        };
        let desired = if send_at.is_some() {
            CampaignStatus::Scheduled
        } else {
            CampaignStatus::Draft
        };

        let mut record = self
            .store
            .update(
                id,
                &CampaignUpdate {
                    name,
                    description,
                    channel,
                    template_id,
                    title,
                    body,
                    audience_mode: audience_mode.as_str().to_string(),
                    selected_user_ids: Self::selected_json(&selected),
                    provider_id,
                    send_at,
                },
            )
            .await
            .map_err(Self::map_store_error)?
            .ok_or_else(|| AppError::NotFound("campaign not found".into()))?;
        // Mutable columns don't carry status; sync it when the schedule
        // mode changed (e.g. `sendAt: null` turns a scheduled campaign
        // back into a draft).
        if CampaignStatus::parse(&current.status).unwrap_or(CampaignStatus::Draft) != desired {
            record = self
                .store
                .transition(id, desired.as_str(), None, None, None)
                .await
                .map_err(Self::map_store_error)?
                .ok_or_else(|| AppError::NotFound("campaign not found".into()))?;
        }
        Ok(Self::to_campaign(record))
    }

    /// Delete a campaign by id. Draft/scheduled/cancelled/completed/failed
    /// rows die; running/paused ones must be settled first. Expanded send
    /// rows are never deleted here — delivery history stays.
    pub async fn delete(
        &self,
        actor: UserId,
        project_id: &str,
        id: &str,
    ) -> Result<bool, AppError> {
        self.check_visible(actor, project_id).await?;
        let existing = self
            .store
            .get(id)
            .await
            .map_err(Self::map_store_error)?
            .filter(|r| r.project_id == project_id);
        let Some(existing) = existing else {
            return Ok(false);
        };
        match CampaignStatus::parse(&existing.status).unwrap_or(CampaignStatus::Draft) {
            CampaignStatus::Running | CampaignStatus::Paused => {
                return Err(AppError::Conflict(
                    "pause or cancel the campaign before deleting it".into(),
                ));
            }
            _ => {}
        }
        self.store.delete(id).await.map_err(Self::map_store_error)
    }

    /// Fire now: expands the audience into pending rows, flips to
    /// `running`, and processes the first batch inline (the worker sweep
    /// drains the rest). Rows deliver directly against connected
    /// providers; nothing is pre-flight-gated. Draft or scheduled only.
    pub async fn run_now(
        &self,
        actor: UserId,
        project_id: &str,
        id: &str,
    ) -> Result<Campaign, AppError> {
        let campaign = self.get(actor, project_id, id).await?;
        match campaign.status {
            CampaignStatus::Draft | CampaignStatus::Scheduled => {}
            CampaignStatus::Running => {
                return Err(AppError::Conflict("campaign is already running".into()));
            }
            CampaignStatus::Paused => {
                return Err(AppError::Conflict(
                    "campaign is paused — resume it instead".into(),
                ));
            }
            CampaignStatus::Cancelled | CampaignStatus::Completed | CampaignStatus::Failed => {
                return Err(AppError::Conflict(
                    "campaign already finished — duplicate it for another run".into(),
                ));
            }
        }

        self.expand_audience(&campaign).await?;
        self.store
            .transition(
                id,
                CampaignStatus::Running.as_str(),
                Some(chrono::Utc::now()),
                None,
                None,
            )
            .await
            .map_err(Self::map_store_error)?;
        self.drain_batch(&campaign).await;
        let campaign = self.get(actor, project_id, id).await?;
        Ok(self.maybe_complete(campaign).await)
    }

    /// Pause: in-flight batches finish; pending send rows wait (until
    /// resume or cancel). Running only.
    pub async fn pause(
        &self,
        actor: UserId,
        project_id: &str,
        id: &str,
    ) -> Result<Campaign, AppError> {
        self.transition_lifecycle(
            actor,
            project_id,
            id,
            CampaignStatus::Running,
            CampaignStatus::Paused,
        )
        .await
    }

    /// Resume a paused campaign — the sweep picks it back up.
    pub async fn resume(
        &self,
        actor: UserId,
        project_id: &str,
        id: &str,
    ) -> Result<Campaign, AppError> {
        self.transition_lifecycle(
            actor,
            project_id,
            id,
            CampaignStatus::Paused,
            CampaignStatus::Running,
        )
        .await
    }

    /// Cancel: drops the campaign's pending send rows (fired ones stay as
    /// delivery history) and marks it terminal.
    pub async fn cancel(
        &self,
        actor: UserId,
        project_id: &str,
        id: &str,
    ) -> Result<Campaign, AppError> {
        self.check_visible(actor, project_id).await?;
        let campaign = self.get(actor, project_id, id).await?;
        match campaign.status {
            CampaignStatus::Draft
            | CampaignStatus::Scheduled
            | CampaignStatus::Running
            | CampaignStatus::Paused => {}
            _ => {
                return Err(AppError::Conflict("campaign already finished".into()));
            }
        }
        self.send_store
            .delete_pending_by_campaign(id)
            .await
            .map_err(Self::map_store_error)?;
        self.store
            .transition(
                id,
                CampaignStatus::Cancelled.as_str(),
                None,
                Some(chrono::Utc::now()),
                None,
            )
            .await
            .map_err(Self::map_store_error)?
            .map(Self::to_campaign)
            .ok_or_else(|| AppError::NotFound("campaign not found".into()))
    }

    async fn transition_lifecycle(
        &self,
        actor: UserId,
        project_id: &str,
        id: &str,
        from: CampaignStatus,
        to: CampaignStatus,
    ) -> Result<Campaign, AppError> {
        self.check_visible(actor, project_id).await?;
        let campaign = self.get(actor, project_id, id).await?;
        if campaign.status != from {
            return Err(AppError::Conflict(format!(
                "campaign is {} — expected {}",
                campaign.status.as_str(),
                from.as_str()
            )));
        }
        self.store
            .transition(id, to.as_str(), None, None, None)
            .await
            .map_err(Self::map_store_error)?
            .map(Self::to_campaign)
            .ok_or_else(|| AppError::NotFound("campaign not found".into()))
    }

    /// Collect — store failures propagate; the empty/capacity checks
    /// belong to the fire path only. `All` = every stored recipient with
    /// a contact for the channel; `Custom` = only the selected keys
    /// (deleted recipients drop out silently; contacts still resolve
    /// live so edits show through).
    async fn collect_audience(&self, campaign: &Campaign) -> Result<Vec<String>, AppError> {
        let stored = self
            .recipients
            .list_by_project(&campaign.project_id, AUDIENCE_MAX as i64)
            .await
            .map_err(Self::map_store_error)?;
        let selected: Option<HashSet<&str>> = (campaign.audience_mode == AudienceMode::Custom)
            .then(|| {
                campaign
                    .selected_user_ids
                    .iter()
                    .map(String::as_str)
                    .collect()
            });
        let mut keys: Vec<String> = Vec::new();
        let mut seen = HashSet::new();
        for record in &stored {
            if let Some(wanted) = &selected
                && !wanted.contains(record.user_id.as_str())
            {
                continue;
            }
            if let Some(value) = contact_value(&campaign.channel, &record.contacts)
                && seen.insert(value.to_string())
            {
                keys.push(value.to_string());
            }
        }
        Ok(keys)
    }

    /// Compose panel preview: audience size for a not-yet-expanded
    /// campaign. Never fails on an empty audience (that's a fire-time
    /// validation, not a read-time one).
    pub async fn estimate_audience(&self, campaign: &Campaign) -> Result<usize, AppError> {
        Ok(self.collect_audience(campaign).await?.len())
    }

    /// Expands the audience into pending send rows.
    async fn expand_audience(&self, campaign: &Campaign) -> Result<usize, AppError> {
        let keys = self.collect_audience(campaign).await?;
        if keys.is_empty() {
            return Err(AppError::Validation(
                "audience is empty — recipients need a contact for this channel".into(),
            ));
        }
        if keys.len() > AUDIENCE_MAX {
            return Err(AppError::Validation(format!(
                "audience exceeds {AUDIENCE_MAX} recipients"
            )));
        }
        let mut expanded = 0usize;
        for key in &keys {
            self.send_store
                .create_for_campaign(
                    &campaign.project_id,
                    &campaign.id,
                    &campaign.channel,
                    key,
                    &campaign.title,
                    &campaign.body,
                )
                .await
                .map_err(Self::map_store_error)?;
            expanded += 1;
        }
        Ok(expanded)
    }

    /// Processes one batch of a campaign's pending rows; failures live on
    /// the send rows (last-error), never abort the campaign. The
    /// campaign's explicit provider pick, when present, is tried first.
    async fn drain_batch(&self, campaign: &Campaign) {
        let pick = (!campaign.provider_id.is_empty()).then_some(campaign.provider_id.as_str());
        if let Err(e) = self
            .sends
            .process_campaign_batch(&campaign.id, pick, BATCH_LIMIT)
            .await
        {
            tracing::warn!(error = %e, campaign_id = campaign.id, "campaign batch failed");
        }
    }

    /// Completes a running campaign once every send row has settled.
    /// Returns the (possibly updated) campaign.
    async fn maybe_complete(&self, campaign: Campaign) -> Campaign {
        if campaign.status != CampaignStatus::Running {
            return campaign;
        }
        let stats = self.stats_for(&campaign.id).await.unwrap_or_default();
        if stats.pending > 0 {
            return campaign;
        }
        self.store
            .transition(
                &campaign.id,
                CampaignStatus::Completed.as_str(),
                None,
                Some(chrono::Utc::now()),
                None,
            )
            .await
            .ok()
            .flatten()
            .map(Self::to_campaign)
            .unwrap_or(campaign)
    }

    /// Worker tick: claims due scheduled campaigns (expanding + draining
    /// a first batch), then drains running campaigns and completes the
    /// finished ones. Returns how many campaigns were processed.
    pub async fn tick(&self, now: chrono::DateTime<chrono::Utc>) -> Result<i64, AppError> {
        let due = self
            .store
            .claim_due(now, CLAIM_LIMIT)
            .await
            .map_err(Self::map_store_error)?;
        let mut processed = 0i64;
        for record in due {
            let campaign = Self::to_campaign(record);
            if let Err(e) = self.expand_audience(&campaign).await {
                let _ = self
                    .store
                    .transition(
                        &campaign.id,
                        CampaignStatus::Failed.as_str(),
                        None,
                        Some(now),
                        Some(&Self::error_detail(&e)),
                    )
                    .await;
                processed += 1;
                continue;
            }
            self.drain_batch(&campaign).await;
            self.maybe_complete(campaign).await;
            processed += 1;
        }

        let running = self
            .store
            .list_running(CLAIM_LIMIT)
            .await
            .map_err(Self::map_store_error)?;
        for record in running {
            let campaign = Self::to_campaign(record);
            self.drain_batch(&campaign).await;
            self.maybe_complete(campaign).await;
            processed += 1;
        }
        Ok(processed)
    }
}
