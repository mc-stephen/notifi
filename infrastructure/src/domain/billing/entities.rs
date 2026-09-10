//! Billing entities: plan catalog + per-project subscriptions.
//!
//! Billing is per project, not per account. Plans are monthly by
//! default; a set `yearly_discount` means 12 months for less.
//! Framework-free by rule: no axum/sqlx types here.

use std::str::FromStr;

use serde::{Deserialize, Serialize};

/// The default plan every project starts on.
pub const FREE_PLAN_ID: &str = "free";

/// What a plan entails. All limits are `None` = unlimited.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct PlanCapability {
    pub notifications_per_month: Option<i64>,
    pub channels: Vec<String>,
    pub team_members: Option<i64>,
    pub retention_days: Option<i64>,
    pub support: String,
    pub branding: bool,
    pub api_calls: Option<i64>,
}

/// Reduction applied when a user pays for 12 months at once.
/// `Percent` is (0, 100) exclusive; `Fixed` is cents off the total.
/// Wire shape mirrors the admin console: `{"kind","value"}`.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", content = "value", rename_all = "snake_case")]
pub enum YearlyDiscount {
    Percent(f64),
    Fixed(i64),
}

/// A pricing plan from the catalog.
#[derive(Debug, Clone)]
pub struct Plan {
    pub id: String,
    pub name: String,
    /// Monthly price in cents. `None` = custom (contact sales).
    pub price_cents: Option<i64>,
    pub currency: String,
    pub interval: String,
    pub capabilities: PlanCapability,
    /// `None` = monthly only.
    pub yearly_discount: Option<YearlyDiscount>,
    pub is_active: bool,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

impl From<crate::ports::billing_store::PlanRecord> for Plan {
    fn from(record: crate::ports::billing_store::PlanRecord) -> Self {
        Self {
            id: record.id,
            name: record.name,
            price_cents: record.price_cents,
            currency: record.currency,
            interval: record.interval,
            capabilities: record
                .capabilities
                .as_ref()
                .and_then(|v| serde_json::from_value(v.clone()).ok())
                .unwrap_or_default(),
            yearly_discount: record
                .yearly_discount
                .as_ref()
                .and_then(|v| serde_json::from_value(v.clone()).ok()),
            is_active: record.is_active,
            created_at: record.created_at,
            updated_at: record.updated_at,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SubscriptionStatus {
    Active,
    PastDue,
    Cancelled,
}

impl SubscriptionStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Active => "active",
            Self::PastDue => "past_due",
            Self::Cancelled => "cancelled",
        }
    }
}

impl FromStr for SubscriptionStatus {
    type Err = ();
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "active" => Ok(Self::Active),
            "past_due" => Ok(Self::PastDue),
            "cancelled" => Ok(Self::Cancelled),
            _ => Err(()),
        }
    }
}

impl std::fmt::Display for SubscriptionStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BillingCycle {
    Monthly,
    Yearly,
}

impl BillingCycle {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Monthly => "monthly",
            Self::Yearly => "yearly",
        }
    }

    /// Period length for a fresh subscription or renewal.
    pub fn period_days(self) -> i64 {
        match self {
            Self::Monthly => 30,
            Self::Yearly => 365,
        }
    }
}

impl FromStr for BillingCycle {
    type Err = ();
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "monthly" => Ok(Self::Monthly),
            "yearly" => Ok(Self::Yearly),
            _ => Err(()),
        }
    }
}

