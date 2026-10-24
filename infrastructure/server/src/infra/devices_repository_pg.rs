//! sqlx implementation of [`DevicesStore`] (PostgreSQL).
//!
//! Devices live in `project_devices` (see `0004_project_devices.sql`).
//! Every query is scoped to a project the caller owns or belongs to
//! (owner-or-member — the same predicate family as the recipients
//! store), and register upserts: re-registering the same
//! `(project, destination, token)` refreshes the row instead of
//! duplicating it.

use chrono::{DateTime, Utc};
use sqlx::PgPool;
use ulid::Ulid;

use crate::domain::auth::entities::UserId;
use crate::ports::auth_store::StoreError;
use crate::ports::devices_store::{BoxFut, DeviceRecord, DeviceRegistration, DevicesStore};

pub struct PgDevicesStore {
    pool: PgPool,
}

impl PgDevicesStore {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

/// Maps driver errors; unique violations become [`StoreError::Conflict`].
fn map_err(err: sqlx::Error) -> StoreError {
    if let sqlx::Error::Database(db) = &err
        && let Some(code) = db.code()
        && code == "23505"
    {
        return StoreError::Conflict;
    }
    StoreError::Storage(err.to_string())
}

/// Signals surfaced to the service layer and promoted to not-found
/// problems there (register has no `None` return shape).
const MSG_PROJECT_NOT_VISIBLE: &str = "project not found or not visible";
const MSG_RECIPIENT_NOT_FOUND: &str = "recipient not found";

#[derive(sqlx::FromRow)]
struct DeviceRow {
    id: String,
    project_id: String,
    recipient_id: Option<String>,
    destination: String,
    vendor: String,
    token: String,
    label: Option<String>,
    status: String,
    last_seen_at: Option<DateTime<Utc>>,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

impl From<DeviceRow> for DeviceRecord {
    fn from(row: DeviceRow) -> Self {
        Self {
            id: row.id,
            project_id: row.project_id,
            recipient_id: row.recipient_id,
            destination: row.destination,
            vendor: row.vendor,
            token: row.token,
            label: row.label,
            status: row.status,
            last_seen_at: row.last_seen_at,
            created_at: row.created_at,
            updated_at: row.updated_at,
        }
    }
}

impl DevicesStore for PgDevicesStore {
    fn register(
        &self,
        actor: UserId,
        project_id: &str,
        registration: DeviceRegistration,
    ) -> BoxFut<'_, Result<DeviceRecord, StoreError>> {
        let pool = self.pool.clone();
        let actor = actor.to_string();
        let project_id = project_id.to_string();
        Box::pin(async move {
            // The caller owns or belongs to the project — checked here so
            // registration can't materialize rows under other projects.
            let visible = sqlx::query_scalar::<_, bool>(
                "SELECT EXISTS (
                     SELECT 1 FROM platform_projects p
                     WHERE p.id = $1
                       AND p.deleted_at IS NULL
                       AND (
                            p.created_by = $2
                            OR EXISTS (
                                SELECT 1 FROM platform_project_members pm
                                WHERE pm.project_id = p.id
                                  AND pm.user_id = $2
                                  AND pm.deleted_at IS NULL
                            )
                       )
                 )",
            )
            .bind(&project_id)
            .bind(&actor)
            .fetch_one(&pool)
            .await
            .map_err(map_err)?;
            if !visible {
                return Err(StoreError::Storage(MSG_PROJECT_NOT_VISIBLE.to_string()));
            }

            // A recipient assignment must target a live recipient inside
            // the same project (the FK alone can't know that).
            if let Some(recipient_id) = &registration.recipient_id {
                let assigned = sqlx::query_scalar::<_, bool>(
                    "SELECT EXISTS (
                         SELECT 1 FROM platform_recipients r
                         WHERE r.id = $1
                           AND r.project_id = $2
                           AND r.deleted_at IS NULL
                     )",
                )
                .bind(recipient_id)
                .bind(&project_id)
                .fetch_one(&pool)
                .await
                .map_err(map_err)?;
                if !assigned {
                    return Err(StoreError::Storage(MSG_RECIPIENT_NOT_FOUND.to_string()));
                }
            }

            let device = Ulid::new().to_string();
            let row = sqlx::query_as::<_, DeviceRow>(
                "INSERT INTO project_devices AS d
                     (id, project_id, recipient_id, destination, vendor, token,
                      label, status, last_seen_at)
                 SELECT $1, $2, $3, $4, $5, $6, $7, 'active', now()
                 ON CONFLICT (project_id, destination, token) DO UPDATE
                     SET recipient_id = EXCLUDED.recipient_id,
                         label        = EXCLUDED.label,
                         status       = 'active',
                         last_seen_at = now(),
                         updated_at   = now()
                 RETURNING id, project_id, recipient_id, destination, vendor,
                           token, label, status, last_seen_at, created_at, updated_at",
            )
            .bind(&device)
            .bind(&project_id)
            .bind(&registration.recipient_id)
            .bind(&registration.destination)
            .bind(&registration.vendor)
            .bind(&registration.token)
            .bind(&registration.label)
            .fetch_one(&pool)
            .await
            .map_err(map_err)?;

