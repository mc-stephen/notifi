//! In-memory fake implementations of the server's port traits for the
//! integration tests (shared via `tests/common/mod.rs`).
//!
//! Lives under `tests/` so the shipped library stays pure production code:
//! every fake implements the pub port/domain API only — no database needed.
#![allow(dead_code)] // fixtures are shared by ~20 test binaries; each binary uses only some

use std::cmp::Reverse;
use std::collections::HashMap;
use std::sync::{Arc, RwLock};

use chrono::{DateTime, Utc};

use server::kernel::Ulid;

use server::domain::audit::entities::AuditEntry;
use server::domain::auth::entities::{AuthToken, Session, TokenPurpose, User, UserId};
use server::domain::notifications::entities::{NotificationOrigin, NotificationType};
use server::ports::audit_store::{AuditFilters, AuditStore};
use server::ports::auth_store::{AuthStore, BoxFut, StoreError};
use server::ports::flow_store::FlowStore;
use server::ports::notifications_store::{
    BroadcastRecipient, BroadcastRecord, BroadcastStats, BroadcastStatus, NotificationRecord,
    NotificationsStore,
};
use server::ports::projects_store::{ProjectSummary, ProjectsStore, PurgeDueProject};
use server::ports::recipients_store::{RecipientRecord, RecipientsStore};
use server::ports::templates_store::{
    AttachmentInput, AttachmentRecord, TemplateRecord, TemplatesStore,
};
use server::ports::tickets_store::{
    AdminTicketMessageRecord, AdminTicketRecord, TicketMessageRecord, TicketRecord, TicketsStore,
};

/// Thread-safe in-memory store.
#[derive(Default)]
pub struct FakeAuthStore {
    users: RwLock<Vec<User>>,
    sessions: RwLock<Vec<Session>>,
    tokens: RwLock<Vec<AuthToken>>,
    /// Users treated as owning/belonging to a project (onboarding done).
    onboarded: RwLock<Vec<String>>,
    /// Projects seeded for a user: (owner_user_id, ProjectSummary).
    projects: RwLock<Vec<(String, ProjectSummary)>>,
    /// Project memberships: (project_id, user_id, role).
    members: RwLock<Vec<(String, String, String)>>,
    /// Team invites: InviteRecord by id.
    invites: RwLock<Vec<server::ports::projects_store::InviteRecord>>,
    /// Data-export downloads: (project_id, requested_by, when) — the rate
    /// limit counter and audit trail in one list.
    exports: RwLock<Vec<(String, String, chrono::DateTime<chrono::Utc>)>>,
}

impl FakeAuthStore {
    pub fn new() -> Self {
        Self::default()
    }

    /// Directly inserts a user (bypasses service validation).
    pub fn seed_user(&self, user: User) {
        self.users.write().unwrap().push(user);
    }

    /// Directly inserts a session (e.g. a pre-expired one for tests).
    pub fn seed_session(&self, session: Session) {
        self.sessions.write().unwrap().push(session);
    }

    /// Directly inserts a token (e.g. an expired one for tests).
    pub fn seed_token(&self, token: AuthToken) {
        self.tokens.write().unwrap().push(token);
    }

    /// Marks `user_id` as already owning/belonging to a project (e.g. an
    /// invited member) so onboarding is considered complete.
    pub fn seed_project(&self, user_id: server::domain::auth::entities::UserId) {
        self.onboarded.write().unwrap().push(user_id.to_string());
    }

    /// Seeds a real project summary for the given user so the projects
    /// API returns data in list/set-active endpoints.
    pub fn seed_project_summary(&self, user_id: UserId, project: ProjectSummary) {
        self.projects
            .write()
            .unwrap()
            .push((user_id.to_string(), project));
    }

    /// Seeds a membership row for member-management tests.
    pub fn seed_member(&self, project_id: &str, user_id: &str, role: &str) {
        self.members.write().unwrap().push((
            project_id.to_string(),
            user_id.to_string(),
            role.to_string(),
        ));
    }

    /// Test lever: tumbles one project straight into "purge due" (expired
    /// tombstone) without waiting out the grace window.
    pub fn mark_purge_due(&self, project_id: &str) {
        let now = chrono::Utc::now();
        for (_, p) in self.projects.write().unwrap().iter_mut() {
            if p.id == project_id {
                p.deleted_at = Some(now - chrono::Duration::hours(48));
                p.purge_after = Some(now - chrono::Duration::seconds(1));
            }
        }
    }

    /// Seeds a pending invite directly (expiry control for tests).
    pub fn seed_invite(
        &self,
        project_id: &str,
        email: &str,
        role: &str,
        token_hash: &str,
        expires_at: chrono::DateTime<chrono::Utc>,
    ) -> String {
        use server::ports::projects_store::InviteRecord;
        let id = server::kernel::Ulid::new().to_string();
        self.invites.write().unwrap().push(InviteRecord {
            id: id.clone(),
            project_id: project_id.to_string(),
            project_name: project_id.to_string(),
            email: email.to_string(),
            role: role.to_string(),
            token_hash: token_hash.to_string(),
            status: "pending".to_string(),
            expires_at,
            created_by: None,
            created_at: chrono::Utc::now(),
        });
        id
    }
}

fn lock_err<T>(_: std::sync::PoisonError<T>) -> StoreError {
    StoreError::Storage("poisoned lock".to_string())
}

impl AuthStore for FakeAuthStore {
    fn create_user(&self, user: &User) -> BoxFut<'_, Result<(), StoreError>> {
        let users = &self.users;
        let user = user.clone();
        Box::pin(async move {
            let mut users = users.write().map_err(lock_err)?;
            if users.iter().any(|u| u.email == user.email) {
                return Err(StoreError::Conflict);
            }
            users.push(user);
            Ok(())
        })
    }

    fn find_user_by_email(&self, email: &str) -> BoxFut<'_, Result<Option<User>, StoreError>> {
        let users = &self.users;
        let email = email.to_string();
        Box::pin(async move {
            Ok(users
                .read()
                .map_err(lock_err)?
                .iter()
                .find(|u| u.email.as_str() == email)
                .cloned())
        })
    }

    fn find_user_by_id(
        &self,
        id: server::domain::auth::entities::UserId,
    ) -> BoxFut<'_, Result<Option<User>, StoreError>> {
        let users = &self.users;
        Box::pin(async move {
            Ok(users
                .read()
                .map_err(lock_err)?
                .iter()
                .find(|u| u.id == id)
                .cloned())
        })
    }

    fn set_email_verified(
        &self,
        user_id: server::domain::auth::entities::UserId,
        verified_at: DateTime<Utc>,
    ) -> BoxFut<'_, Result<(), StoreError>> {
        let users = &self.users;
        Box::pin(async move {
            let mut users = users.write().map_err(lock_err)?;
            if let Some(user) = users.iter_mut().find(|u| u.id == user_id) {
                user.email_verified_at = Some(verified_at);
            }
            Ok(())
        })
    }

    fn update_password(
        &self,
        user_id: server::domain::auth::entities::UserId,
        password_hash: String,
    ) -> BoxFut<'_, Result<(), StoreError>> {
        let users = &self.users;
        Box::pin(async move {
            let mut users = users.write().map_err(lock_err)?;
            if let Some(user) = users.iter_mut().find(|u| u.id == user_id) {
                user.password_hash = password_hash;
            }
            Ok(())
        })
    }

    fn touch_last_login(
        &self,
        user_id: server::domain::auth::entities::UserId,
        at: DateTime<Utc>,
    ) -> BoxFut<'_, Result<(), StoreError>> {
        let users = &self.users;
        Box::pin(async move {
            let mut users = users.write().map_err(lock_err)?;
            if let Some(user) = users.iter_mut().find(|u| u.id == user_id) {
                user.last_login_at = Some(at);
            }
            Ok(())
        })
    }

    fn set_totp_secret(
        &self,
        user_id: server::domain::auth::entities::UserId,
        secret: String,
    ) -> BoxFut<'_, Result<(), StoreError>> {
        let users = &self.users;
        Box::pin(async move {
            let mut users = users.write().map_err(lock_err)?;
            if let Some(user) = users.iter_mut().find(|u| u.id == user_id) {
                user.totp_secret = Some(secret);
            }
            Ok(())
        })
    }

    fn enable_totp(
        &self,
        user_id: server::domain::auth::entities::UserId,
    ) -> BoxFut<'_, Result<(), StoreError>> {
        let users = &self.users;
        Box::pin(async move {
            let mut users = users.write().map_err(lock_err)?;
            if let Some(user) = users.iter_mut().find(|u| u.id == user_id) {
                user.totp_enabled = true;
            }
            Ok(())
        })
    }

    fn disable_totp(
        &self,
        user_id: server::domain::auth::entities::UserId,
    ) -> BoxFut<'_, Result<(), StoreError>> {
        let users = &self.users;
        Box::pin(async move {
            let mut users = users.write().map_err(lock_err)?;
            if let Some(user) = users.iter_mut().find(|u| u.id == user_id) {
                user.totp_enabled = false;
                user.totp_secret = None;
            }
            Ok(())
        })
    }

    fn list_users(
        &self,
        search: Option<&str>,
        status: Option<server::domain::auth::entities::UserStatus>,
        limit: i64,
        offset: i64,
    ) -> BoxFut<'_, Result<(Vec<User>, i64), StoreError>> {
        let search_owned = search.map(|s| s.to_lowercase());
        let users = &self.users;
        Box::pin(async move {
            let all = users.read().map_err(lock_err)?;
            let mut result: Vec<User> = all
                .iter()
                .filter(|u| {
                    if let Some(s) = status {
                        u.status == s
                    } else {
                        true
                    }
                })
                .filter(|u| {
                    if let Some(ref q) = search_owned {
                        u.name.to_lowercase().contains(q.as_str())
                            || u.email.as_str().to_lowercase().contains(q.as_str())
                    } else {
                        true
                    }
                })
                .cloned()
                .collect();
            result.sort_by(|a, b| {
                b.created_at
                    .cmp(&a.created_at)
                    .then(b.id.to_string().cmp(&a.id.to_string()))
            });
            let total = result.len() as i64;
            let page: Vec<User> = result
                .into_iter()
                .skip(offset.max(0) as usize)
                .take(limit.max(0) as usize)
                .collect();
            Ok((page, total))
        })
    }

    fn set_user_status(
        &self,
        user_id: server::domain::auth::entities::UserId,
        status: server::domain::auth::entities::UserStatus,
    ) -> BoxFut<'_, Result<bool, StoreError>> {
        let users = &self.users;
        Box::pin(async move {
            let mut users = users.write().map_err(lock_err)?;
            if let Some(user) = users.iter_mut().find(|u| u.id == user_id) {
                user.status = status;
                return Ok(true);
            }
            Ok(false)
        })
    }

    fn list_all_user_ids(
        &self,
        status: Option<server::domain::auth::entities::UserStatus>,
    ) -> BoxFut<'_, Result<Vec<server::domain::auth::entities::UserId>, StoreError>> {
        let users = &self.users;
        Box::pin(async move {
            let mut ids: Vec<server::domain::auth::entities::UserId> = users
                .read()
                .map_err(lock_err)?
                .iter()
                .filter(|u| {
                    if let Some(s) = status {
                        u.status == s
                    } else {
                        true
                    }
                })
                .map(|u| u.id)
                .collect();
            ids.sort();
            Ok(ids)
        })
    }

    fn create_session(&self, session: &Session) -> BoxFut<'_, Result<(), StoreError>> {
        let sessions = &self.sessions;
        let session = session.clone();
        Box::pin(async move {
            sessions.write().map_err(lock_err)?.push(session);
            Ok(())
        })
    }

    fn find_session_by_hash(
        &self,
        token_hash: &str,
    ) -> BoxFut<'_, Result<Option<Session>, StoreError>> {
        let sessions = &self.sessions;
        let token_hash = token_hash.to_string();
        Box::pin(async move {
            Ok(sessions
                .read()
                .map_err(lock_err)?
                .iter()
                .find(|s| s.token_hash == token_hash)
                .cloned())
        })
    }

    fn revoke_session(
        &self,
        id: server::domain::auth::entities::SessionId,
    ) -> BoxFut<'_, Result<(), StoreError>> {
        let sessions = &self.sessions;
        Box::pin(async move {
            let mut sessions = sessions.write().map_err(lock_err)?;
            if let Some(session) = sessions.iter_mut().find(|s| s.id == id) {
                session.revoked_at = Some(Utc::now());
            }
            Ok(())
        })
    }

    fn revoke_all_sessions_for_user(
        &self,
        user_id: server::domain::auth::entities::UserId,
    ) -> BoxFut<'_, Result<(), StoreError>> {
        let sessions = &self.sessions;
        Box::pin(async move {
            let mut sessions = sessions.write().map_err(lock_err)?;
            for session in sessions.iter_mut().filter(|s| s.user_id == user_id) {
                if session.revoked_at.is_none() {
                    session.revoked_at = Some(Utc::now());
                }
            }
            Ok(())
        })
    }

    fn create_token(&self, token: &AuthToken) -> BoxFut<'_, Result<(), StoreError>> {
        let tokens = &self.tokens;
        let token = token.clone();
        Box::pin(async move {
            tokens.write().map_err(lock_err)?.push(token);
            Ok(())
        })
    }

    fn find_token_by_hash(
        &self,
        token_hash: &str,
        purpose: TokenPurpose,
    ) -> BoxFut<'_, Result<Option<AuthToken>, StoreError>> {
        let tokens = &self.tokens;
        let token_hash = token_hash.to_string();
        Box::pin(async move {
            Ok(tokens
                .read()
                .map_err(lock_err)?
                .iter()
                .find(|t| t.token_hash == token_hash && t.purpose == purpose)
                .cloned())
        })
    }

    fn consume_token(
        &self,
        id: server::domain::auth::entities::AuthTokenId,
    ) -> BoxFut<'_, Result<(), StoreError>> {
        let tokens = &self.tokens;
        Box::pin(async move {
            let mut tokens = tokens.write().map_err(lock_err)?;
            if let Some(token) = tokens.iter_mut().find(|t| t.id == id) {
                token.consumed_at = Some(Utc::now());
            }
            Ok(())
        })
    }

    fn consume_tokens_for_user(
        &self,
        user_id: server::domain::auth::entities::UserId,
        purpose: TokenPurpose,
    ) -> BoxFut<'_, Result<(), StoreError>> {
        let tokens = &self.tokens;
        Box::pin(async move {
            let mut tokens = tokens.write().map_err(lock_err)?;
            for token in tokens
                .iter_mut()
                .filter(|t| t.user_id == user_id && t.purpose == purpose)
            {
                if token.consumed_at.is_none() {
                    token.consumed_at = Some(Utc::now());
                }
            }
            Ok(())
        })
    }

    fn find_user_by_oauth(
        &self,
        provider: &str,
        subject: &str,
    ) -> BoxFut<'_, Result<Option<User>, StoreError>> {
        let users = &self.users;
        let provider = provider.to_string();
        let subject = subject.to_string();
        Box::pin(async move {
            Ok(users
                .read()
                .map_err(lock_err)?
                .iter()
                .find(|u| {
                    u.oauth_provider.as_deref() == Some(provider.as_str())
                        && u.oauth_subject.as_deref() == Some(subject.as_str())
                })
                .cloned())
        })
    }

    fn link_oauth_to_user(
        &self,
        user_id: server::domain::auth::entities::UserId,
        provider: &str,
        subject: &str,
    ) -> BoxFut<'_, Result<(), StoreError>> {
        let users = &self.users;
        let provider = provider.to_string();
        let subject = subject.to_string();
        Box::pin(async move {
            let mut users = users.write().map_err(lock_err)?;
            if let Some(user) = users.iter_mut().find(|u| u.id == user_id) {
                user.oauth_provider = Some(provider);
                user.oauth_subject = Some(subject);
            }
            Ok(())
        })
    }

    fn has_project(
        &self,
        user_id: server::domain::auth::entities::UserId,
    ) -> BoxFut<'_, Result<bool, StoreError>> {
        let onboarded = &self.onboarded;
        Box::pin(async move {
            Ok(onboarded
                .read()
                .map_err(lock_err)?
                .iter()
                .any(|id| *id == user_id.to_string()))
        })
    }

    fn complete_onboarding(
        &self,
        user_id: server::domain::auth::entities::UserId,
        input: server::ports::auth_store::OnboardingInput,
    ) -> BoxFut<'_, Result<(), StoreError>> {
        let onboarded = &self.onboarded;
        let id = user_id.to_string();
        let name = input.project_name;
        let description = input.project_description;
        Box::pin(async move {
            {
                let mut list = onboarded.write().map_err(lock_err)?;
                if !list.contains(&id) {
                    list.push(id);
                }
            }
            // Delegate project creation to the ProjectsStore implementation —
            // single source of truth for the fake's project-seeding logic.
            server::ports::projects_store::ProjectsStore::create_project(
                self,
                user_id,
                &name,
                description.as_deref(),
            )
            .await?;
            Ok(())
        })
    }
}

