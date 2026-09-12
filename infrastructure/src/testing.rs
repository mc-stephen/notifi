//! In-memory [`AuthStore`] and [`ProjectsStore`] implementations for tests.
//!
//! Not compiled out: they are small, dependency-free, and let downstream
//! crates (the `api` binary's HTTP tests) build fully functional auth
//! and project services without a database.

use std::collections::HashMap;
use std::sync::RwLock;

use chrono::{DateTime, Utc};

use notifi_core::Ulid;

use crate::domain::audit::entities::AuditEntry;
use crate::domain::auth::entities::{AuthToken, Session, TokenPurpose, User, UserId};
use crate::domain::notifications::entities::{NotificationOrigin, NotificationType};
use crate::ports::audit_store::{AuditFilters, AuditStore};
use crate::ports::auth_store::{AuthStore, BoxFut, StoreError};
use crate::ports::notifications_store::{
    BroadcastRecipient, BroadcastRecord, BroadcastStats, BroadcastStatus, NotificationRecord,
    NotificationsStore,
};
use crate::ports::projects_store::{ProjectSummary, ProjectsStore};
use crate::ports::recipients_store::{RecipientRecord, RecipientsStore};
use crate::ports::templates_store::{
    AttachmentInput, AttachmentRecord, TemplateRecord, TemplatesStore,
};
use crate::ports::tickets_store::{
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
    invites: RwLock<Vec<crate::ports::projects_store::InviteRecord>>,
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
    pub fn seed_project(&self, user_id: crate::domain::auth::entities::UserId) {
        self.onboarded.write().unwrap().push(user_id.to_string());
    }

    /// Seeds a real project summary for the given user so the projects
    /// API returns data in list/set-environment endpoints.
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

    /// Seeds a pending invite directly (expiry control for tests).
    pub fn seed_invite(
        &self,
        project_id: &str,
        email: &str,
        role: &str,
        token_hash: &str,
        expires_at: chrono::DateTime<chrono::Utc>,
    ) -> String {
        use crate::ports::projects_store::InviteRecord;
        let id = notifi_core::Ulid::new().to_string();
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
        id: crate::domain::auth::entities::UserId,
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
        user_id: crate::domain::auth::entities::UserId,
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
        user_id: crate::domain::auth::entities::UserId,
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
        user_id: crate::domain::auth::entities::UserId,
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
        user_id: crate::domain::auth::entities::UserId,
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
        user_id: crate::domain::auth::entities::UserId,
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
        user_id: crate::domain::auth::entities::UserId,
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
        status: Option<crate::domain::auth::entities::UserStatus>,
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
        user_id: crate::domain::auth::entities::UserId,
        status: crate::domain::auth::entities::UserStatus,
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
        status: Option<crate::domain::auth::entities::UserStatus>,
    ) -> BoxFut<'_, Result<Vec<crate::domain::auth::entities::UserId>, StoreError>> {
        let users = &self.users;
        Box::pin(async move {
            let mut ids: Vec<crate::domain::auth::entities::UserId> = users
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
        id: crate::domain::auth::entities::SessionId,
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
        user_id: crate::domain::auth::entities::UserId,
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
        id: crate::domain::auth::entities::AuthTokenId,
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
        user_id: crate::domain::auth::entities::UserId,
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
        user_id: crate::domain::auth::entities::UserId,
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
        user_id: crate::domain::auth::entities::UserId,
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
        user_id: crate::domain::auth::entities::UserId,
        input: crate::ports::auth_store::OnboardingInput,
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
            crate::ports::projects_store::ProjectsStore::create_project(
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
        Box::pin(async move {
            let owned = projects.read().map_err(lock_err)?;
            Ok(owned
                .iter()
                .filter(|(owner, _)| *owner == user_id.to_string())
                .map(|(_, p)| p.clone())
                .collect())
        })
    }

    fn set_project_environment(
        &self,
        user_id: UserId,
        project_id: &str,
        environment: &str,
    ) -> BoxFut<'_, Result<Option<ProjectSummary>, StoreError>> {
        let projects = &self.projects;
        let project_id = project_id.to_string();
        let environment = environment.to_string();
        let user_id_str = user_id.to_string();
        Box::pin(async move {
            let mut projects = projects.write().map_err(lock_err)?;
            Ok(projects
                .iter_mut()
                .find(|(owner, p)| *owner == user_id_str && p.id == project_id)
                .map(|(_, p)| {
                    p.environment = environment;
                    p.clone()
                }))
        })
    }

    fn get_project_access(
        &self,
        user_id: UserId,
        project_id: &str,
    ) -> BoxFut<'_, Result<Option<crate::ports::projects_store::ProjectAccess>, StoreError>> {
        use crate::ports::projects_store::ProjectAccess;
        let project_id = project_id.to_string();
        let user_id_str = user_id.to_string();
        let projects = self.projects.read().unwrap().clone();
        let members = self.members.read().unwrap().clone();
        Box::pin(async move {
            Ok(projects
                .into_iter()
                .find(|(_, p)| p.id == project_id)
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
        use crate::ports::projects_store::InviteRecord;
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
            let id = notifi_core::Ulid::new().to_string();
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
    ) -> BoxFut<'_, Result<Option<crate::ports::projects_store::InviteRecord>, StoreError>> {
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
    ) -> BoxFut<'_, Result<Option<crate::ports::projects_store::InviteRecord>, StoreError>> {
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
    ) -> BoxFut<'_, Result<Vec<crate::ports::projects_store::TeamMemberRecord>, StoreError>> {
        use crate::ports::projects_store::TeamMemberRecord;
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
            let slug = name
                .to_lowercase()
                .replace(|c: char| !c.is_ascii_alphanumeric() && c != '-', "")
                .replace(char::is_whitespace, "-")
                .trim_matches('-')
                .to_string();
            let slug = if slug.is_empty() {
                "project".to_string()
            } else {
                slug
            };
            // Ensure uniqueness
            let slug = if projects.iter().any(|(_, p)| p.slug == slug) {
                format!("{slug}-{}", projects.len() + 1)
            } else {
                slug
            };
            let record = ProjectSummary {
                id: notifi_core::Ulid::new().to_string(),
                name,
                slug,
                description,
                environment: "development".to_string(),
                require_2fa: false,
                created_at: chrono::Utc::now(),
            };
            projects.push((user_id_str, record.clone()));
            Ok(record)
        })
    }

    // === Admin-scoped reads (no actor visibility checks) =================

    fn list_all_projects(
        &self,
        search: Option<&str>,
        environment: Option<&str>,
        limit: i64,
        offset: i64,
    ) -> BoxFut<'_, Result<(Vec<crate::ports::projects_store::AdminProjectRecord>, i64), StoreError>>
    {
        let search_owned = search.map(|s| s.to_lowercase());
        let environment_owned = environment.map(str::to_owned);
        let projects = self.projects.read().unwrap().clone();
        let users = self.users.read().unwrap().clone();
        Box::pin(async move {
            let mut result: Vec<crate::ports::projects_store::AdminProjectRecord> = projects
                .iter()
                .filter(|(_, p)| {
                    if let Some(ref env) = environment_owned {
                        p.environment == *env
                    } else {
                        true
                    }
                })
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
                    crate::ports::projects_store::AdminProjectRecord {
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
            let page: Vec<crate::ports::projects_store::AdminProjectRecord> = result
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
    ) -> BoxFut<'_, Result<Option<crate::ports::projects_store::AdminProjectRecord>, StoreError>>
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
                    crate::ports::projects_store::AdminProjectRecord {
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
    ) -> BoxFut<'_, Result<Vec<crate::ports::projects_store::ProjectMemberRecord>, StoreError>>
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
        filters: crate::ports::audit_store::AdminAuditFilters<'_>,
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
        all.sort_by(|a, b| b.created_at.cmp(&a.created_at));
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
        actor: crate::domain::auth::entities::UserId,
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
        actor: crate::domain::auth::entities::UserId,
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
            list.sort_by(|a, b| b.created_at.cmp(&a.created_at));
            list.truncate(limit.max(0) as usize);
            Ok(list)
        })
    }

    fn get(
        &self,
        actor: crate::domain::auth::entities::UserId,
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
        actor: crate::domain::auth::entities::UserId,
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
        actor: crate::domain::auth::entities::UserId,
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
        all.sort_by(|a, b| b.created_at.cmp(&a.created_at));
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
        actor: crate::domain::auth::entities::UserId,
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
        actor: crate::domain::auth::entities::UserId,
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
            list.sort_by(|a, b| b.created_at.cmp(&a.created_at));
            list.truncate(limit.max(0) as usize);
            Ok(list)
        })
    }

    fn get(
        &self,
        actor: crate::domain::auth::entities::UserId,
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

    #[allow(clippy::too_many_arguments)]
    fn update(
        &self,
        actor: crate::domain::auth::entities::UserId,
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
        actor: crate::domain::auth::entities::UserId,
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
            crate::domain::support::entities::MessageAuthor::Customer => users
                .iter()
                .find(|(id, _, _)| id == author_id)
                .map(|(_, name, _)| name.clone()),
            crate::domain::support::entities::MessageAuthor::Support => admins
                .iter()
                .find(|(id, _)| id == author_id)
                .map(|(_, name)| name.clone()),
        }
    }

    pub fn all(&self) -> Vec<TicketRecord> {
        let mut all = self.tickets.read().unwrap().clone();
        all.sort_by(|a, b| b.created_at.cmp(&a.created_at));
        all
    }
}

