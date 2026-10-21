use chrono::{DateTime, Utc};
use sqlx::PgPool;
use ulid::Ulid;

use crate::domain::auth::entities::UserId;
use crate::domain::billing::entities::{BillingCycle, SubscriptionStatus};
use crate::domain::billing::{NewPlan, UpdatePlan};
use crate::ports::auth_store::StoreError;
use crate::ports::billing_store::{
    BillingStore, BoxFut, PlanRecord, SubscriberRecord, SubscriptionRecord,
};

pub struct PgBillingStore {
    pool: PgPool,
}

impl PgBillingStore {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

fn map_err(err: sqlx::Error) -> StoreError {
    if let sqlx::Error::Database(db) = &err
        && let Some(code) = db.code()
        && code == "23505"
    {
        return StoreError::Conflict;
    }
    StoreError::Storage(err.to_string())
}

#[derive(sqlx::FromRow)]
struct PlanRow {
    id: String,
    name: String,
    price_cents: Option<i64>,
    currency: String,
    interval: String,
    capabilities: serde_json::Value,
    yearly_discount: Option<serde_json::Value>,
    is_active: bool,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

impl From<PlanRow> for PlanRecord {
    fn from(row: PlanRow) -> Self {
        Self {
            id: row.id,
            name: row.name,
            price_cents: row.price_cents,
            currency: row.currency,
            interval: row.interval,
            capabilities: Some(row.capabilities),
            yearly_discount: row.yearly_discount,
            is_active: row.is_active,
            created_at: row.created_at,
            updated_at: row.updated_at,
        }
    }
}

#[derive(sqlx::FromRow)]
struct SubscriptionRow {
    id: String,
    project_id: String,
    plan_id: String,
    status: String,
    billing_cycle: String,
    cancel_at_period_end: bool,
    current_period_start: DateTime<Utc>,
    current_period_end: DateTime<Utc>,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

impl From<SubscriptionRow> for SubscriptionRecord {
    fn from(row: SubscriptionRow) -> Self {
        Self {
            id: row.id,
            project_id: row.project_id,
            plan_id: row.plan_id,
            status: row
                .status
                .parse::<SubscriptionStatus>()
                .unwrap_or(SubscriptionStatus::Active),
            billing_cycle: row
                .billing_cycle
                .parse::<BillingCycle>()
                .unwrap_or(BillingCycle::Monthly),
            cancel_at_period_end: row.cancel_at_period_end,
            period_start: row.current_period_start,
            period_end: row.current_period_end,
            created_at: row.created_at,
            updated_at: row.updated_at,
        }
    }
}

#[derive(sqlx::FromRow)]
struct SubscriberRow {
    id: String,
    project_id: String,
    plan_id: String,
    status: String,
    billing_cycle: String,
    current_period_start: DateTime<Utc>,
    current_period_end: DateTime<Utc>,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
    project_name: String,
    customer_name: Option<String>,
    customer_email: Option<String>,
}

impl From<SubscriberRow> for SubscriberRecord {
    fn from(row: SubscriberRow) -> Self {
        Self {
            subscription: SubscriptionRecord {
                id: row.id,
                project_id: row.project_id,
                plan_id: row.plan_id,
                status: row
                    .status
                    .parse::<SubscriptionStatus>()
                    .unwrap_or(SubscriptionStatus::Active),
                billing_cycle: row
                    .billing_cycle
                    .parse::<BillingCycle>()
                    .unwrap_or(BillingCycle::Monthly),
                cancel_at_period_end: false,
                period_start: row.current_period_start,
                period_end: row.current_period_end,
                created_at: row.created_at,
                updated_at: row.updated_at,
            },
            project_name: row.project_name,
            customer_name: row.customer_name,
            customer_email: row.customer_email,
        }
    }
}

/// Owner or member, ignoring soft-deleted rows (mirrors the projects store).
async fn is_visible(pool: &PgPool, actor: &str, project_id: &str) -> Result<bool, StoreError> {
    let row: (bool,) = sqlx::query_as(
        "SELECT EXISTS(
            SELECT 1 FROM platform_projects p
            WHERE p.id = $1 AND p.deleted_at IS NULL
              AND (p.created_by = $2
                   OR EXISTS(SELECT 1 FROM platform_project_members pm
                             WHERE pm.project_id = p.id AND pm.user_id = $2
                               AND pm.deleted_at IS NULL))
         )",
    )
    .bind(project_id)
    .bind(actor)
    .fetch_one(pool)
    .await
    .map_err(map_err)?;
    Ok(row.0)
}

impl BillingStore for PgBillingStore {
    fn project_visible(
        &self,
        actor: UserId,
        project_id: &str,
    ) -> BoxFut<'_, Result<bool, StoreError>> {
        let pool = self.pool.clone();
        let actor = actor.to_string();
        let project_id = project_id.to_string();
        Box::pin(async move { is_visible(&pool, &actor, &project_id).await })
    }

