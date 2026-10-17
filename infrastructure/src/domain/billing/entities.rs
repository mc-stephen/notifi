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

/// One billed period for the user-facing receipt history (`GET
/// /app/billing/history`). Derived, not stored: there is no ledger table.
///
/// Free ($0) periods emit rows too — Cloudflare-style, every project gets
/// an invoice from birth, even on free. Source of truth limits (v1):
/// amounts compute from *current* plan prices (a later price edit
/// backfills older rows), and renewals that wrote no audit event appear
/// via timeline continuity. Exactness arrives with the payment provider.
#[derive(Debug, Clone, PartialEq)]
pub struct Receipt {
    pub id: String,
    pub period_start: chrono::DateTime<chrono::Utc>,
    pub plan_id: String,
    pub plan_name: String,
    pub billing_cycle: BillingCycle,
    pub amount_cents: Option<i64>,
    pub currency: String,
    pub status: ReceiptStatus,
}

/// Receipt outcome. `$0` rows (free periods, nothing owed) read as paid.
/// The latest row flips to failed while the live subscription is past due.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReceiptStatus {
    Paid,
    Failed,
}

impl ReceiptStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Paid => "paid",
            Self::Failed => "failed",
        }
    }
}

/// Maximum receipts emitted per project (newest kept implicitly by the
/// caller reversing chronological output).
const MAX_RECEIPTS: usize = 120;

