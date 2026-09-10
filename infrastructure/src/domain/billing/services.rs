//! Billing use cases: plan catalog reads, per-project subscriptions,
//! and admin plan management. Record-only (no payment provider yet).

use std::sync::Arc;

use serde_json::json;

use crate::domain::admin::entities::AdminUser;
use crate::domain::audit::AuditService;
use crate::domain::audit::entities::{AuditAction, AuditEvent};
use crate::domain::auth::entities::UserId;
use crate::domain::auth::errors::AuthError;
use crate::domain::billing::entities::{
    BillingCycle, Plan, PlanCapability, Subscription, SubscriptionStatus, YearlyDiscount,
    cycle_price_cents,
};
use crate::ports::auth_store::StoreError;
use crate::ports::billing_store::BillingStore;

use super::entities::FREE_PLAN_ID;
const MAX_NAME: usize = 60;

pub struct BillingService {
    store: Arc<dyn BillingStore>,
    audit: Arc<AuditService>,
}

impl BillingService {
    pub fn new(store: Arc<dyn BillingStore>, audit: Arc<AuditService>) -> Self {
        Self { store, audit }
    }

    fn map_store_error(e: StoreError) -> AuthError {
        match e {
            StoreError::Conflict => AuthError::Conflict("conflicting billing change".into()),
            StoreError::Storage(m) => AuthError::Storage(m),
        }
    }

    // === User catalog + subscription ========================================

    /// Active plans for the customer pricing grid.
    pub async fn list_plans(&self) -> Result<Vec<Plan>, AuthError> {
        Ok(self
            .store
            .list_active_plans()
            .await
            .map_err(Self::map_store_error)?
            .into_iter()
            .map(Plan::from)
            .collect())
    }

    /// The project's subscription, creating the free one on first read so
    /// new projects land on free by default without a backfill. A pending
    /// cancel whose period has passed reverts to free here (lazy, so no
    /// scheduler is needed — every read converges).
    pub async fn get_subscription(
        &self,
        actor: UserId,
        project_id: &str,
    ) -> Result<Subscription, AuthError> {
        if !self
            .store
            .project_visible(actor, project_id)
            .await
            .map_err(Self::map_store_error)?
        {
            return Err(AuthError::NotFound("project not found".into()));
        }
        let record = match self
            .store
            .get_subscription(project_id)
            .await
            .map_err(Self::map_store_error)?
        {
            Some(record) => record,
            None => self
                .store
                .ensure_free_subscription(project_id)
                .await
                .map_err(Self::map_store_error)?,
        };
        if record.cancel_at_period_end && record.period_end <= chrono::Utc::now() {
            return self.revert_to_free(actor, project_id).await;
        }
        self.with_plan(record).await
    }

    /// Subscribe or change plan. `billing_cycle` is "monthly" or "yearly".
    /// Resolves the plan's current price server-side (never trusts client
    /// totals) and refuses archived plans.
    pub async fn subscribe(
        &self,
        actor: UserId,
        project_id: &str,
        plan_id: &str,
        billing_cycle: &str,
    ) -> Result<Subscription, AuthError> {
        use std::str::FromStr;
        let cycle = BillingCycle::from_str(billing_cycle.trim())
            .map_err(|_| AuthError::Validation("billing cycle must be monthly or yearly".into()))?;
        if !self
            .store
            .project_visible(actor, project_id)
            .await
            .map_err(Self::map_store_error)?
        {
            return Err(AuthError::NotFound("project not found".into()));
        }
        let plan = self
            .store
            .get_plan(plan_id.trim())
            .await
            .map_err(Self::map_store_error)?
            .map(Plan::from)
            .ok_or_else(|| AuthError::NotFound("plan not found".into()))?;
        if !plan.is_active {
            return Err(AuthError::PlanRetired(
                "this plan is retired and can no longer be purchased or renewed".into(),
            ));
        }
        if plan.price_cents.is_none() {
            return Err(AuthError::Validation(
                "this plan needs a tailored quote — contact sales".into(),
            ));
        }
        if cycle == BillingCycle::Yearly && plan.yearly_discount.is_none() {
            return Err(AuthError::Validation(
                "this plan has no yearly billing option".into(),
            ));
        }

        let now = chrono::Utc::now();
        let days = cycle.period_days();
        let record = self
            .store
            .set_subscription(
                project_id,
                &plan.id,
                SubscriptionStatus::Active,
                cycle,
                now,
                now + chrono::Duration::days(days),
            )
            .await
            .map_err(Self::map_store_error)?;
        let subscription = self.with_plan(record).await?;
        let amount = cycle_price_cents(&plan, cycle);

        self.audit
            .record(
                chrono::Utc::now(),
                &AuditEvent::new(
                    AuditAction::SubscriptionChanged,
                    Some(&actor.to_string()),
                    None,
                    Some(project_id),
                    format!("project subscribed to '{}' ({})", plan.name, cycle.as_str()),
                    Some(json!({
                        "plan_id": plan.id,
                        "billing_cycle": cycle.as_str(),
                        "amount_cents": amount,
                    })),
                ),
            )
            .await;
        Ok(subscription)
    }