    fn list_active_plans(&self) -> BoxFut<'_, Result<Vec<PlanRecord>, StoreError>> {
        let pool = self.pool.clone();
        Box::pin(async move {
            let rows: Vec<PlanRow> = sqlx::query_as(
                "SELECT id, name, price_cents, currency, interval, capabilities,
                        yearly_discount, is_active, created_at, updated_at
                 FROM billing_plans WHERE is_active ORDER BY price_cents NULLS LAST, id",
            )
            .fetch_all(&pool)
            .await
            .map_err(map_err)?;
            Ok(rows.into_iter().map(PlanRecord::from).collect())
        })
    }

    fn get_plan(&self, plan_id: &str) -> BoxFut<'_, Result<Option<PlanRecord>, StoreError>> {
        let pool = self.pool.clone();
        let plan_id = plan_id.to_string();
        Box::pin(async move {
            let row: Option<PlanRow> = sqlx::query_as(
                "SELECT id, name, price_cents, currency, interval, capabilities,
                        yearly_discount, is_active, created_at, updated_at
                 FROM billing_plans WHERE id = $1",
            )
            .bind(plan_id)
            .fetch_optional(&pool)
            .await
            .map_err(map_err)?;
            Ok(row.map(PlanRecord::from))
        })
    }

    fn get_subscription(
        &self,
        project_id: &str,
    ) -> BoxFut<'_, Result<Option<SubscriptionRecord>, StoreError>> {
        let pool = self.pool.clone();
        let project_id = project_id.to_string();
        Box::pin(async move {
            let row: Option<SubscriptionRow> = sqlx::query_as(
                "SELECT id, project_id, plan_id, status, billing_cycle, cancel_at_period_end,
                        current_period_start, current_period_end, created_at, updated_at
                 FROM project_subscriptions WHERE project_id = $1",
            )
            .bind(project_id)
            .fetch_optional(&pool)
            .await
            .map_err(map_err)?;
            Ok(row.map(SubscriptionRecord::from))
        })
    }

    fn ensure_free_subscription(
        &self,
        project_id: &str,
    ) -> BoxFut<'_, Result<SubscriptionRecord, StoreError>> {
        let pool = self.pool.clone();
        let project_id = project_id.to_string();
        Box::pin(async move {
            let row: SubscriptionRow = sqlx::query_as(
                "INSERT INTO project_subscriptions
                     (id, project_id, plan_id, status, billing_cycle,
                      current_period_start, current_period_end)
                 VALUES ($1, $2, 'free', 'active', 'monthly', now(), now() + INTERVAL '30 days')
                 RETURNING id, project_id, plan_id, status, billing_cycle, cancel_at_period_end,
                           current_period_start, current_period_end, created_at, updated_at",
            )
            .bind(Ulid::new().to_string())
            .bind(project_id)
            .fetch_one(&pool)
            .await
            .map_err(map_err)?;
            Ok(SubscriptionRecord::from(row))
        })
    }

    fn set_subscription(
        &self,
        project_id: &str,
        plan_id: &str,
        status: SubscriptionStatus,
        billing_cycle: BillingCycle,
        period_start: DateTime<Utc>,
        period_end: DateTime<Utc>,
    ) -> BoxFut<'_, Result<SubscriptionRecord, StoreError>> {
        let pool = self.pool.clone();
        let project_id = project_id.to_string();
        let plan_id = plan_id.to_string();
        Box::pin(async move {
            let row: SubscriptionRow = sqlx::query_as(
                "INSERT INTO project_subscriptions
                     (id, project_id, plan_id, status, billing_cycle,
                      current_period_start, current_period_end)
                 VALUES ($1, $2, $3, $4, $5, $6, $7)
                 ON CONFLICT (project_id) DO UPDATE SET
                     plan_id = EXCLUDED.plan_id,
                     status = EXCLUDED.status,
                     billing_cycle = EXCLUDED.billing_cycle,
                     current_period_start = EXCLUDED.current_period_start,
                     current_period_end = EXCLUDED.current_period_end,
                     cancel_at_period_end = FALSE,
                     updated_at = now()
                 RETURNING id, project_id, plan_id, status, billing_cycle, cancel_at_period_end,
                           current_period_start, current_period_end, created_at, updated_at",
            )
            .bind(Ulid::new().to_string())
            .bind(project_id)
            .bind(plan_id)
            .bind(status.as_str())
            .bind(billing_cycle.as_str())
            .bind(period_start)
            .bind(period_end)
            .fetch_one(&pool)
            .await
            .map_err(map_err)?;
            Ok(SubscriptionRecord::from(row))
        })
    }

    fn set_subscription_status(
        &self,
        project_id: &str,
        status: SubscriptionStatus,
    ) -> BoxFut<'_, Result<Option<SubscriptionRecord>, StoreError>> {
        let pool = self.pool.clone();
        let project_id = project_id.to_string();
        Box::pin(async move {
            let row: Option<SubscriptionRow> = sqlx::query_as(
                "UPDATE project_subscriptions SET status = $2, updated_at = now()
                 WHERE project_id = $1
                 RETURNING id, project_id, plan_id, status, billing_cycle, cancel_at_period_end,
                           current_period_start, current_period_end, created_at, updated_at",
            )
            .bind(project_id)
            .bind(status.as_str())
            .fetch_optional(&pool)
            .await
            .map_err(map_err)?;
            Ok(row.map(SubscriptionRecord::from))
        })
    }

    fn set_cancel_at_period_end(
        &self,
        project_id: &str,
        cancel: bool,
    ) -> BoxFut<'_, Result<Option<SubscriptionRecord>, StoreError>> {
        let pool = self.pool.clone();
        let project_id = project_id.to_string();
        Box::pin(async move {
            let row: Option<SubscriptionRow> = sqlx::query_as(
                "UPDATE project_subscriptions SET cancel_at_period_end = $2, updated_at = now()
                 WHERE project_id = $1
                 RETURNING id, project_id, plan_id, status, billing_cycle, cancel_at_period_end,
                           current_period_start, current_period_end, created_at, updated_at",
            )
            .bind(project_id)
            .bind(cancel)
            .fetch_optional(&pool)
            .await
            .map_err(map_err)?;
            Ok(row.map(SubscriptionRecord::from))
        })
    }

    fn list_all_plans(&self) -> BoxFut<'_, Result<Vec<PlanRecord>, StoreError>> {
        let pool = self.pool.clone();
        Box::pin(async move {
            let rows: Vec<PlanRow> = sqlx::query_as(
                "SELECT id, name, price_cents, currency, interval, capabilities,
                        yearly_discount, is_active, created_at, updated_at
                 FROM billing_plans ORDER BY price_cents NULLS LAST, id",
            )
            .fetch_all(&pool)
            .await
            .map_err(map_err)?;
            Ok(rows.into_iter().map(PlanRecord::from).collect())
        })
    }

    fn list_projects_with_created(
        &self,
    ) -> BoxFut<'_, Result<Vec<crate::ports::billing_store::ProjectBirth>, StoreError>> {
        let pool = self.pool.clone();
        Box::pin(async move {
            let rows: Vec<(String, DateTime<Utc>)> = sqlx::query_as(
                "SELECT id, created_at FROM platform_projects WHERE deleted_at IS NULL",
            )
            .fetch_all(&pool)
            .await
            .map_err(map_err)?;
            Ok(rows)
        })
    }

    fn count_active_subscribers(&self, plan_id: &str) -> BoxFut<'_, Result<i64, StoreError>> {
        let pool = self.pool.clone();
        let plan_id = plan_id.to_string();
        Box::pin(async move {
            let row: (i64,) = sqlx::query_as(
                "SELECT COUNT(*) FROM project_subscriptions
                 WHERE plan_id = $1 AND status IN ('active', 'past_due')",
            )
            .bind(plan_id)
            .fetch_one(&pool)
            .await
            .map_err(map_err)?;
            Ok(row.0)
        })
    }

    fn list_subscribers(
        &self,
        plan_id: &str,
    ) -> BoxFut<'_, Result<Vec<SubscriberRecord>, StoreError>> {
        let pool = self.pool.clone();
        let plan_id = plan_id.to_string();
        Box::pin(async move {
            let rows: Vec<SubscriberRow> = sqlx::query_as(
                "SELECT s.id, s.project_id, s.plan_id, s.status, s.billing_cycle,
                        s.current_period_start, s.current_period_end,
                        s.created_at, s.updated_at,
                        p.name AS project_name,
                        u.name AS customer_name, u.email AS customer_email
                 FROM project_subscriptions s
                 JOIN platform_projects p
                   ON p.id = s.project_id AND p.deleted_at IS NULL
                 LEFT JOIN auth_users u
                   ON u.id = p.created_by AND u.deleted_at IS NULL
                 WHERE s.plan_id = $1 AND s.status IN ('active', 'past_due')
                 ORDER BY s.created_at DESC",
            )
            .bind(plan_id)
            .fetch_all(&pool)
            .await
            .map_err(map_err)?;
            Ok(rows.into_iter().map(SubscriberRecord::from).collect())
        })
    }

    fn create_plan(&self, id: &str, input: &NewPlan) -> BoxFut<'_, Result<PlanRecord, StoreError>> {
        let pool = self.pool.clone();
        let id = id.to_string();
        let name = input.name.clone();
        let price_cents = input.price_cents;
        let capabilities = serde_json::to_value(&input.capabilities)
            .unwrap_or(serde_json::Value::Object(Default::default()));
        let yearly_discount = input
            .yearly_discount
            .as_ref()
            .and_then(|d| serde_json::to_value(d).ok());
        Box::pin(async move {
            let row: PlanRow = sqlx::query_as(
                "INSERT INTO billing_plans
                     (id, name, price_cents, currency, interval, capabilities, yearly_discount, is_active)
                 VALUES ($1, $2, $3, 'USD', 'month', $4, $5, TRUE)
                 RETURNING id, name, price_cents, currency, interval, capabilities,
                           yearly_discount, is_active, created_at, updated_at",
            )
            .bind(id)
            .bind(name)
            .bind(price_cents)
            .bind(capabilities)
            .bind(yearly_discount)
            .fetch_one(&pool)
            .await
            .map_err(map_err)?;
            Ok(PlanRecord::from(row))
        })
    }

    fn update_plan(
        &self,
        plan_id: &str,
        input: &UpdatePlan,
    ) -> BoxFut<'_, Result<Option<PlanRecord>, StoreError>> {
        let pool = self.pool.clone();
        let plan_id = plan_id.to_string();
        let name = input.name.clone();
        let price_cents = input.price_cents;
        let capabilities = serde_json::to_value(&input.capabilities)
            .unwrap_or(serde_json::Value::Object(Default::default()));
        let yearly_discount = input
            .yearly_discount
            .as_ref()
            .and_then(|d| serde_json::to_value(d).ok());
        let is_active = input.is_active;
        let interval = if price_cents.is_none() {
            "custom"
        } else {
            "month"
        };
        Box::pin(async move {
            let row: Option<PlanRow> = sqlx::query_as(
                "UPDATE billing_plans SET name = $2, price_cents = $3, interval = $4,
                        capabilities = $5, yearly_discount = $6, is_active = $7, updated_at = now()
                 WHERE id = $1
                 RETURNING id, name, price_cents, currency, interval, capabilities,
                           yearly_discount, is_active, created_at, updated_at",
            )
            .bind(plan_id)
            .bind(name)
            .bind(price_cents)
            .bind(interval)
            .bind(capabilities)
            .bind(yearly_discount)
            .bind(is_active)
            .fetch_optional(&pool)
            .await
            .map_err(map_err)?;
            Ok(row.map(PlanRecord::from))
        })
    }

    fn delete_plan(&self, plan_id: &str) -> BoxFut<'_, Result<bool, StoreError>> {
        let pool = self.pool.clone();
        let plan_id = plan_id.to_string();
        Box::pin(async move {
            let result = sqlx::query("DELETE FROM billing_plans WHERE id = $1")
                .bind(plan_id)
                .execute(&pool)
                .await
                .map_err(map_err)?;
            Ok(result.rows_affected() > 0)
        })
    }
}
