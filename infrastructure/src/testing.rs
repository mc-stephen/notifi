//! In-memory [`AuthStore`] and [`ProjectsStore`] implementations for tests.
//!
//! Not compiled out: they are small, dependency-free, and let downstream
//! crates (the `api` binary's HTTP tests) build fully functional auth
//! and project services without a database.

use std::sync::RwLock;

use chrono::{DateTime, Utc};

use notifi_core::Ulid;

use crate::domain::auth::entities::{AuthToken, Session, TokenPurpose, User, UserId};
use crate::domain::audit::entities::AuditEntry;
use crate::ports::audit_store::{AuditFilters, AuditStore};
use crate::ports::auth_store::{AuthStore, BoxFut, StoreError};
use crate::ports::projects_store::{ProjectSummary, ProjectsStore};
use crate::ports::recipients_store::{RecipientRecord, RecipientsStore};
use crate::ports::templates_store::{
    AttachmentInput, AttachmentRecord, TemplateRecord, TemplatesStore,
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
        let projects = &self.projects;
        Box::pin(async move {
            // Mark onboarded.
            {
                let mut list = onboarded.write().map_err(lock_err)?;
                let id = user_id.to_string();
                if !list.contains(&id) {
                    list.push(id);
                }
            }
            // Seed a real project so GET /v1/projects returns data.
            {
                let mut list = projects.write().map_err(lock_err)?;
                let slug = input
                    .project_name
                    .to_lowercase()
                    .replace(' ', "-")
                    .chars()
                    .filter(|c| c.is_alphanumeric() || *c == '-')
                    .take(40)
                    .collect::<String>();
                list.push((
                    user_id.to_string(),
                    ProjectSummary {
                        id: ulid::Ulid::new().to_string(),
                        name: input.project_name,
                        slug,
                        description: input.project_description,
                        environment: "development".to_string(),
                        created_at: chrono::Utc::now(),
                    },
                ));
            }
            Ok(())
        })
    }
}

impl ProjectsStore for FakeAuthStore {
    fn list_projects(&self, user_id: UserId) -> BoxFut<'_, Result<Vec<ProjectSummary>, StoreError>> {
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
            if !visible
                .iter()
                .any(|(u, p)| *u == actor && *p == project_id)
            {
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