/// Rebuilds one project's billed periods by replaying plan events. Pure
/// (no I/O) so it unit-tests directly. `plans` maps plan id to its
/// *current* catalog entry; plans missing from the map (deleted since)
/// emit with unknown price. `current_failed` marks the latest row failed
/// when the live subscription is past due.
pub fn build_receipts(
    project: &HistoryProject,
    events: &[HistoryEvent],
    plans: &std::collections::HashMap<String, Plan>,
    current_failed: bool,
    now: chrono::DateTime<chrono::Utc>,
) -> Vec<Receipt> {
    // Timeline of (at, plan_id, cycle), starting free at project creation.
    // Cancelled holds the plan through its billing period, then free —
    // mirroring `build_history`.
    let mut timeline: Vec<(chrono::DateTime<chrono::Utc>, String, BillingCycle)> = vec![(
        project.created_at,
        FREE_PLAN_ID.to_string(),
        BillingCycle::Monthly,
    )];
    let mut ordered: Vec<&HistoryEvent> = events
        .iter()
        .filter(|e| e.project_id == project.id)
        .collect();
    ordered.sort_by_key(|e| e.occurred_at);
    for event in ordered {
        match event.kind {
            HistoryEventKind::Changed => {
                timeline.push((
                    event.occurred_at,
                    event.plan_id.clone(),
                    event.billing_cycle,
                ));
            }
            HistoryEventKind::Cancelled => {
                let hold_until =
                    event.occurred_at + chrono::Duration::days(event.billing_cycle.period_days());
                timeline.push((hold_until, FREE_PLAN_ID.to_string(), BillingCycle::Monthly));
            }
        }
    }
    timeline.sort_by_key(|s| s.0);

    let mut receipts = Vec::new();
    for (i, (seg_start, plan_id, cycle)) in timeline.iter().enumerate() {
        if receipts.len() >= MAX_RECEIPTS {
            break;
        }
        let seg_end = timeline
            .get(i + 1)
            .map(|(at, _, _)| *at)
            .unwrap_or(now)
            .min(now);
        if *seg_start >= seg_end {
            continue;
        }
        let (plan_name, amount_cents, currency) = match plans.get(plan_id) {
            Some(plan) => (
                plan.name.clone(),
                cycle_price_cents(plan, *cycle),
                plan.currency.clone(),
            ),
            // Deleted since: keep the row, mark the price unknown.
            None => (plan_id.clone(), None, "USD".to_string()),
        };
        // Deleted plans keep their row with unknown price; free segments
        // emit $0 rows like any other period.
        if plan_id != FREE_PLAN_ID && amount_cents == Some(0) {
            continue;
        }
        let period_days = cycle.period_days();
        let mut period_start = *seg_start;
        let mut seq = receipts.len();
        while period_start < seg_end && receipts.len() < MAX_RECEIPTS {
            seq += 1;
            receipts.push(Receipt {
                id: format!("rcpt_{}_{:04}", project.id, seq),
                period_start,
                plan_id: plan_id.clone(),
                plan_name: plan_name.clone(),
                billing_cycle: *cycle,
                amount_cents,
                currency: currency.clone(),
                status: ReceiptStatus::Paid,
            });
            period_start += chrono::Duration::days(period_days);
        }
    }

    if current_failed && let Some(last) = receipts.last_mut() {
        last.status = ReceiptStatus::Failed;
    }
    receipts
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
    ordered.sort_by_key(|e| e.occurred_at);
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
        timeline.sort_by_key(|s| s.0);
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

    fn pro_plans() -> std::collections::HashMap<String, Plan> {
        let at = utc(2025, 1, 1);
        let free = Plan {
            id: FREE_PLAN_ID.into(),
            name: "Free".into(),
            price_cents: Some(0),
            currency: "USD".into(),
            interval: "month".into(),
            capabilities: PlanCapability::default(),
            yearly_discount: None,
            is_active: true,
            created_at: at,
            updated_at: at,
        };
        let pro = Plan {
            id: "pro".into(),
            name: "Pro".into(),
            price_cents: Some(9900),
            currency: "USD".into(),
            interval: "month".into(),
            capabilities: PlanCapability::default(),
            yearly_discount: Some(YearlyDiscount::Percent(20.0)),
            is_active: true,
            created_at: at,
            updated_at: at,
        };
        let mut plans = std::collections::HashMap::new();
        plans.insert(FREE_PLAN_ID.to_string(), free);
        plans.insert("pro".to_string(), pro);
        plans
    }

    fn changed(at: chrono::DateTime<chrono::Utc>, plan: &str) -> HistoryEvent {
        HistoryEvent {
            occurred_at: at,
            project_id: "a".into(),
            kind: HistoryEventKind::Changed,
            plan_id: plan.into(),
            billing_cycle: BillingCycle::Monthly,
        }
    }

    #[test]
    fn free_only_project_emits_zero_dollar_rows() {
        let receipts = build_receipts(
            &project("a", utc(2025, 1, 5)),
            &[],
            &pro_plans(),
            false,
            utc(2025, 5, 10),
        );
        // Jan 5, Feb 4, Mar 6, Apr 5, May 5 (30-day slices); Jun 4 is past `now`.
        assert_eq!(receipts.len(), 5);
        assert!(receipts.iter().all(|r| r.plan_id == "free"));
        assert!(receipts.iter().all(|r| r.amount_cents == Some(0)));
        assert!(receipts.iter().all(|r| r.status == ReceiptStatus::Paid));
    }

    #[test]
    fn monthly_pro_coexists_with_free_rows() {
        let receipts = build_receipts(
            &project("a", utc(2025, 1, 5)),
            &[changed(utc(2025, 2, 10), "pro")],
            &pro_plans(),
            false,
            utc(2025, 5, 10),
        );
        // Free Jan 5, Feb 4; pro Feb 10, Mar 12, Apr 11.
        assert_eq!(receipts.len(), 5);
        assert_eq!(receipts[0].plan_id, "free");
        assert_eq!(receipts[0].amount_cents, Some(0));
        assert_eq!(receipts[2].plan_id, "pro");
        assert_eq!(receipts[2].amount_cents, Some(9900));
        assert_eq!(receipts[2].period_start, utc(2025, 2, 10));
        let mut ids: Vec<_> = receipts.iter().map(|r| r.id.clone()).collect();
        ids.sort();
        ids.dedup();
        assert_eq!(ids.len(), 5);
    }

    #[test]
    fn cancelled_hold_then_free_rows() {
        let events = vec![
            changed(utc(2025, 1, 10), "pro"),
            HistoryEvent {
                occurred_at: utc(2025, 3, 10),
                project_id: "a".into(),
                kind: HistoryEventKind::Cancelled,
                plan_id: "pro".into(),
                billing_cycle: BillingCycle::Monthly,
            },
        ];
        let receipts = build_receipts(
            &project("a", utc(2025, 1, 5)),
            &events,
            &pro_plans(),
            false,
            utc(2025, 6, 10),
        );
        // Free Jan 5; pro Jan 10, Feb 9, Mar 11 (hold ends Apr 9);
        // free Apr 9, May 9, Jun 8.
        assert_eq!(receipts.len(), 7);
        assert_eq!(receipts[0].plan_id, "free");
        assert!(receipts[1..4].iter().all(|r| r.plan_id == "pro"));
        assert!(receipts[4..].iter().all(|r| r.plan_id == "free"));
    }

    #[test]
    fn yearly_plan_emits_annual_rows() {
        let events = vec![HistoryEvent {
            occurred_at: utc(2025, 6, 1),
            project_id: "a".into(),
            kind: HistoryEventKind::Changed,
            plan_id: "pro".into(),
            billing_cycle: BillingCycle::Yearly,
        }];
        let receipts = build_receipts(
            &project("a", utc(2025, 1, 5)),
            &events,
            &pro_plans(),
            false,
            utc(2026, 9, 1),
        );
        // Free Jan–May 2025, then annual pro rows Jun 2025 + Jun 2026.
        assert_eq!(receipts.len(), 7);
        assert_eq!(receipts[5].billing_cycle, BillingCycle::Yearly);
        // 9900 * 12 * 0.8 = 95040.
        assert_eq!(receipts[5].amount_cents, Some(95040));
    }

    #[test]
    fn current_past_due_marks_latest_row_failed() {
        let receipts = build_receipts(
            &project("a", utc(2025, 1, 5)),
            &[changed(utc(2025, 2, 10), "pro")],
            &pro_plans(),
            true,
            utc(2025, 5, 10),
        );
        assert_eq!(receipts.len(), 5);
        assert!(
            receipts[..4]
                .iter()
                .all(|r| r.status == ReceiptStatus::Paid)
        );
        assert_eq!(receipts[4].status, ReceiptStatus::Failed);
    }

    #[test]
    fn free_then_pro_then_free_marks_the_boundary() {
        let events = vec![
            changed(utc(2025, 2, 10), "pro"),
            HistoryEvent {
                occurred_at: utc(2025, 3, 15),
                project_id: "a".into(),
                kind: HistoryEventKind::Changed,
                plan_id: "free".into(),
                billing_cycle: BillingCycle::Monthly,
            },
        ];
        let receipts = build_receipts(
            &project("a", utc(2025, 1, 5)),
            &events,
            &pro_plans(),
            false,
            utc(2025, 6, 10),
        );
        // Free Jan 5, Feb 4; pro Feb 10, Mar 12; free Mar 15, Apr 14, May 14.
        let plans: Vec<_> = receipts.iter().map(|r| r.plan_id.as_str()).collect();
        assert_eq!(
            plans,
            ["free", "free", "pro", "pro", "free", "free", "free"]
        );
        assert_eq!(receipts[2].amount_cents, Some(9900));
        assert_eq!(receipts[4].amount_cents, Some(0));
    }
}