impl ProjectsStore for FakeAuthStore {
    fn list_projects(
        &self,
        user_id: UserId,
    ) -> BoxFut<'_, Result<Vec<ProjectSummary>, StoreError>> {
        let projects = &self.projects;
        let members = &self.members;
        let user_id_str = user_id.to_string();
        Box::pin(async move {
            // The PG store's rule: creator OR active member sees the
            // project — tombstones included (the deletion window stays
            // downloadable + cancellable).
            let members = members.read().map_err(lock_err)?;
            Ok(projects
                .read()
                .map_err(lock_err)?
                .iter()
                .filter(|(owner, p)| {
                    *owner == user_id_str
                        || members
                            .iter()
                            .any(|(pid, uid, _)| *pid == p.id && uid == &user_id_str)
                })
                .map(|(_, p)| p.clone())
                .collect())
        })
    }

    fn set_project_active(
        &self,
        user_id: UserId,
        project_id: &str,
        active: bool,
    ) -> BoxFut<'_, Result<Option<ProjectSummary>, StoreError>> {
        let projects = &self.projects;
        let project_id = project_id.to_string();
        let user_id_str = user_id.to_string();
        Box::pin(async move {
            let mut projects = projects.write().map_err(lock_err)?;
            Ok(projects
                .iter_mut()
                .find(|(owner, p)| *owner == user_id_str && p.id == project_id)
                .map(|(_, p)| {
                    p.active = active;
                    p.clone()
                }))
        })
    }

    fn get_project_access(
        &self,
        user_id: UserId,
        project_id: &str,
    ) -> BoxFut<'_, Result<Option<server::ports::projects_store::ProjectAccess>, StoreError>> {
        use server::ports::projects_store::ProjectAccess;
        let project_id = project_id.to_string();
        let user_id_str = user_id.to_string();
        let projects = self.projects.read().unwrap().clone();
        let members = self.members.read().unwrap().clone();
        Box::pin(async move {
            Ok(projects
                .into_iter()
                .find(|(_, p)| p.id == project_id && p.deleted_at.is_none())
                .and_then(|(owner, p)| {
                    let is_owner = owner == user_id_str;
                    let member_role = members
                        .iter()
                        .find(|(pid, uid, _)| *pid == project_id && *uid == user_id_str)
                        .map(|(_, _, role)| role.clone());
                    if !is_owner && member_role.is_none() {
                        return None;
                    }
                    Some(ProjectAccess {
                        project: p,
                        created_by: Some(owner),
                        member_role,
                    })
                }))
        })
    }

    fn request_project_deletion(
        &self,
        user_id: UserId,
        project_id: &str,
        purge_after: chrono::DateTime<chrono::Utc>,
    ) -> BoxFut<'_, Result<Option<ProjectSummary>, StoreError>> {
        let projects = &self.projects;
        let members = &self.members;
        let project_id = project_id.to_string();
        let user_id_str = user_id.to_string();
        Box::pin(async move {
            let mut projects = projects.write().map_err(lock_err)?;
            let members = members.read().map_err(lock_err)?;
            let actor_role = members
                .iter()
                .find(|(pid, uid, _)| *pid == project_id && *uid == user_id_str)
                .map(|(_, _, role)| role.clone());
            let boss = matches!(actor_role.as_deref(), Some("owner") | Some("admin"));
            Ok(projects
                .iter_mut()
                .find(|(owner, p)| {
                    p.id == project_id && p.deleted_at.is_none() && (*owner == user_id_str || boss)
                })
                .map(|(_, p)| {
                    p.deleted_at = Some(chrono::Utc::now());
                    p.purge_after = Some(purge_after);
                    p.clone()
                }))
        })
    }

    fn cancel_project_deletion(
        &self,
        user_id: UserId,
        project_id: &str,
    ) -> BoxFut<'_, Result<Option<ProjectSummary>, StoreError>> {
        let projects = &self.projects;
        let members = &self.members;
        let project_id = project_id.to_string();
        let user_id_str = user_id.to_string();
        Box::pin(async move {
            let mut projects = projects.write().map_err(lock_err)?;
            let members = members.read().map_err(lock_err)?;
            let actor_role = members
                .iter()
                .find(|(pid, uid, _)| *pid == project_id && *uid == user_id_str)
                .map(|(_, _, role)| role.clone());
            let boss = matches!(actor_role.as_deref(), Some("owner") | Some("admin"));
            Ok(projects
                .iter_mut()
                .find(|(owner, p)| {
                    p.id == project_id && p.deleted_at.is_some() && (*owner == user_id_str || boss)
                })
                .map(|(_, p)| {
                    p.deleted_at = None;
                    p.purge_after = None;
                    p.clone()
                }))
        })
    }

    fn list_projects_due_for_purge(
        &self,
        now: chrono::DateTime<chrono::Utc>,
        limit: i64,
    ) -> BoxFut<'_, Result<Vec<PurgeDueProject>, StoreError>> {
        let projects = &self.projects;
        Box::pin(async move {
            let projects = projects.read().map_err(lock_err)?;
            Ok(projects
                .iter()
                .filter(|(_, p)| {
                    p.deleted_at.is_some() && p.purge_after.is_some_and(|after| after <= now)
                })
                .take(limit as usize)
                .map(|(_, p)| PurgeDueProject {
                    id: p.id.clone(),
                    slug: p.slug.clone(),
                })
                .collect())
        })
    }

    fn delete_project(&self, project_id: &str) -> BoxFut<'_, Result<bool, StoreError>> {
        let projects = &self.projects;
        let project_id = project_id.to_string();
        Box::pin(async move {
            let mut projects = projects.write().map_err(lock_err)?;
            let before = projects.len();
            projects.retain(|(_, p)| p.id != project_id);
            Ok(projects.len() < before)
        })
    }

    fn record_project_export(
        &self,
        project_id: &str,
        requested_by: &UserId,
    ) -> BoxFut<'_, Result<(), StoreError>> {
        let exports = &self.exports;
        let project_id = project_id.to_string();
        let requested_by = requested_by.to_string();
        Box::pin(async move {
            exports
                .write()
                .map_err(lock_err)?
                .push((project_id, requested_by, chrono::Utc::now()));
            Ok(())
        })
    }

    fn count_project_export(
        &self,
        project_id: &str,
        since: chrono::DateTime<chrono::Utc>,
    ) -> BoxFut<'_, Result<i64, StoreError>> {
        let exports = &self.exports;
        let project_id = project_id.to_string();
        Box::pin(async move {
            let exports = exports.read().map_err(lock_err)?;
            Ok(exports
                .iter()
                .filter(|(pid, _, at)| pid == &project_id && *at > since)
                .count() as i64)
        })
    }

    fn set_require_2fa(
        &self,
        project_id: &str,
        enabled: bool,
    ) -> BoxFut<'_, Result<Option<bool>, StoreError>> {
        let project_id = project_id.to_string();
        let projects = &self.projects;
        Box::pin(async move {
            let mut projects = projects.write().map_err(lock_err)?;
            Ok(projects
                .iter_mut()
                .find(|(_, p)| p.id == project_id)
                .map(|(_, p)| {
                    p.require_2fa = enabled;
                    enabled
                }))
        })
    }

    fn project_require_2fa(
        &self,
        project_id: &str,
    ) -> BoxFut<'_, Result<Option<bool>, StoreError>> {
        let project_id = project_id.to_string();
        let found = self
            .projects
            .read()
            .unwrap()
            .iter()
            .find(|(_, p)| p.id == project_id)
            .map(|(_, p)| p.require_2fa);
        Box::pin(async move { Ok(found) })
    }

    fn insert_member(
        &self,
        project_id: &str,
        user_id: UserId,
        role: &str,
    ) -> BoxFut<'_, Result<bool, StoreError>> {
        let project_id = project_id.to_string();
        let member_id = user_id.to_string();
        let role = role.to_string();
        let members = &self.members;
        Box::pin(async move {
            let mut members = members.write().map_err(lock_err)?;
            if members
                .iter()
                .any(|(pid, uid, _)| *pid == project_id && *uid == member_id)
            {
                return Ok(false);
            }
            members.push((project_id, member_id, role));
            Ok(true)
        })
    }

    fn create_invite(
        &self,
        project_id: &str,
        email: &str,
        role: &str,
        token_hash: &str,
        expires_at: chrono::DateTime<chrono::Utc>,
        created_by: UserId,
    ) -> BoxFut<'_, Result<String, StoreError>> {
        use server::ports::projects_store::InviteRecord;
        let project_id = project_id.to_string();
        let email = email.to_string();
        let role = role.to_string();
        let token_hash = token_hash.to_string();
        let created_by = created_by.to_string();
        let invites = &self.invites;
        let projects = self.projects.read().unwrap().clone();
        Box::pin(async move {
            let mut invites = invites.write().map_err(lock_err)?;
            // Fresh invite supersedes prior pending ones for the pair.
            for invite in invites
                .iter_mut()
                .filter(|i| i.project_id == project_id && i.email == email && i.status == "pending")
            {
                invite.status = "declined".to_string();
            }
            let project_name = projects
                .iter()
                .find(|(_, p)| p.id == project_id)
                .map(|(_, p)| p.name.clone())
                .unwrap_or_else(|| project_id.clone());
            let now = chrono::Utc::now();
            let id = server::kernel::Ulid::new().to_string();
            invites.push(InviteRecord {
                id: id.clone(),
                project_id,
                project_name,
                email,
                role,
                token_hash,
                status: "pending".to_string(),
                expires_at,
                created_by: Some(created_by),
                created_at: now,
            });
            Ok(id)
        })
    }

    fn find_pending_invite(
        &self,
        token_hash: &str,
    ) -> BoxFut<'_, Result<Option<server::ports::projects_store::InviteRecord>, StoreError>> {
        let token_hash = token_hash.to_string();
        let found = self
            .invites
            .read()
            .unwrap()
            .iter()
            .find(|i| i.token_hash == token_hash && i.status == "pending")
            .cloned();
        Box::pin(async move { Ok(found) })
    }

    fn find_invite_by_hash(
        &self,
        token_hash: &str,
    ) -> BoxFut<'_, Result<Option<server::ports::projects_store::InviteRecord>, StoreError>> {
        let token_hash = token_hash.to_string();
        let found = self
            .invites
            .read()
            .unwrap()
            .iter()
            .find(|i| i.token_hash == token_hash)
            .cloned();
        Box::pin(async move { Ok(found) })
    }

    fn decide_invite(
        &self,
        invite_id: &str,
        accepted: bool,
    ) -> BoxFut<'_, Result<bool, StoreError>> {
        let invite_id = invite_id.to_string();
        let invites = &self.invites;
        Box::pin(async move {
            let mut invites = invites.write().map_err(lock_err)?;
            Ok(invites
                .iter_mut()
                .find(|i| i.id == invite_id && i.status == "pending")
                .map(|i| {
                    i.status = if accepted {
                        "accepted".to_string()
                    } else {
                        "declined".to_string()
                    };
                    true
                })
                .unwrap_or(false))
        })
    }

    fn list_members(
        &self,
        user_id: UserId,
        project_id: &str,
    ) -> BoxFut<'_, Result<Vec<server::ports::projects_store::TeamMemberRecord>, StoreError>> {
        use server::ports::projects_store::TeamMemberRecord;
        let project_id = project_id.to_string();
        let user_id_str = user_id.to_string();
        let projects = self.projects.read().unwrap().clone();
        let members = self.members.read().unwrap().clone();
        let users = self.users.read().unwrap().clone();
        Box::pin(async move {
            let Some((owner, _)) = projects.iter().find(|(_, p)| p.id == project_id) else {
                return Ok(Vec::new());
            };
            let visible = *owner == user_id_str
                || members
                    .iter()
                    .any(|(pid, uid, _)| *pid == project_id && *uid == user_id_str);
            if !visible {
                return Ok(Vec::new());
            }
            let mut out: Vec<TeamMemberRecord> = Vec::new();
            // Creator first as implicit owner.
            if let Some(creator) = users.iter().find(|u| u.id.to_string() == *owner) {
                out.push(TeamMemberRecord {
                    user_id: owner.clone(),
                    name: creator.name.clone(),
                    email: creator.email.as_str().to_string(),
                    role: "owner".to_string(),
                    has_2fa: creator.totp_enabled,
                    last_active_at: creator.last_login_at,
                });
            }
            let mut rest: Vec<TeamMemberRecord> = members
                .iter()
                .filter(|(pid, uid, _)| *pid == project_id && *uid != *owner)
                .filter_map(|(_, uid, role)| {
                    users
                        .iter()
                        .find(|u| u.id.to_string() == *uid)
                        .map(|u| TeamMemberRecord {
                            user_id: uid.clone(),
                            name: u.name.clone(),
                            email: u.email.as_str().to_string(),
                            role: role.clone(),
                            has_2fa: u.totp_enabled,
                            last_active_at: u.last_login_at,
                        })
                })
                .collect();
            rest.sort_by(|a, b| a.name.cmp(&b.name));
            out.extend(rest);
            Ok(out)
        })
    }

    fn create_project(
        &self,
        user_id: UserId,
        name: &str,
        description: Option<&str>,
    ) -> BoxFut<'_, Result<ProjectSummary, StoreError>> {
        let name = name.trim().to_string();
        let description = description.map(str::to_owned);
        let user_id_str = user_id.to_string();
        let projects = &self.projects;
        Box::pin(async move {
            let mut projects = projects.write().map_err(lock_err)?;
            // Same slug rule as the PG store (kernel is the single source
            // of truth), matching how project-folder keys get derived.
            let slug = server::kernel::config::project_slug(&name);
            let slug = if projects.iter().any(|(_, p)| p.slug == slug) {
                format!("{slug}-{}", projects.len() + 1)
            } else {
                slug
            };
            let record = ProjectSummary {
                id: server::kernel::Ulid::new().to_string(),
                name,
                slug,
                description,
                active: false,
                require_2fa: false,
                created_at: chrono::Utc::now(),
                deleted_at: None,
                purge_after: None,
            };
            projects.push((user_id_str, record.clone()));
            Ok(record)
        })
    }

    // === Admin-scoped reads (no actor visibility checks) =================

    fn list_all_projects(
        &self,
        search: Option<&str>,
        limit: i64,
        offset: i64,
    ) -> BoxFut<'_, Result<(Vec<server::ports::projects_store::AdminProjectRecord>, i64), StoreError>>
    {
        let search_owned = search.map(|s| s.to_lowercase());
        let projects = self.projects.read().unwrap().clone();
        let users = self.users.read().unwrap().clone();
        Box::pin(async move {
            let mut result: Vec<server::ports::projects_store::AdminProjectRecord> = projects
                .iter()
                .filter(|(_, p)| {
                    if let Some(ref q) = search_owned {
                        p.name.to_lowercase().contains(q.as_str())
                            || p.slug.to_lowercase().contains(q.as_str())
                    } else {
                        true
                    }
                })
                .map(|(owner, p)| {
                    let owner_user = users.iter().find(|u| u.id.to_string() == *owner);
                    server::ports::projects_store::AdminProjectRecord {
                        project: p.clone(),
                        owner_id: Some(owner.clone()),
                        owner_name: owner_user.map(|u| u.name.clone()),
                        owner_email: owner_user.map(|u| u.email.to_string()),
                        member_count: 0,
                        updated_at: p.created_at,
                    }
                })
                .collect();
            result.sort_by(|a, b| {
                b.project
                    .created_at
                    .cmp(&a.project.created_at)
                    .then(b.project.id.cmp(&a.project.id))
            });
            let total = result.len() as i64;
            let page: Vec<server::ports::projects_store::AdminProjectRecord> = result
                .into_iter()
                .skip(offset.max(0) as usize)
                .take(limit.max(0) as usize)
                .collect();
            Ok((page, total))
        })
    }

    fn get_any_project(
        &self,
        project_id: &str,
    ) -> BoxFut<'_, Result<Option<server::ports::projects_store::AdminProjectRecord>, StoreError>>
    {
        let project_id = project_id.to_string();
        let projects = self.projects.read().unwrap().clone();
        let users = self.users.read().unwrap().clone();
        Box::pin(async move {
            Ok(projects
                .iter()
                .find(|(_, p)| p.id == project_id)
                .map(|(owner, p)| {
                    let owner_user = users.iter().find(|u| u.id.to_string() == *owner);
                    server::ports::projects_store::AdminProjectRecord {
                        project: p.clone(),
                        owner_id: Some(owner.clone()),
                        owner_name: owner_user.map(|u| u.name.clone()),
                        owner_email: owner_user.map(|u| u.email.to_string()),
                        member_count: 0,
                        updated_at: p.created_at,
                    }
                }))
        })
    }

    fn list_project_members(
        &self,
        _project_id: &str,
    ) -> BoxFut<'_, Result<Vec<server::ports::projects_store::ProjectMemberRecord>, StoreError>>
    {
        // The fake tracks ownership only, not membership rows.
        Box::pin(async move { Ok(Vec::new()) })
    }
}

/// In-memory, append-only [`AuditStore`] for tests.
#[derive(Default)]
pub struct FakeAuditStore {
    entries: RwLock<Vec<AuditEntry>>,
}

impl FakeAuditStore {
    pub fn new() -> Self {
        Self::default()
    }

    /// Directly seeds an audit entry (bypasses the service).
    pub fn seed(&self, entry: AuditEntry) {
        self.entries.write().unwrap().push(entry);
    }

    /// The current contents, newest-first as inserted.
    pub fn all(&self) -> Vec<AuditEntry> {
        let mut all = self.entries.read().unwrap().clone();
        all.reverse();
        all
    }
}

impl AuditStore for FakeAuditStore {
    fn record(&self, entry: &AuditEntry) -> BoxFut<'_, Result<(), StoreError>> {
        let entries = &self.entries;
        let entry = entry.clone();
        Box::pin(async move {
            entries.write().map_err(lock_err)?.push(entry);
            Ok(())
        })
    }

    fn list(
        &self,
        user_id: &str,
        _filters: AuditFilters<'_>,
        limit: i64,
        _before_id: Option<&str>,
    ) -> BoxFut<'_, Result<Vec<AuditEntry>, StoreError>> {
        let entries = self.entries.read().unwrap().clone();
        let user_id = user_id.to_string();
        Box::pin(async move {
            let mut mine: Vec<_> = entries
                .into_iter()
                .filter(|e| {
                    e.user_id.as_deref() == Some(user_id.as_str())
                        || e.project_id.as_deref().is_some()
                })
                .collect();
            mine.reverse();
            mine.truncate(limit.max(0) as usize);
            Ok(mine)
        })
    }

    fn list_all(
        &self,
        filters: server::ports::audit_store::AdminAuditFilters<'_>,
        limit: i64,
        offset: i64,
    ) -> BoxFut<'_, Result<(Vec<AuditEntry>, i64), StoreError>> {
        let entries = self.entries.read().unwrap().clone();
        let event_type = filters.event_type.map(str::to_owned);
        let project_id = filters.project_id.map(str::to_owned);
        let actor_type = filters.actor_type.map(str::to_owned);
        let actor_id = filters.actor_id.map(str::to_owned);
        let search = filters.search.map(|s| s.to_lowercase());
        Box::pin(async move {
            let mut result: Vec<AuditEntry> = entries
                .into_iter()
                .filter(|e| {
                    if let Some(ref t) = event_type
                        && e.event_type != *t
                    {
                        return false;
                    }
                    if let Some(ref p) = project_id
                        && e.project_id.as_deref() != Some(p.as_str())
                    {
                        return false;
                    }
                    if let Some(ref t) = actor_type
                        && e.actor_type.as_str() != t.as_str()
                    {
                        return false;
                    }
                    if let Some(ref a) = actor_id
                        && e.user_id.as_deref() != Some(a.as_str())
                        && e.admin_id.as_deref() != Some(a.as_str())
                    {
                        return false;
                    }
                    if let Some(ref q) = search {
                        let haystack = format!(
                            "{} {} {}",
                            e.event_type,
                            e.message,
                            e.actor_name.as_deref().unwrap_or("")
                        )
                        .to_lowercase();
                        if !haystack.contains(q.as_str()) {
                            return false;
                        }
                    }
                    true
                })
                .collect();
            result.sort_by(|a, b| b.occurred_at.cmp(&a.occurred_at).then(b.id.cmp(&a.id)));
            let total = result.len() as i64;
            let page: Vec<AuditEntry> = result
                .into_iter()
                .skip(offset.max(0) as usize)
                .take(limit.max(0) as usize)
                .collect();
            Ok((page, total))
        })
    }
}

/// In-memory [`RecipientsStore`] for tests.
///
/// Visibility mirrors the Postgres store: an actor can only see/create/
/// delete recipients in a project they own or belong to, tracked via
/// [`FakeRecipientsStore::seed_visible`].
#[derive(Default)]
pub struct FakeRecipientsStore {
    recipients: RwLock<Vec<RecipientRecord>>,
    /// (user_id, project_id) pairs the actor may access.
    visible: RwLock<Vec<(String, String)>>,
}

impl FakeRecipientsStore {
    pub fn new() -> Self {
        Self::default()
    }

    /// Grants `user_id` access to `project_id` (ownership/membership).
    pub fn seed_visible(&self, user_id: &str, project_id: &str) {
        self.visible
            .write()
            .unwrap()
            .push((user_id.to_string(), project_id.to_string()));
    }

    /// The full contents (for assertions).
    pub fn all(&self) -> Vec<RecipientRecord> {
        let mut all = self.recipients.read().unwrap().clone();
        all.sort_by_key(|e| Reverse(e.created_at));
        all
    }
    fn is_visible(&self, user_id: &str, project_id: &str) -> bool {
        self.visible
            .read()
            .unwrap()
            .iter()
            .any(|(u, p)| u == user_id && p == project_id)
    }
}

