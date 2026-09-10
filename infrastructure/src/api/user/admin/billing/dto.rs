//! Request/response shapes for the admin billing API (camelCase).

use serde::{Deserialize, Serialize};

use crate::api::user::billing::dto::{CapabilitiesInput, PlanDto};
use crate::domain::billing::entities::{Plan, PlanSubscriber, YearlyDiscount};
use crate::domain::billing::{NewPlan, UpdatePlan};

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AdminPlanDto {
    #[serde(flatten)]
    pub plan: PlanDto,
    pub active_subscribers: i64,
}

impl AdminPlanDto {
    pub fn with_count(plan: Plan, active_subscribers: i64) -> Self {
        Self {
            plan: PlanDto::from(plan),
            active_subscribers,
        }
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PlanSubscriberDto {
    pub subscription_id: String,
    pub project_id: String,
    pub project_name: String,
    pub customer_name: Option<String>,
    pub customer_email: Option<String>,
    pub status: String,
    pub billing_cycle: String,
    pub period_end: chrono::DateTime<chrono::Utc>,
}

impl From<PlanSubscriber> for PlanSubscriberDto {
    fn from(s: PlanSubscriber) -> Self {
        Self {
            subscription_id: s.subscription_id,
            project_id: s.project_id,
            project_name: s.project_name,
            customer_name: s.customer_name,
            customer_email: s.customer_email,
            status: s.status.as_str().to_string(),
            billing_cycle: s.billing_cycle.as_str().to_string(),
            period_end: s.period_end,
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreatePlanRequest {
    pub name: String,
    pub price_cents: Option<i64>,
    #[serde(default)]
    pub capabilities: CapabilitiesInput,
    #[serde(default)]
    pub yearly_discount: Option<YearlyDiscount>,
}

impl From<CreatePlanRequest> for NewPlan {
    fn from(req: CreatePlanRequest) -> Self {
        Self {
            name: req.name,
            price_cents: req.price_cents,
            capabilities: req.capabilities.into(),
            yearly_discount: req.yearly_discount,
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdatePlanRequest {
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub price_cents: Option<Option<i64>>,
    #[serde(default)]
    pub capabilities: Option<CapabilitiesInput>,
    #[serde(default)]
    pub yearly_discount: Option<Option<YearlyDiscount>>,
    #[serde(default)]
    pub is_active: Option<bool>,
}

impl UpdatePlanRequest {
    /// Overlay onto the stored plan (full-replacement service input).
    pub fn apply_to(self, plan: &Plan) -> UpdatePlan {
        UpdatePlan {
            name: self.name.unwrap_or_else(|| plan.name.clone()),
            price_cents: self.price_cents.unwrap_or(plan.price_cents),
            capabilities: self
                .capabilities
                .map(crate::domain::billing::PlanCapability::from)
                .unwrap_or_else(|| plan.capabilities.clone()),
            yearly_discount: self.yearly_discount.unwrap_or(plan.yearly_discount),
            is_active: self.is_active.unwrap_or(plan.is_active),
        }
    }
}