impl std::fmt::Display for BillingCycle {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

/// One project's subscription, with the plan resolved.
#[derive(Debug, Clone)]
pub struct Subscription {
    pub id: String,
    pub project_id: String,
    pub plan: Plan,
    pub status: SubscriptionStatus,
    pub billing_cycle: BillingCycle,
    /// Paid plan ends at `period_end`, then reverts to free.
    pub cancel_at_period_end: bool,
    pub period_start: chrono::DateTime<chrono::Utc>,
    pub period_end: chrono::DateTime<chrono::Utc>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

/// A subscription plus the billed project and its owner, for admin views.
#[derive(Debug, Clone)]
pub struct PlanSubscriber {
    pub subscription_id: String,
    pub project_id: String,
    pub project_name: String,
    pub customer_name: Option<String>,
    pub customer_email: Option<String>,
    pub status: SubscriptionStatus,
    pub billing_cycle: BillingCycle,
    pub period_end: chrono::DateTime<chrono::Utc>,
}

impl From<crate::ports::billing_store::SubscriberRecord> for PlanSubscriber {
    fn from(record: crate::ports::billing_store::SubscriberRecord) -> Self {
        Self {
            subscription_id: record.subscription.id,
            project_id: record.subscription.project_id,
            project_name: record.project_name,
            customer_name: record.customer_name,
            customer_email: record.customer_email,
            status: record.subscription.status,
            billing_cycle: record.subscription.billing_cycle,
            period_end: record.subscription.period_end,
        }
    }
}

/// 12-month total in cents after the discount. `None` for custom plans.
pub fn yearly_price_cents(plan: &Plan) -> Option<i64> {
    let monthly = plan.price_cents?;
    let base = monthly.saturating_mul(12);
    match plan.yearly_discount {
        None => Some(base),
        Some(YearlyDiscount::Percent(p)) => Some((base as f64 * (1.0 - p / 100.0)).round() as i64),
        Some(YearlyDiscount::Fixed(cents)) => Some(base.saturating_sub(cents).max(0)),
    }
}

/// Price in cents for one billing period. `None` for custom plans.
pub fn cycle_price_cents(plan: &Plan, cycle: BillingCycle) -> Option<i64> {
    match cycle {
        BillingCycle::Monthly => plan.price_cents,
        BillingCycle::Yearly => yearly_price_cents(plan),
    }
}

/// One month's active-subscriber snapshot for the admin history chart.
/// `date` is the month's first day (ISO `YYYY-MM-DD`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SubscriberHistoryPoint {
    pub date: String,
    pub total: i64,
    pub by_plan: std::collections::BTreeMap<String, i64>,
}

/// A project's birth for history replay.
#[derive(Debug, Clone)]
pub struct HistoryProject {
    pub id: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

/// A plan-affecting moment for history replay.
#[derive(Debug, Clone)]
pub struct HistoryEvent {
    pub occurred_at: chrono::DateTime<chrono::Utc>,
    pub project_id: String,
    pub kind: HistoryEventKind,
    /// Target plan for `Changed`; holding plan for `Cancelled`.
    pub plan_id: String,
    pub billing_cycle: BillingCycle,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HistoryEventKind {
    Changed,
    Cancelled,
}

/// Rebuilds trailing monthly active-subscriber snapshots by replaying
/// plan events. Pure (no I/O) so it unit-tests directly.
///
/// Rules mirror the live model: projects start free at creation;
/// `Changed` switches plan at event time; `Cancelled` holds the plan
/// through its billing period, then falls back to free.
pub fn build_history(
    projects: &[HistoryProject],
    events: &[HistoryEvent],
    months: usize,
    now: chrono::DateTime<chrono::Utc>,
) -> Vec<SubscriberHistoryPoint> {
    use chrono::Datelike;
    let months = months.clamp(1, 24);

    // Oldest bucket first: first day of each trailing calendar month.
    let mut buckets: Vec<(i32, u32)> = Vec::with_capacity(months);
    let (mut year, mut month) = (now.year(), now.month());
    for _ in 0..months {
        buckets.push((year, month));
        if month == 1 {
            year -= 1;
            month = 12;
        } else {
            month -= 1;
        }
    }
    buckets.reverse();

    let mut by_project: std::collections::HashMap<
        &str,
        Vec<(chrono::DateTime<chrono::Utc>, String)>,
    > = std::collections::HashMap::new();
    for project in projects {
        by_project
            .entry(project.id.as_str())
            .or_default()
            .push((project.created_at, FREE_PLAN_ID.to_string()));
    }
    let mut ordered: Vec<&HistoryEvent> = events.iter().collect();
    ordered.sort_by(|a, b| a.occurred_at.cmp(&b.occurred_at));
    for event in ordered {
        let timeline = match by_project.get_mut(event.project_id.as_str()) {
            Some(timeline) => timeline,
            None => continue, // project gone; skip
        };
        match event.kind {
            HistoryEventKind::Changed => {
                timeline.push((event.occurred_at, event.plan_id.clone()));
            }
            HistoryEventKind::Cancelled => {
                let hold_until =
                    event.occurred_at + chrono::Duration::days(event.billing_cycle.period_days());
                timeline.push((hold_until, FREE_PLAN_ID.to_string()));
            }
        }
    }
    for timeline in by_project.values_mut() {
        timeline.sort_by(|a, b| a.0.cmp(&b.0));
    }

    buckets
        .iter()
        .map(|(year, month)| {
            // Snapshot at month end: transitions strictly before the next
            // month's start apply; projects born later don't count yet.
            let next_start = if *month == 12 {
                chrono::NaiveDate::from_ymd_opt(year + 1, 1, 1)
            } else {
                chrono::NaiveDate::from_ymd_opt(*year, month + 1, 1)
            }
            .and_then(|d| d.and_hms_opt(0, 0, 0))
            .map(|dt| chrono::DateTime::<chrono::Utc>::from_naive_utc_and_offset(dt, chrono::Utc));
            let mut by_plan: std::collections::BTreeMap<String, i64> =
                std::collections::BTreeMap::new();
            if let Some(next_start) = next_start {
                for (project_id, timeline) in &by_project {
                    let project = projects.iter().find(|p| &p.id == project_id);
                    let born = project.map(|p| p.created_at);
                    if born.is_none_or(|at| at >= next_start) {
                        continue;
                    }
                    let mut plan = FREE_PLAN_ID;
                    for (at, id) in timeline.iter() {
                        if *at < next_start {
                            plan = id;
                        } else {
                            break;
                        }
                    }
                    *by_plan.entry(plan.to_string()).or_insert(0) += 1;
                }
            }
            SubscriberHistoryPoint {
                date: format!("{year:04}-{month:02}-01"),
                total: by_plan.values().sum(),
                by_plan,
            }
        })
        .collect()
}

#[cfg(test)]
mod history_tests {
    use super::*;
    use chrono::TimeZone;