impl RecipientsStore for FakeRecipientsStore {
    fn create(
        &self,
        actor: server::domain::auth::entities::UserId,
        project_id: &str,
        user_id: &str,
        name: &str,
        contacts: serde_json::Value,
    ) -> BoxFut<'_, Result<RecipientRecord, StoreError>> {
        let recipients = &self.recipients;
        let actor = actor.to_string();
        let project_id = project_id.to_string();
        let user_id = user_id.to_string();
        let name = name.to_string();
        let visible = self.visible.read().unwrap().clone();
        Box::pin(async move {
            if !visible.iter().any(|(u, p)| *u == actor && *p == project_id) {
                return Err(StoreError::Storage(
                    "project not found or not visible".to_string(),
                ));
            }
            let mut list = recipients.write().map_err(lock_err)?;
            if list
                .iter()
                .any(|r| r.project_id == project_id && r.user_id == user_id)
            {
                return Err(StoreError::Conflict);
            }
            let record = RecipientRecord {
                id: format!("rcp_{}", list.len() + 1),
                project_id: project_id.clone(),
                user_id: user_id.clone(),
                name,
                contacts,
                created_at: chrono::Utc::now(),
            };
            list.push(record.clone());
            Ok(record)
        })
    }

    fn list(
        &self,
        actor: server::domain::auth::entities::UserId,
        project_id: &str,
        _search: Option<&str>,
        limit: i64,
        before: Option<&str>,
    ) -> BoxFut<'_, Result<Vec<RecipientRecord>, StoreError>> {
        let actor = actor.to_string();
        let project_id = project_id.to_string();
        let before = before.map(str::to_owned);
        let recipients = self.recipients.read().unwrap().clone();
        Box::pin(async move {
            if !self.is_visible(&actor, &project_id) {
                return Ok(Vec::new());
            }
            let mut list: Vec<_> = recipients
                .into_iter()
                .filter(|r| r.project_id == project_id)
                .filter(|r| before.as_deref().is_none_or(|b| r.id.as_str() < b))
                .collect();
            list.sort_by_key(|e| Reverse(e.created_at));
            list.truncate(limit.max(0) as usize);
            Ok(list)
        })
    }

    fn list_by_project(
        &self,
        project_id: &str,
        limit: i64,
    ) -> BoxFut<'_, Result<Vec<RecipientRecord>, StoreError>> {
        let project_id = project_id.to_string();
        let recipients = self.recipients.read().unwrap().clone();
        Box::pin(async move {
            let mut list: Vec<_> = recipients
                .into_iter()
                .filter(|r| r.project_id == project_id)
                .collect();
            list.sort_by_key(|e| Reverse(e.created_at));
            list.truncate(limit.max(0) as usize);
            Ok(list)
        })
    }

    fn get(
        &self,
        actor: server::domain::auth::entities::UserId,
        project_id: &str,
        recipient_id: &str,
    ) -> BoxFut<'_, Result<Option<RecipientRecord>, StoreError>> {
        let actor = actor.to_string();
        let project_id = project_id.to_string();
        let recipient_id = recipient_id.to_string();
        let recipients = self.recipients.read().unwrap().clone();
        Box::pin(async move {
            if !self.is_visible(&actor, &project_id) {
                return Ok(None);
            }
            Ok(recipients
                .into_iter()
                .find(|r| r.id == recipient_id && r.project_id == project_id))
        })
    }

    fn update(
        &self,
        actor: server::domain::auth::entities::UserId,
        project_id: &str,
        recipient_id: &str,
        name: &str,
        contacts: serde_json::Value,
    ) -> BoxFut<'_, Result<Option<RecipientRecord>, StoreError>> {
        let actor = actor.to_string();
        let project_id = project_id.to_string();
        let recipient_id = recipient_id.to_string();
        let name = name.to_string();
        let recipients = &self.recipients;
        Box::pin(async move {
            if !self.is_visible(&actor, &project_id) {
                return Ok(None);
            }
            let mut list = recipients.write().map_err(lock_err)?;
            let result = list
                .iter_mut()
                .find(|r| r.id == recipient_id && r.project_id == project_id);
            if let Some(record) = result {
                record.name = name.clone();
                record.contacts = contacts.clone();
                Ok(Some(record.clone()))
            } else {
                Ok(None)
            }
        })
    }

    fn remove(
        &self,
        actor: server::domain::auth::entities::UserId,
        project_id: &str,
        recipient_id: &str,
    ) -> BoxFut<'_, Result<bool, StoreError>> {
        let actor = actor.to_string();
        let project_id = project_id.to_string();
        let recipient_id = recipient_id.to_string();
        let recipients = &self.recipients;
        Box::pin(async move {
            if !self.is_visible(&actor, &project_id) {
                return Ok(false);
            }
            let mut list = recipients.write().map_err(lock_err)?;
            let before = list.len();
            list.retain(|r| !(r.id == recipient_id && r.project_id == project_id));
            Ok(list.len() != before)
        })
    }
}

/// In-memory [`TemplatesStore`] for tests.
///
/// Visibility mirrors the Postgres store: an actor can only see/create/update
/// templates in a project they own or belong to, tracked via
/// [`FakeTemplatesStore::seed_visible`].
#[derive(Default)]
pub struct FakeTemplatesStore {
    templates: RwLock<Vec<TemplateRecord>>,
    /// (user_id, project_id) pairs the actor may access.
    visible: RwLock<Vec<(String, String)>>,
}

impl FakeTemplatesStore {
    pub fn new() -> Self {
        Self::default()
    }

    /// Grants `user_id` access to `project_id` (ownership/membership).
    pub fn seed_visible(&self, user_id: &str, project_id: &str) {
        self.visible
            .write()
            .unwrap()
            .push((user_id.to_string(), project_id.to_string()));
    }

    /// The full contents (for assertions), newest first.
    pub fn all(&self) -> Vec<TemplateRecord> {
        let mut all = self.templates.read().unwrap().clone();
        all.sort_by_key(|e| Reverse(e.created_at));
        all
    }

    fn is_visible(&self, user_id: &str, project_id: &str) -> bool {
        self.visible
            .read()
            .unwrap()
            .iter()
            .any(|(u, p)| u == user_id && p == project_id)
    }
}

fn attachments_from_inputs(inputs: Vec<AttachmentInput>) -> Vec<AttachmentRecord> {
    inputs
        .into_iter()
        .map(|a| AttachmentRecord {
            id: format!("att_{}", Ulid::new()),
            name: a.name,
            mime_type: a.mime_type,
            size_bytes: a.size_bytes,
            url: a.url,
        })
        .collect()
}

impl TemplatesStore for FakeTemplatesStore {
    #[allow(clippy::too_many_arguments)]
    fn create(
        &self,
        actor: server::domain::auth::entities::UserId,
        project_id: &str,
        name: &str,
        description: Option<&str>,
        channel: &str,
        content: serde_json::Value,
        attachments: Vec<AttachmentInput>,
    ) -> BoxFut<'_, Result<TemplateRecord, StoreError>> {
        let templates = &self.templates;
        let actor = actor.to_string();
        let project_id = project_id.to_string();
        let name = name.to_string();
        let description = description.map(str::to_owned);
        let channel = channel.to_string();
        let visible = self.visible.read().unwrap().clone();
        Box::pin(async move {
            if !visible.iter().any(|(u, p)| *u == actor && *p == project_id) {
                return Err(StoreError::Storage(
                    "project not found or not visible".to_string(),
                ));
            }
            let now = chrono::Utc::now();
            let record = TemplateRecord {
                id: format!("tpl_{}", Ulid::new()),
                project_id: project_id.clone(),
                name,
                description,
                channel,
                content,
                version: 1,
                attachments: attachments_from_inputs(attachments),
                created_at: now,
                updated_at: now,
            };
            templates.write().map_err(lock_err)?.push(record.clone());
            Ok(record)
        })
    }

    fn list(
        &self,
        actor: server::domain::auth::entities::UserId,
        project_id: &str,
        _search: Option<&str>,
        limit: i64,
        before: Option<&str>,
    ) -> BoxFut<'_, Result<Vec<TemplateRecord>, StoreError>> {
        let actor = actor.to_string();
        let project_id = project_id.to_string();
        let before = before.map(str::to_owned);
        let templates = self.templates.read().unwrap().clone();
        Box::pin(async move {
            if !self.is_visible(&actor, &project_id) {
                return Ok(Vec::new());
            }
            let mut list: Vec<_> = templates
                .into_iter()
                .filter(|t| t.project_id == project_id)
                .filter(|t| before.as_deref().is_none_or(|b| t.id.as_str() < b))
                .collect();
            list.sort_by_key(|e| Reverse(e.created_at));
            list.truncate(limit.max(0) as usize);
            Ok(list)
        })
    }

    fn get(
        &self,
        actor: server::domain::auth::entities::UserId,
        project_id: &str,
        template_id: &str,
    ) -> BoxFut<'_, Result<Option<TemplateRecord>, StoreError>> {
        let actor = actor.to_string();
        let project_id = project_id.to_string();
        let template_id = template_id.to_string();
        let templates = self.templates.read().unwrap().clone();
        Box::pin(async move {
            if !self.is_visible(&actor, &project_id) {
                return Ok(None);
            }
            Ok(templates
                .into_iter()
                .find(|t| t.id == template_id && t.project_id == project_id))
        })
    }

    fn get_by_id(
        &self,
        template_id: &str,
    ) -> BoxFut<'_, Result<Option<TemplateRecord>, StoreError>> {
        let template_id = template_id.to_string();
        let templates = self.templates.read().unwrap().clone();
        Box::pin(async move { Ok(templates.into_iter().find(|t| t.id == template_id)) })
    }

    #[allow(clippy::too_many_arguments)]
    fn update(
        &self,
        actor: server::domain::auth::entities::UserId,
        project_id: &str,
        template_id: &str,
        name: &str,
        description: Option<&str>,
        channel: &str,
        content: serde_json::Value,
        attachments: Vec<AttachmentInput>,
    ) -> BoxFut<'_, Result<Option<TemplateRecord>, StoreError>> {
        let actor = actor.to_string();
        let project_id = project_id.to_string();
        let template_id = template_id.to_string();
        let name = name.to_string();
        let description = description.map(str::to_owned);
        let channel = channel.to_string();
        let templates = &self.templates;
        let new_attachments = attachments_from_inputs(attachments);
        Box::pin(async move {
            if !self.is_visible(&actor, &project_id) {
                return Ok(None);
            }
            let mut list = templates.write().map_err(lock_err)?;
            let result = list
                .iter_mut()
                .find(|t| t.id == template_id && t.project_id == project_id);
            if let Some(record) = result {
                record.name = name.clone();
                record.description = description.clone();
                record.channel = channel.clone();
                record.content = content.clone();
                record.version += 1;
                record.updated_at = chrono::Utc::now();
                record.attachments = new_attachments.clone();
                Ok(Some(record.clone()))
            } else {
                Ok(None)
            }
        })
    }

    fn remove(
        &self,
        actor: server::domain::auth::entities::UserId,
        project_id: &str,
        template_id: &str,
    ) -> BoxFut<'_, Result<bool, StoreError>> {
        let actor = actor.to_string();
        let project_id = project_id.to_string();
        let template_id = template_id.to_string();
        let templates = &self.templates;
        Box::pin(async move {
            if !self.is_visible(&actor, &project_id) {
                return Ok(false);
            }
            let mut list = templates.write().map_err(lock_err)?;
            let before = list.len();
            list.retain(|t| !(t.id == template_id && t.project_id == project_id));
            Ok(list.len() != before)
        })
    }
}

// ---------------------------------------------------------------------------
// FakeTicketsStore — in-memory [`TicketsStore`] for tests.
// ---------------------------------------------------------------------------

#[derive(Default)]
pub struct FakeTicketsStore {
    tickets: RwLock<Vec<TicketRecord>>,
    messages: RwLock<Vec<TicketMessageRecord>>,
    /// (user_id, project_id) pairs the actor may access (project visibility).
    visible: RwLock<Vec<(String, String)>>,
    /// user_id -> (name, email) for admin ticket views.
    users: RwLock<Vec<(String, String, String)>>,
    /// admin_id -> name for admin message views.
    admins: RwLock<Vec<(String, String)>>,
}

impl FakeTicketsStore {
    pub fn new() -> Self {
        Self::default()
    }

    /// Grants `user_id` access to `project_id`.
    pub fn seed_visible(&self, user_id: &str, project_id: &str) {
        self.visible
            .write()
            .unwrap()
            .push((user_id.to_string(), project_id.to_string()));
    }

    /// Seeds a customer's identity for admin ticket views.
    pub fn seed_user(&self, user_id: &str, name: &str, email: &str) {
        self.users.write().unwrap().push((
            user_id.to_string(),
            name.to_string(),
            email.to_string(),
        ));
    }

    /// Seeds an admin's identity for admin message views.
    pub fn seed_admin(&self, admin_id: &str, name: &str) {
        self.admins
            .write()
            .unwrap()
            .push((admin_id.to_string(), name.to_string()));
    }

    fn customer_identity(users: &[(String, String, String)], user_id: &str) -> (String, String) {
        users
            .iter()
            .find(|(id, _, _)| id == user_id)
            .map(|(_, name, email)| (name.clone(), email.clone()))
            .unwrap_or_else(|| ("Unknown".to_string(), "unknown@example.com".to_string()))
    }

    fn author_name(
        users: &[(String, String, String)],
        admins: &[(String, String)],
        message: &TicketMessageRecord,
    ) -> Option<String> {
        let author_id = message.author_id.as_deref()?;
        match message.author {
            server::domain::support::entities::MessageAuthor::Customer => users
                .iter()
                .find(|(id, _, _)| id == author_id)
                .map(|(_, name, _)| name.clone()),
            server::domain::support::entities::MessageAuthor::Support => admins
                .iter()
                .find(|(id, _)| id == author_id)
                .map(|(_, name)| name.clone()),
        }
    }

    pub fn all(&self) -> Vec<TicketRecord> {
        let mut all = self.tickets.read().unwrap().clone();
        all.sort_by_key(|e| Reverse(e.created_at));
        all
    }
}

impl TicketsStore for FakeTicketsStore {
    fn create(
        &self,
        actor: server::domain::auth::entities::UserId,
        project_id: Option<&str>,
        subject: &str,
        category: &str,
        priority: &str,
        description: &str,
    ) -> BoxFut<'_, Result<TicketRecord, StoreError>> {
        let actor_str = actor.to_string();
        let project_id_owned = project_id.map(str::to_owned);
        let subject = subject.to_string();
        let category = category.to_string();
        let priority = priority.to_string();
        let description = description.to_string();
        let visible = self.visible.read().unwrap().clone();
        let tickets = &self.tickets;

