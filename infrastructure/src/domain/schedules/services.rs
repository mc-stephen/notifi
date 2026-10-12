//! Flow schedule management: CRUD over stored schedules plus the
//! execution path the worker and run-now share.
//!
//! Assigning a flow to a schedule hides it from the `/flows` list (see
//! [`ScheduleService::scheduled_flow_ids`]); the builder stays reachable
//! by id, and the ad-hoc send path is untouched.

use std::collections::HashSet;
use std::sync::Arc;

use crate::domain::auth::entities::UserId;
use crate::domain::auth::errors::AuthError;
use crate::domain::flows::provider_chain;
use crate::domain::schedules::entities::{
    MAX_INTERVAL_SECS, MAX_RECIPIENTS, MIN_INTERVAL_SECS, Schedule, ScheduleInput, ScheduleKind,
    advance_next_run, describe_timing, initial_next_run, next_weekly_run, normalize_days,
    parse_run_time, parse_timezone, render_template_content,
};
use crate::ports::auth_store::StoreError;
use crate::ports::flow_store::FlowStore;
use crate::ports::schedule_store::{NewSchedule, ScheduleRecord, ScheduleStore, ScheduleUpdate};
use crate::ports::send_store::SendStore;
use crate::ports::templates_store::TemplatesStore;

/// Worker batch bound per sweep.
const CLAIM_LIMIT: i64 = 25;
/// Partial update for a schedule. `run_at`/`interval_secs` are three-state
/// (absent = keep, `Some(None)` = clear) so kind switches can drop the
/// field the new kind doesn't use.
#[derive(Debug, Clone, Default)]
pub struct SchedulePatch {
    pub name: Option<String>,
    pub flow_id: Option<String>,
    pub kind: Option<ScheduleKind>,
    pub run_at: Option<Option<chrono::DateTime<chrono::Utc>>>,
    pub interval_secs: Option<Option<i64>>,
    pub days: Option<Vec<i64>>,
    pub run_time: Option<Option<String>>,
    pub timezone: Option<String>,
    pub template_id: Option<Option<String>>,
    pub channel: Option<String>,
    pub title: Option<String>,
    pub body: Option<String>,
    pub recipients: Option<Vec<String>>,
    pub active: Option<bool>,
}

pub struct ScheduleService {
    store: Arc<dyn ScheduleStore>,
    flows: Arc<dyn FlowStore>,
    sends: Arc<dyn SendStore>,
    templates: Arc<dyn TemplatesStore>,
    providers: Arc<dyn crate::ports::ChannelProviderStore>,
    adapters: Arc<dyn notifi_domain_ports::AdapterFactory>,
    http: Arc<dyn crate::ports::HttpRequester>,
}

impl ScheduleService {
    pub fn new(
        store: Arc<dyn ScheduleStore>,
        flows: Arc<dyn FlowStore>,
        sends: Arc<dyn SendStore>,
        templates: Arc<dyn TemplatesStore>,
        providers: Arc<dyn crate::ports::ChannelProviderStore>,
        adapters: Arc<dyn notifi_domain_ports::AdapterFactory>,
        http: Arc<dyn crate::ports::HttpRequester>,
    ) -> Self {
        Self {
            store,
            flows,
            sends,
            templates,
            providers,
            adapters,
            http,
        }
    }

