//! Project use cases: listing and the environment gate.

use std::sync::Arc;

use crate::domain::audit::AuditService;
use crate::domain::audit::entities::{AuditAction, AuditEvent};
use crate::domain::auth::entities::UserId;
use crate::domain::auth::errors::AuthError;
use crate::domain::projects::entities::Project;
use crate::ports::auth_store::StoreError;
use crate::ports::projects_store::ProjectsStore;

/// Allowed project gate values (mirrors the schema CHECK).
pub const ENVIRONMENTS: [&str; 2] = ["development", "production"];

pub struct ProjectService {
    store: Arc<dyn ProjectsStore>,
    audit: Arc<AuditService>,
}

impl ProjectService {
    pub fn new(store: Arc<dyn ProjectsStore>, audit: Arc<AuditService>) -> Self {
        Self { store, audit }
    }

    /// Projects the user owns or belongs to, oldest first.
    pub async fn list(&self, user_id: UserId) -> Result<Vec<Project>, AuthError> {
        Ok(self
            .store
            .list_projects(user_id)
            .await?
            .into_iter()
            .map(Project::from)
            .collect())
    }

    /// Creates a new project.
    pub async fn create(
        &self,
        user_id: UserId,
        name: &str,
        description: Option<&str>,
    ) -> Result<Project, AuthError> {
        if name.trim().is_empty() {
            return Err(AuthError::Validation("project name is required".into()));
        }
        let record = self
            .store
            .create_project(user_id, name, description)
            .await
            .map_err(|e| match e {
                StoreError::Conflict => {
                    AuthError::Conflict("a project with this name already exists".into())
                }
                StoreError::Storage(m) => AuthError::Storage(m),
            })?;

        let project = Project::from(record);

        self.audit
            .record(
                chrono::Utc::now(),
                &AuditEvent::new(
                    AuditAction::ProjectCreated,
                    Some(&user_id.to_string()),
                    None,
                    Some(&project.id),
                    format!("project '{}' created", project.name),
                    None,
                ),
            )
            .await;

        Ok(project)
    }

    /// Switches the project's environment gate.
    pub async fn set_environment(
        &self,
        user_id: UserId,
        project_id: &str,
        environment: &str,
    ) -> Result<Project, AuthError> {
        if !ENVIRONMENTS.contains(&environment) {
            return Err(AuthError::Validation(format!(
                "environment must be one of: {}",
                ENVIRONMENTS.join(", ")
            )));
        }

        let project = self
            .store
            .set_project_environment(user_id, project_id, environment)
            .await?
            .map(Project::from)
            .ok_or_else(|| AuthError::NotFound("project not found".to_string()))?;

        self.audit
            .record(
                chrono::Utc::now(),
                &AuditEvent::new(
                    AuditAction::ProjectEnvironmentChanged,
                    Some(&user_id.to_string()),
                    None,
                    Some(project_id),
                    format!("environment switched to {environment}"),
                    None,
                ),
            )
            .await;

        Ok(project)
    }
}

/// Assignable team roles (mirrors the dashboard role union).
pub const MEMBER_ROLES: [&str; 5] = ["owner", "admin", "developer", "viewer", "billing"];

/// Team membership use cases: the 2FA gate flag and member invites.
/// Needs user lookup (TOTP status), so it holds both stores.
pub struct ProjectMembersService {
    projects: Arc<dyn ProjectsStore>,
    auth: Arc<dyn crate::ports::auth_store::AuthStore>,
    audit: Arc<AuditService>,
}

impl ProjectMembersService {
    pub fn new(
        projects: Arc<dyn ProjectsStore>,
        auth: Arc<dyn crate::ports::auth_store::AuthStore>,
        audit: Arc<AuditService>,
    ) -> Self {
        Self {
            projects,
            auth,
            audit,
        }
    }

    fn is_manager(created_by: Option<&str>, member_role: Option<&str>, actor: &UserId) -> bool {
        created_by == Some(actor.to_string().as_str())
            || matches!(member_role, Some("owner") | Some("admin"))
    }

    /// Flips the "new members must have 2FA" flag. Owner or admin only.
    pub async fn set_require_2fa(
        &self,
        actor: UserId,
        project_id: &str,
        enabled: bool,
    ) -> Result<bool, AuthError> {
        let access = self
            .projects
            .get_project_access(actor, project_id)
            .await
            .map_err(|e| match e {
                StoreError::Conflict => AuthError::Conflict("conflicting update".into()),
                StoreError::Storage(m) => AuthError::Storage(m),
            })?
            .ok_or_else(|| AuthError::NotFound("project not found".to_string()))?;
        if !Self::is_manager(
            access.created_by.as_deref(),
            access.member_role.as_deref(),
            &actor,
        ) {
            return Err(AuthError::Forbidden(
                "only project owners and admins can change this setting".into(),
            ));
        }
        let enabled = self
            .projects
            .set_require_2fa(project_id, enabled)
            .await
            .map_err(|e| match e {
                StoreError::Conflict => AuthError::Conflict("conflicting update".into()),
                StoreError::Storage(m) => AuthError::Storage(m),
            })?
            .ok_or_else(|| AuthError::NotFound("project not found".to_string()))?;

        self.audit
            .record(
                chrono::Utc::now(),
                &AuditEvent::new(
                    AuditAction::ProjectRequire2faChanged,
                    Some(&actor.to_string()),
                    None,
                    Some(project_id),
                    format!(
                        "two-factor requirement for new members {}",
                        if enabled { "enabled" } else { "disabled" }
                    ),
                    None,
                ),
            )
            .await;
        Ok(enabled)
    }