    /// Cancel: the Free plan is the default and can't be cancelled.
    /// A paid plan keeps its benefits until `period_end`, then reverts
    /// to free (see [`Self::get_subscription`]).
    pub async fn cancel(&self, actor: UserId, project_id: &str) -> Result<Subscription, AuthError> {
        let current = self.get_subscription(actor, project_id).await?;
        if current.plan.id == FREE_PLAN_ID {
            return Err(AuthError::Validation(
                "the Free plan is the default and can't be cancelled".into(),
            ));
        }
        let record = self
            .store
            .set_cancel_at_period_end(project_id, true)
            .await
            .map_err(Self::map_store_error)?
            .ok_or_else(|| AuthError::NotFound("project not found".into()))?;
        let subscription = self.with_plan(record).await?;

        self.audit
            .record(
                chrono::Utc::now(),
                &AuditEvent::new(
                    AuditAction::SubscriptionCancelled,
                    Some(&actor.to_string()),
                    None,
                    Some(project_id),
                    format!(
                        "subscription to '{}' cancels at period end ({})",
                        current.plan.name,
                        subscription.period_end.format("%Y-%m-%d"),
                    ),
                    Some(json!({
                        "plan_id": current.plan.id,
                        "billing_cycle": current.billing_cycle.as_str(),
                    })),
                ),
            )
            .await;
        Ok(subscription)
    }

    /// Restart the current period. Refuses archived plans.
    pub async fn renew(&self, actor: UserId, project_id: &str) -> Result<Subscription, AuthError> {
        let current = self.get_subscription(actor, project_id).await?;
        if !current.plan.is_active {
            return Err(AuthError::PlanRetired(
                "this plan is retired and can no longer be renewed".into(),
            ));
        }
        let now = chrono::Utc::now();
        let days = current.billing_cycle.period_days();
        let record = self
            .store
            .set_subscription(
                project_id,
                &current.plan.id,
                SubscriptionStatus::Active,
                current.billing_cycle,
                now,
                now + chrono::Duration::days(days),
            )
            .await
            .map_err(Self::map_store_error)?;
        self.with_plan(record).await
    }

    async fn with_plan(
        &self,
        record: crate::ports::billing_store::SubscriptionRecord,
    ) -> Result<Subscription, AuthError> {
        let plan = self
            .store
            .get_plan(&record.plan_id)
            .await
            .map_err(Self::map_store_error)?
            .map(Plan::from)
            .ok_or_else(|| AuthError::Storage("subscription references a missing plan".into()))?;
        Ok(Subscription {
            id: record.id,
            project_id: record.project_id,
            plan,
            status: record.status,
            billing_cycle: record.billing_cycle,
            cancel_at_period_end: record.cancel_at_period_end,
            period_start: record.period_start,
            period_end: record.period_end,
            created_at: record.created_at,
            updated_at: record.updated_at,
        })
    }

    /// Flip an expired pending-cancel back to free/monthly. Called from
    /// reads so no scheduler is needed.
    async fn revert_to_free(
        &self,
        actor: UserId,
        project_id: &str,
    ) -> Result<Subscription, AuthError> {
        let now = chrono::Utc::now();
        let record = self
            .store
            .set_subscription(
                project_id,
                FREE_PLAN_ID,
                SubscriptionStatus::Active,
                BillingCycle::Monthly,
                now,
                now + chrono::Duration::days(BillingCycle::Monthly.period_days()),
            )
            .await
            .map_err(Self::map_store_error)?;
        let subscription = self.with_plan(record).await?;
        self.audit
            .record(
                chrono::Utc::now(),
                &AuditEvent::new(
                    AuditAction::SubscriptionChanged,
                    Some(&actor.to_string()),
                    None,
                    Some(project_id),
                    "cancelled subscription reverted to the Free plan at period end".to_string(),
                    Some(json!({ "plan_id": FREE_PLAN_ID })),
                ),
            )
            .await;
        Ok(subscription)
    }

    // === Admin plan management ==============================================

    pub async fn list_all_plans(&self) -> Result<Vec<Plan>, AuthError> {
        Ok(self
            .store
            .list_all_plans()
            .await
            .map_err(Self::map_store_error)?
            .into_iter()
            .map(Plan::from)
            .collect())
    }