        Box::pin(async move {
            // Validate project visibility when project_id is provided.
            if let Some(ref pid) = project_id_owned
                && !visible.iter().any(|(u, p)| *u == actor_str && *p == *pid)
            {
                return Err(StoreError::Storage(
                    "project not found or not visible".to_string(),
                ));
            }

            let record = TicketRecord {
                id: Ulid::new().to_string(),
                project_id: project_id_owned,
                created_by: actor_str,
                subject,
                category,
                priority,
                description,
                status: server::domain::support::entities::TicketStatus::Open,
                created_at: Utc::now(),
                updated_at: Utc::now(),
                deleted_at: None,
            };

            tickets.write().map_err(lock_err)?.push(record.clone());

            Ok(record)
        })
    }

    fn list(
        &self,
        actor: server::domain::auth::entities::UserId,
        project_id: Option<&str>,
        status: Option<&str>,
        limit: i64,
        before: Option<&str>,
    ) -> BoxFut<'_, Result<Vec<TicketRecord>, StoreError>> {
        let actor_str = actor.to_string();
        let project_id_owned = project_id.map(str::to_owned);
        let status_owned = status.map(str::to_owned);
        let before_owned = before.map(str::to_owned);
        let visible = self.visible.read().unwrap().clone();
        let tickets = &self.tickets;

        Box::pin(async move {
            let all = tickets.read().map_err(lock_err)?;
            let filtered: Vec<TicketRecord> = all
                .iter()
                .filter(|t| {
                    if t.deleted_at.is_some() {
                        return false;
                    }
                    // When project_id is provided, only show tickets for that
                    // specific project — and only if the actor has visibility.
                    if let Some(ref pid) = project_id_owned {
                        return t.project_id.as_deref() == Some(pid.as_str())
                            && visible
                                .iter()
                                .any(|(u, p)| *u == actor_str && p.as_str() == pid.as_str());
                    }
                    // No project filter — show all tickets visible to the actor.
                    let matches_creator = t.created_by == actor_str;
                    let matches_project = t.project_id.is_some()
                        && visible.iter().any(|(u, p)| {
                            *u == actor_str && p.as_str() == t.project_id.as_deref().unwrap_or("")
                        });
                    (t.project_id.is_none() && matches_creator) || matches_project
                })
                .filter(|t| {
                    if let Some(ref s) = status_owned {
                        t.status.as_str() == s.as_str()
                    } else {
                        true
                    }
                })
                .filter(|t| {
                    if let Some(ref b) = before_owned {
                        t.id < *b
                    } else {
                        true
                    }
                })
                .cloned()
                .collect();

            let mut result: Vec<TicketRecord> = filtered.into_iter().collect();
            result.sort_by(|a, b| b.created_at.cmp(&a.created_at).then(b.id.cmp(&a.id)));
            result.truncate(limit.max(0) as usize);
            Ok(result)
        })
    }

    fn get(
        &self,
        actor: server::domain::auth::entities::UserId,
        ticket_id: &str,
    ) -> BoxFut<'_, Result<Option<TicketRecord>, StoreError>> {
        let actor_str = actor.to_string();
        let ticket_id = ticket_id.to_string();
        let visible = self.visible.read().unwrap().clone();
        let tickets = &self.tickets;

        Box::pin(async move {
            let all = tickets.read().map_err(lock_err)?;
            let found = all.iter().find(|t| {
                if t.id != ticket_id || t.deleted_at.is_some() {
                    return false;
                }
                if t.project_id.is_none() {
                    return t.created_by == actor_str;
                }
                visible.iter().any(|(u, p)| {
                    *u == actor_str && p.as_str() == t.project_id.as_deref().unwrap_or("")
                })
            });
            Ok(found.cloned())
        })
    }

    fn list_messages(
        &self,
        actor: server::domain::auth::entities::UserId,
        ticket_id: &str,
    ) -> BoxFut<'_, Result<Vec<TicketMessageRecord>, StoreError>> {
        let actor_str = actor.to_string();
        let ticket_id = ticket_id.to_string();
        let visible = self.visible.read().unwrap().clone();
        let tickets = self.tickets.read().unwrap().clone();
        let messages = self.messages.read().unwrap().clone();

        Box::pin(async move {
            let ticket = tickets.iter().find(|t| {
                t.id == ticket_id
                    && t.deleted_at.is_none()
                    && (t.project_id.is_none() && t.created_by == actor_str
                        || t.project_id.is_some()
                            && visible.iter().any(|(u, p)| {
                                *u == actor_str
                                    && p.as_str() == t.project_id.as_deref().unwrap_or("")
                            }))
            });
            if ticket.is_none() {
                return Err(StoreError::Storage(
                    "ticket not found or not visible".to_string(),
                ));
            }
            let mut result: Vec<TicketMessageRecord> = messages
                .into_iter()
                .filter(|m| m.ticket_id == ticket_id)
                .collect();
            result.sort_by(|a, b| a.created_at.cmp(&b.created_at).then(a.id.cmp(&b.id)));
            Ok(result)
        })
    }

    fn add_message(
        &self,
        actor: server::domain::auth::entities::UserId,
        ticket_id: &str,
        body: &str,
    ) -> BoxFut<'_, Result<Option<TicketMessageRecord>, StoreError>> {
        let actor_str = actor.to_string();
        let ticket_id = ticket_id.to_string();
        let body = body.to_string();
        let visible = self.visible.read().unwrap().clone();
        let tickets = self.tickets.read().unwrap().clone();

        Box::pin(async move {
            let ticket = tickets.iter().find(|t| {
                t.id == ticket_id
                    && t.deleted_at.is_none()
                    && (t.project_id.is_none() && t.created_by == actor_str
                        || t.project_id.is_some()
                            && visible.iter().any(|(u, p)| {
                                *u == actor_str
                                    && p.as_str() == t.project_id.as_deref().unwrap_or("")
                            }))
            });
            if ticket.is_none() {
                return Ok(None);
            }
            let record = TicketMessageRecord {
                id: Ulid::new().to_string(),
                ticket_id: ticket_id.clone(),
                author: server::domain::support::entities::MessageAuthor::Customer,
                author_id: Some(actor_str),
                body,
                created_at: Utc::now(),
            };
            self.messages.write().unwrap().push(record.clone());
            // Touch updated_at on the ticket.
            if let Some(t) = self
                .tickets
                .write()
                .unwrap()
                .iter_mut()
                .find(|t| t.id == ticket_id)
            {
                t.updated_at = Utc::now();
            }
            Ok(Some(record))
        })
    }

    fn reopen(
        &self,
        actor: server::domain::auth::entities::UserId,
        ticket_id: &str,
    ) -> BoxFut<'_, Result<bool, StoreError>> {
        let actor_str = actor.to_string();
        let ticket_id = ticket_id.to_string();
        let visible = self.visible.read().unwrap().clone();

        Box::pin(async move {
            let mut tickets = self.tickets.write().unwrap();
            if let Some(t) = tickets.iter_mut().find(|t| {
                t.id == ticket_id
                    && t.deleted_at.is_none()
                    && (t.project_id.is_none() && t.created_by == actor_str
                        || t.project_id.is_some()
                            && visible.iter().any(|(u, p)| {
                                *u == actor_str
                                    && p.as_str() == t.project_id.as_deref().unwrap_or("")
                            }))
            }) && t.status == server::domain::support::entities::TicketStatus::Resolved
            {
                t.status = server::domain::support::entities::TicketStatus::Open;
                t.updated_at = Utc::now();
                return Ok(true);
            }
            Ok(false)
        })
    }

    // === Admin-scoped methods (no actor visibility checks) =================

    fn list_all(
        &self,
        status: Option<&str>,
        search: Option<&str>,
        limit: i64,
        offset: i64,
    ) -> BoxFut<'_, Result<(Vec<AdminTicketRecord>, i64), StoreError>> {
        let status_owned = status.map(str::to_owned);
        let search_owned = search.map(|s| s.to_lowercase());
        let tickets = &self.tickets;
        let users = self.users.read().unwrap().clone();

        Box::pin(async move {
            let all = tickets.read().map_err(lock_err)?;
            let mut result: Vec<AdminTicketRecord> = all
                .iter()
                .filter(|t| t.deleted_at.is_none())
                .filter(|t| {
                    if let Some(ref s) = status_owned {
                        t.status.as_str() == s.as_str()
                    } else {
                        true
                    }
                })
                .filter(|t| {
                    if let Some(ref q) = search_owned {
                        let (name, email) = Self::customer_identity(&users, &t.created_by);
                        t.subject.to_lowercase().contains(q.as_str())
                            || name.to_lowercase().contains(q.as_str())
                            || email.to_lowercase().contains(q.as_str())
                            || t.id.to_lowercase().contains(q.as_str())
                    } else {
                        true
                    }
                })
                .map(|t| {
                    let (customer_name, customer_email) =
                        Self::customer_identity(&users, &t.created_by);
                    AdminTicketRecord {
                        ticket: t.clone(),
                        customer_name,
                        customer_email,
                    }
                })
                .collect();
            result.sort_by(|a, b| {
                b.ticket
                    .created_at
                    .cmp(&a.ticket.created_at)
                    .then(b.ticket.id.cmp(&a.ticket.id))
            });
            let total = result.len() as i64;
            let page: Vec<AdminTicketRecord> = result
                .into_iter()
                .skip(offset.max(0) as usize)
                .take(limit.max(0) as usize)
                .collect();
            Ok((page, total))
        })
    }

    fn get_any(
        &self,
        ticket_id: &str,
    ) -> BoxFut<'_, Result<Option<AdminTicketRecord>, StoreError>> {
        let ticket_id = ticket_id.to_string();
        let tickets = &self.tickets;
        let users = self.users.read().unwrap().clone();

        Box::pin(async move {
            let all = tickets.read().map_err(lock_err)?;
            Ok(all
                .iter()
                .find(|t| t.id == ticket_id && t.deleted_at.is_none())
                .map(|t| {
                    let (customer_name, customer_email) =
                        Self::customer_identity(&users, &t.created_by);
                    AdminTicketRecord {
                        ticket: t.clone(),
                        customer_name,
                        customer_email,
                    }
                }))
        })
    }

    fn list_messages_any(
        &self,
        ticket_id: &str,
    ) -> BoxFut<'_, Result<Vec<AdminTicketMessageRecord>, StoreError>> {
        let ticket_id = ticket_id.to_string();
        let messages = self.messages.read().unwrap().clone();
        let users = self.users.read().unwrap().clone();
        let admins = self.admins.read().unwrap().clone();

        Box::pin(async move {
            let mut result: Vec<AdminTicketMessageRecord> = messages
                .into_iter()
                .filter(|m| m.ticket_id == ticket_id)
                .map(|m| {
                    let author_name = Self::author_name(&users, &admins, &m);
                    AdminTicketMessageRecord {
                        message: m,
                        author_name,
                    }
                })
                .collect();
            result.sort_by(|a, b| {
                a.message
                    .created_at
                    .cmp(&b.message.created_at)
                    .then(a.message.id.cmp(&b.message.id))
            });
            Ok(result)
        })
    }

    fn add_support_message(
        &self,
        admin_id: server::domain::admin::entities::AdminUserId,
        ticket_id: &str,
        body: &str,
    ) -> BoxFut<'_, Result<Option<TicketMessageRecord>, StoreError>> {
        let admin_id_str = admin_id.to_string();
        let ticket_id = ticket_id.to_string();
        let body = body.to_string();

        Box::pin(async move {
            let exists = self
                .tickets
                .read()
                .unwrap()
                .iter()
                .any(|t| t.id == ticket_id && t.deleted_at.is_none());
            if !exists {
                return Ok(None);
            }
            let record = TicketMessageRecord {
                id: Ulid::new().to_string(),
                ticket_id: ticket_id.clone(),
                author: server::domain::support::entities::MessageAuthor::Support,
                author_id: Some(admin_id_str),
                body,
                created_at: Utc::now(),
            };
            self.messages.write().unwrap().push(record.clone());
            if let Some(t) = self
                .tickets
                .write()
                .unwrap()
                .iter_mut()
                .find(|t| t.id == ticket_id)
            {
                t.updated_at = Utc::now();
            }
            Ok(Some(record))
        })
    }

    fn set_status(
        &self,
        ticket_id: &str,
        status: server::domain::support::entities::TicketStatus,
    ) -> BoxFut<'_, Result<bool, StoreError>> {
        let ticket_id = ticket_id.to_string();

        Box::pin(async move {
            let mut tickets = self.tickets.write().unwrap();
            if let Some(t) = tickets
                .iter_mut()
                .find(|t| t.id == ticket_id && t.deleted_at.is_none())
            {
                t.status = status;
                t.updated_at = Utc::now();
                return Ok(true);
            }
            Ok(false)
        })
    }

    fn set_priority(
        &self,
        ticket_id: &str,
        priority: &str,
    ) -> BoxFut<'_, Result<bool, StoreError>> {
        let ticket_id = ticket_id.to_string();
        let priority = priority.to_string();

        Box::pin(async move {
            let mut tickets = self.tickets.write().unwrap();
            if let Some(t) = tickets
                .iter_mut()
                .find(|t| t.id == ticket_id && t.deleted_at.is_none())
            {
                t.priority = priority;
                t.updated_at = Utc::now();
                return Ok(true);
            }
            Ok(false)
        })
    }

    fn count_tickets_for_user(
        &self,
        user_id: &str,
    ) -> BoxFut<'_, Result<server::ports::tickets_store::TicketCounts, StoreError>> {
        let user_id = user_id.to_string();
        let tickets = self.tickets.read().unwrap().clone();

        Box::pin(async move {
            let mine: Vec<_> = tickets
                .iter()
                .filter(|t| t.created_by == user_id && t.deleted_at.is_none())
                .collect();
            let total = mine.len() as i64;
            let open = mine
                .iter()
                .filter(|t| {
                    matches!(
                        t.status,
                        server::domain::support::entities::TicketStatus::Open
                            | server::domain::support::entities::TicketStatus::InProgress
                    )
                })
                .count() as i64;
            Ok(server::ports::tickets_store::TicketCounts { total, open })
        })
    }
}

// ---------------------------------------------------------------------------
// FakeApiKeysStore — in-memory [`ApiKeysStore`] for tests.
// ---------------------------------------------------------------------------

#[derive(Default)]
pub struct FakeApiKeysStore {
    keys: RwLock<Vec<server::ports::api_keys_store::ApiKeyRecord>>,
    ips: RwLock<Vec<server::ports::api_keys_store::IpAllowEntry>>,
    /// (user_id, project_id) pairs the actor may access (project visibility).
    visible: RwLock<Vec<(String, String)>>,
}

impl FakeApiKeysStore {
    pub fn new() -> Self {
        Self::default()
    }

    /// Grants `user_id` access to `project_id`.
    pub fn seed_visible(&self, user_id: &str, project_id: &str) {
        self.visible
            .write()
            .unwrap()
            .push((user_id.to_string(), project_id.to_string()));
    }

    fn can_see(&self, actor: &str, project_id: &str) -> bool {
        self.visible
            .read()
            .unwrap()
            .iter()
            .any(|(u, p)| u == actor && p == project_id)
    }

    fn visible_key(
        &self,
        actor: &str,
        project_id: &str,
        key_id: &str,
    ) -> Option<server::ports::api_keys_store::ApiKeyRecord> {
        if !self.can_see(actor, project_id) {
            return None;
        }
        self.keys
            .read()
            .unwrap()
            .iter()
            .find(|k| k.id == key_id && k.project_id == project_id)
            .cloned()
    }
}

impl server::ports::api_keys_store::ApiKeysStore for FakeApiKeysStore {
    #[allow(clippy::too_many_arguments)]
    fn create_key(
        &self,
        actor: UserId,
        project_id: &str,
        name: &str,
        permissions: &[String],
        expires_at: Option<DateTime<Utc>>,
        prefix: &str,
        key_hash: &str,
    ) -> BoxFut<'_, Result<server::ports::api_keys_store::ApiKeyRecord, StoreError>> {
        use server::ports::api_keys_store::ApiKeyRecord;
        let actor_str = actor.to_string();
        let project_id = project_id.to_string();
        let name = name.to_string();
        let permissions = permissions.to_vec();
        let prefix = prefix.to_string();
        let key_hash = key_hash.to_string();
        Box::pin(async move {
            if !self.can_see(&actor_str, &project_id) {
                return Err(StoreError::Storage(
                    "project not found or not visible".to_string(),
                ));
            }
            let mut keys = self.keys.write().map_err(lock_err)?;
            if keys
                .iter()
                .any(|k| k.project_id == project_id && k.name == name)
            {
                return Err(StoreError::Conflict);
            }
            if keys.iter().any(|k| k.key_hash == key_hash) {
                return Err(StoreError::Conflict);
            }
            let record = ApiKeyRecord {
                id: Ulid::new().to_string(),
                project_id,
                name,
                prefix,
                key_hash,
                permissions,
                expires_at,
                last_used_at: None,
                enabled: true,
                revoked_at: None,
                created_at: Utc::now(),
            };
            keys.push(record.clone());
            Ok(record)
        })
    }

    fn list_keys(
        &self,
        actor: UserId,
        project_id: &str,
    ) -> BoxFut<'_, Result<Vec<server::ports::api_keys_store::ApiKeyRecord>, StoreError>> {
        let actor_str = actor.to_string();
        let project_id = project_id.to_string();
        Box::pin(async move {
            if !self.can_see(&actor_str, &project_id) {
                return Ok(vec![]);
            }
            let mut keys: Vec<_> = self
                .keys
                .read()
                .map_err(lock_err)?
                .iter()
                .filter(|k| k.project_id == project_id)
                .cloned()
                .collect();
            keys.sort_by_key(|e| Reverse(e.created_at));
            Ok(keys)
        })
    }

    fn get_key(
        &self,
        actor: UserId,
        project_id: &str,
        key_id: &str,
    ) -> BoxFut<'_, Result<Option<server::ports::api_keys_store::ApiKeyRecord>, StoreError>> {
        let actor_str = actor.to_string();
        let project_id = project_id.to_string();
        let key_id = key_id.to_string();
        Box::pin(async move { Ok(self.visible_key(&actor_str, &project_id, &key_id)) })
    }

    fn rename_key(
        &self,
        actor: UserId,
        project_id: &str,
        key_id: &str,
        name: &str,
    ) -> BoxFut<'_, Result<bool, StoreError>> {
        let actor_str = actor.to_string();
        let project_id = project_id.to_string();
        let key_id = key_id.to_string();
        let name = name.to_string();
        Box::pin(async move {
            let mut keys = self.keys.write().map_err(lock_err)?;
            if keys
                .iter()
                .any(|k| k.id != key_id && k.project_id == project_id && k.name == name)
            {
                return Err(StoreError::Conflict);
            }
            match keys
                .iter_mut()
                .find(|k| k.id == key_id && k.project_id == project_id)
            {
                Some(k) if self.can_see(&actor_str, &project_id) => {
                    k.name = name;
                    Ok(true)
                }
                _ => Ok(false),
            }
        })
    }

    fn set_key_enabled(
        &self,
        actor: UserId,
        project_id: &str,
        key_id: &str,
        enabled: bool,
    ) -> BoxFut<'_, Result<bool, StoreError>> {
        let actor_str = actor.to_string();
        let project_id = project_id.to_string();
        let key_id = key_id.to_string();
        Box::pin(async move {
            let mut keys = self.keys.write().map_err(lock_err)?;
            match keys
                .iter_mut()
                .find(|k| k.id == key_id && k.project_id == project_id)
            {
                Some(k) if self.can_see(&actor_str, &project_id) => {
                    k.enabled = enabled;
                    Ok(true)
                }
                _ => Ok(false),
            }
        })
    }

    fn revoke_key(
        &self,
        actor: UserId,
        project_id: &str,
        key_id: &str,
    ) -> BoxFut<'_, Result<bool, StoreError>> {
        let actor_str = actor.to_string();
        let project_id = project_id.to_string();
        let key_id = key_id.to_string();
        Box::pin(async move {
            let mut keys = self.keys.write().map_err(lock_err)?;
            match keys
                .iter_mut()
                .find(|k| k.id == key_id && k.project_id == project_id && k.revoked_at.is_none())
            {
                Some(k) if self.can_see(&actor_str, &project_id) => {
                    k.revoked_at = Some(Utc::now());
                    Ok(true)
                }
                _ => Ok(false),
            }
        })
    }

    fn find_key_by_hash(
        &self,
        key_hash: &str,
    ) -> BoxFut<'_, Result<Option<server::ports::api_keys_store::ApiKeyRecord>, StoreError>> {
        let key_hash = key_hash.to_string();
        Box::pin(async move {
            Ok(self
                .keys
                .read()
                .map_err(lock_err)?
                .iter()
                .find(|k| k.key_hash == key_hash)
                .cloned())
        })
    }

    fn touch_key_last_used(&self, key_id: &str) -> BoxFut<'_, Result<(), StoreError>> {
        let key_id = key_id.to_string();
        Box::pin(async move {
            if let Some(k) = self
                .keys
                .write()
                .map_err(lock_err)?
                .iter_mut()
                .find(|k| k.id == key_id)
            {
                k.last_used_at = Some(Utc::now());
            }
            Ok(())
        })
    }

    fn list_ips(
        &self,
        actor: UserId,
        project_id: &str,
        key_id: &str,
    ) -> BoxFut<'_, Result<Vec<server::ports::api_keys_store::IpAllowEntry>, StoreError>> {
        let actor_str = actor.to_string();
        let project_id = project_id.to_string();
        let key_id = key_id.to_string();
        Box::pin(async move {
            if self.visible_key(&actor_str, &project_id, &key_id).is_none() {
                return Ok(vec![]);
            }
            let mut entries: Vec<_> = self
                .ips
                .read()
                .map_err(lock_err)?
                .iter()
                .filter(|e| e.key_id == key_id)
                .cloned()
                .collect();
            entries.sort_by_key(|e| e.created_at);
            Ok(entries)
        })
    }

    fn list_ips_for_key(
        &self,
        key_id: &str,
    ) -> BoxFut<'_, Result<Vec<server::ports::api_keys_store::IpAllowEntry>, StoreError>> {
        let key_id = key_id.to_string();
        Box::pin(async move {
            Ok(self
                .ips
                .read()
                .map_err(lock_err)?
                .iter()
                .filter(|e| e.key_id == key_id)
                .cloned()
                .collect())
        })
    }

    fn add_ip(
        &self,
        actor: UserId,
        project_id: &str,
        key_id: &str,
        cidr: &str,
        label: &str,
    ) -> BoxFut<'_, Result<server::ports::api_keys_store::IpAllowEntry, StoreError>> {
        use server::ports::api_keys_store::IpAllowEntry;
        let actor_str = actor.to_string();
        let project_id = project_id.to_string();
        let key_id = key_id.to_string();
        let cidr = cidr.to_string();
        let label = label.to_string();
        Box::pin(async move {
            if self.visible_key(&actor_str, &project_id, &key_id).is_none() {
                return Err(StoreError::Storage("API key not found".to_string()));
            }
            let mut ips = self.ips.write().map_err(lock_err)?;
            if ips.iter().any(|e| e.key_id == key_id && e.cidr == cidr) {
                return Err(StoreError::Conflict);
            }
            let entry = IpAllowEntry {
                id: Ulid::new().to_string(),
                key_id,
                cidr,
                label,
                created_at: Utc::now(),
            };
            ips.push(entry.clone());
            Ok(entry)
        })
    }

    fn remove_ip(
        &self,
        actor: UserId,
        project_id: &str,
        key_id: &str,
        entry_id: &str,
    ) -> BoxFut<'_, Result<bool, StoreError>> {
        let actor_str = actor.to_string();
        let project_id = project_id.to_string();
        let key_id = key_id.to_string();
        let entry_id = entry_id.to_string();
        Box::pin(async move {
            if self.visible_key(&actor_str, &project_id, &key_id).is_none() {
                return Ok(false);
            }
            let mut ips = self.ips.write().map_err(lock_err)?;
            let before = ips.len();
            ips.retain(|e| !(e.id == entry_id && e.key_id == key_id));
            Ok(ips.len() < before)
        })
    }
}