            Ok(row.into())
        })
    }

    fn list(
        &self,
        actor: UserId,
        project_id: &str,
        recipient_id: Option<&str>,
    ) -> BoxFut<'_, Result<Vec<DeviceRecord>, StoreError>> {
        let pool = self.pool.clone();
        let actor = actor.to_string();
        let project_id = project_id.to_string();
        let recipient_id = recipient_id.map(str::to_string);
        Box::pin(async move {
            let rows = sqlx::query_as::<_, DeviceRow>(
                "SELECT d.id, d.project_id, d.recipient_id, d.destination, d.vendor,
                        d.token, d.label, d.status, d.last_seen_at, d.created_at, d.updated_at
                 FROM project_devices d
                 WHERE d.project_id = $1
                   AND EXISTS (
                       SELECT 1 FROM platform_projects p
                       WHERE p.id = d.project_id
                         AND p.deleted_at IS NULL
                         AND (
                              p.created_by = $2
                              OR EXISTS (
                                  SELECT 1 FROM platform_project_members pm
                                  WHERE pm.project_id = p.id
                                    AND pm.user_id = $2
                                    AND pm.deleted_at IS NULL
                              )
                         )
                   )
                   AND ($3::TEXT IS NULL OR d.recipient_id = $3)
                 ORDER BY d.created_at DESC, d.id DESC
                 LIMIT 200",
            )
            .bind(&project_id)
            .bind(&actor)
            .bind(&recipient_id)
            .fetch_all(&pool)
            .await
            .map_err(map_err)?;

            Ok(rows.into_iter().map(DeviceRecord::from).collect())
        })
    }

    fn get(
        &self,
        actor: UserId,
        project_id: &str,
        device_id: &str,
    ) -> BoxFut<'_, Result<Option<DeviceRecord>, StoreError>> {
        let pool = self.pool.clone();
        let actor = actor.to_string();
        let project_id = project_id.to_string();
        let device_id = device_id.to_string();
        Box::pin(async move {
            let row = sqlx::query_as::<_, DeviceRow>(
                "SELECT d.id, d.project_id, d.recipient_id, d.destination, d.vendor,
                        d.token, d.label, d.status, d.last_seen_at, d.created_at, d.updated_at
                 FROM project_devices d
                 WHERE d.id = $1
                   AND d.project_id = $2
                   AND EXISTS (
                       SELECT 1 FROM platform_projects p
                       WHERE p.id = d.project_id
                         AND p.deleted_at IS NULL
                         AND (
                              p.created_by = $3
                              OR EXISTS (
                                  SELECT 1 FROM platform_project_members pm
                                  WHERE pm.project_id = p.id
                                    AND pm.user_id = $3
                                    AND pm.deleted_at IS NULL
                              )
                         )
                   )",
            )
            .bind(&device_id)
            .bind(&project_id)
            .bind(&actor)
            .fetch_optional(&pool)
            .await
            .map_err(map_err)?;

            Ok(row.map(DeviceRecord::from))
        })
    }

    fn revoke(
        &self,
        actor: UserId,
        project_id: &str,
        device_id: &str,
    ) -> BoxFut<'_, Result<Option<DeviceRecord>, StoreError>> {
        let pool = self.pool.clone();
        let actor = actor.to_string();
        let project_id = project_id.to_string();
        let device_id = device_id.to_string();
        Box::pin(async move {
            let row = sqlx::query_as::<_, DeviceRow>(
                "UPDATE project_devices d
                 SET status = 'revoked', updated_at = now()
                 WHERE d.id = $1
                   AND d.project_id = $2
                   AND EXISTS (
                       SELECT 1 FROM platform_projects p
                       WHERE p.id = d.project_id
                         AND p.deleted_at IS NULL
                         AND (
                              p.created_by = $3
                              OR EXISTS (
                                  SELECT 1 FROM platform_project_members pm
                                  WHERE pm.project_id = p.id
                                    AND pm.user_id = $3
                                    AND pm.deleted_at IS NULL
                              )
                         )
                   )
                 RETURNING d.id, d.project_id, d.recipient_id, d.destination, d.vendor,
                           d.token, d.label, d.status, d.last_seen_at, d.created_at, d.updated_at",
            )
            .bind(&device_id)
            .bind(&project_id)
            .bind(&actor)
            .fetch_optional(&pool)
            .await
            .map_err(map_err)?;

            Ok(row.map(DeviceRecord::from))
        })
    }
}