    /// Adds an existing account to the project team. Owner or admin only;
    /// granting `owner` additionally requires being the creator. When the
    /// project requires 2FA, the invited account must have TOTP enabled.
    pub async fn add_member(
        &self,
        actor: UserId,
        project_id: &str,
        email: &str,
        role: &str,
    ) -> Result<crate::domain::projects::entities::ProjectMember, AuthError> {
        use crate::domain::auth::value_objects::Email;

        let role = role.trim().to_lowercase();
        if !MEMBER_ROLES.contains(&role.as_str()) {
            return Err(AuthError::Validation(format!(
                "role must be one of: {}",
                MEMBER_ROLES.join(", ")
            )));
        }
        let access = self
            .projects
            .get_project_access(actor, project_id)
            .await
            .map_err(|e| match e {
                StoreError::Conflict => AuthError::Conflict("conflicting update".into()),
                StoreError::Storage(m) => AuthError::Storage(m),
            })?
            .ok_or_else(|| AuthError::NotFound("project not found".to_string()))?;
        let is_owner = access.created_by.as_deref() == Some(actor.to_string().as_str());
        if !is_owner && !matches!(access.member_role.as_deref(), Some("owner") | Some("admin")) {
            return Err(AuthError::Forbidden(
                "only project owners and admins can invite members".into(),
            ));
        }
        if role == "owner" && !is_owner {
            return Err(AuthError::Forbidden(
                "only the project owner can grant the owner role".into(),
            ));
        }

        let parsed = Email::parse(email)
            .map_err(|_| AuthError::Validation("enter a valid email address".to_string()))?;
        let target = self
            .auth
            .find_user_by_email(parsed.as_str())
            .await
            .map_err(|e| match e {
                StoreError::Conflict => AuthError::Conflict("conflicting lookup".into()),
                StoreError::Storage(m) => AuthError::Storage(m),
            })?
            .ok_or_else(|| AuthError::NotFound("no account with this email address".to_string()))?;
        if target.status == crate::domain::auth::entities::UserStatus::Suspended {
            return Err(AuthError::Forbidden(
                "that account is suspended".to_string(),
            ));
        }
        if access.project.require_2fa && !target.totp_enabled {
            return Err(AuthError::Forbidden(
                "this project requires two-factor authentication: the invited account must enable 2FA first".to_string(),
            ));
        }

        let inserted = self
            .projects
            .insert_member(project_id, target.id, &role)
            .await
            .map_err(|e| match e {
                StoreError::Conflict => AuthError::Conflict("already a member".into()),
                StoreError::Storage(m) => AuthError::Storage(m),
            })?;
        if !inserted {
            return Err(AuthError::Conflict(
                "that account is already on this team".to_string(),
            ));
        }

        self.audit
            .record(
                chrono::Utc::now(),
                &AuditEvent::new(
                    AuditAction::ProjectMemberAdded,
                    Some(&actor.to_string()),
                    None,
                    Some(project_id),
                    format!("{} joined as {role}", target.name),
                    Some(serde_json::json!({ "user_id": target.id.to_string() })),
                ),
            )
            .await;

        Ok(crate::domain::projects::entities::ProjectMember {
            user_id: target.id.to_string(),
            name: target.name.clone(),
            email: target.email.as_str().to_string(),
            role,
            has_2fa: target.totp_enabled,
            last_active_at: target.last_login_at,
        })
    }

    /// Team roster for a project the caller belongs to, creator first.
    pub async fn list_members(
        &self,
        actor: UserId,
        project_id: &str,
    ) -> Result<Vec<crate::domain::projects::entities::ProjectMember>, AuthError> {
        self.projects
            .get_project_access(actor, project_id)
            .await
            .map_err(|e| match e {
                StoreError::Conflict => AuthError::Conflict("conflicting read".into()),
                StoreError::Storage(m) => AuthError::Storage(m),
            })?
            .ok_or_else(|| AuthError::NotFound("project not found".to_string()))?;
        Ok(self
            .projects
            .list_members(actor, project_id)
            .await
            .map_err(|e| match e {
                StoreError::Conflict => AuthError::Conflict("conflicting read".into()),
                StoreError::Storage(m) => AuthError::Storage(m),
            })?
            .into_iter()
            .map(|record| crate::domain::projects::entities::ProjectMember {
                user_id: record.user_id,
                name: record.name,
                email: record.email,
                role: record.role,
                has_2fa: record.has_2fa,
                last_active_at: record.last_active_at,
            })
            .collect())
    }
}