// ---------------------------------------------------------------------------
// FakeNotificationsStore — in-memory [`NotificationsStore`] for tests.
// ---------------------------------------------------------------------------

#[derive(Default)]
pub struct FakeNotificationsStore {
    notifications: RwLock<Vec<NotificationRecord>>,
    broadcasts: RwLock<Vec<BroadcastRecord>>,
}

impl FakeNotificationsStore {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn all(&self) -> Vec<NotificationRecord> {
        let mut all = self.notifications.read().unwrap().clone();
        all.sort_by_key(|e| Reverse(e.created_at));
        all
    }
}

impl NotificationsStore for FakeNotificationsStore {
    fn create(
        &self,
        user_id: UserId,
        notification_type: NotificationType,
        origin: NotificationOrigin,
        title: &str,
        content: &str,
    ) -> BoxFut<'_, Result<NotificationRecord, StoreError>> {
        let notifications = &self.notifications;
        let user_str = user_id.to_string();
        let title = title.to_string();
        let content = content.to_string();
        Box::pin(async move {
            let record = NotificationRecord {
                id: server::kernel::Ulid::new().to_string(),
                user_id: user_str,
                notification_type,
                origin,
                title,
                content,
                read_at: None,
                created_at: Utc::now(),
                deleted_at: None,
                broadcast_id: None,
            };
            notifications
                .write()
                .map_err(lock_err)?
                .push(record.clone());
            Ok(record)
        })
    }

    fn list(
        &self,
        user_id: UserId,
        unread_only: bool,
        limit: i64,
        before: Option<&str>,
    ) -> BoxFut<'_, Result<Vec<NotificationRecord>, StoreError>> {
        let user_str = user_id.to_string();
        let before_owned = before.map(str::to_owned);
        let all = self.notifications.read().unwrap().clone();
        Box::pin(async move {
            let mut list: Vec<_> = all
                .into_iter()
                .filter(|n| n.user_id == user_str && n.deleted_at.is_none())
                .filter(|n| !unread_only || n.read_at.is_none())
                .filter(|n| before_owned.as_deref().is_none_or(|b| n.id.as_str() < b))
                .collect();
            list.sort_by_key(|e| Reverse(e.created_at));
            list.truncate(limit.max(0) as usize);
            Ok(list)
        })
    }

    fn count_unread(&self, user_id: UserId) -> BoxFut<'_, Result<i64, StoreError>> {
        let user_str = user_id.to_string();
        let all = self.notifications.read().unwrap().clone();
        Box::pin(async move {
            let count = all
                .iter()
                .filter(|n| n.user_id == user_str && n.read_at.is_none() && n.deleted_at.is_none())
                .count() as i64;
            Ok(count)
        })
    }

    fn count_all_for_user(&self, user_id: UserId) -> BoxFut<'_, Result<i64, StoreError>> {
        let user_str = user_id.to_string();
        let all = self.notifications.read().unwrap().clone();
        Box::pin(async move {
            let count = all
                .iter()
                .filter(|n| n.user_id == user_str && n.deleted_at.is_none())
                .count() as i64;
            Ok(count)
        })
    }

    fn get(
        &self,
        user_id: UserId,
        notification_id: &str,
    ) -> BoxFut<'_, Result<Option<NotificationRecord>, StoreError>> {
        let user_str = user_id.to_string();
        let id = notification_id.to_string();
        let all = self.notifications.read().unwrap().clone();
        Box::pin(async move {
            Ok(all
                .into_iter()
                .find(|n| n.id == id && n.user_id == user_str && n.deleted_at.is_none()))
        })
    }

    fn set_read(
        &self,
        user_id: UserId,
        notification_id: &str,
        read: bool,
    ) -> BoxFut<'_, Result<Option<NotificationRecord>, StoreError>> {
        let user_str = user_id.to_string();
        let id = notification_id.to_string();
        let notifications = &self.notifications;
        Box::pin(async move {
            let mut list = notifications.write().map_err(lock_err)?;
            if let Some(record) = list
                .iter_mut()
                .find(|n| n.id == id && n.user_id == user_str && n.deleted_at.is_none())
            {
                record.read_at = if read { Some(Utc::now()) } else { None };
                Ok(Some(record.clone()))
            } else {
                Ok(None)
            }
        })
    }

    fn mark_all_read(&self, user_id: UserId) -> BoxFut<'_, Result<i64, StoreError>> {
        let user_str = user_id.to_string();
        let notifications = &self.notifications;
        Box::pin(async move {
            let mut list = notifications.write().map_err(lock_err)?;
            let now = Utc::now();
            let count = list
                .iter_mut()
                .filter(|n| n.user_id == user_str && n.read_at.is_none() && n.deleted_at.is_none())
                .map(|n| {
                    n.read_at = Some(now);
                    1
                })
                .sum::<i64>();
            Ok(count)
        })
    }

    fn delete(
        &self,
        user_id: UserId,
        notification_id: &str,
    ) -> BoxFut<'_, Result<bool, StoreError>> {
        let user_str = user_id.to_string();
        let id = notification_id.to_string();
        let notifications = &self.notifications;
        Box::pin(async move {
            let mut list = notifications.write().map_err(lock_err)?;
            if let Some(record) = list
                .iter_mut()
                .find(|n| n.id == id && n.user_id == user_str && n.deleted_at.is_none())
            {
                record.deleted_at = Some(Utc::now());
                Ok(true)
            } else {
                Ok(false)
            }
        })
    }

    // === Admin broadcast methods ===========================================

    fn create_broadcast(&self, broadcast: &BroadcastRecord) -> BoxFut<'_, Result<(), StoreError>> {
        let broadcasts = &self.broadcasts;
        let broadcast = broadcast.clone();
        Box::pin(async move {
            broadcasts.write().map_err(lock_err)?.push(broadcast);
            Ok(())
        })
    }

    fn get_broadcast(&self, id: &str) -> BoxFut<'_, Result<Option<BroadcastRecord>, StoreError>> {
        let broadcasts = &self.broadcasts;
        let id = id.to_string();
        Box::pin(async move {
            Ok(broadcasts
                .read()
                .map_err(lock_err)?
                .iter()
                .find(|b| b.id == id)
                .cloned())
        })
    }

    fn list_broadcasts(
        &self,
        status: Option<BroadcastStatus>,
        notification_type: Option<&str>,
        limit: i64,
        offset: i64,
    ) -> BoxFut<'_, Result<(Vec<BroadcastRecord>, i64), StoreError>> {
        let status_owned = status;
        let type_owned = notification_type.map(str::to_owned);
        let broadcasts = &self.broadcasts;
        Box::pin(async move {
            let all = broadcasts.read().map_err(lock_err)?;
            let mut result: Vec<BroadcastRecord> = all
                .iter()
                .filter(|b| {
                    if let Some(s) = status_owned {
                        b.status == s
                    } else {
                        true
                    }
                })
                .filter(|b| {
                    if let Some(ref t) = type_owned {
                        b.notification_type.to_string() == *t
                    } else {
                        true
                    }
                })
                .cloned()
                .collect();
            result.sort_by(|a, b| b.created_at.cmp(&a.created_at).then(b.id.cmp(&a.id)));
            let total = result.len() as i64;
            let page: Vec<BroadcastRecord> = result
                .into_iter()
                .skip(offset.max(0) as usize)
                .take(limit.max(0) as usize)
                .collect();
            Ok((page, total))
        })
    }

    fn create_many(
        &self,
        broadcast_id: &str,
        notification_type: server::domain::notifications::entities::NotificationType,
        recipients: &[BroadcastRecipient],
    ) -> BoxFut<'_, Result<i64, StoreError>> {
        let notifications = &self.notifications;
        let broadcast_id = broadcast_id.to_string();
        let recipients = recipients.to_vec();
        Box::pin(async move {
            let mut list = notifications.write().map_err(lock_err)?;
            for r in &recipients {
                list.push(NotificationRecord {
                    id: server::kernel::Ulid::new().to_string(),
                    user_id: r.user_id.clone(),
                    notification_type,
                    origin: server::domain::notifications::entities::NotificationOrigin::Admin,
                    title: r.title.clone(),
                    content: r.content.clone(),
                    read_at: None,
                    created_at: Utc::now(),
                    deleted_at: None,
                    broadcast_id: Some(broadcast_id.clone()),
                });
            }
            Ok(recipients.len() as i64)
        })
    }

    fn claim_due_scheduled(
        &self,
        now: chrono::DateTime<Utc>,
        limit: i64,
    ) -> BoxFut<'_, Result<Vec<BroadcastRecord>, StoreError>> {
        let broadcasts = &self.broadcasts;
        Box::pin(async move {
            let mut all = broadcasts.write().map_err(lock_err)?;
            let mut due: Vec<BroadcastRecord> = all
                .iter_mut()
                .filter(|b| {
                    b.status == BroadcastStatus::Scheduled
                        && b.scheduled_for.is_some_and(|t| t <= now)
                })
                .take(limit.max(0) as usize)
                .map(|b| {
                    b.status = BroadcastStatus::Sending;
                    b.clone()
                })
                .collect();
            due.sort_by(|a, b| a.scheduled_for.cmp(&b.scheduled_for).then(a.id.cmp(&b.id)));
            Ok(due)
        })
    }

    fn mark_broadcast_sent(
        &self,
        id: &str,
        recipient_count: i64,
    ) -> BoxFut<'_, Result<bool, StoreError>> {
        let broadcasts = &self.broadcasts;
        let id = id.to_string();
        Box::pin(async move {
            let mut all = broadcasts.write().map_err(lock_err)?;
            if let Some(b) = all
                .iter_mut()
                .find(|b| b.id == id && b.status == BroadcastStatus::Sending)
            {
                b.status = BroadcastStatus::Sent;
                b.sent_at = Some(Utc::now());
                b.recipient_count = recipient_count;
                return Ok(true);
            }
            Ok(false)
        })
    }

    fn cancel_broadcast(&self, id: &str) -> BoxFut<'_, Result<bool, StoreError>> {
        let broadcasts = &self.broadcasts;
        let id = id.to_string();
        Box::pin(async move {
            let mut all = broadcasts.write().map_err(lock_err)?;
            if let Some(b) = all
                .iter_mut()
                .find(|b| b.id == id && b.status == BroadcastStatus::Scheduled)
            {
                b.status = BroadcastStatus::Cancelled;
                return Ok(true);
            }
            Ok(false)
        })
    }

    fn broadcast_read_stats(&self, id: &str) -> BoxFut<'_, Result<BroadcastStats, StoreError>> {
        let notifications = &self.notifications;
        let id = id.to_string();
        Box::pin(async move {
            let all = notifications.read().map_err(lock_err)?;
            let mine: Vec<_> = all
                .iter()
                .filter(|n| {
                    n.broadcast_id.as_deref() == Some(id.as_str()) && n.deleted_at.is_none()
                })
                .collect();
            Ok(BroadcastStats {
                total: mine.len() as i64,
                read: mine.iter().filter(|n| n.read_at.is_some()).count() as i64,
            })
        })
    }
}

// ------------------------------------------------------------------
// FakeAdminStore — in-memory admin store for tests
// ------------------------------------------------------------------

use server::domain::admin::entities::{
    AdminPasswordResetToken, AdminPasswordResetTokenId, AdminSession, AdminSessionId, AdminStatus,
    AdminUser, AdminUserId,
};
use server::ports::admin_store::AdminStore;

#[derive(Default)]
pub struct FakeAdminStore {
    admins: RwLock<Vec<AdminUser>>,
    sessions: RwLock<Vec<AdminSession>>,
    reset_tokens: RwLock<Vec<AdminPasswordResetToken>>,
    deleted_admins: RwLock<Vec<AdminUserId>>,
}

impl FakeAdminStore {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn seed_admin(&self, admin: AdminUser) {
        self.admins.write().unwrap().push(admin);
    }
}

impl AdminStore for FakeAdminStore {
    fn admin_exists(&self) -> BoxFut<'_, Result<bool, StoreError>> {
        let admins = &self.admins;
        Box::pin(async move { Ok(!admins.read().map_err(lock_err)?.is_empty()) })
    }

    fn create_admin(&self, admin: &AdminUser) -> BoxFut<'_, Result<(), StoreError>> {
        let admins = &self.admins;
        let admin = admin.clone();
        Box::pin(async move {
            admins.write().map_err(lock_err)?.push(admin);
            Ok(())
        })
    }

    fn find_admin_by_email(
        &self,
        email: &str,
    ) -> BoxFut<'_, Result<Option<AdminUser>, StoreError>> {
        let admins = &self.admins;
        let email = email.to_string();
        let deleted = self.deleted_admins.read().unwrap().clone();
        Box::pin(async move {
            Ok(admins
                .read()
                .map_err(lock_err)?
                .iter()
                .find(|a| a.email.as_str() == email && !deleted.contains(&a.id))
                .cloned())
        })
    }

    fn find_admin_by_id(
        &self,
        id: AdminUserId,
    ) -> BoxFut<'_, Result<Option<AdminUser>, StoreError>> {
        let admins = &self.admins;
        let deleted = self.deleted_admins.read().unwrap().clone();
        Box::pin(async move {
            Ok(admins
                .read()
                .map_err(lock_err)?
                .iter()
                .find(|a| a.id == id && !deleted.contains(&a.id))
                .cloned())
        })
    }

    fn set_totp_secret(
        &self,
        id: AdminUserId,
        secret: String,
    ) -> BoxFut<'_, Result<(), StoreError>> {
        let admins = &self.admins;
        Box::pin(async move {
            let mut admins = admins.write().map_err(lock_err)?;
            if let Some(admin) = admins.iter_mut().find(|a| a.id == id) {
                admin.totp_secret = Some(secret);
            }
            Ok(())
        })
    }

    fn enable_totp(&self, id: AdminUserId) -> BoxFut<'_, Result<(), StoreError>> {
        let admins = &self.admins;
        Box::pin(async move {
            let mut admins = admins.write().map_err(lock_err)?;
            if let Some(admin) = admins.iter_mut().find(|a| a.id == id) {
                admin.totp_enabled = true;
            }
            Ok(())
        })
    }

    fn touch_admin_last_login(
        &self,
        id: AdminUserId,
        at: DateTime<Utc>,
    ) -> BoxFut<'_, Result<(), StoreError>> {
        let admins = &self.admins;
        Box::pin(async move {
            let mut admins = admins.write().map_err(lock_err)?;
            if let Some(admin) = admins.iter_mut().find(|a| a.id == id) {
                admin.last_login_at = Some(at);
            }
            Ok(())
        })
    }

    fn create_admin_session(&self, session: &AdminSession) -> BoxFut<'_, Result<(), StoreError>> {
        let sessions = &self.sessions;
        let session = session.clone();
        Box::pin(async move {
            sessions.write().map_err(lock_err)?.push(session);
            Ok(())
        })
    }

    fn find_admin_session_by_hash(
        &self,
        hash: &str,
    ) -> BoxFut<'_, Result<Option<AdminSession>, StoreError>> {
        let sessions = &self.sessions;
        let hash = hash.to_string();
        Box::pin(async move {
            Ok(sessions
                .read()
                .map_err(lock_err)?
                .iter()
                .find(|s| s.token_hash == hash)
                .cloned())
        })
    }

    fn revoke_admin_sessions(&self, id: AdminUserId) -> BoxFut<'_, Result<(), StoreError>> {
        let sessions = &self.sessions;
        Box::pin(async move {
            let mut sessions = sessions.write().map_err(lock_err)?;
            for s in sessions.iter_mut().filter(|s| s.admin_id == id) {
                s.revoked_at = Some(Utc::now());
            }
            Ok(())
        })
    }

    fn revoke_admin_session(&self, id: AdminSessionId) -> BoxFut<'_, Result<(), StoreError>> {
        let sessions = &self.sessions;
        Box::pin(async move {
            let mut sessions = sessions.write().map_err(lock_err)?;
            if let Some(s) = sessions.iter_mut().find(|s| s.id == id) {
                s.revoked_at = Some(Utc::now());
            }
            Ok(())
        })
    }

    fn update_admin_password(
        &self,
        id: AdminUserId,
        password_hash: String,
    ) -> BoxFut<'_, Result<(), StoreError>> {
        let admins = &self.admins;
        Box::pin(async move {
            let mut admins = admins.write().map_err(lock_err)?;
            if let Some(admin) = admins.iter_mut().find(|a| a.id == id) {
                admin.password_hash = password_hash;
            }
            Ok(())
        })
    }

    fn create_admin_reset_token(
        &self,
        token: &AdminPasswordResetToken,
    ) -> BoxFut<'_, Result<(), StoreError>> {
        let tokens = &self.reset_tokens;
        let token = token.clone();
        Box::pin(async move {
            tokens.write().map_err(lock_err)?.push(token);
            Ok(())
        })
    }

    fn consume_admin_reset_tokens_for_admin(
        &self,
        id: AdminUserId,
    ) -> BoxFut<'_, Result<(), StoreError>> {
        let tokens = &self.reset_tokens;
        Box::pin(async move {
            let mut tokens = tokens.write().map_err(lock_err)?;
            for t in tokens.iter_mut().filter(|t| t.admin_id == id) {
                t.consumed_at = Some(Utc::now());
            }
            Ok(())
        })
    }

    fn find_admin_reset_token_by_hash(
        &self,
        hash: &str,
    ) -> BoxFut<'_, Result<Option<AdminPasswordResetToken>, StoreError>> {
        let tokens = &self.reset_tokens;
        let hash = hash.to_string();
        Box::pin(async move {
            Ok(tokens
                .read()
                .map_err(lock_err)?
                .iter()
                .find(|t| t.token_hash == hash)
                .cloned())
        })
    }

    fn consume_admin_reset_token(
        &self,
        id: AdminPasswordResetTokenId,
    ) -> BoxFut<'_, Result<(), StoreError>> {
        let tokens = &self.reset_tokens;
        Box::pin(async move {
            let mut tokens = tokens.write().map_err(lock_err)?;
            if let Some(t) = tokens.iter_mut().find(|t| t.id == id) {
                t.consumed_at = Some(Utc::now());
            }
            Ok(())
        })
    }

    fn list_admins(
        &self,
        limit: i64,
        offset: i64,
    ) -> BoxFut<'_, Result<(Vec<AdminUser>, i64), StoreError>> {
        let admins = &self.admins;
        let deleted = self.deleted_admins.read().unwrap().clone();
        Box::pin(async move {
            let mut result: Vec<AdminUser> = admins
                .read()
                .map_err(lock_err)?
                .iter()
                .filter(|a| !deleted.contains(&a.id))
                .cloned()
                .collect();
            result.sort_by(|a, b| {
                a.created_at
                    .cmp(&b.created_at)
                    .then(a.id.to_string().cmp(&b.id.to_string()))
            });
            let total = result.len() as i64;
            let page: Vec<AdminUser> = result
                .into_iter()
                .skip(offset.max(0) as usize)
                .take(limit.max(0) as usize)
                .collect();
            Ok((page, total))
        })
    }

    fn set_admin_status(
        &self,
        id: AdminUserId,
        status: AdminStatus,
    ) -> BoxFut<'_, Result<bool, StoreError>> {
        let admins = &self.admins;
        let deleted = self.deleted_admins.read().unwrap().clone();
        Box::pin(async move {
            let mut admins = admins.write().map_err(lock_err)?;
            if let Some(admin) = admins
                .iter_mut()
                .find(|a| a.id == id && !deleted.contains(&a.id))
            {
                admin.status = status;
                return Ok(true);
            }
            Ok(false)
        })
    }

    fn soft_delete_admin(&self, id: AdminUserId) -> BoxFut<'_, Result<bool, StoreError>> {
        let admins = &self.admins;
        let deleted = &self.deleted_admins;
        Box::pin(async move {
            let exists = admins.read().map_err(lock_err)?.iter().any(|a| a.id == id);
            if !exists {
                return Ok(false);
            }
            let mut deleted = deleted.write().map_err(lock_err)?;
            if deleted.contains(&id) {
                return Ok(false);
            }
            deleted.push(id);
            Ok(true)
        })
    }
}

