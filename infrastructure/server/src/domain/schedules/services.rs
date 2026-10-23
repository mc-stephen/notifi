//! Flow schedule management: CRUD over stored schedules plus the
//! execution path the worker and run-now share.
//!
//! Ownership doctrine: a schedule **creates** its flow at birth (named
//! after the schedule, seeded with a locked schedule trigger node) and
//! deletes it with itself — flows are never assigned to schedules, so
//! they never reappear in `/flows` on delete. Owned flows stay off the
//! `/flows` list (see [`ScheduleService::scheduled_flow_ids`]); the
//! builder stays reachable by id. `archived` is the non-destructive
//! alternative: archived schedules stop claiming runs but keep their
//! flow bound and restorable.
//!
//! Renames sync: renaming a schedule renames its flow (the owner owns the
//! flow's identity).
//!
//! Content doctrine: the message lives in the flow — a `template` node
//! resolves and renders at every fire (tier one, picked fresh off the
//! templates store); rows born before the node existed fall back to
//! their stored template snapshot (tier two), then to the legacy
//! title/body fields (tier three). New schedules carry neither fallback:
//! content is optional at create, and bare rows fail closed at execute
//! (proven by `template_node_executes_rendered_content`,
//! `legacy_template_rows_still_render_snapshot`, and
//! `bare_schedules_fail_closed_with_a_directive` in `schedules_api.rs`).

use std::collections::HashSet;
use std::sync::Arc;