    fn utc(y: i32, m: u32, d: u32) -> chrono::DateTime<chrono::Utc> {
        chrono::Utc.with_ymd_and_hms(y, m, d, 12, 0, 0).unwrap()
    }

    fn project(id: &str, at: chrono::DateTime<chrono::Utc>) -> HistoryProject {
        HistoryProject {
            id: id.to_string(),
            created_at: at,
        }
    }

    #[test]
    fn pre_launch_projects_count_as_free_from_birth() {
        let projects = vec![
            project("a", utc(2024, 10, 5)),
            project("b", utc(2025, 1, 20)),
        ];
        let points = build_history(&projects, &[], 3, utc(2025, 3, 10));
        assert_eq!(points.len(), 3);
        assert_eq!(points[0].date, "2025-01-01");
        // Both born before February: a (Oct) + b (Jan 20).
        assert_eq!(points[0].total, 2);
        assert_eq!(points[0].by_plan.get("free"), Some(&2));
        assert_eq!(points[2].total, 2);
    }

    #[test]
    fn subscribe_and_cancel_hold_replay() {
        let projects = vec![project("a", utc(2025, 1, 5))];
        let events = vec![
            HistoryEvent {
                occurred_at: utc(2025, 2, 10),
                project_id: "a".into(),
                kind: HistoryEventKind::Changed,
                plan_id: "pro".into(),
                billing_cycle: BillingCycle::Monthly,
            },
            HistoryEvent {
                occurred_at: utc(2025, 3, 10),
                project_id: "a".into(),
                kind: HistoryEventKind::Cancelled,
                plan_id: "pro".into(),
                billing_cycle: BillingCycle::Monthly,
            },
        ];
        let points = build_history(&projects, &events, 5, utc(2025, 5, 10));
        // Jan free, Feb pro, Mar pro (changed Feb 10), Apr free (30d hold
        // from Mar 10 ends Apr 9), May free.
        let pro = |p: &SubscriberHistoryPoint| p.by_plan.get("pro").copied().unwrap_or(0);
        assert_eq!(points[0].date, "2025-01-01");
        assert_eq!(pro(&points[0]), 0);
        assert_eq!(pro(&points[1]), 1);
        assert_eq!(pro(&points[2]), 1);
        assert_eq!(pro(&points[3]), 0);
        assert_eq!(pro(&points[4]), 0);
        assert_eq!(points[4].total, 1);
    }

    #[test]
    fn unknown_projects_and_empty_input() {
        let events = vec![HistoryEvent {
            occurred_at: utc(2025, 1, 5),
            project_id: "ghost".into(),
            kind: HistoryEventKind::Changed,
            plan_id: "pro".into(),
            billing_cycle: BillingCycle::Monthly,
        }];
        let points = build_history(&[], &events, 2, utc(2025, 2, 10));
        assert!(points.iter().all(|p| p.total == 0));
    }
}
