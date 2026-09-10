//! Request/response shapes for the customer billing API (camelCase).

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::domain::billing::entities::{
    BillingCycle, Plan, PlanCapability, Subscription, SubscriptionStatus, YearlyDiscount,
    cycle_price_cents, yearly_price_cents,
};

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CapabilitiesDto {
    pub notifications_per_month: Option<i64>,
    pub channels: Vec<String>,
    pub team_members: Option<i64>,
    pub retention_days: Option<i64>,
    pub support: String,
    pub branding: bool,
    pub api_calls: Option<i64>,
}

impl From<PlanCapability> for CapabilitiesDto {
    fn from(c: PlanCapability) -> Self {
        Self {
            notifications_per_month: c.notifications_per_month,
            channels: c.channels,
            team_members: c.team_members,
            retention_days: c.retention_days,
            support: c.support,
            branding: c.branding,
            api_calls: c.api_calls,
        }
    }
}

#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CapabilitiesInput {
    #[serde(default)]
    pub notifications_per_month: Option<i64>,
    #[serde(default)]
    pub channels: Vec<String>,
    #[serde(default)]
    pub team_members: Option<i64>,
    #[serde(default)]
    pub retention_days: Option<i64>,
    #[serde(default)]
    pub support: String,
    #[serde(default)]
    pub branding: bool,
    #[serde(default)]
    pub api_calls: Option<i64>,
}

impl From<CapabilitiesInput> for PlanCapability {
    fn from(c: CapabilitiesInput) -> Self {
        Self {
            notifications_per_month: c.notifications_per_month,
            channels: c.channels,
            team_members: c.team_members,
            retention_days: c.retention_days,
            support: c.support,
            branding: c.branding,
            api_calls: c.api_calls,
        }
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PlanDto {
    pub id: String,
    pub name: String,
    pub price_cents: Option<i64>,
    pub currency: String,
    pub capabilities: CapabilitiesDto,
    pub yearly_discount: Option<YearlyDiscount>,
    /// 12-month total after the discount (None for custom plans).
    pub yearly_price_cents: Option<i64>,
    pub is_active: bool,
}

impl From<Plan> for PlanDto {
    fn from(plan: Plan) -> Self {
        let yearly = yearly_price_cents(&plan);
        Self {
            id: plan.id,
            name: plan.name,
            price_cents: plan.price_cents,
            currency: plan.currency,
            capabilities: CapabilitiesDto::from(plan.capabilities),
            yearly_discount: plan.yearly_discount,
            yearly_price_cents: yearly,
            is_active: plan.is_active,
        }
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UsageDto {
    /// Notifications sent in the current period. The send engine (M2)
    /// feeds this; until then it reports zero against real limits.
    pub notifications_used: i64,
    pub notifications_limit: Option<i64>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SubscriptionDto {
    pub id: String,
    pub project_id: String,
    pub plan: PlanDto,
    pub status: SubscriptionStatus,
    pub billing_cycle: BillingCycle,
    /// Paid plan ends at `period_end`, then reverts to free.
    pub cancel_at_period_end: bool,
    /// Price in cents for the current period (None for custom plans).
    pub period_price_cents: Option<i64>,
    pub period_start: DateTime<Utc>,
    pub period_end: DateTime<Utc>,
    pub usage: UsageDto,
    pub updated_at: DateTime<Utc>,
}

impl From<Subscription> for SubscriptionDto {
    fn from(sub: Subscription) -> Self {
        let period_price_cents = cycle_price_cents(&sub.plan, sub.billing_cycle);
        let notifications_limit = sub.plan.capabilities.notifications_per_month;
        Self {
            id: sub.id,
            project_id: sub.project_id,
            plan: PlanDto::from(sub.plan),
            status: sub.status,
            billing_cycle: sub.billing_cycle,
            cancel_at_period_end: sub.cancel_at_period_end,
            period_price_cents,
            period_start: sub.period_start,
            period_end: sub.period_end,
            usage: UsageDto {
                notifications_used: 0,
                notifications_limit,
            },
            updated_at: sub.updated_at,
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SubscribeRequest {
    pub project_id: String,
    pub plan_id: String,
    /// "monthly" (default) or "yearly".
    pub billing_cycle: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CancelRequest {
    pub project_id: String,
}