// ---------------------------------------------------------------------------
// FakeBillingStore — in-memory [`BillingStore`] for tests.
// ---------------------------------------------------------------------------

use server::domain::billing::entities::{BillingCycle, SubscriptionStatus};
use server::domain::billing::{NewPlan, UpdatePlan};
use server::ports::billing_store::{BillingStore, PlanRecord, SubscriptionRecord};

fn seed_plan(
    id: &str,
    name: &str,
    price_cents: Option<i64>,
    interval: &str,
    capabilities: serde_json::Value,
    yearly_discount: Option<serde_json::Value>,
) -> PlanRecord {
    let now = chrono::Utc::now();
    PlanRecord {
        id: id.to_string(),
        name: name.to_string(),
        price_cents,
        currency: "USD".to_string(),
        interval: interval.to_string(),
        capabilities: Some(capabilities),
        yearly_discount,
        is_active: true,
        created_at: now,
        updated_at: now,
    }
}

/// In-memory [`BillingStore`]. Plans seed to the catalog defaults;
/// subscriptions start empty (lazily ensured as free per project).
#[derive(Default)]
pub struct FakeBillingStore {
    plans: RwLock<Vec<PlanRecord>>,
    subs: RwLock<Vec<SubscriptionRecord>>,
    /// (user_id, project_id) pairs the actor may access.
    visible: RwLock<Vec<(String, String)>>,
    /// project_id -> project name, for admin subscriber views.
    projects: RwLock<Vec<(String, String)>>,
    /// project_id -> seeded birth timestamp, for history replay.
    /// Recorded at seed time (tests seed before acting), so events
    /// always postdate births as in production.
    born: RwLock<Vec<(String, chrono::DateTime<chrono::Utc>)>>,
    /// user_id -> (name, email), for admin subscriber views.
    users: RwLock<Vec<(String, String, String)>>,
}

impl FakeBillingStore {
    pub fn new() -> Self {
        let store = Self::default();
        let mut plans = store.plans.write().unwrap();
        plans.push(seed_plan(
            "free",
            "Free",
            Some(0),
            "month",
            serde_json::json!({"notificationsPerMonth": 1000, "channels": ["email"], "teamMembers": 1, "retentionDays": 7, "support": "Community", "branding": false, "apiCalls": 10000}),
            None,
        ));
        plans.push(seed_plan(
            "starter",
            "Starter",
            Some(1900),
            "month",
            serde_json::json!({"notificationsPerMonth": 10000, "channels": ["email", "sms", "push"], "teamMembers": 3, "retentionDays": 30, "support": "Email", "branding": false, "apiCalls": 100000}),
            Some(serde_json::json!({"kind": "percent", "value": 10})),
        ));
        plans.push(seed_plan(
            "pro",
            "Pro",
            Some(9900),
            "month",
            serde_json::json!({"notificationsPerMonth": 100000, "channels": ["all"], "teamMembers": 10, "retentionDays": 90, "support": "Priority", "branding": true, "apiCalls": null}),
            Some(serde_json::json!({"kind": "percent", "value": 20})),
        ));
        plans.push(seed_plan(
            "enterprise",
            "Enterprise",
            None,
            "custom",
            serde_json::json!({"notificationsPerMonth": null, "channels": ["all"], "teamMembers": null, "retentionDays": null, "support": "Dedicated", "branding": true, "apiCalls": null}),
            None,
        ));
        drop(plans);
        store
    }

    /// Grants `user_id` access to `project_id`.
    pub fn seed_visible(&self, user_id: &str, project_id: &str) {
        self.visible
            .write()
            .unwrap()
            .push((user_id.to_string(), project_id.to_string()));
        // Birth recorded at seed time: tests seed before acting, so
        // events always postdate births as in production.
        let mut born = self.born.write().unwrap();
        if !born.iter().any(|(id, _)| id == project_id) {
            born.push((project_id.to_string(), chrono::Utc::now()));
        }
    }

    /// Seeds a project's display name for admin subscriber views.
    pub fn seed_billing_project(&self, project_id: &str, name: &str) {
        self.projects
            .write()
            .unwrap()
            .push((project_id.to_string(), name.to_string()));
    }

    /// Seeds a customer's identity for admin subscriber views.
    pub fn seed_billing_user(&self, user_id: &str, name: &str, email: &str) {
        self.users.write().unwrap().push((
            user_id.to_string(),
            name.to_string(),
            email.to_string(),
        ));
    }

    fn is_visible(&self, user_id: &str, project_id: &str) -> bool {
        self.visible
            .read()
            .unwrap()
            .iter()
            .any(|(u, p)| u == user_id && p == project_id)
    }

    fn ordered(plans: Vec<PlanRecord>) -> Vec<PlanRecord> {
        let mut out = plans;
        out.sort_by(|a, b| match (a.price_cents, b.price_cents) {
            (None, None) => std::cmp::Ordering::Equal,
            (None, _) => std::cmp::Ordering::Greater,
            (_, None) => std::cmp::Ordering::Less,
            (Some(x), Some(y)) => x.cmp(&y).then(a.id.cmp(&b.id)),
        });
        out
    }
}

impl BillingStore for FakeBillingStore {
    fn project_visible(
        &self,
        actor: server::domain::auth::entities::UserId,
        project_id: &str,
    ) -> BoxFut<'_, Result<bool, StoreError>> {
        let visible = self.is_visible(&actor.to_string(), project_id);
        Box::pin(async move { Ok(visible) })
    }

    fn list_active_plans(&self) -> BoxFut<'_, Result<Vec<PlanRecord>, StoreError>> {
        let plans: Vec<PlanRecord> = self
            .plans
            .read()
            .unwrap()
            .iter()
            .filter(|p| p.is_active)
            .cloned()
            .collect();
        Box::pin(async move { Ok(Self::ordered(plans)) })
    }

    fn get_plan(&self, plan_id: &str) -> BoxFut<'_, Result<Option<PlanRecord>, StoreError>> {
        let plan_id = plan_id.to_string();
        let found = self
            .plans
            .read()
            .unwrap()
            .iter()
            .find(|p| p.id == plan_id)
            .cloned();
        Box::pin(async move { Ok(found) })
    }

    fn get_subscription(
        &self,
        project_id: &str,
    ) -> BoxFut<'_, Result<Option<SubscriptionRecord>, StoreError>> {
        let project_id = project_id.to_string();
        let found = self
            .subs
            .read()
            .unwrap()
            .iter()
            .find(|s| s.project_id == project_id)
            .cloned();
        Box::pin(async move { Ok(found) })
    }

    fn ensure_free_subscription(
        &self,
        project_id: &str,
    ) -> BoxFut<'_, Result<SubscriptionRecord, StoreError>> {
        let project_id = project_id.to_string();
        let subs = &self.subs;
        Box::pin(async move {
            let mut list = subs.write().map_err(lock_err)?;
            if let Some(existing) = list.iter().find(|s| s.project_id == project_id) {
                return Ok(existing.clone());
            }
            let now = chrono::Utc::now();
            let record = SubscriptionRecord {
                id: Ulid::new().to_string(),
                project_id: project_id.clone(),
                plan_id: "free".to_string(),
                status: SubscriptionStatus::Active,
                billing_cycle: BillingCycle::Monthly,
                cancel_at_period_end: false,
                period_start: now,
                period_end: now + chrono::Duration::days(30),
                created_at: now,
                updated_at: now,
            };
            list.push(record.clone());
            Ok(record)
        })
    }

    fn set_subscription(
        &self,
        project_id: &str,
        plan_id: &str,
        status: SubscriptionStatus,
        billing_cycle: BillingCycle,
        period_start: chrono::DateTime<chrono::Utc>,
        period_end: chrono::DateTime<chrono::Utc>,
    ) -> BoxFut<'_, Result<SubscriptionRecord, StoreError>> {
        let project_id = project_id.to_string();
        let plan_id = plan_id.to_string();
        let subs = &self.subs;
        Box::pin(async move {
            let mut list = subs.write().map_err(lock_err)?;
            if let Some(existing) = list.iter_mut().find(|s| s.project_id == project_id) {
                existing.plan_id = plan_id;
                existing.status = status;
                existing.billing_cycle = billing_cycle;
                existing.cancel_at_period_end = false;
                existing.period_start = period_start;
                existing.period_end = period_end;
                existing.updated_at = chrono::Utc::now();
                return Ok(existing.clone());
            }
            let now = chrono::Utc::now();
            let record = SubscriptionRecord {
                id: Ulid::new().to_string(),
                project_id,
                plan_id,
                status,
                billing_cycle,
                cancel_at_period_end: false,
                period_start,
                period_end,
                created_at: now,
                updated_at: now,
            };
            list.push(record.clone());
            Ok(record)
        })
    }

    fn set_subscription_status(
        &self,
        project_id: &str,
        status: SubscriptionStatus,
    ) -> BoxFut<'_, Result<Option<SubscriptionRecord>, StoreError>> {
        let project_id = project_id.to_string();
        let subs = &self.subs;
        Box::pin(async move {
            let mut list = subs.write().map_err(lock_err)?;
            Ok(list
                .iter_mut()
                .find(|s| s.project_id == project_id)
                .map(|s| {
                    s.status = status;
                    s.updated_at = chrono::Utc::now();
                    s.clone()
                }))
        })
    }

    fn set_cancel_at_period_end(
        &self,
        project_id: &str,
        cancel: bool,
    ) -> BoxFut<'_, Result<Option<SubscriptionRecord>, StoreError>> {
        let project_id = project_id.to_string();
        let subs = &self.subs;
        Box::pin(async move {
            let mut list = subs.write().map_err(lock_err)?;
            Ok(list
                .iter_mut()
                .find(|s| s.project_id == project_id)
                .map(|s| {
                    s.cancel_at_period_end = cancel;
                    s.updated_at = chrono::Utc::now();
                    s.clone()
                }))
        })
    }

    fn list_all_plans(&self) -> BoxFut<'_, Result<Vec<PlanRecord>, StoreError>> {
        let plans = self.plans.read().unwrap().clone();
        Box::pin(async move { Ok(Self::ordered(plans)) })
    }

    fn list_projects_with_created(
        &self,
    ) -> BoxFut<'_, Result<Vec<server::ports::billing_store::ProjectBirth>, StoreError>> {
        let born = self.born.read().unwrap().clone();
        let visible = self.visible.read().unwrap().clone();
        let projects = self.projects.read().unwrap().clone();
        Box::pin(async move {
            let mut ids: Vec<String> = visible.iter().map(|(_, p)| p.clone()).collect();
            for (id, _) in projects.iter() {
                if !ids.contains(id) {
                    ids.push(id.clone());
                }
            }
            // Unknown birth (never seeded) falls back to now; seeded
            // births predate test actions, as in production.
            let now = chrono::Utc::now();
            Ok(ids
                .into_iter()
                .map(|id| {
                    let at = born
                        .iter()
                        .find(|(b, _)| *b == id)
                        .map(|(_, at)| *at)
                        .unwrap_or(now);
                    (id, at)
                })
                .collect())
        })
    }

    fn count_active_subscribers(&self, plan_id: &str) -> BoxFut<'_, Result<i64, StoreError>> {
        let plan_id = plan_id.to_string();
        let count = self
            .subs
            .read()
            .unwrap()
            .iter()
            .filter(|s| {
                s.plan_id == plan_id
                    && matches!(
                        s.status,
                        SubscriptionStatus::Active | SubscriptionStatus::PastDue
                    )
            })
            .count() as i64;
        Box::pin(async move { Ok(count) })
    }

    fn create_plan(&self, id: &str, input: &NewPlan) -> BoxFut<'_, Result<PlanRecord, StoreError>> {
        let id = id.to_string();
        let name = input.name.clone();
        let price_cents = input.price_cents;
        let capabilities =
            serde_json::to_value(&input.capabilities).unwrap_or(serde_json::Value::Null);
        let yearly_discount = input
            .yearly_discount
            .as_ref()
            .and_then(|d| serde_json::to_value(d).ok());
        let plans = &self.plans;
        Box::pin(async move {
            let mut list = plans.write().map_err(lock_err)?;
            if list
                .iter()
                .any(|p| p.id == id || p.name.to_lowercase() == name.to_lowercase())
            {
                return Err(StoreError::Conflict);
            }
            let now = chrono::Utc::now();
            let record = PlanRecord {
                id,
                name,
                price_cents,
                currency: "USD".to_string(),
                interval: if price_cents.is_none() {
                    "custom".to_string()
                } else {
                    "month".to_string()
                },
                capabilities: Some(capabilities),
                yearly_discount,
                is_active: true,
                created_at: now,
                updated_at: now,
            };
            list.push(record.clone());
            Ok(record)
        })
    }

    fn update_plan(
        &self,
        plan_id: &str,
        input: &UpdatePlan,
    ) -> BoxFut<'_, Result<Option<PlanRecord>, StoreError>> {
        let plan_id = plan_id.to_string();
        let name = input.name.clone();
        let price_cents = input.price_cents;
        let capabilities =
            serde_json::to_value(&input.capabilities).unwrap_or(serde_json::Value::Null);
        let yearly_discount = input
            .yearly_discount
            .as_ref()
            .and_then(|d| serde_json::to_value(d).ok());
        let is_active = input.is_active;
        let plans = &self.plans;
        Box::pin(async move {
            let mut list = plans.write().map_err(lock_err)?;
            if list
                .iter()
                .any(|p| p.id != plan_id && p.name.to_lowercase() == name.to_lowercase())
            {
                return Err(StoreError::Conflict);
            }
            Ok(list.iter_mut().find(|p| p.id == plan_id).map(|p| {
                p.name = name.clone();
                p.price_cents = price_cents;
                p.interval = if price_cents.is_none() {
                    "custom".to_string()
                } else {
                    "month".to_string()
                };
                p.capabilities = Some(capabilities.clone());
                p.yearly_discount = yearly_discount.clone();
                p.is_active = is_active;
                p.updated_at = chrono::Utc::now();
                p.clone()
            }))
        })
    }

    fn delete_plan(&self, plan_id: &str) -> BoxFut<'_, Result<bool, StoreError>> {
        let plan_id = plan_id.to_string();
        let plans = &self.plans;
        Box::pin(async move {
            let mut list = plans.write().map_err(lock_err)?;
            let before = list.len();
            list.retain(|p| p.id != plan_id);
            Ok(list.len() != before)
        })
    }

    fn list_subscribers(
        &self,
        plan_id: &str,
    ) -> BoxFut<'_, Result<Vec<server::ports::billing_store::SubscriberRecord>, StoreError>> {
        use server::domain::billing::entities::SubscriptionStatus;
        let plan_id = plan_id.to_string();
        let subs = self.subs.read().unwrap().clone();
        let projects = self.projects.read().unwrap().clone();
        let users = self.users.read().unwrap().clone();
        // Owner lookup mirrors the Postgres join: the first visible actor
        // recorded for the project stands in for its creator.
        let visible = self.visible.read().unwrap().clone();
        Box::pin(async move {
            let mut out = Vec::new();
            for s in subs.into_iter().filter(|s| {
                s.plan_id == plan_id
                    && matches!(
                        s.status,
                        SubscriptionStatus::Active | SubscriptionStatus::PastDue
                    )
            }) {
                let project_name = projects
                    .iter()
                    .find(|(id, _)| *id == s.project_id)
                    .map(|(_, name)| name.clone())
                    .unwrap_or_else(|| s.project_id.clone());
                let owner = visible.iter().find(|(_, p)| *p == s.project_id);
                let (customer_name, customer_email) = owner
                    .and_then(|(u, _)| {
                        users
                            .iter()
                            .find(|(id, _, _)| id == u)
                            .map(|(_, n, e)| (Some(n.clone()), Some(e.clone())))
                    })
                    .unwrap_or((None, None));
                out.push(server::ports::billing_store::SubscriberRecord {
                    subscription: s,
                    project_name,
                    customer_name,
                    customer_email,
                });
            }
            Ok(out)
        })
    }
}

// ---------------------------------------------------------------------------
// FakeMailer + FakeTemplates — in-memory system mail for tests.
// ---------------------------------------------------------------------------

use server::ports::mailer::{
    BoxFut as MailBoxFut, MailError, RenderedTemplate, SmtpMailer, SystemSender, SystemTemplates,
};

/// A queued system email, for assertions.
#[derive(Debug, Clone)]
pub struct SentMail {
    pub from: SystemSender,
    pub to: String,
    pub subject: String,
    pub text: String,
    pub html: Option<String>,
}

/// In-memory [`SmtpMailer`] that records sends instead of delivering.
#[derive(Default)]
pub struct FakeMailer {
    sent: RwLock<Vec<SentMail>>,
}

impl FakeMailer {
    pub fn new() -> Self {
        Self::default()
    }

    /// Everything queued so far, oldest first.
    pub fn sent(&self) -> Vec<SentMail> {
        self.sent.read().unwrap().clone()
    }
}

impl SmtpMailer for FakeMailer {
    fn send(
        &self,
        from: SystemSender,
        to: &str,
        subject: &str,
        text: &str,
        html: Option<&str>,
    ) -> MailBoxFut<'_, Result<String, MailError>> {
        let to = to.to_string();
        let subject = subject.to_string();
        let text = text.to_string();
        let html = html.map(str::to_string);
        let sent = &self.sent;
        Box::pin(async move {
            sent.write()
                .map_err(|_| MailError::Transport("lock poisoned".to_string()))?
                .push(SentMail {
                    from,
                    to,
                    subject,
                    text,
                    html,
                });
            Ok("fake-message-id".to_string())
        })
    }
}

/// In-memory [`SystemTemplates`] with canned renders per name.
#[derive(Default)]
pub struct FakeTemplates {
    renders: RwLock<HashMap<String, RenderedTemplate>>,
}

impl FakeTemplates {
    pub fn new() -> Self {
        Self::default()
    }

    /// Seeds the render returned for `name`.
    pub fn seed(&self, name: &str, subject: &str, html: &str, text: &str) {
        self.renders.write().unwrap().insert(
            name.to_string(),
            RenderedTemplate {
                subject: subject.to_string(),
                html: html.to_string(),
                text: text.to_string(),
            },
        );
    }
}

impl SystemTemplates for FakeTemplates {
    fn render(&self, name: &str, _vars: &[(&str, &str)]) -> Result<RenderedTemplate, MailError> {
        self.renders
            .read()
            .unwrap()
            .get(name)
            .cloned()
            .ok_or_else(|| MailError::Template(format!("unknown template: {name}")))
    }
}