    pub async fn subscriber_count(&self, plan_id: &str) -> Result<i64, AuthError> {
        self.store
            .count_active_subscribers(plan_id)
            .await
            .map_err(Self::map_store_error)
    }

    /// Any plan by id, active or archived (admin + internal use).
    pub async fn get_plan(&self, plan_id: &str) -> Result<Option<Plan>, AuthError> {
        Ok(self
            .store
            .get_plan(plan_id)
            .await
            .map_err(Self::map_store_error)?
            .map(Plan::from))
    }

    /// Who holds this plan right now (admin drill-down).
    pub async fn list_subscribers(
        &self,
        plan_id: &str,
    ) -> Result<Vec<crate::domain::billing::PlanSubscriber>, AuthError> {
        if self.get_plan(plan_id).await?.is_none() {
            return Err(AuthError::NotFound("plan not found".into()));
        }
        Ok(self
            .store
            .list_subscribers(plan_id)
            .await
            .map_err(Self::map_store_error)?
            .into_iter()
            .map(crate::domain::billing::PlanSubscriber::from)
            .collect())
    }

    /// Trailing monthly active-subscriber snapshots, replayed from
    /// project births + billing audit events (no ledger table needed).
    pub async fn subscription_history(
        &self,
        months: usize,
    ) -> Result<Vec<crate::domain::billing::SubscriberHistoryPoint>, AuthError> {
        use crate::domain::billing::{
            HistoryEvent, HistoryEventKind, HistoryProject, build_history,
        };
        use crate::ports::audit_store::AdminAuditFilters;

        let projects = self
            .store
            .list_projects_with_created()
            .await
            .map_err(Self::map_store_error)?
            .into_iter()
            .map(|(id, created_at)| HistoryProject { id, created_at })
            .collect::<Vec<_>>();

        let mut events = Vec::new();
        for event_type in [
            "billing.subscription_changed",
            "billing.subscription_cancelled",
        ] {
            let mut offset = 0;
            loop {
                let (page, _) = self
                    .audit
                    .list_all(
                        AdminAuditFilters {
                            event_type: Some(event_type),
                            ..Default::default()
                        },
                        5000,
                        offset,
                    )
                    .await?;
                let done = (page.len() as i64) < 5000;
                events.extend(page.into_iter().filter_map(|entry| {
                    let project_id = entry.project_id?;
                    let meta = entry.metadata?;
                    let plan_id = meta.get("plan_id")?.as_str()?.to_string();
                    let kind = match entry.event_type.as_str() {
                        "billing.subscription_changed" => HistoryEventKind::Changed,
                        _ => HistoryEventKind::Cancelled,
                    };
                    let billing_cycle = meta
                        .get("billing_cycle")
                        .and_then(|v| v.as_str())
                        .and_then(|s| {
                            use std::str::FromStr;
                            crate::domain::billing::BillingCycle::from_str(s).ok()
                        })
                        .unwrap_or(crate::domain::billing::BillingCycle::Monthly);
                    Some(HistoryEvent {
                        occurred_at: entry.occurred_at,
                        project_id,
                        kind,
                        plan_id,
                        billing_cycle,
                    })
                }));
                if done {
                    break;
                }
                offset += 5000;
            }
        }

        Ok(build_history(
            &projects,
            &events,
            months,
            chrono::Utc::now(),
        ))
    }

    pub async fn create_plan(&self, admin: &AdminUser, input: NewPlan) -> Result<Plan, AuthError> {
        let input = input.validated()?;
        let id = self.unique_id(&input.name, None).await?;
        let record = self
            .store
            .create_plan(&id, &input)
            .await
            .map_err(Self::map_store_error)?;
        let plan = Plan::from(record);
        self.audit
            .record(
                chrono::Utc::now(),
                &AuditEvent::new_admin(
                    AuditAction::PlanCreated,
                    &admin.id.to_string(),
                    Some(&admin.name),
                    None,
                    format!("pricing plan '{}' created", plan.name),
                    Some(json!({ "plan_id": plan.id })),
                ),
            )
            .await;
        Ok(plan)
    }

    pub async fn update_plan(
        &self,
        admin: &AdminUser,
        plan_id: &str,
        input: UpdatePlan,
    ) -> Result<Plan, AuthError> {
        let input = input.validated()?;
        let record = self
            .store
            .update_plan(plan_id, &input)
            .await
            .map_err(Self::map_store_error)?
            .ok_or_else(|| AuthError::NotFound("plan not found".into()))?;
        let plan = Plan::from(record);
        self.audit
            .record(
                chrono::Utc::now(),
                &AuditEvent::new_admin(
                    AuditAction::PlanUpdated,
                    &admin.id.to_string(),
                    Some(&admin.name),
                    None,
                    format!("pricing plan '{}' updated", plan.name),
                    Some(json!({ "plan_id": plan.id })),
                ),
            )
            .await;
        Ok(plan)
    }