    fn map_store_error(e: StoreError) -> AuthError {
        match e {
            StoreError::Conflict => AuthError::Conflict("conflicting schedule change".into()),
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

    fn to_schedule(record: ScheduleRecord) -> Schedule {
        Schedule {
            id: record.id,
            project_id: record.project_id,
            name: record.name,
            flow_id: record.flow_id,
            flow_name: record.flow_name,
            flow_definition: record.flow_definition,
            kind: ScheduleKind::parse(&record.kind).unwrap_or(ScheduleKind::Once),
            run_at: record.run_at,
            interval_secs: record.interval_secs,
            days: record.days,
            run_time: record.run_time,
            timezone: record.timezone,
            template_id: record.template_id,
            template_name: record.template_name,
            template_content: record.template_content,
            channel: record.channel,
            title: record.title,
            body: record.body,
            recipients: record
                .recipients
                .as_array()
                .map(|a| {
                    a.iter()
                        .filter_map(|v| v.as_str().map(str::to_string))
                        .collect()
                })
                .unwrap_or_default(),
            active: record.active,
            next_run_at: record.next_run_at,
            last_run_at: record.last_run_at,
            last_status: record.last_status,
            last_error: record.last_error,
            run_count: record.run_count,
            created_at: record.created_at,
            updated_at: record.updated_at,
        }
    }

    fn check_name(name: &str) -> Result<String, AuthError> {
        let name = name.trim();
        if name.is_empty() {
            return Err(AuthError::Validation("schedule name is required".into()));
        }
        Ok(name.to_string())
    }

    fn check_timing(
        kind: &ScheduleKind,
        run_at: Option<chrono::DateTime<chrono::Utc>>,
        interval_secs: Option<i64>,
        days: &[i64],
        run_time: Option<&str>,
        timezone: &str,
    ) -> Result<(), AuthError> {
        match kind {
            ScheduleKind::Once => {
                if run_at.is_none() {
                    return Err(AuthError::Validation(
                        "run_at is required for one-shot schedules".into(),
                    ));
                }
            }
            ScheduleKind::Recurring => match interval_secs {
                Some(s) if (MIN_INTERVAL_SECS..=MAX_INTERVAL_SECS).contains(&s) => {}
                _ => {
                    return Err(AuthError::Validation(format!(
                        "interval_secs must be between {MIN_INTERVAL_SECS} and {MAX_INTERVAL_SECS}"
                    )));
                }
            },
            ScheduleKind::Weekly => {
                if normalize_days(days).is_none() {
                    return Err(AuthError::Validation(
                        "weekly schedules need at least one day (0 = Sunday .. 6 = Saturday)"
                            .into(),
                    ));
                }
                if run_time.and_then(parse_run_time).is_none() {
                    return Err(AuthError::Validation(
                        "weekly schedules need a time as HH:mm".into(),
                    ));
                }
                if parse_timezone(timezone).is_none() {
                    return Err(AuthError::Validation(
                        "unknown timezone — use an IANA name like America/New_York".into(),
                    ));
                }
            }
        }
        Ok(())
    }

    /// Legacy payload rules (template schedules skip these — the template
    /// owns channel/title, and recipients arrive with flow targeting).
    fn check_payload(channel: &str, title: &str, recipients: &[String]) -> Result<(), AuthError> {
        if channel.trim().is_empty() {
            return Err(AuthError::Validation("channel is required".into()));
        }
        if title.trim().is_empty() {
            return Err(AuthError::Validation("title is required".into()));
        }
        if recipients.is_empty() {
            return Err(AuthError::Validation(
                "at least one recipient is required".into(),
            ));
        }
        if recipients.len() > MAX_RECIPIENTS {
            return Err(AuthError::Validation(format!(
                "at most {MAX_RECIPIENTS} recipients per schedule"
            )));
        }
        Ok(())
    }

    /// The assigned flow must exist in the same project.
    async fn check_flow(&self, project_id: &str, flow_id: &str) -> Result<(), AuthError> {
        let flow = self
            .flows
            .get(flow_id)
            .await
            .map_err(Self::map_store_error)?
            .filter(|f| f.project_id == project_id);
        if flow.is_none() {
            return Err(AuthError::Validation(
                "flow not found in this project".into(),
            ));
        }
        Ok(())
    }

    /// The assigned template must exist in the same project.
    async fn check_template(
        &self,
        actor: UserId,
        project_id: &str,
        template_id: &str,
    ) -> Result<(), AuthError> {
        let template = self
            .templates
            .get(actor, project_id, template_id)
            .await
            .map_err(Self::map_store_error)?;
        if template.is_none() {
            return Err(AuthError::Validation(
                "template not found in this project".into(),
            ));
        }
        Ok(())
    }

    fn recipients_json(recipients: &[String]) -> serde_json::Value {
        serde_json::Value::Array(
            recipients
                .iter()
                .map(|r| serde_json::Value::String(r.clone()))
                .collect(),
        )
    }

    /// All schedules for a project, oldest first.
    pub async fn list(&self, actor: UserId, project_id: &str) -> Result<Vec<Schedule>, AuthError> {
        self.check_visible(actor, project_id).await?;
        Ok(self
            .store
            .list_by_project(project_id)
            .await
            .map_err(Self::map_store_error)?
            .into_iter()
            .map(Self::to_schedule)
            .collect())
    }

    /// Flow ids hidden from the `/flows` list by an assignment.
    pub async fn scheduled_flow_ids(
        &self,
        actor: UserId,
        project_id: &str,
    ) -> Result<HashSet<String>, AuthError> {
        self.check_visible(actor, project_id).await?;
        Ok(self
            .store
            .scheduled_flow_ids(project_id)
            .await
            .map_err(Self::map_store_error)?
            .into_iter()
            .collect())
    }

    /// One schedule by id. Project-scoped: foreign rows read as missing.
    pub async fn get(
        &self,
        actor: UserId,
        project_id: &str,
        id: &str,
    ) -> Result<Schedule, AuthError> {
        self.check_visible(actor, project_id).await?;
        self.store
            .get(id)
            .await
            .map_err(Self::map_store_error)?
            .filter(|r| r.project_id == project_id)
            .map(Self::to_schedule)
            .ok_or_else(|| AuthError::NotFound("schedule not found".into()))
    }

    /// Insert a schedule. First fire is `run_at` (once), now + interval
    /// (recurring), or the next selected weekday (weekly). Template
    /// schedules skip payload validation — the template owns the message
    /// and recipients arrive with flow targeting.
    pub async fn create(
        &self,
        actor: UserId,
        project_id: &str,
        input: ScheduleInput,
    ) -> Result<Schedule, AuthError> {
        self.check_visible(actor, project_id).await?;
        let name = Self::check_name(&input.name)?;
        Self::check_timing(
            &input.kind,
            input.run_at,
            input.interval_secs,
            &input.days,
            input.run_time.as_deref(),
            &input.timezone,
        )?;
        if let Some(tid) = input.template_id.as_deref() {
            self.check_template(actor, project_id, tid).await?;
        } else {
            Self::check_payload(&input.channel, &input.title, &input.recipients)?;
        }
        self.check_flow(project_id, &input.flow_id).await?;
        let now = chrono::Utc::now();
        let timezone = input.timezone.trim();
        let timezone = if timezone.is_empty() { "UTC" } else { timezone };
        let days = normalize_days(&input.days).unwrap_or_default();
        let next_run_at = initial_next_run(
            &input.kind,
            input.run_at,
            input.interval_secs,
            &input.days,
            input.run_time.as_deref(),
            timezone,
            now,
        );
        let draft = NewSchedule {
            project_id: project_id.to_string(),
            name,
            flow_id: input.flow_id,
            kind: input.kind.as_str().to_string(),
            run_at: input.run_at,
            interval_secs: input.interval_secs,
            days,
            run_time: input.run_time,
            timezone: timezone.to_string(),
            template_id: input.template_id,
            channel: input.channel.trim().to_string(),
            title: input.title.trim().to_string(),
            body: input.body,
            recipients: Self::recipients_json(&input.recipients),
            active: true,
            next_run_at,
        };
        self.store
            .create(&draft)
            .await
            .map_err(Self::map_store_error)
            .map(Self::to_schedule)
    }

    /// Patch a schedule. Reactivating a stale recurring schedule re-anchors
    /// its next run to now + interval instead of firing a burst of missed
    /// beats.
    pub async fn update(
        &self,
        actor: UserId,
        project_id: &str,
        id: &str,
        patch: SchedulePatch,
    ) -> Result<Schedule, AuthError> {
        self.check_visible(actor, project_id).await?;
        let current = self
            .store
            .get(id)
            .await
            .map_err(Self::map_store_error)?
            .filter(|r| r.project_id == project_id);
        let current = current.ok_or_else(|| AuthError::NotFound("schedule not found".into()))?;

        let name = match patch.name {
            Some(n) => Self::check_name(&n)?,
            None => current.name.clone(),
        };
        let flow_id = patch.flow_id.unwrap_or(current.flow_id.clone());
        self.check_flow(project_id, &flow_id).await?;
        let kind = patch
            .kind
            .unwrap_or_else(|| ScheduleKind::parse(&current.kind).unwrap_or(ScheduleKind::Once));
        let run_at = patch.run_at.unwrap_or(current.run_at);
        let interval_secs = patch.interval_secs.unwrap_or(current.interval_secs);
        let days = patch.days.unwrap_or(current.days.clone());
        let run_time = patch.run_time.unwrap_or(current.run_time.clone());
        let timezone = patch.timezone.unwrap_or(current.timezone.clone());
        Self::check_timing(
            &kind,
            run_at,
            interval_secs,
            &days,
            run_time.as_deref(),
            &timezone,
        )?;
        let template_id = patch.template_id.unwrap_or(current.template_id.clone());
        if let Some(tid) = template_id.as_deref() {
            self.check_template(actor, project_id, tid).await?;
        }
        let channel = patch.channel.unwrap_or(current.channel.clone());
        let title = patch.title.unwrap_or(current.title.clone());
        let body = patch.body.unwrap_or(current.body.clone());
        let recipients = match patch.recipients {
            Some(r) => r,
            None => current
                .recipients
                .as_array()
                .map(|a| {
                    a.iter()
                        .filter_map(|v| v.as_str().map(str::to_string))
                        .collect()
                })
                .unwrap_or_default(),
        };
        if template_id.is_none() {
            Self::check_payload(&channel, &title, &recipients)?;
        }
        let active = patch.active.unwrap_or(current.active);

        // Reactivation with a stale next run re-anchors instead of bursting.
        let now = chrono::Utc::now();
        let stale = current.next_run_at.is_none_or(|next| next <= now);
        let next_run_at = if active && (!current.active || stale) {
            match kind {
                ScheduleKind::Once => run_at,
                ScheduleKind::Recurring => {
                    interval_secs.map(|s| now + chrono::Duration::seconds(s))
                }
                ScheduleKind::Weekly => next_weekly_run(&days, run_time.as_deref(), &timezone, now),
            }
        } else {
            current.next_run_at
        };

        let update = ScheduleUpdate {
            name,
            flow_id,
            kind: kind.as_str().to_string(),
            run_at,
            interval_secs,
            days: normalize_days(&days).unwrap_or_default(),
            run_time,
            timezone,
            template_id,
            channel: channel.trim().to_string(),
            title: title.trim().to_string(),
            body,
            recipients: Self::recipients_json(&recipients),
            active,
            next_run_at,
        };
        self.store
            .update(id, &update)
            .await
            .map_err(Self::map_store_error)?
            .map(Self::to_schedule)
            .ok_or_else(|| AuthError::NotFound("schedule not found".into()))
    }

    /// Delete a schedule. The assigned flow automatically reappears in the
    /// `/flows` list (hiding is existence-based, no flags to clear).
    pub async fn delete(
        &self,
        actor: UserId,
        project_id: &str,
        id: &str,
    ) -> Result<bool, AuthError> {
        self.check_visible(actor, project_id).await?;
        let existing = self
            .store
            .get(id)
            .await
            .map_err(Self::map_store_error)?
            .filter(|r| r.project_id == project_id);
        if existing.is_none() {
            return Ok(false);
        }
        self.store.delete(id).await.map_err(Self::map_store_error)
    }

    /// Manual fire: runs the assigned flow now with the stored payload.
    /// Uses the flow's current definition (edits since scheduling apply)
    /// and fails cleanly if the flow was deleted. Timing is untouched (a
    /// future one-shot still fires on schedule).
    pub async fn run_now(
        &self,
        actor: UserId,
        project_id: &str,
        id: &str,
    ) -> Result<Schedule, AuthError> {
        let mut schedule = self.get(actor, project_id, id).await?;
        let flow = self
            .flows
            .get(&schedule.flow_id)
            .await
            .map_err(Self::map_store_error)?
            .filter(|f| f.project_id == project_id);
        match flow {
            Some(f) => schedule.flow_definition = f.definition,
            None => {
                return Err(AuthError::Conflict(
                    "the assigned flow was deleted — delete this schedule or pick another flow"
                        .into(),
                ));
            }
        }
        let (status, error, next_run_at, active) = self.execute(&schedule).await;
        self.store
            .mark_run(&schedule.id, &status, error.as_deref(), next_run_at, active)
            .await
            .map_err(Self::map_store_error)?
            .map(Self::to_schedule)
            .ok_or_else(|| AuthError::NotFound("schedule not found".into()))
    }

    /// Worker step: claims due schedules and runs each assigned flow once.
    /// Returns the number of schedules processed.
    pub async fn run_due(&self, now: chrono::DateTime<chrono::Utc>) -> Result<i64, AuthError> {
        let due = self
            .store
            .claim_due(now, CLAIM_LIMIT)
            .await
            .map_err(Self::map_store_error)?;
        let mut processed = 0i64;
        for record in due {
            let schedule = Self::to_schedule(record);
            let (status, error, next_run_at, active) = self.execute(&schedule).await;
            self.store
                .mark_run(&schedule.id, &status, error.as_deref(), next_run_at, active)
                .await
                .map_err(Self::map_store_error)?;
            processed += 1;
        }
        Ok(processed)
    }

    /// Runs one schedule's assigned flow and records the send. Template
    /// schedules render their template at fire time; legacy rows use the
    /// stored payload. Recipient targeting still lives outside schedules,
    /// so template runs fail closed without recipients (see module docs).
    /// Returns (status, error, next_run_at, active).
    async fn execute(
        &self,
        schedule: &Schedule,
    ) -> (
        String,
        Option<String>,
        Option<chrono::DateTime<chrono::Utc>>,
        bool,
    ) {
        let now = chrono::Utc::now();
        // One-shot schedules stop after firing; the rest keep going.
        let settle = |status: &str, error: Option<String>| {
            let next = advance_next_run(
                &schedule.kind,
                schedule.interval_secs,
                &schedule.days,
                schedule.run_time.as_deref(),
                &schedule.timezone,
                schedule.next_run_at,
                now,
            );
            let active = schedule.kind != ScheduleKind::Once && schedule.active;
            (status.to_string(), error, next, active)
        };

        let (channel, title, body) = match (&schedule.template_id, &schedule.template_content) {
            (Some(_), Some(content)) => match render_template_content(content) {
                Some(rendered) => (rendered.channel, rendered.title, rendered.body),
                None => {
                    return settle(
                        "failed",
                        Some("template covers no renderable channels".to_string()),
                    );
                }
            },
            (Some(_), None) => {
                return settle(
                    "failed",
                    Some("the assigned template was deleted — pick another one".to_string()),
                );
            }
            (None, _) => (
                schedule.channel.clone(),
                schedule.title.clone(),
                schedule.body.clone(),
            ),
        };
        if schedule.recipients.is_empty() {
            return settle("failed", Some("schedule has no recipients yet".to_string()));
        }

        let chain = provider_chain(&schedule.flow_definition, &channel);
        if chain.is_empty() {
            return settle(
                "failed",
                Some(format!("no provider route for channel '{channel}'")),
            );
        }
        let record = match self
            .sends
            .create(
                &schedule.project_id,
                &channel,
                &Self::recipients_json(&schedule.recipients),
                &title,
                &body,
                now,
            )
            .await
        {
            Ok(r) => r,
            Err(e) => {
                return settle("failed", Some(format!("send record failed: {e}")));
            }
        };

        let outcome = crate::domain::flows::deliver_through_chain(
            &self.providers,
            &self.adapters,
            &schedule.project_id,
            &chain,
            &channel,
            &schedule.recipients,
            &title,
            &body,
        )
        .await;
        // HTTP Request steps fire once per run whatever the outcome.
        // Failures log server-side only: never fatal, never in deliveries.
        for step_outcome in crate::domain::flows::fire_http_steps(
            &self.http,
            &crate::domain::flows::collect_http_steps(&schedule.flow_definition),
        )
        .await
        {
            if !step_outcome.ok {
                tracing::warn!(
                    schedule_id = %schedule.id,
                    node_id = %step_outcome.node_id,
                    status = ?step_outcome.status,
                    error = ?step_outcome.error,
                    "schedule HTTP step failed"
                );
            }
        }
        match outcome {
            Ok((provider, external_id)) => {
                let _ = self
                    .sends
                    .mark_completed(&record.id, &provider, external_id.as_deref())
                    .await;
                settle("completed", None)
            }
            Err(error) => {
                let _ = self.sends.mark_failed(&record.id, &error).await;
                settle("failed", Some(error))
            }
        }
    }

    /// Short cadence summary for list views ("Every 6 hours").
    pub fn timing_summary(schedule: &Schedule) -> String {
        let mut parts = vec![describe_timing(
            &schedule.kind,
            schedule.run_at,
            schedule.interval_secs,
            &schedule.days,
            schedule.run_time.as_deref(),
        )];
        if !schedule.timezone.is_empty() && schedule.timezone != "UTC" {
            parts.push(schedule.timezone.clone());
        }
        parts.join(" · ")
    }
}

#[cfg(test)]
mod schedule_service_tests {
    use super::*;

    #[test]
    fn timing_validation_rejects_bad_intervals() {
        assert!(
            ScheduleService::check_timing(
                &ScheduleKind::Recurring,
                None,
                Some(59),
                &[],
                None,
                "UTC"
            )
            .is_err()
        );
        assert!(
            ScheduleService::check_timing(
                &ScheduleKind::Recurring,
                None,
                Some(3600),
                &[],
                None,
                "UTC"
            )
            .is_ok()
        );
        assert!(
            ScheduleService::check_timing(&ScheduleKind::Once, None, None, &[], None, "UTC")
                .is_err()
        );
        assert!(
            ScheduleService::check_timing(
                &ScheduleKind::Weekly,
                None,
                None,
                &[1, 3],
                Some("09:00"),
                "America/New_York"
            )
            .is_ok()
        );
        assert!(
            ScheduleService::check_timing(
                &ScheduleKind::Weekly,
                None,
                None,
                &[],
                Some("09:00"),
                "UTC"
            )
            .is_err()
        );
    }

    #[test]
    fn payload_validation_bounds_recipients() {
        let many = vec!["r".to_string(); MAX_RECIPIENTS + 1];
        assert!(ScheduleService::check_payload("email", "t", &many).is_err());
        assert!(ScheduleService::check_payload("", "t", &["r".to_string()]).is_err());
        assert!(ScheduleService::check_payload("email", "t", &["r".to_string()]).is_ok());
    }
}
