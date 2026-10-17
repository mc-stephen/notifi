//! Request/response shapes for flow schedules (camelCase).

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::domain::schedules::entities::Schedule;
use crate::domain::schedules::services::ScheduleService;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ScheduleDto {
    pub id: String,
    pub project_id: String,
    pub name: String,
    pub flow_id: String,
    pub flow_name: String,
    pub kind: String,
    pub run_at: Option<DateTime<Utc>>,
    pub interval_secs: Option<i64>,
    pub days: Vec<i64>,
    pub run_time: Option<String>,
    pub timezone: String,
    pub template_id: Option<String>,
    pub template_name: Option<String>,
    pub channel: String,
    pub title: String,
    pub body: String,
    pub recipients: Vec<String>,
    pub active: bool,
    pub next_run_at: Option<DateTime<Utc>>,
    pub last_run_at: Option<DateTime<Utc>>,
    pub last_status: Option<String>,
    pub last_error: Option<String>,
    pub run_count: i32,
    pub timing: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl ScheduleDto {
    fn from_schedule(s: Schedule) -> Self {
        let timing = ScheduleService::timing_summary(&s);
        Self {
            id: s.id,
            project_id: s.project_id,
            name: s.name,
            flow_id: s.flow_id,
            flow_name: s.flow_name,
            kind: s.kind.as_str().to_string(),
            run_at: s.run_at,
            interval_secs: s.interval_secs,
            days: s.days,
            run_time: s.run_time,
            timezone: s.timezone,
            template_id: s.template_id,
            template_name: s.template_name,
            channel: s.channel,
            title: s.title,
            body: s.body,
            recipients: s.recipients,
            active: s.active,
            next_run_at: s.next_run_at,
            last_run_at: s.last_run_at,
            last_status: s.last_status,
            last_error: s.last_error,
            run_count: s.run_count,
            timing,
            created_at: s.created_at,
            updated_at: s.updated_at,
        }
    }
}

impl From<Schedule> for ScheduleDto {
    fn from(s: Schedule) -> Self {
        Self::from_schedule(s)
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateScheduleRequest {
    pub project_id: String,
    pub name: String,
    pub flow_id: String,
    pub kind: String,
    pub run_at: Option<DateTime<Utc>>,
    pub interval_secs: Option<i64>,
    pub days: Option<Vec<i64>>,
    pub run_time: Option<String>,
    pub timezone: Option<String>,
    pub template_id: Option<String>,
    pub channel: Option<String>,
    pub title: Option<String>,
    pub body: Option<String>,
    pub recipients: Vec<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateScheduleRequest {
    pub project_id: String,
    pub name: Option<String>,
    pub flow_id: Option<String>,
    pub kind: Option<String>,
    pub run_at: Option<Option<DateTime<Utc>>>,
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

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RunNowRequest {
    pub project_id: String,
}