    /// Hard delete. Blocked while the plan still has subscribers — the
    /// admin console archives (`is_active = false`) instead.
    pub async fn delete_plan(&self, admin: &AdminUser, plan_id: &str) -> Result<(), AuthError> {
        let count = self.subscriber_count(plan_id).await?;
        if count > 0 {
            return Err(AuthError::Conflict(format!(
                "plan still has {count} active subscriber(s); archive it instead"
            )));
        }
        let deleted = self
            .store
            .delete_plan(plan_id)
            .await
            .map_err(Self::map_store_error)?;
        if !deleted {
            return Err(AuthError::NotFound("plan not found".into()));
        }
        self.audit
            .record(
                chrono::Utc::now(),
                &AuditEvent::new_admin(
                    AuditAction::PlanDeleted,
                    &admin.id.to_string(),
                    Some(&admin.name),
                    None,
                    format!("pricing plan '{plan_id}' deleted"),
                    Some(json!({ "plan_id": plan_id })),
                ),
            )
            .await;
        Ok(())
    }

    async fn unique_id(&self, name: &str, ignore: Option<&str>) -> Result<String, AuthError> {
        let base = slugify(name);
        let plans = self.list_all_plans().await?;
        let taken: std::collections::HashSet<&str> = plans
            .iter()
            .filter(|p| Some(p.id.as_str()) != ignore)
            .map(|p| p.id.as_str())
            .collect();
        if !taken.contains(base.as_str()) {
            return Ok(base);
        }
        let mut n = 2;
        loop {
            let candidate = format!("{base}-{n}");
            if !taken.contains(candidate.as_str()) {
                return Ok(candidate);
            }
            n += 1;
        }
    }
}

fn slugify(name: &str) -> String {
    let slug: String = name
        .to_lowercase()
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '-' })
        .collect::<String>()
        .split('-')
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>()
        .join("-");
    if slug.is_empty() {
        "plan".to_string()
    } else {
        slug
    }
}

/// Validated input for creating a plan (admin).
#[derive(Debug, Clone)]
pub struct NewPlan {
    pub name: String,
    pub price_cents: Option<i64>,
    pub capabilities: PlanCapability,
    pub yearly_discount: Option<YearlyDiscount>,
}

impl NewPlan {
    fn validated(self) -> Result<Self, AuthError> {
        validate_plan_fields(&self.name, self.price_cents, &self.yearly_discount)?;
        Ok(Self {
            name: self.name.trim().to_string(),
            ..self
        })
    }
}

/// Full-replacement update (admin edits all fields at once).
#[derive(Debug, Clone)]
pub struct UpdatePlan {
    pub name: String,
    pub price_cents: Option<i64>,
    pub capabilities: PlanCapability,
    pub yearly_discount: Option<YearlyDiscount>,
    pub is_active: bool,
}

impl UpdatePlan {
    fn validated(self) -> Result<Self, AuthError> {
        validate_plan_fields(&self.name, self.price_cents, &self.yearly_discount)?;
        Ok(Self {
            name: self.name.trim().to_string(),
            ..self
        })
    }
}

fn validate_plan_fields(
    name: &str,
    price_cents: Option<i64>,
    yearly_discount: &Option<YearlyDiscount>,
) -> Result<(), AuthError> {
    let name = name.trim();
    if name.is_empty() || name.len() > MAX_NAME {
        return Err(AuthError::Validation(
            "plan name is required (60 characters max)".into(),
        ));
    }
    if let Some(price) = price_cents
        && price < 0
    {
        return Err(AuthError::Validation("price can't be negative".into()));
    }
    match yearly_discount {
        None => Ok(()),
        Some(_) if price_cents.is_none() => Err(AuthError::Validation(
            "custom-price plans can't offer a yearly discount".into(),
        )),
        Some(YearlyDiscount::Percent(p)) => {
            if !(*p > 0.0 && *p < 100.0) {
                return Err(AuthError::Validation(
                    "percent discount must be between 0 and 100".into(),
                ));
            }
            Ok(())
        }
        Some(YearlyDiscount::Fixed(cents)) => {
            let base = price_cents.unwrap_or(0).saturating_mul(12);
            if *cents <= 0 || *cents >= base {
                return Err(AuthError::Validation(
                    "fixed discount must be below the 12-month total".into(),
                ));
            }
            Ok(())
        }
    }
}