/// Thread-safe in-memory flow store.
#[derive(Default)]
pub struct FakeFlowStore {
    flows: RwLock<Vec<server::ports::flow_store::FlowRecord>>,
    visible: RwLock<Vec<(String, String)>>,
}

impl FakeFlowStore {
    pub fn new() -> Self {
        Self::default()
    }

    /// Grants `user_id` access to `project_id`.
    pub fn seed_visible(&self, user_id: &str, project_id: &str) {
        self.visible
            .write()
            .unwrap()
            .push((user_id.to_string(), project_id.to_string()));
    }

    fn is_visible(&self, user_id: &str, project_id: &str) -> bool {
        self.visible
            .read()
            .unwrap()
            .iter()
            .any(|(u, p)| u == user_id && p == project_id)
    }
}

impl server::ports::flow_store::FlowStore for FakeFlowStore {
    fn project_visible(
        &self,
        actor: server::domain::auth::entities::UserId,
        project_id: &str,
    ) -> BoxFut<'_, Result<bool, StoreError>> {
        let visible = self.is_visible(&actor.to_string(), project_id);
        Box::pin(async move { Ok(visible) })
    }

    fn list_by_project(
        &self,
        project_id: &str,
    ) -> BoxFut<'_, Result<Vec<server::ports::flow_store::FlowRecord>, StoreError>> {
        let mut rows: Vec<_> = self
            .flows
            .read()
            .unwrap()
            .iter()
            .filter(|f| f.project_id == project_id)
            .cloned()
            .collect();
        rows.sort_by(|a, b| a.created_at.cmp(&b.created_at).then(a.id.cmp(&b.id)));
        Box::pin(async move { Ok(rows) })
    }

    fn get(
        &self,
        id: &str,
    ) -> BoxFut<'_, Result<Option<server::ports::flow_store::FlowRecord>, StoreError>> {
        let found = self
            .flows
            .read()
            .unwrap()
            .iter()
            .find(|f| f.id == id)
            .cloned();
        Box::pin(async move { Ok(found) })
    }

    fn create(
        &self,
        project_id: &str,
        name: &str,
        definition: &serde_json::Value,
    ) -> BoxFut<'_, Result<server::ports::flow_store::FlowRecord, StoreError>> {
        use server::ports::flow_store::FlowRecord;
        let now = chrono::Utc::now();
        let record = FlowRecord {
            id: Ulid::new().to_string(),
            project_id: project_id.to_string(),
            name: name.to_string(),
            definition: definition.clone(),
            version: 1,
            created_at: now,
            updated_at: now,
        };
        self.flows.write().unwrap().push(record.clone());
        Box::pin(async move { Ok(record) })
    }

    fn update(
        &self,
        id: &str,
        name: &str,
        definition: &serde_json::Value,
    ) -> BoxFut<'_, Result<Option<server::ports::flow_store::FlowRecord>, StoreError>> {
        let mut flows = self.flows.write().unwrap();
        let updated = flows.iter_mut().find(|f| f.id == id).map(|f| {
            f.name = name.to_string();
            f.definition = definition.clone();
            f.version += 1;
            f.updated_at = chrono::Utc::now();
            f.clone()
        });
        Box::pin(async move { Ok(updated) })
    }

    fn delete(&self, id: &str) -> BoxFut<'_, Result<bool, StoreError>> {
        let deleted = {
            let mut flows = self.flows.write().unwrap();
            let before = flows.len();
            flows.retain(|f| f.id != id);
            flows.len() != before
        };
        Box::pin(async move { Ok(deleted) })
    }
}

/// Thread-safe in-memory send store.
#[derive(Default)]
pub struct FakeSendStore {
    sends: RwLock<Vec<server::ports::send_store::SendRecord>>,
    visible: RwLock<Vec<(String, String)>>,
}

impl FakeSendStore {
    pub fn new() -> Self {
        Self::default()
    }

    /// Grants `user_id` access to `project_id`.
    pub fn seed_visible(&self, user_id: &str, project_id: &str) {
        self.visible
            .write()
            .unwrap()
            .push((user_id.to_string(), project_id.to_string()));
    }

    fn is_visible(&self, user_id: &str, project_id: &str) -> bool {
        self.visible
            .read()
            .unwrap()
            .iter()
            .any(|(u, p)| u == user_id && p == project_id)
    }

    /// The full contents, oldest first (for assertions).
    pub fn all(&self) -> Vec<server::ports::send_store::SendRecord> {
        let mut all = self.sends.read().unwrap().clone();
        all.sort_by_key(|s| (s.created_at, s.id.clone()));
        all
    }
}

impl server::ports::send_store::SendStore for FakeSendStore {
    fn project_visible(
        &self,
        actor: server::domain::auth::entities::UserId,
        project_id: &str,
    ) -> BoxFut<'_, Result<bool, StoreError>> {
        let visible = self.is_visible(&actor.to_string(), project_id);
        Box::pin(async move { Ok(visible) })
    }

    fn create(
        &self,
        project_id: &str,
        channel: &str,
        recipients: &serde_json::Value,
        title: &str,
        body: &str,
        send_at: DateTime<Utc>,
    ) -> BoxFut<'_, Result<server::ports::send_store::SendRecord, StoreError>> {
        use server::ports::send_store::SendRecord;
        let now = chrono::Utc::now();
        let record = SendRecord {
            id: Ulid::new().to_string(),
            project_id: project_id.to_string(),
            channel: channel.to_string(),
            recipients: recipients.clone(),
            title: title.to_string(),
            body: body.to_string(),
            send_at,
            status: "sending".to_string(),
            attempts: 0,
            provider: None,
            external_id: None,
            last_error: None,
            campaign_id: None,
            created_at: now,
            updated_at: now,
        };
        self.sends.write().unwrap().push(record.clone());
        Box::pin(async move { Ok(record) })
    }

    fn mark_completed(
        &self,
        id: &str,
        provider: &str,
        external_id: Option<&str>,
    ) -> BoxFut<'_, Result<Option<server::ports::send_store::SendRecord>, StoreError>> {
        let mut sends = self.sends.write().unwrap();
        let updated = sends.iter_mut().find(|s| s.id == id).map(|s| {
            s.status = "completed".to_string();
            s.provider = Some(provider.to_string());
            s.external_id = external_id.map(str::to_string);
            s.updated_at = chrono::Utc::now();
            s.clone()
        });
        Box::pin(async move { Ok(updated) })
    }

    fn mark_failed(
        &self,
        id: &str,
        last_error: &str,
    ) -> BoxFut<'_, Result<Option<server::ports::send_store::SendRecord>, StoreError>> {
        let mut sends = self.sends.write().unwrap();
        let updated = sends.iter_mut().find(|s| s.id == id).map(|s| {
            s.status = "failed".to_string();
            s.attempts += 1;
            s.last_error = Some(last_error.to_string());
            s.updated_at = chrono::Utc::now();
            s.clone()
        });
        Box::pin(async move { Ok(updated) })
    }

    fn list_by_project(
        &self,
        project_id: &str,
        limit: i64,
    ) -> BoxFut<'_, Result<Vec<server::ports::send_store::SendRecord>, StoreError>> {
        let mut rows: Vec<_> = self
            .sends
            .read()
            .unwrap()
            .iter()
            .filter(|s| s.project_id == project_id)
            .cloned()
            .collect();
        rows.sort_by_key(|e| Reverse(e.created_at));
        rows.truncate(limit.max(0) as usize);
        Box::pin(async move { Ok(rows) })
    }

    fn create_for_campaign(
        &self,
        project_id: &str,
        campaign_id: &str,
        channel: &str,
        recipient: &str,
        title: &str,
        body: &str,
    ) -> BoxFut<'_, Result<server::ports::send_store::SendRecord, StoreError>> {
        use server::ports::send_store::SendRecord;
        let now = chrono::Utc::now();
        let record = SendRecord {
            id: Ulid::new().to_string(),
            project_id: project_id.to_string(),
            channel: channel.to_string(),
            recipients: serde_json::json!([recipient]),
            title: title.to_string(),
            body: body.to_string(),
            send_at: now,
            status: "pending".to_string(),
            attempts: 0,
            provider: None,
            external_id: None,
            last_error: None,
            campaign_id: Some(campaign_id.to_string()),
            created_at: now,
            updated_at: now,
        };
        self.sends.write().unwrap().push(record.clone());
        Box::pin(async move { Ok(record) })
    }

    fn list_by_campaign(
        &self,
        campaign_id: &str,
    ) -> BoxFut<'_, Result<Vec<server::ports::send_store::SendRecord>, StoreError>> {
        let mut rows: Vec<_> = self
            .sends
            .read()
            .unwrap()
            .iter()
            .filter(|s| s.campaign_id == Some(campaign_id.to_string()))
            .cloned()
            .collect();
        rows.sort_by_key(|e| (e.created_at, e.id.clone()));
        Box::pin(async move { Ok(rows) })
    }

    fn delete_pending_by_campaign(&self, campaign_id: &str) -> BoxFut<'_, Result<i64, StoreError>> {
        let removed;
        {
            let mut sends = self.sends.write().unwrap();
            let before = sends.len();
            sends.retain(|s| {
                !(s.campaign_id.as_deref() == Some(campaign_id) && s.status == "pending")
            });
            removed = (before - sends.len()) as i64;
        }
        Box::pin(async move { Ok(removed) })
    }
}

/// Thread-safe in-memory channel provider config store.
#[derive(Default)]
pub struct FakeChannelProviderStore {
    configs: RwLock<Vec<server::domain::channels::ProjectProviderConfig>>,
    visible: RwLock<Vec<(String, String)>>,
}

impl FakeChannelProviderStore {
    pub fn new() -> Self {
        Self::default()
    }

    /// Grants `user_id` access to `project_id` (visibility is checked
    /// separately by services; the fake mirrors the rule loosely).
    pub fn seed_visible(&self, user_id: &str, project_id: &str) {
        self.visible
            .write()
            .unwrap()
            .push((user_id.to_string(), project_id.to_string()));
    }

    /// Seeds an enabled provider config for a project channel.
    pub fn seed_config(&self, project_id: &str, channel_id: &str, provider_id: &str) {
        use server::domain::channels::ProjectProviderConfig;
        let now = chrono::Utc::now();
        self.configs.write().unwrap().push(ProjectProviderConfig {
            id: Ulid::new().to_string(),
            project_id: project_id.to_string(),
            channel_id: channel_id.to_string(),
            provider_id: provider_id.to_string(),
            config: serde_json::json!({}),
            smtp_fallback: None,
            enabled: true,
            created_at: now,
            updated_at: now,
        });
    }
}

#[async_trait::async_trait]
impl server::ports::channel_provider_store::ChannelProviderStore for FakeChannelProviderStore {
    async fn list_by_project(
        &self,
        project_id: &str,
    ) -> Result<Vec<server::domain::channels::ProjectProviderConfig>, String> {
        Ok(self
            .configs
            .read()
            .unwrap()
            .iter()
            .filter(|c| c.project_id == project_id)
            .cloned()
            .collect())
    }

    async fn get(
        &self,
        id: &str,
    ) -> Result<Option<server::domain::channels::ProjectProviderConfig>, String> {
        Ok(self
            .configs
            .read()
            .unwrap()
            .iter()
            .find(|c| c.id == id)
            .cloned())
    }

    async fn get_by_project_channel_provider(
        &self,
        project_id: &str,
        channel_id: &str,
        provider_id: &str,
    ) -> Result<Option<server::domain::channels::ProjectProviderConfig>, String> {
        Ok(self
            .configs
            .read()
            .unwrap()
            .iter()
            .find(|c| {
                c.project_id == project_id
                    && c.channel_id == channel_id
                    && c.provider_id == provider_id
            })
            .cloned())
    }

    async fn create(
        &self,
        input: server::domain::channels::ProjectProviderConfigInput,
        project_id: &str,
    ) -> Result<server::domain::channels::ProjectProviderConfig, String> {
        use server::domain::channels::ProjectProviderConfig;
        let now = chrono::Utc::now();
        let record = ProjectProviderConfig {
            id: Ulid::new().to_string(),
            project_id: project_id.to_string(),
            channel_id: input.channel_id,
            provider_id: input.provider_id,
            config: input.config,
            smtp_fallback: None,
            enabled: input.enabled,
            created_at: now,
            updated_at: now,
        };
        self.configs.write().unwrap().push(record.clone());
        Ok(record)
    }

    async fn update(
        &self,
        id: &str,
        input: server::domain::channels::ProjectProviderConfigInput,
    ) -> Result<server::domain::channels::ProjectProviderConfig, String> {
        let mut configs = self.configs.write().unwrap();
        let found = configs.iter_mut().find(|c| c.id == id).map(|c| {
            c.channel_id = input.channel_id.clone();
            c.provider_id = input.provider_id.clone();
            c.config = input.config.clone();
            c.enabled = input.enabled;
            c.updated_at = chrono::Utc::now();
            c.clone()
        });
        found.ok_or_else(|| "provider config not found".to_string())
    }

    async fn delete(&self, id: &str) -> Result<(), String> {
        self.configs.write().unwrap().retain(|c| c.id != id);
        Ok(())
    }

    async fn delete_by_project(&self, project_id: &str) -> Result<(), String> {
        self.configs
            .write()
            .unwrap()
            .retain(|c| c.project_id != project_id);
        Ok(())
    }
}

/// In-memory schedule store for HTTP tests.
#[derive(Default)]
/// Thread-safe in-memory schedule store. The flow JOIN reads **live**
/// from a shared [`FakeFlowStore`] (mirrors the Postgres JOIN, so
/// owner-created flows resolve without manual seeding).
pub struct FakeScheduleStore {
    schedules: RwLock<Vec<server::ports::schedule_store::ScheduleRecord>>,
    visible: RwLock<Vec<(String, String)>>,
    /// Shared with the app wiring — the Postgres flow JOIN, in-memory.
    flow_store: Arc<FakeFlowStore>,
    /// Joined template snapshots, mirroring the Postgres template JOIN.
    templates: RwLock<HashMap<String, (String, serde_json::Value)>>,
}

impl FakeScheduleStore {
    pub fn new(flow_store: Arc<FakeFlowStore>) -> Self {
        Self {
            schedules: RwLock::new(Vec::new()),
            visible: RwLock::new(Vec::new()),
            flow_store,
            templates: RwLock::new(HashMap::new()),
        }
    }

    /// Grants `user_id` access to `project_id`.
    pub fn seed_visible(&self, user_id: &str, project_id: &str) {
        self.visible
            .write()
            .unwrap()
            .push((user_id.to_string(), project_id.to_string()));
    }

    /// Seeds the joined template row (name + content) for `template_id`.
    pub fn seed_template(&self, template_id: &str, name: &str, content: serde_json::Value) {
        self.templates
            .write()
            .unwrap()
            .insert(template_id.to_string(), (name.to_string(), content));
    }

    fn is_visible(&self, user_id: &str, project_id: &str) -> bool {
        self.visible
            .read()
            .unwrap()
            .iter()
            .any(|(u, p)| u == user_id && p == project_id)
    }

    /// Re-resolves the flow JOIN columns (name + definition) from the
    /// shared flow store, mirroring the live Postgres JOIN.
    async fn resolve_join(
        &self,
        mut record: server::ports::schedule_store::ScheduleRecord,
    ) -> server::ports::schedule_store::ScheduleRecord {
        if let Ok(Some(flow)) = self.flow_store.get(&record.flow_id).await {
            record.flow_name = flow.name;
            record.flow_definition = flow.definition;
        }
        record
    }
}

impl server::ports::schedule_store::ScheduleStore for FakeScheduleStore {
    fn project_visible(
        &self,
        actor: server::domain::auth::entities::UserId,
        project_id: &str,
    ) -> BoxFut<'_, Result<bool, StoreError>> {
        let visible = self.is_visible(&actor.to_string(), project_id);
        Box::pin(async move { Ok(visible) })
    }

    fn list_by_project(
        &self,
        project_id: &str,
    ) -> BoxFut<'_, Result<Vec<server::ports::schedule_store::ScheduleRecord>, StoreError>> {
        let mut rows: Vec<_> = self
            .schedules
            .read()
            .unwrap()
            .iter()
            .filter(|s| s.project_id == project_id)
            .cloned()
            .collect();
        rows.sort_by(|a, b| a.created_at.cmp(&b.created_at).then(a.id.cmp(&b.id)));
        Box::pin(async move {
            for row in rows.iter_mut() {
                *row = self.resolve_join(row.clone()).await;
            }
            Ok(rows)
        })
    }

    fn get(
        &self,
        id: &str,
    ) -> BoxFut<'_, Result<Option<server::ports::schedule_store::ScheduleRecord>, StoreError>> {
        let row = self
            .schedules
            .read()
            .unwrap()
            .iter()
            .find(|s| s.id == id)
            .cloned();
        Box::pin(async move {
            match row {
                Some(record) => Ok(Some(self.resolve_join(record).await)),
                None => Ok(None),
            }
        })
    }

    fn scheduled_flow_ids(&self, project_id: &str) -> BoxFut<'_, Result<Vec<String>, StoreError>> {
        let mut ids: Vec<String> = self
            .schedules
            .read()
            .unwrap()
            .iter()
            .filter(|s| s.project_id == project_id)
            .map(|s| s.flow_id.clone())
            .collect();
        ids.sort();
        ids.dedup();
        Box::pin(async move { Ok(ids) })
    }

    fn create(
        &self,
        draft: &server::ports::schedule_store::NewSchedule,
    ) -> BoxFut<'_, Result<server::ports::schedule_store::ScheduleRecord, StoreError>> {
        use server::ports::schedule_store::ScheduleRecord;
        let now = chrono::Utc::now();
        let (template_name, template_content) = draft
            .template_id
            .as_deref()
            .and_then(|tid| {
                self.templates
                    .read()
                    .unwrap()
                    .get(tid)
                    .cloned()
                    .map(|(n, c)| (Some(n), Some(c)))
            })
            .unwrap_or((None, None));
        let flow_store = self.flow_store.clone();
        let draft = draft.clone();
        Box::pin(async move {
            let (flow_name, flow_definition) = flow_store
                .get(&draft.flow_id)
                .await
                .map(|f| f.map(|f| (f.name, f.definition)).unwrap_or_default())
                .unwrap_or_default();
            let record = ScheduleRecord {
                id: Ulid::new().to_string(),
                project_id: draft.project_id.clone(),
                name: draft.name.clone(),
                flow_id: draft.flow_id.clone(),
                flow_name,
                flow_definition,
                kind: draft.kind.clone(),
                run_at: draft.run_at,
                interval_secs: draft.interval_secs,
                days: draft.days.clone(),
                run_time: draft.run_time.clone(),
                timezone: draft.timezone.clone(),
                template_id: draft.template_id.clone(),
                template_name,
                template_content,
                channel: draft.channel.clone(),
                title: draft.title.clone(),
                body: draft.body.clone(),
                recipients: draft.recipients.clone(),
                active: draft.active,
                archived_at: draft.archived_at,
                next_run_at: draft.next_run_at,
                last_run_at: None,
                last_status: None,
                last_error: None,
                run_count: 0,
                created_at: now,
                updated_at: now,
            };
            self.schedules.write().unwrap().push(record.clone());
            Ok(record)
        })
    }

    fn update(
        &self,
        id: &str,
        patch: &server::ports::schedule_store::ScheduleUpdate,
    ) -> BoxFut<'_, Result<Option<server::ports::schedule_store::ScheduleRecord>, StoreError>> {
        let mut schedules = self.schedules.write().unwrap();
        let updated = schedules.iter_mut().find(|s| s.id == id).map(|s| {
            s.name = patch.name.clone();
            s.flow_id = patch.flow_id.clone();
            s.kind = patch.kind.clone();
            s.run_at = patch.run_at;
            s.interval_secs = patch.interval_secs;
            s.days = patch.days.clone();
            s.run_time = patch.run_time.clone();
            s.timezone = patch.timezone.clone();
            s.template_id = patch.template_id.clone();
            s.channel = patch.channel.clone();
            s.title = patch.title.clone();
            s.body = patch.body.clone();
            s.recipients = patch.recipients.clone();
            s.active = patch.active;
            s.archived_at = patch.archived_at;
            s.next_run_at = patch.next_run_at;
            s.updated_at = chrono::Utc::now();
            s.clone()
        });
        // Mirror the Postgres UPDATE ... JOIN flows: the joined columns
        // resolve live (so service-side flow renames show through).
        Box::pin(async move {
            match updated {
                Some(record) => Ok(Some(self.resolve_join(record).await)),
                None => Ok(None),
            }
        })
    }

    fn delete(&self, id: &str) -> BoxFut<'_, Result<bool, StoreError>> {
        let mut schedules = self.schedules.write().unwrap();
        let before = schedules.len();
        schedules.retain(|s| s.id != id);
        let deleted = schedules.len() < before;
        drop(schedules);
        Box::pin(async move { Ok(deleted) })
    }

    fn claim_due(
        &self,
        now: chrono::DateTime<chrono::Utc>,
        limit: i64,
    ) -> BoxFut<'_, Result<Vec<server::ports::schedule_store::ScheduleRecord>, StoreError>> {
        let mut schedules = self.schedules.write().unwrap();
        let mut claimed = Vec::new();
        let mut ids: Vec<String> = schedules
            .iter()
            .filter(|s| {
                s.active && s.archived_at.is_none() && s.next_run_at.is_some_and(|n| n <= now)
            })
            .map(|s| (s.next_run_at, s.id.clone()))
            .collect::<Vec<_>>()
            .into_iter()
            .take(limit.max(0) as usize)
            .map(|(_, id)| id)
            .collect();
        ids.sort();
        for schedule in schedules.iter_mut() {
            if ids.contains(&schedule.id) {
                schedule.last_run_at = Some(now);
                schedule.updated_at = now;
                claimed.push(schedule.clone());
            }
        }
        claimed.sort_by(|a, b| a.next_run_at.cmp(&b.next_run_at).then(a.id.cmp(&b.id)));
        Box::pin(async move { Ok(claimed) })
    }

    fn mark_run(
        &self,
        id: &str,
        last_status: &str,
        last_error: Option<&str>,
        next_run_at: Option<chrono::DateTime<chrono::Utc>>,
        active: bool,
    ) -> BoxFut<'_, Result<Option<server::ports::schedule_store::ScheduleRecord>, StoreError>> {
        let mut schedules = self.schedules.write().unwrap();
        let updated = schedules.iter_mut().find(|s| s.id == id).map(|s| {
            s.last_status = Some(last_status.to_string());
            s.last_error = last_error.map(str::to_string);
            s.next_run_at = next_run_at;
            s.active = active;
            s.run_count += 1;
            if s.last_run_at.is_none() {
                s.last_run_at = Some(chrono::Utc::now());
            }
            s.updated_at = chrono::Utc::now();
            s.clone()
        });
        Box::pin(async move { Ok(updated) })
    }
}

