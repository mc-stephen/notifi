//! The persistence port for the projects slice.
//!
//! Implemented by `infra` (Postgres) and `testing` (in-memory fakes), same
//! object-safe boxed-future pattern as [`crate::ports::auth_store::AuthStore`].

use std::future::Future;
use std::pin::Pin;

use crate::domain::auth::entities::UserId;
use crate::ports::auth_store::StoreError;

/// Boxed future returned by every port method.
pub type BoxFut<'a, T> = Pin<Box<dyn Future<Output = T> + Send + 'a>>;

/// A project as shown to its owner/members.
#[derive(Debug, Clone)]
pub struct ProjectSummary {
    pub id: String,
    pub name: String,
    pub slug: String,
    pub description: Option<String>,
    /// The project-level environment gate: `development` | `production`.
    pub environment: String,
    /// New members must have TOTP 2FA enabled when true.
    pub require_2fa: bool,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

/// A project plus the caller's access context for member management.
#[derive(Debug, Clone)]
pub struct ProjectAccess {
    pub project: ProjectSummary,
    /// Creator's user id (`None` when the creator account was deleted).
    pub created_by: Option<String>,
    /// Caller's membership role, if any (creator need not be a member row).
    pub member_role: Option<String>,
}

pub trait ProjectsStore: Send + Sync {
    /// All projects the user owns or belongs to, oldest first.
    fn list_projects(&self, user_id: UserId)
    -> BoxFut<'_, Result<Vec<ProjectSummary>, StoreError>>;

    /// Creates a new project owned by the user.
    fn create_project(
        &self,
        user_id: UserId,
        name: &str,
        description: Option<&str>,
    ) -> BoxFut<'_, Result<ProjectSummary, StoreError>>;

    /// Updates the environment gate on a project the user owns or belongs
    /// to; `None` when the project doesn't exist or isn't visible to them
    /// (indistinguishable on purpose — no resource enumeration).
    fn set_project_environment(
        &self,
        user_id: UserId,
        project_id: &str,
        environment: &str,
    ) -> BoxFut<'_, Result<Option<ProjectSummary>, StoreError>>;

    /// The project plus the caller's access context; `None` when missing
    /// or invisible (indistinguishable on purpose).
    fn get_project_access(
        &self,
        user_id: UserId,
        project_id: &str,
    ) -> BoxFut<'_, Result<Option<ProjectAccess>, StoreError>>;

    /// Flips the 2FA requirement for new members. Returns the new flag,
    /// or `None` when the project doesn't exist.
    fn set_require_2fa(
        &self,
        project_id: &str,
        enabled: bool,
    ) -> BoxFut<'_, Result<Option<bool>, StoreError>>;

    /// The 2FA requirement flag for any project, regardless of actor.
    /// Needed at invite-accept time, when the actor is not on the team
    /// yet. `None` when the project doesn't exist.
    fn project_require_2fa(&self, project_id: &str)
    -> BoxFut<'_, Result<Option<bool>, StoreError>>;

    /// Adds a membership row. Returns false when the user is already a
    /// member (unique violation surfaces as `Conflict` instead).
    fn insert_member(
        &self,
        project_id: &str,
        user_id: UserId,
        role: &str,
    ) -> BoxFut<'_, Result<bool, StoreError>>;

    /// Team memberships for a project the caller belongs to, ordered by
    /// name. Visibility is enforced inside; outsiders get an empty list.
    fn list_members(
        &self,
        actor: UserId,
        project_id: &str,
    ) -> BoxFut<'_, Result<Vec<TeamMemberRecord>, StoreError>>;

    /// Creates a pending invite, superseding any prior pending invite for
    /// the same project + email. Returns the row id.
    fn create_invite(
        &self,
        project_id: &str,
        email: &str,
        role: &str,
        token_hash: &str,
        expires_at: chrono::DateTime<chrono::Utc>,
        created_by: UserId,
    ) -> BoxFut<'_, Result<String, StoreError>>;

    /// Finds a pending invite by token hash (ignores decided/expired rows;
    /// expiry is enforced by the service against `now`).
    fn find_pending_invite(
        &self,
        token_hash: &str,
    ) -> BoxFut<'_, Result<Option<InviteRecord>, StoreError>>;

    /// Finds any invite by token hash, decided or not (idempotent
    /// accept and preview-of-handled-link flows).
    fn find_invite_by_hash(
        &self,
        token_hash: &str,
    ) -> BoxFut<'_, Result<Option<InviteRecord>, StoreError>>;

    /// Marks an invite accepted or declined. Returns false when the row
    /// is already decided or missing.
    fn decide_invite(
        &self,
        invite_id: &str,
        accepted: bool,
    ) -> BoxFut<'_, Result<bool, StoreError>>;

    // === Admin-scoped reads (no actor visibility checks) =================

    /// All projects (newest first) with owner identity + member count.
    /// Returns the page plus the total matching count.
    fn list_all_projects(
        &self,
        search: Option<&str>,
        environment: Option<&str>,
        limit: i64,
        offset: i64,
    ) -> BoxFut<'_, Result<(Vec<AdminProjectRecord>, i64), StoreError>>;

    /// Any project by id, with owner identity + member count.
    fn get_any_project(
        &self,
        project_id: &str,
    ) -> BoxFut<'_, Result<Option<AdminProjectRecord>, StoreError>>;

    /// Members of a project with identity, for admin views.
    fn list_project_members(
        &self,
        project_id: &str,
    ) -> BoxFut<'_, Result<Vec<ProjectMemberRecord>, StoreError>>;
}

/// A project plus its owner's identity, for admin views. Owner fields are
/// `None` when the creator account was deleted.
#[derive(Debug, Clone)]
pub struct AdminProjectRecord {
    pub project: ProjectSummary,
    pub owner_id: Option<String>,
    pub owner_name: Option<String>,
    pub owner_email: Option<String>,
    pub member_count: i64,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

/// One project membership with the member's identity, for admin views.
#[derive(Debug, Clone)]
pub struct ProjectMemberRecord {
    pub user_id: String,
    pub name: Option<String>,
    pub email: Option<String>,
    pub role: String,
}

/// One team membership as seen by project members: identity, role,
/// 2FA standing, and last activity for the team table.
#[derive(Debug, Clone)]
pub struct TeamMemberRecord {
    pub user_id: String,
    pub name: String,
    pub email: String,
    pub role: String,
    pub has_2fa: bool,
    pub last_active_at: Option<chrono::DateTime<chrono::Utc>>,
}

/// A pending team invite (resolved by token hash).
#[derive(Debug, Clone)]
pub struct InviteRecord {
    pub id: String,
    pub project_id: String,
    pub project_name: String,
    pub email: String,
    pub role: String,
    pub token_hash: String,
    pub status: String,
    pub expires_at: chrono::DateTime<chrono::Utc>,
    pub created_by: Option<String>,
    pub created_at: chrono::DateTime<chrono::Utc>,
}