impl TicketsStore for FakeTicketsStore {
    fn create(
        &self,
        actor: crate::domain::auth::entities::UserId,
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
                status: crate::domain::support::entities::TicketStatus::Open,
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
        actor: crate::domain::auth::entities::UserId,
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
        actor: crate::domain::auth::entities::UserId,
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
        actor: crate::domain::auth::entities::UserId,
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
        actor: crate::domain::auth::entities::UserId,
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
                author: crate::domain::support::entities::MessageAuthor::Customer,
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
        actor: crate::domain::auth::entities::UserId,
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
            }) && t.status == crate::domain::support::entities::TicketStatus::Resolved
            {
                t.status = crate::domain::support::entities::TicketStatus::Open;
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
        admin_id: crate::domain::admin::entities::AdminUserId,
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
                author: crate::domain::support::entities::MessageAuthor::Support,
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
        status: crate::domain::support::entities::TicketStatus,
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

    fn count_tickets_for_user(
        &self,
        user_id: &str,
    ) -> BoxFut<'_, Result<crate::ports::tickets_store::TicketCounts, StoreError>> {
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
                        crate::domain::support::entities::TicketStatus::Open
                            | crate::domain::support::entities::TicketStatus::InProgress
                    )
                })
                .count() as i64;
            Ok(crate::ports::tickets_store::TicketCounts { total, open })
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
        all.sort_by(|a, b| b.created_at.cmp(&a.created_at));
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
                id: notifi_core::Ulid::new().to_string(),
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
            list.sort_by(|a, b| b.created_at.cmp(&a.created_at));
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
        notification_type: crate::domain::notifications::entities::NotificationType,
        recipients: &[BroadcastRecipient],
    ) -> BoxFut<'_, Result<i64, StoreError>> {
        let notifications = &self.notifications;
        let broadcast_id = broadcast_id.to_string();
        let recipients = recipients.to_vec();
        Box::pin(async move {
            let mut list = notifications.write().map_err(lock_err)?;
            for r in &recipients {
                list.push(NotificationRecord {
                    id: notifi_core::Ulid::new().to_string(),
                    user_id: r.user_id.clone(),
                    notification_type,
                    origin: crate::domain::notifications::entities::NotificationOrigin::Admin,
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

use crate::domain::admin::entities::{
    AdminPasswordResetToken, AdminPasswordResetTokenId, AdminSession, AdminSessionId, AdminStatus,
    AdminUser, AdminUserId,
};
use crate::ports::admin_store::AdminStore;

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

use crate::domain::billing::entities::{BillingCycle, SubscriptionStatus};
use crate::domain::billing::{NewPlan, UpdatePlan};
use crate::ports::billing_store::{BillingStore, PlanRecord, SubscriptionRecord};

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
        actor: crate::domain::auth::entities::UserId,
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
    ) -> BoxFut<'_, Result<Vec<crate::ports::billing_store::ProjectBirth>, StoreError>> {
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
    ) -> BoxFut<'_, Result<Vec<crate::ports::billing_store::SubscriberRecord>, StoreError>> {
        use crate::domain::billing::entities::SubscriptionStatus;
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
                out.push(crate::ports::billing_store::SubscriberRecord {
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

use crate::ports::mailer::{
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