use crate::domain::auth::entities::UserId;
use crate::domain::error::AppError;
use crate::domain::flows::{provider_chain, template_node_id};
use crate::domain::schedules::entities::{
    MAX_INTERVAL_SECS, MAX_RECIPIENTS, MIN_INTERVAL_SECS, Schedule, ScheduleInput, ScheduleKind,
    advance_next_run, describe_timing, initial_next_run, next_weekly_run, normalize_days,
    parse_run_time, parse_timezone, render_template_content, seed_definition,
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
/// field the new kind doesn't use. The flow is not patchable — the
/// schedule owns it for life.
#[derive(Debug, Clone, Default)]
pub struct SchedulePatch {
    pub name: Option<String>,
    pub kind: Option<ScheduleKind>,
    pub run_at: Option<Option<chrono::DateTime<chrono::Utc>>>,
    pub interval_secs: Option<Option<i64>>,
    pub days: Option<Vec<i64>>,
    pub run_time: Option<Option<String>>,
    pub timezone: Option<String>,
    pub channel: Option<String>,
    pub title: Option<String>,
    pub body: Option<String>,
    pub recipients: Option<Vec<String>>,
    pub active: Option<bool>,
    /// `true` archives (stamps `archived_at`), `false` restores.
    pub archived: Option<bool>,
}

pub struct ScheduleService {
    store: Arc<dyn ScheduleStore>,
    flows: Arc<dyn FlowStore>,
    sends: Arc<dyn SendStore>,
    templates: Arc<dyn TemplatesStore>,
    providers: Arc<dyn crate::ports::ChannelProviderStore>,
    adapters: Arc<dyn crate::ports::AdapterFactory>,
    http: Arc<dyn crate::ports::HttpRequester>,
}

impl ScheduleService {
    pub fn new(
        store: Arc<dyn ScheduleStore>,
        flows: Arc<dyn FlowStore>,
        sends: Arc<dyn SendStore>,
        templates: Arc<dyn TemplatesStore>,
        providers: Arc<dyn crate::ports::ChannelProviderStore>,
        adapters: Arc<dyn crate::ports::AdapterFactory>,
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

    fn map_store_error(e: StoreError) -> AppError {
        match e {
            StoreError::Conflict => AppError::Conflict("conflicting schedule change".into()),
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
            archived_at: record.archived_at,
            next_run_at: record.next_run_at,
            last_run_at: record.last_run_at,
            last_status: record.last_status,
            last_error: record.last_error,
            run_count: record.run_count,
            created_at: record.created_at,
            updated_at: record.updated_at,
        }
    }

    fn check_name(name: &str) -> Result<String, AppError> {
        let name = name.trim();
        if name.is_empty() {
            return Err(AppError::Validation("schedule name is required".into()));
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
    ) -> Result<(), AppError> {
        match kind {
            ScheduleKind::Once => {
                if run_at.is_none() {
                    return Err(AppError::Validation(
                        "run_at is required for one-shot schedules".into(),
                    ));
                }
            }
            ScheduleKind::Recurring => match interval_secs {
                Some(s) if (MIN_INTERVAL_SECS..=MAX_INTERVAL_SECS).contains(&s) => {}
                _ => {
                    return Err(AppError::Validation(format!(
                        "interval_secs must be between {MIN_INTERVAL_SECS} and {MAX_INTERVAL_SECS}"
                    )));
                }
            },
            ScheduleKind::Weekly => {
                if normalize_days(days).is_none() {
                    return Err(AppError::Validation(
                        "weekly schedules need at least one day (0 = Sunday .. 6 = Saturday)"
                            .into(),
                    ));
                }
                if run_time.and_then(parse_run_time).is_none() {
                    return Err(AppError::Validation(
                        "weekly schedules need a time as HH:mm".into(),
                    ));
                }
                if parse_timezone(timezone).is_none() {
                    return Err(AppError::Validation(
                        "unknown timezone — use an IANA name like America/New_York".into(),
                    ));
                }
            }
        }
        Ok(())
    }

    /// Recipients are capped but may be empty — targeting arrives with
    /// flow targeting; empty rows fail closed at execute.
    fn check_recipients(recipients: &[String]) -> Result<(), AppError> {
        if recipients.len() > MAX_RECIPIENTS {
            return Err(AppError::Validation(format!(
                "at most {MAX_RECIPIENTS} recipients per schedule"
            )));
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
    pub async fn list(&self, actor: UserId, project_id: &str) -> Result<Vec<Schedule>, AppError> {
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
    ) -> Result<HashSet<String>, AppError> {
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
    ) -> Result<Schedule, AppError> {
        self.check_visible(actor, project_id).await?;
        self.store
            .get(id)
            .await
            .map_err(Self::map_store_error)?
            .filter(|r| r.project_id == project_id)
            .map(Self::to_schedule)
            .ok_or_else(|| AppError::NotFound("schedule not found".into()))
    }

    /// Insert a schedule — creating its flow first. The flow is born
    /// named after the schedule with a locked schedule trigger node
    /// (see [`seed_definition`]), so clients never pick or assign one.
    /// First fire is `run_at` (once), now + interval (recurring), or the
    /// next selected weekday (weekly). Content is optional at create —
    /// the flow's template node or the legacy payload fields own the
    /// message, and bare rows fail closed at execute (see [`ScheduleService::execute`]).
    pub async fn create(
        &self,
        actor: UserId,
        project_id: &str,
        input: ScheduleInput,
    ) -> Result<Schedule, AppError> {
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
        Self::check_recipients(&input.recipients)?;
        // The flow is owned data: created here, deleted with the schedule.
        let flow = self
            .flows
            .create(project_id, &name, &seed_definition())
            .await
            .map_err(Self::map_store_error)?;
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
            name: name.clone(),
            flow_id: flow.id.clone(),
            kind: input.kind.as_str().to_string(),
            run_at: input.run_at,
            interval_secs: input.interval_secs,
            days,
            run_time: input.run_time,
            timezone: timezone.to_string(),
            template_id: None,
            channel: input.channel.trim().to_string(),
            title: input.title.trim().to_string(),
            body: input.body,
            recipients: Self::recipients_json(&input.recipients),
            active: true,
            archived_at: None,
            next_run_at,
        };
        match self.store.create(&draft).await {
            Ok(record) => Ok(Self::to_schedule(record)),
            Err(e) => {
                // Leave no owner-less flow behind (it would never surface
                // in the flows list — the owner row is the only door in).
                let _ = self.flows.delete(&flow.id).await;
                Err(Self::map_store_error(e))
            }
        }
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
    ) -> Result<Schedule, AppError> {
        self.check_visible(actor, project_id).await?;
        let current = self
            .store
            .get(id)
            .await
            .map_err(Self::map_store_error)?
            .filter(|r| r.project_id == project_id);
        let current = current.ok_or_else(|| AppError::NotFound("schedule not found".into()))?;

        let name = match patch.name {
            Some(n) => Self::check_name(&n)?,
            None => current.name.clone(),
        };
        // The flow is not reassignable — the schedule owns it for life.
        let flow_id = current.flow_id.clone();
        // Rename sync: the owner owns the flow's identity too.
        if name != current.name
            && let Some(flow) = self
                .flows
                .get(&flow_id)
                .await
                .map_err(Self::map_store_error)?
        {
            let _ = self.flows.update(&flow_id, &name, &flow.definition).await;
        }
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
        Self::check_recipients(&recipients)?;
        let active = patch.active.unwrap_or(current.active);
        let now = chrono::Utc::now();
        // Archive is a stamp, not a state machine: true stamps now (once),
        // false clears, absent keeps.
        let archived_at = match patch.archived {
            Some(true) => current.archived_at.or(Some(now)),
            Some(false) => None,
            None => current.archived_at,
        };

        // Reactivating or restoring a schedule with a stale next run
        // re-anchors instead of bursting missed beats.
        let stale = current.next_run_at.is_none_or(|next| next <= now);
        let next_run_at = if active
            && archived_at.is_none()
            && (!current.active || stale || current.archived_at.is_some())
        {
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
            template_id: current.template_id.clone(),
            channel: channel.trim().to_string(),
            title: title.trim().to_string(),
            body,
            recipients: Self::recipients_json(&recipients),
            active,
            archived_at,
            next_run_at,
        };
        self.store
            .update(id, &update)
            .await
            .map_err(Self::map_store_error)?
            .map(Self::to_schedule)
            .ok_or_else(|| AppError::NotFound("schedule not found".into()))
    }

    /// Delete a schedule — and its flow with it. Owned flows never show
    /// in `/flows`, so releasing them would only resurrect an orphan the
    /// user can't find; the archive path exists for that instead.
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
        let deleted = self.store.delete(id).await.map_err(Self::map_store_error)?;
        if deleted {
            // The flow is owned data; it dies with its owner. The row's
            // FK cascade would do this too — an explicit delete keeps the
            // fakes and reads honest.
            let _ = self.flows.delete(&existing.flow_id).await;
        }
        Ok(deleted)
    }

    /// Manual fire: runs the assigned flow now with the stored payload.
    /// Uses the flow's current definition (edits since scheduling apply)
    /// and fails cleanly if the flow was deleted. Timing is untouched (a
    /// future one-shot still fires on schedule). Archived schedules must
    /// be restored first.
    pub async fn run_now(
        &self,
        actor: UserId,
        project_id: &str,
        id: &str,
    ) -> Result<Schedule, AppError> {
        let mut schedule = self.get(actor, project_id, id).await?;
        if schedule.archived_at.is_some() {
            return Err(AppError::Conflict(
                "schedule is archived — restore it to run it".into(),
            ));
        }
        let flow = self
            .flows
            .get(&schedule.flow_id)
            .await
            .map_err(Self::map_store_error)?
            .filter(|f| f.project_id == project_id);
        match flow {
            Some(f) => schedule.flow_definition = f.definition,
            None => {
                return Err(AppError::Conflict(
                    "the flow was deleted — recreate the schedule".to_string(),
                ));
            }
        }
        let (status, error, next_run_at, active) = self.execute(&schedule).await;
        self.store
            .mark_run(&schedule.id, &status, error.as_deref(), next_run_at, active)
            .await
            .map_err(Self::map_store_error)?
            .map(Self::to_schedule)
            .ok_or_else(|| AppError::NotFound("schedule not found".into()))
    }

    /// Worker step: claims due schedules and runs each assigned flow once.
    /// Returns the number of schedules processed.
    pub async fn run_due(&self, now: chrono::DateTime<chrono::Utc>) -> Result<i64, AppError> {
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

    /// Runs one schedule's assigned flow and records the send. Content
    /// resolves in three tiers — the flow's template node first (rendered
    /// fresh at fire time), then legacy rows' stored template snapshot,
    /// then the legacy title/body fields. Recipient targeting still lives
    /// outside schedules, so runs fail closed without recipients (see
    /// module docs). Returns (status, error, next_run_at, active).
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

        // Content resolves in three tiers (see module doc + execute docs):
        // the flow's template node wins, then the row snapshot, then the
        // legacy payload fields.
        let (channel, title, body) = match template_node_id(&schedule.flow_definition) {
            Some(node_template_id) => {
                let template = match self.templates.get_by_id(&node_template_id).await {
                    Ok(t) => t.filter(|t| t.project_id == schedule.project_id),
                    Err(e) => return settle("failed", Some(format!("template read failed: {e}"))),
                };
                let Some(template) = template else {
                    return settle(
                        "failed",
                        Some(
                            "the template node's template was deleted — pick another one"
                                .to_string(),
                        ),
                    );
                };
                match render_template_content(&template.content, &std::collections::HashMap::new())
                {
                    Some(rendered) => (rendered.channel, rendered.title, rendered.body),
                    None => {
                        return settle(
                            "failed",
                            Some("template covers no renderable channels".to_string()),
                        );
                    }
                }
            }
            None => match (&schedule.template_id, &schedule.template_content) {
                (Some(_), Some(content)) => {
                    // No run-level personalization yet; the values map lands
                    // per-recipient data the day schedules carry it.
                    match render_template_content(content, &std::collections::HashMap::new()) {
                        Some(rendered) => (rendered.channel, rendered.title, rendered.body),
                        None => {
                            return settle(
                                "failed",
                                Some("template covers no renderable channels".to_string()),
                            );
                        }
                    }
                }
                (Some(_), None) => {
                    return settle(
                        "failed",
                        Some("the assigned template was deleted — pick another one".to_string()),
                    );
                }
                (None, _) => {
                    if schedule.title.is_empty() && schedule.body.is_empty() {
                        return settle(
                            "failed",
                            Some(
                                "the schedule has no message yet — add a template node to its flow"
                                    .to_string(),
                            ),
                        );
                    }
                    (
                        schedule.channel.clone(),
                        schedule.title.clone(),
                        schedule.body.clone(),
                    )
                }
            },
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
    fn recipient_validation_bounds_the_count() {
        let many = vec!["r".to_string(); MAX_RECIPIENTS + 1];
        assert!(ScheduleService::check_recipients(&many).is_err());
        assert!(ScheduleService::check_recipients(&["r".to_string()]).is_ok());
        // Empty is legal — bare schedules fail closed at execute, not here.
        assert!(ScheduleService::check_recipients(&[]).is_ok());
    }
}