/// Thread-safe in-memory webhook store. The flow JOIN reads **live**
/// from a shared [`FakeFlowStore`] (mirrors the Postgres JOIN, so
/// owner-managed renames show up on read).
pub struct FakeWebhookStore {
    webhooks: RwLock<Vec<server::ports::webhook_store::WebhookRecord>>,
    visible: RwLock<Vec<(String, String)>>,
    /// Shared with the app wiring — the Postgres flow JOIN, in-memory.
    flow_store: Arc<FakeFlowStore>,
}

impl FakeWebhookStore {
    pub fn new(flow_store: Arc<FakeFlowStore>) -> Self {
        Self {
            webhooks: RwLock::new(Vec::new()),
            visible: RwLock::new(Vec::new()),
            flow_store,
        }
    }

    /// Grants `user_id` access to `project_id`.
    pub fn seed_visible(&self, user_id: &str, project_id: &str) {
        self.visible
            .write()
            .unwrap()
            .push((user_id.to_string(), project_id.to_string()));
    }

    fn is_visible(&self, user_id: &str, project_id: &str) -> bool {
        self.visible
            .read()
            .unwrap()
            .iter()
            .any(|(u, p)| u == user_id && p == project_id)
    }

    /// Re-resolves the flow JOIN column (name) from the shared flow
    /// store, mirroring the live Postgres JOIN.
    async fn resolve_join(
        &self,
        mut record: server::ports::webhook_store::WebhookRecord,
    ) -> server::ports::webhook_store::WebhookRecord {
        if let Ok(Some(flow)) = self.flow_store.get(&record.flow_id).await {
            record.flow_name = flow.name;
        }
        record
    }
}

impl server::ports::webhook_store::WebhookStore for FakeWebhookStore {
    fn project_visible(
        &self,
        actor: server::domain::auth::entities::UserId,
        project_id: &str,
    ) -> BoxFut<'_, Result<bool, StoreError>> {
        let visible = self.is_visible(&actor.to_string(), project_id);
        Box::pin(async move { Ok(visible) })
    }

    fn list_by_project(
        &self,
        project_id: &str,
    ) -> BoxFut<'_, Result<Vec<server::ports::webhook_store::WebhookRecord>, StoreError>> {
        let mut rows: Vec<_> = self
            .webhooks
            .read()
            .unwrap()
            .iter()
            .filter(|w| w.project_id == project_id)
            .cloned()
            .collect();
        rows.sort_by(|a, b| a.created_at.cmp(&b.created_at).then(a.id.cmp(&b.id)));
        Box::pin(async move {
            for row in rows.iter_mut() {
                *row = self.resolve_join(row.clone()).await;
            }
            Ok(rows)
        })
    }

    fn get(
        &self,
        id: &str,
    ) -> BoxFut<'_, Result<Option<server::ports::webhook_store::WebhookRecord>, StoreError>> {
        let row = self
            .webhooks
            .read()
            .unwrap()
            .iter()
            .find(|w| w.id == id)
            .cloned();
        Box::pin(async move {
            match row {
                Some(record) => Ok(Some(self.resolve_join(record).await)),
                None => Ok(None),
            }
        })
    }

    fn owned_flow_ids(&self, project_id: &str) -> BoxFut<'_, Result<Vec<String>, StoreError>> {
        let mut ids: Vec<String> = self
            .webhooks
            .read()
            .unwrap()
            .iter()
            .filter(|w| w.project_id == project_id)
            .map(|w| w.flow_id.clone())
            .collect();
        ids.sort();
        ids.dedup();
        Box::pin(async move { Ok(ids) })
    }

    fn create(
        &self,
        draft: &server::ports::webhook_store::NewWebhook,
    ) -> BoxFut<'_, Result<server::ports::webhook_store::WebhookRecord, StoreError>> {
        use server::ports::webhook_store::WebhookRecord;
        let now = chrono::Utc::now();
        let flow_store = self.flow_store.clone();
        let draft = draft.clone();
        Box::pin(async move {
            // Mirror the Postgres create: the flow JOIN resolves the name.
            let flow_name = flow_store
                .get(&draft.flow_id)
                .await
                .ok()
                .flatten()
                .map(|f| f.name)
                .unwrap_or_default();
            let record = WebhookRecord {
                id: Ulid::new().to_string(),
                project_id: draft.project_id.clone(),
                name: draft.name.clone(),
                description: draft.description.clone(),
                flow_id: draft.flow_id.clone(),
                flow_name,
                events: draft.events.clone(),
                active: draft.active,
                archived_at: None,
                created_at: now,
                updated_at: now,
            };
            self.webhooks.write().unwrap().push(record.clone());
            Ok(record)
        })
    }

    fn update(
        &self,
        id: &str,
        patch: &server::ports::webhook_store::WebhookUpdate,
    ) -> BoxFut<'_, Result<Option<server::ports::webhook_store::WebhookRecord>, StoreError>> {
        let mut webhooks = self.webhooks.write().unwrap();
        let updated = webhooks.iter_mut().find(|w| w.id == id).map(|w| {
            w.name = patch.name.clone();
            w.description = patch.description.clone();
            w.events = patch.events.clone();
            w.active = patch.active;
            w.archived_at = patch.archived_at;
            w.updated_at = chrono::Utc::now();
            w.clone()
        });
        drop(webhooks);
        // Mirror the Postgres UPDATE ... JOIN flows: the joined flow name
        // resolves live (so service-side flow renames show through).
        Box::pin(async move {
            match updated {
                Some(record) => Ok(Some(self.resolve_join(record).await)),
                None => Ok(None),
            }
        })
    }

    fn delete(&self, id: &str) -> BoxFut<'_, Result<bool, StoreError>> {
        let mut webhooks = self.webhooks.write().unwrap();
        let before = webhooks.len();
        webhooks.retain(|w| w.id != id);
        let deleted = webhooks.len() < before;
        drop(webhooks);
        Box::pin(async move { Ok(deleted) })
    }
}

/// In-memory campaigns store: plain CRUD + lifecycle transitions +
/// claim-due mechanics, mirroring the Postgres semantics.
#[derive(Default)]
pub struct FakeCampaignStore {
    campaigns: RwLock<Vec<server::ports::campaigns_store::CampaignRecord>>,
    visible: RwLock<Vec<(String, String)>>,
}

impl FakeCampaignStore {
    pub fn new() -> Self {
        Self::default()
    }

    /// Grants `user_id` access to `project_id`.
    pub fn seed_visible(&self, user_id: &str, project_id: &str) {
        self.visible
            .write()
            .unwrap()
            .push((user_id.to_string(), project_id.to_string()));
    }

    /// The full contents (for assertions).
    pub fn all(&self) -> Vec<server::ports::campaigns_store::CampaignRecord> {
        let mut all = self.campaigns.read().unwrap().clone();
        all.sort_by_key(|c| c.created_at);
        all
    }
}

impl server::ports::campaigns_store::CampaignStore for FakeCampaignStore {
    fn project_visible(
        &self,
        actor: server::domain::auth::entities::UserId,
        project_id: &str,
    ) -> BoxFut<'_, Result<bool, StoreError>> {
        let visible = self
            .visible
            .read()
            .unwrap()
            .iter()
            .any(|(u, p)| u.as_str() == actor.to_string().as_str() && p.as_str() == project_id);
        Box::pin(async move { Ok(visible) })
    }

    fn list_by_project(
        &self,
        project_id: &str,
    ) -> BoxFut<'_, Result<Vec<server::ports::campaigns_store::CampaignRecord>, StoreError>> {
        let mut rows: Vec<_> = self
            .campaigns
            .read()
            .unwrap()
            .iter()
            .filter(|c| c.project_id == project_id)
            .cloned()
            .collect();
        rows.sort_by_key(|c| (c.created_at, c.id.clone()));
        Box::pin(async move { Ok(rows) })
    }

    fn get(
        &self,
        id: &str,
    ) -> BoxFut<'_, Result<Option<server::ports::campaigns_store::CampaignRecord>, StoreError>>
    {
        let row = self
            .campaigns
            .read()
            .unwrap()
            .iter()
            .find(|c| c.id == id)
            .cloned();
        Box::pin(async move { Ok(row) })
    }

    fn create(
        &self,
        draft: &server::ports::campaigns_store::NewCampaign,
    ) -> BoxFut<'_, Result<server::ports::campaigns_store::CampaignRecord, StoreError>> {
        use server::ports::campaigns_store::CampaignRecord;
        let now = chrono::Utc::now();
        let record = CampaignRecord {
            id: Ulid::new().to_string(),
            project_id: draft.project_id.clone(),
            name: draft.name.clone(),
            description: draft.description.clone(),
            created_by: draft.created_by.clone(),
            channel: draft.channel.clone(),
            template_id: draft.template_id.clone(),
            title: draft.title.clone(),
            body: draft.body.clone(),
            audience_mode: draft.audience_mode.clone(),
            selected_user_ids: draft.selected_user_ids.clone(),
            provider_id: draft.provider_id.clone(),
            status: draft.status.clone(),
            send_at: draft.send_at,
            started_at: None,
            completed_at: None,
            last_error: None,
            created_at: now,
            updated_at: now,
        };
        self.campaigns.write().unwrap().push(record.clone());
        Box::pin(async move { Ok(record) })
    }

    fn update(
        &self,
        id: &str,
        patch: &server::ports::campaigns_store::CampaignUpdate,
    ) -> BoxFut<'_, Result<Option<server::ports::campaigns_store::CampaignRecord>, StoreError>>
    {
        let mut campaigns = self.campaigns.write().unwrap();
        let updated = campaigns.iter_mut().find(|c| c.id == id).map(|c| {
            c.name = patch.name.clone();
            c.description = patch.description.clone();
            c.channel = patch.channel.clone();
            c.template_id = patch.template_id.clone();
            c.title = patch.title.clone();
            c.body = patch.body.clone();
            c.audience_mode = patch.audience_mode.clone();
            c.selected_user_ids = patch.selected_user_ids.clone();
            c.provider_id = patch.provider_id.clone();
            c.send_at = patch.send_at;
            c.updated_at = chrono::Utc::now();
            c.clone()
        });
        drop(campaigns);
        Box::pin(async move { Ok(updated) })
    }

    fn delete(&self, id: &str) -> BoxFut<'_, Result<bool, StoreError>> {
        let mut campaigns = self.campaigns.write().unwrap();
        let before = campaigns.len();
        campaigns.retain(|c| c.id != id);
        let deleted = campaigns.len() < before;
        drop(campaigns);
        Box::pin(async move { Ok(deleted) })
    }

    fn claim_due(
        &self,
        now: chrono::DateTime<chrono::Utc>,
        limit: i64,
    ) -> BoxFut<'_, Result<Vec<server::ports::campaigns_store::CampaignRecord>, StoreError>> {
        let mut campaigns = self.campaigns.write().unwrap();
        let mut due: Vec<usize> = campaigns
            .iter()
            .enumerate()
            .filter(|(_, c)| c.status == "scheduled" && c.send_at.is_some_and(|ts| ts <= now))
            .map(|(i, _)| i)
            .collect();
        due.sort_by(|&a, &b| {
            let key = |c: &server::ports::campaigns_store::CampaignRecord| {
                (
                    c.send_at
                        .unwrap_or(chrono::DateTime::<chrono::Utc>::MAX_UTC),
                    c.id.clone(),
                )
            };
            key(&campaigns[a]).cmp(&key(&campaigns[b]))
        });
        due.truncate(limit.max(0) as usize);
        let claimed: Vec<_> = due
            .into_iter()
            .map(|i| {
                let c = &mut campaigns[i];
                c.status = "running".to_string();
                c.started_at = Some(now);
                c.updated_at = now;
                c.clone()
            })
            .collect();
        drop(campaigns);
        Box::pin(async move { Ok(claimed) })
    }

    fn list_running(
        &self,
        limit: i64,
    ) -> BoxFut<'_, Result<Vec<server::ports::campaigns_store::CampaignRecord>, StoreError>> {
        let mut rows: Vec<_> = self
            .campaigns
            .read()
            .unwrap()
            .iter()
            .filter(|c| c.status == "running")
            .cloned()
            .collect();
        rows.sort_by_key(|c| (c.created_at, c.id.clone()));
        rows.truncate(limit.max(0) as usize);
        Box::pin(async move { Ok(rows) })
    }

    fn transition(
        &self,
        id: &str,
        status: &str,
        started_at: Option<chrono::DateTime<chrono::Utc>>,
        completed_at: Option<chrono::DateTime<chrono::Utc>>,
        last_error: Option<&str>,
    ) -> BoxFut<'_, Result<Option<server::ports::campaigns_store::CampaignRecord>, StoreError>>
    {
        let mut campaigns = self.campaigns.write().unwrap();
        let updated = campaigns.iter_mut().find(|c| c.id == id).map(|c| {
            c.status = status.to_string();
            if let Some(ts) = started_at {
                c.started_at = Some(ts);
            }
            if let Some(ts) = completed_at {
                c.completed_at = Some(ts);
            }
            c.last_error = last_error.map(str::to_string);
            c.updated_at = chrono::Utc::now();
            c.clone()
        });
        drop(campaigns);
        Box::pin(async move { Ok(updated) })
    }
}

/// Recording HTTP requester for flow HTTP step tests.
#[derive(Default)]
pub struct FakeHttpRequester {
    calls: RwLock<Vec<server::ports::http_requester::HttpCall>>,
    fail: RwLock<bool>,
}

impl FakeHttpRequester {
    pub fn new() -> Self {
        Self::default()
    }

    /// Recorded outbound calls, in order.
    pub fn calls(&self) -> Vec<server::ports::http_requester::HttpCall> {
        self.calls.read().unwrap().clone()
    }

    /// When true, every call fails (to exercise log-only semantics).
    pub fn set_fail(&self, fail: bool) {
        *self.fail.write().unwrap() = fail;
    }
}

impl server::ports::http_requester::HttpRequester for FakeHttpRequester {
    fn request(
        &self,
        call: &server::ports::http_requester::HttpCall,
    ) -> server::ports::http_requester::BoxHttpFut<
        '_,
        Result<server::ports::http_requester::HttpResult, String>,
    > {
        self.calls.write().unwrap().push(call.clone());
        let fail = *self.fail.read().unwrap();
        Box::pin(async move {
            if fail {
                Err("fake http failure".to_string())
            } else {
                Ok(server::ports::http_requester::HttpResult {
                    status: 200,
                    ok: true,
                })
            }
        })
    }
}

/// In-memory trigger token store for inbound webhook tests.
#[derive(Default)]
pub struct FakeTriggerTokenStore {
    tokens: RwLock<Vec<server::ports::trigger_token_store::TriggerTokenRecord>>,
}

impl FakeTriggerTokenStore {
    pub fn new() -> Self {
        Self::default()
    }
}

impl server::ports::trigger_token_store::TriggerTokenStore for FakeTriggerTokenStore {
    fn replace_flow_tokens(
        &self,
        flow_id: &str,
        project_id: &str,
        definition: &serde_json::Value,
        pairs: &[(String, String)],
    ) -> BoxFut<'_, Result<(), StoreError>> {
        let mut tokens = self.tokens.write().unwrap();
        tokens.retain(|t| t.flow_id != flow_id);
        for (token, node_id) in pairs {
            tokens.push(server::ports::trigger_token_store::TriggerTokenRecord {
                token: token.clone(),
                flow_id: flow_id.to_string(),
                node_id: node_id.clone(),
                project_id: project_id.to_string(),
                flow_definition: definition.clone(),
            });
        }
        Box::pin(async move { Ok(()) })
    }

    fn lookup(
        &self,
        token: &str,
    ) -> BoxFut<
        '_,
        Result<Option<server::ports::trigger_token_store::TriggerTokenRecord>, StoreError>,
    > {
        let row = self
            .tokens
            .read()
            .unwrap()
            .iter()
            .find(|t| t.token == token)
            .cloned();
        Box::pin(async move { Ok(row) })
    }
}
