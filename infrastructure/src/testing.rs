//! In-memory [`AuthStore`] and [`ProjectsStore`] implementations for tests.
//!
//! Not compiled out: they are small, dependency-free, and let downstream
//! crates (the `api` binary's HTTP tests) build fully functional auth
//! and project services without a database.

use std::sync::RwLock;

use chrono::{DateTime, Utc};

use crate::domain::auth::entities::{AuthToken, Session, TokenPurpose, User, UserId};
use crate::ports::auth_store::{AuthStore, BoxFut, StoreError};
use crate::ports::projects_store::{ProjectSummary, ProjectsStore};

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
