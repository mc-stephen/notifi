//! Project use cases: listing and the active switch.

use std::sync::Arc;

use crate::domain::audit::AuditService;
use crate::domain::audit::entities::{AuditAction, AuditEvent};
use crate::domain::auth::entities::UserId;
use crate::domain::error::AppError;
use crate::domain::projects::entities::Project;
use crate::ports::auth_store::StoreError;
use crate::ports::config_store::ConfigStore;
use crate::ports::projects_store::ProjectsStore;

pub struct ProjectService {
    store: Arc<dyn ProjectsStore>,
    audit: Arc<AuditService>,
    /// Project storage: `Some` in the composed app (driver built at boot);
    /// `None` in unit tests that only exercise DB semantics.
    config_store: Option<Arc<dyn ConfigStore>>,
}

impl ProjectService {
    pub fn new(store: Arc<dyn ProjectsStore>, audit: Arc<AuditService>) -> Self {
        Self {
            store,
            audit,
            config_store: None,
        }
    }

    /// Wires project storage for lifecycle actions (creation seeding,
    /// deletion wipe, export file collection).
    pub fn with_config_store(mut self, config_store: Arc<dyn ConfigStore>) -> Self {
        self.config_store = Some(config_store);
        self
    }

    /// Projects the user owns or belongs to, oldest first.
    pub async fn list(&self, user_id: UserId) -> Result<Vec<Project>, AppError> {
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
    ) -> Result<Project, AppError> {
        if name.trim().is_empty() {
            return Err(AppError::Validation("project name is required".into()));
        }
        let record = self
            .store
            .create_project(user_id, name, description)
            .await
            .map_err(|e| match e {
                StoreError::Conflict => {
                    AppError::Conflict("a project with this name already exists".into())
                }
                StoreError::Storage(m) => AppError::Storage(m),
            })?;

        let project = Project::from(record);

        // The project folder springs into existence WITH the project: the
        // code-owned default layout lands under projects/{slug}/ before
        // the user ever sees the project. A failed copy fails the
        // creation — best-effort row cleanup keeps "no folder" from
        // becoming a mutilated project nobody can finish configuring.
        if let Some(config_store) = &self.config_store
            && let Err(e) = config_store.materialize_project(&project.slug).await
        {
            tracing::error!(error = %e, slug = %project.slug, "project materialization failed; rolling the project row back");
            self.store.delete_project(&project.id).await.ok();
            return Err(AppError::Storage(format!(
                "project storage could not be prepared: {e}"
            )));
        }

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

    /// Flips the project's active switch. `false` stops the project's
    /// delivery and external traffic entirely.
    pub async fn set_active(
        &self,
        user_id: UserId,
        project_id: &str,
        active: bool,
    ) -> Result<Project, AppError> {
        let project = self
            .store
            .set_project_active(user_id, project_id, active)
            .await?
            .map(Project::from)
            .ok_or_else(|| AppError::NotFound("project not found".to_string()))?;

        self.audit
            .record(
                chrono::Utc::now(),
                &AuditEvent::new(
                    AuditAction::ProjectActiveChanged,
                    Some(&user_id.to_string()),
                    None,
                    Some(project_id),
                    format!(
                        "project marked {}",
                        if active { "active" } else { "inactive" }
                    ),
                    None,
                ),
            )
            .await;

        Ok(project)
    }

    /// Requests deletion: the project turns into a tombstone that stays
    /// visible to its members (so they can download data or change their
    /// mind), stops all delivery immediately, and gets hard-purged by the
    /// worker after [`PURGE_GRACE`]. Owner or admin only (the store's SQL
    /// gate mirrors the members table the same way membership reads do).
    pub async fn request_deletion(
        &self,
        actor: UserId,
        project_id: &str,
    ) -> Result<Project, AppError> {
        let project = self
            .store
            .request_project_deletion(actor, project_id, chrono::Utc::now() + PURGE_GRACE)
            .await?
            .map(Project::from)
            .ok_or_else(|| AppError::NotFound("project not found".to_string()))?;

        self.audit
            .record(
                chrono::Utc::now(),
                &AuditEvent::new(
                    AuditAction::ProjectDeletionRequested,
                    Some(&actor.to_string()),
                    None,
                    Some(project_id),
                    format!(
                        "project '{}' deletion requested; data erases around {}",
                        project.name,
                        project
                            .purge_after
                            .map(|t| t.to_rfc3339())
                            .unwrap_or_else(|| "unknown".into())
                    ),
                    None,
                ),
            )
            .await;

        Ok(project)
    }

    /// Cancels a pending deletion inside the countdown — a full undo,
    /// since nothing was destroyed. Owner or admin only.
    pub async fn cancel_deletion(
        &self,
        actor: UserId,
        project_id: &str,
    ) -> Result<Project, AppError> {
        let project = self
            .store
            .cancel_project_deletion(actor, project_id)
            .await?
            .map(Project::from)
            .ok_or_else(|| {
                AppError::NotFound("no pending deletion for this project".to_string())
            })?;

        self.audit
            .record(
                chrono::Utc::now(),
                &AuditEvent::new(
                    AuditAction::ProjectDeletionCancelled,
                    Some(&actor.to_string()),
                    None,
                    Some(project_id),
                    format!("project '{}' deletion cancelled", project.name),
                    None,
                ),
            )
            .await;

        Ok(project)
    }

    /// Rate-guard + record for one data-export download: any member of
    /// the project may export, `[EXPORTS_PER_DAY]` downloads per rolling
    /// 24h across the whole project pool. The inserted row doubles as
    /// the export audit trail.
    pub async fn guard_export(&self, actor: UserId, project_id: &str) -> Result<(), AppError> {
        let since = chrono::Utc::now() - chrono::Duration::hours(24);
        let used = self
            .store
            .count_project_export(project_id, since)
            .await
            .map_err(AppError::from)?;
        if used >= EXPORTS_PER_DAY {
            return Err(AppError::RateLimited(format!(
                "project export limit ({EXPORTS_PER_DAY} per day) reached; try again later"
            )));
        }
        self.store
            .record_project_export(project_id, &actor)
            .await
            .map_err(AppError::from)?;

        self.audit
            .record(
                chrono::Utc::now(),
                &AuditEvent::new(
                    AuditAction::ProjectExported,
                    Some(&actor.to_string()),
                    None,
                    Some(project_id),
                    "project data exported".to_string(),
                    None,
                ),
            )
            .await;

        Ok(())
    }

    /// The worker's hard-purge: every project whose countdown expired
    /// loses its project folder first (retry-able on storage failure — the
    /// row survives to the next sweep), then its row (cascades take every
    /// child table with it). Returns how many projects were erased.
    pub async fn purge_due(&self, now: chrono::DateTime<chrono::Utc>) -> Result<usize, AppError> {
        let due = self
            .store
            .list_projects_due_for_purge(now, 20)
            .await
            .map_err(AppError::from)?;

        let mut erased = 0usize;
        for project in due {
            // Storage first: on failure the row stays due for a later
            // sweep instead of becoming an orphan folder nobody can
            // reach again.
            if let Some(config_store) = &self.config_store {
                let prefix = crate::kernel::config::ConfigResolver::project_prefix(&project.slug);
                match config_store.list_keys(&prefix).await {
                    Ok(keys) => {
                        let mut wipes = true;
                        for key in keys {
                            if let Err(e) = config_store.delete_raw(&key).await {
                                tracing::error!(error = %e, key = %key, project_id = %project.id, "purge: storage delete failed; retrying project on the next sweep");
                                wipes = false;
                                break;
                            }
                        }
                        if !wipes {
                            continue;
                        }
                    }
                    Err(e) => {
                        tracing::error!(error = %e, project_id = %project.id, "purge: storage list failed; retrying project on the next sweep");
                        continue;
                    }
                }
            }

            match self.store.delete_project(&project.id).await {
                Ok(true) => {
                    erased += 1;
                    tracing::info!(project_id = %project.id, slug = %project.slug, "project purged (storage folder + row + cascades)");
                    self.audit
                        .record(
                            chrono::Utc::now(),
                            &AuditEvent::new(
                                AuditAction::ProjectPurged,
                                None,
                                None,
                                Some(&project.id),
                                format!(
                                    "project '{}' fully erased after the deletion window",
                                    project.slug
                                ),
                                None,
                            ),
                        )
                        .await;
                }
                Ok(false) => {}
                Err(e) => {
                    tracing::error!(error = %e, project_id = %project.id, "purge: row delete failed; leaving for the next sweep");
                }
            }
        }
        Ok(erased)
    }
}

/// The deletion grace period: how long a requested deletion waits before
/// the worker erases the folder + rows. Members can download data and
/// cancel inside it.
pub const PURGE_GRACE: chrono::Duration = chrono::Duration::days(14);

/// Data-export downloads a project may make per rolling 24 hours — "a
/// few per day" so export clicks can't clock the server. Counts across
/// the whole project (all members share one pool).
pub const EXPORTS_PER_DAY: i64 = 3;

/// Assignable team roles (mirrors the dashboard role union).
pub const MEMBER_ROLES: [&str; 5] = ["owner", "admin", "developer", "viewer", "billing"];

/// Team membership use cases: the 2FA gate flag and GitHub-style
/// invites (invite → email → accept/decline). Needs user lookup plus
/// mail, so it holds both stores and the system-mail collaborators.
pub struct ProjectMembersService {
    projects: Arc<dyn ProjectsStore>,
    auth: Arc<dyn crate::ports::auth_store::AuthStore>,
    audit: Arc<AuditService>,
    mailer: Option<Arc<dyn crate::ports::mailer::SmtpMailer>>,
    templates: Option<Arc<dyn crate::ports::mailer::SystemTemplates>>,
    dashboard_url: String,
    expose_dev_tokens: bool,
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
            mailer: None,
            templates: None,
            dashboard_url: String::new(),
            expose_dev_tokens: false,
        }
    }

    /// Wires invite emails. All pieces travel together.
    pub fn with_system_mail(
        mut self,
        mailer: Arc<dyn crate::ports::mailer::SmtpMailer>,
        templates: Arc<dyn crate::ports::mailer::SystemTemplates>,
        dashboard_url: String,
        expose_dev_tokens: bool,
    ) -> Self {
        self.mailer = Some(mailer);
        self.templates = Some(templates);
        self.dashboard_url = dashboard_url;
        self.expose_dev_tokens = expose_dev_tokens;
        self
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
    ) -> Result<bool, AppError> {
        let access = self
            .projects
            .get_project_access(actor, project_id)
            .await
            .map_err(|e| match e {
                StoreError::Conflict => AppError::Conflict("conflicting update".into()),
                StoreError::Storage(m) => AppError::Storage(m),
            })?
            .ok_or_else(|| AppError::NotFound("project not found".to_string()))?;
        if !Self::is_manager(
            access.created_by.as_deref(),
            access.member_role.as_deref(),
            &actor,
        ) {
            return Err(AppError::Forbidden(
                "only project owners and admins can change this setting".into(),
            ));
        }
        let enabled = self
            .projects
            .set_require_2fa(project_id, enabled)
            .await
            .map_err(|e| match e {
                StoreError::Conflict => AppError::Conflict("conflicting update".into()),
                StoreError::Storage(m) => AppError::Storage(m),
            })?
            .ok_or_else(|| AppError::NotFound("project not found".to_string()))?;

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

    /// Invites an existing account to the project team (GitHub-style:
    /// this creates a pending invite + email, not a membership).
    /// Owner or admin only; granting `owner` additionally requires being
    /// the creator. The 2FA gate is enforced at accept time, not here,
    /// so invitees can enable 2FA between invite and accept.
    /// Returns the invite plus the raw token only in dev mode.
    pub async fn invite_member(
        &self,
        actor: UserId,
        project_id: &str,
        email: &str,
        role: &str,
    ) -> Result<
        (
            crate::domain::projects::entities::ProjectInvite,
            Option<String>,
        ),
        AppError,
    > {
        use crate::domain::auth::value_objects::{Email, new_token};

        let role = role.trim().to_lowercase();
        if !MEMBER_ROLES.contains(&role.as_str()) {
            return Err(AppError::Validation(format!(
                "role must be one of: {}",
                MEMBER_ROLES.join(", ")
            )));
        }
        let access = self
            .projects
            .get_project_access(actor, project_id)
            .await
            .map_err(|e| match e {
                StoreError::Conflict => AppError::Conflict("conflicting update".into()),
                StoreError::Storage(m) => AppError::Storage(m),
            })?
            .ok_or_else(|| AppError::NotFound("project not found".to_string()))?;
        let is_owner = access.created_by.as_deref() == Some(actor.to_string().as_str());
        if !is_owner && !matches!(access.member_role.as_deref(), Some("owner") | Some("admin")) {
            return Err(AppError::Forbidden(
                "only project owners and admins can invite members".into(),
            ));
        }
        if role == "owner" && !is_owner {
            return Err(AppError::Forbidden(
                "only the project owner can grant the owner role".into(),
            ));
        }

        let parsed = Email::parse(email)
            .map_err(|_| AppError::Validation("enter a valid email address".to_string()))?;
        let target = self
            .auth
            .find_user_by_email(parsed.as_str())
            .await
            .map_err(|e| match e {
                StoreError::Conflict => AppError::Conflict("conflicting lookup".into()),
                StoreError::Storage(m) => AppError::Storage(m),
            })?
            .ok_or_else(|| AppError::NotFound("no account with this email address".to_string()))?;
        if target.status == crate::domain::auth::entities::UserStatus::Suspended {
            return Err(AppError::Forbidden("that account is suspended".to_string()));
        }
        if Self::is_member(&self.projects, &target.id, project_id).await? {
            return Err(AppError::Conflict(
                "that account is already on this team".to_string(),
            ));
        }

        let (raw_token, token_hash) = new_token();
        let expires_at = chrono::Utc::now() + chrono::Duration::days(7);
        let invite_id = self
            .projects
            .create_invite(
                project_id,
                parsed.as_str(),
                &role,
                &token_hash,
                expires_at,
                actor,
            )
            .await
            .map_err(|e| match e {
                StoreError::Conflict => AppError::Conflict("conflicting update".into()),
                StoreError::Storage(m) => AppError::Storage(m),
            })?;

        self.audit
            .record(
                chrono::Utc::now(),
                &AuditEvent::new(
                    AuditAction::ProjectMemberInvited,
                    Some(&actor.to_string()),
                    None,
                    Some(project_id),
                    format!("{} invited as {role}", target.name),
                    Some(serde_json::json!({ "user_id": target.id.to_string() })),
                ),
            )
            .await;

        if let (Some(mailer), Some(templates)) = (self.mailer.as_ref(), self.templates.as_ref()) {
            let link = format!(
                "{}/invite/{raw_token}",
                self.dashboard_url.trim_end_matches('/')
            );
            // Best-effort render so a missing template never blocks invites.
            if let Ok(rendered) = templates.render(
                "member-invite",
                &[
                    ("name", target.name.as_str()),
                    ("project", access.project.name.as_str()),
                    ("role", role.as_str()),
                    ("link", link.as_str()),
                ],
            ) {
                if let Err(e) = mailer
                    .send(
                        crate::ports::mailer::SystemSender::NoReply,
                        parsed.as_str(),
                        &rendered.subject,
                        &rendered.text,
                        Some(&rendered.html),
                    )
                    .await
                {
                    tracing::warn!(error = %e, "invite email delivery failed");
                }
            } else {
                tracing::warn!("invite template render failed");
            }
        }

        Ok((
            crate::domain::projects::entities::ProjectInvite {
                id: invite_id,
                project_id: project_id.to_string(),
                project_name: access.project.name.clone(),
                email: parsed.as_str().to_string(),
                role,
                expires_at,
            },
            self.dev_token(&raw_token),
        ))
    }

    fn dev_token(&self, raw_token: &str) -> Option<String> {
        self.expose_dev_tokens.then(|| raw_token.to_string())
    }

    fn is_on_team(
        access: &crate::ports::projects_store::ProjectAccess,
        user_id: &crate::domain::auth::entities::UserId,
    ) -> bool {
        access.created_by.as_deref() == Some(user_id.to_string().as_str())
            || access.member_role.is_some()
    }

    /// Whether `user_id` owns or belongs to the project.
    async fn is_member(
        store: &Arc<dyn ProjectsStore>,
        user_id: &crate::domain::auth::entities::UserId,
        project_id: &str,
    ) -> Result<bool, AppError> {
        let access = store
            .get_project_access(*user_id, project_id)
            .await
            .map_err(|e| match e {
                StoreError::Conflict => AppError::Conflict("conflicting read".into()),
                StoreError::Storage(m) => AppError::Storage(m),
            })?;
        Ok(access
            .map(|a| Self::is_on_team(&a, user_id))
            .unwrap_or(false))
    }

    /// Accepts a pending invite. The invitee must be signed in as the
    /// invited address. The project 2FA gate is enforced here — at the
    /// moment of joining, not at invite time. Re-accepting an already
    /// accepted invite returns the membership (double-submit safe).
    pub async fn accept_invite(
        &self,
        actor: UserId,
        raw_token: &str,
    ) -> Result<crate::domain::projects::entities::ProjectMember, AppError> {
        use crate::domain::auth::value_objects::hash_token;

        let token_hash = hash_token(raw_token);
        let invite = self
            .projects
            .find_pending_invite(&token_hash)
            .await
            .map_err(|e| match e {
                StoreError::Conflict => AppError::Conflict("conflicting read".into()),
                StoreError::Storage(m) => AppError::Storage(m),
            })?;
        let invite = match invite {
            Some(invite) => invite,
            None => return self.accept_idempotent(actor, &token_hash).await,
        };
        if invite.expires_at <= chrono::Utc::now() {
            let _ = self.projects.decide_invite(&invite.id, false).await;
            return Err(AppError::TokenExpired(
                "this invitation has expired".to_string(),
            ));
        }

        let me = self
            .auth
            .find_user_by_id(actor)
            .await
            .map_err(|e| match e {
                StoreError::Conflict => AppError::Conflict("conflicting lookup".into()),
                StoreError::Storage(m) => AppError::Storage(m),
            })?
            .ok_or(AppError::Unauthorized)?;
        if !me.email.as_str().eq_ignore_ascii_case(&invite.email) {
            return Err(AppError::Forbidden(
                "this invitation belongs to a different account".to_string(),
            ));
        }
        if me.status == crate::domain::auth::entities::UserStatus::Suspended {
            return Err(AppError::Forbidden("that account is suspended".to_string()));
        }

        // The actor isn't on the team yet, so the flag must be read
        // without actor scoping; a gone project refuses the accept.
        let require_2fa = self
            .projects
            .project_require_2fa(&invite.project_id)
            .await
            .map_err(|e| match e {
                StoreError::Conflict => AppError::Conflict("conflicting read".into()),
                StoreError::Storage(m) => AppError::Storage(m),
            })?
            .ok_or_else(|| AppError::NotFound("project not found".into()))?;
        if require_2fa && !me.totp_enabled {
            return Err(AppError::Forbidden(
                "this project requires two-factor authentication: enable 2FA on your profile first"
                    .to_string(),
            ));
        }

        let already_member = self
            .projects
            .get_project_access(actor, &invite.project_id)
            .await
            .map_err(|e| match e {
                StoreError::Conflict => AppError::Conflict("conflicting read".into()),
                StoreError::Storage(m) => AppError::Storage(m),
            })?
            .as_ref()
            .map(|a| Self::is_on_team(a, &actor) || a.member_role.is_some())
            .unwrap_or(false);
        if !already_member {
            let inserted = self
                .projects
                .insert_member(&invite.project_id, actor, &invite.role)
                .await
                .map_err(|e| match e {
                    StoreError::Conflict => AppError::Conflict("already a member".into()),
                    StoreError::Storage(m) => AppError::Storage(m),
                })?;
            if !inserted {
                // Lost a race with another accept; fall through idempotently.
            }
        }
        self.projects
            .decide_invite(&invite.id, true)
            .await
            .map_err(|e| match e {
                StoreError::Conflict => AppError::Conflict("conflicting update".into()),
                StoreError::Storage(m) => AppError::Storage(m),
            })?;

        self.audit
            .record(
                chrono::Utc::now(),
                &AuditEvent::new(
                    AuditAction::ProjectMemberAdded,
                    Some(&actor.to_string()),
                    None,
                    Some(invite.project_id.as_str()),
                    format!("{} joined as {}", me.name, invite.role),
                    Some(serde_json::json!({ "user_id": actor.to_string() })),
                ),
            )
            .await;

        Ok(crate::domain::projects::entities::ProjectMember {
            user_id: actor.to_string(),
            name: me.name.clone(),
            email: me.email.as_str().to_string(),
            role: invite.role.clone(),
            has_2fa: me.totp_enabled,
            last_active_at: me.last_login_at,
        })
    }

    /// Double-submit safety: a consumed invite resolves to the membership
    /// when the caller already joined through it; anything else stays 404.
    async fn accept_idempotent(
        &self,
        actor: UserId,
        token_hash: &str,
    ) -> Result<crate::domain::projects::entities::ProjectMember, AppError> {
        let invite = self
            .projects
            .find_invite_by_hash(token_hash)
            .await
            .map_err(|e| match e {
                StoreError::Conflict => AppError::Conflict("conflicting read".into()),
                StoreError::Storage(m) => AppError::Storage(m),
            })?
            .ok_or_else(|| AppError::NotFound("invitation not found".into()))?;
        if invite.status != "accepted" {
            return Err(AppError::NotFound("invitation not found".into()));
        }
        let me = self
            .auth
            .find_user_by_id(actor)
            .await
            .map_err(|e| match e {
                StoreError::Conflict => AppError::Conflict("conflicting lookup".into()),
                StoreError::Storage(m) => AppError::Storage(m),
            })?
            .ok_or(AppError::Unauthorized)?;
        if !me.email.as_str().eq_ignore_ascii_case(&invite.email) {
            return Err(AppError::NotFound("invitation not found".into()));
        }
        // Report the live membership role, not the stale invite copy.
        let access = self
            .projects
            .get_project_access(actor, &invite.project_id)
            .await
            .map_err(|e| match e {
                StoreError::Conflict => AppError::Conflict("conflicting read".into()),
                StoreError::Storage(m) => AppError::Storage(m),
            })?;
        let role = match access.as_ref() {
            Some(a) if Self::is_on_team(a, &actor) => {
                if a.created_by.as_deref() == Some(actor.to_string().as_str()) {
                    "owner".to_string()
                } else {
                    a.member_role.clone().unwrap_or_else(|| invite.role.clone())
                }
            }
            _ => return Err(AppError::NotFound("invitation not found".into())),
        };
        Ok(crate::domain::projects::entities::ProjectMember {
            user_id: actor.to_string(),
            name: me.name.clone(),
            email: me.email.as_str().to_string(),
            role,
            has_2fa: me.totp_enabled,
            last_active_at: me.last_login_at,
        })
    }

    /// Preview of a pending invite for the accept page. Email-matched:
    /// other accounts get NotFound (no enumeration of invites).
    pub async fn get_invite(
        &self,
        actor: UserId,
        raw_token: &str,
    ) -> Result<crate::domain::projects::entities::ProjectInvite, AppError> {
        use crate::domain::auth::value_objects::hash_token;

        let invite = self
            .projects
            .find_pending_invite(&hash_token(raw_token))
            .await
            .map_err(|e| match e {
                StoreError::Conflict => AppError::Conflict("conflicting read".into()),
                StoreError::Storage(m) => AppError::Storage(m),
            })?
            .ok_or_else(|| AppError::NotFound("invitation not found".into()))?;
        if invite.expires_at <= chrono::Utc::now() {
            return Err(AppError::TokenExpired(
                "this invitation has expired".to_string(),
            ));
        }
        let me = self
            .auth
            .find_user_by_id(actor)
            .await
            .map_err(|e| match e {
                StoreError::Conflict => AppError::Conflict("conflicting lookup".into()),
                StoreError::Storage(m) => AppError::Storage(m),
            })?
            .ok_or(AppError::Unauthorized)?;
        if !me.email.as_str().eq_ignore_ascii_case(&invite.email) {
            return Err(AppError::NotFound("invitation not found".into()));
        }
        Ok(crate::domain::projects::entities::ProjectInvite {
            id: invite.id,
            project_id: invite.project_id,
            project_name: invite.project_name,
            email: invite.email,
            role: invite.role,
            expires_at: invite.expires_at,
        })
    }

    /// Declines a pending invite. Idempotent and quiet on unknown tokens
    /// (no enumeration of outstanding invites).
    pub async fn decline_invite(&self, actor: UserId, raw_token: &str) -> Result<(), AppError> {
        use crate::domain::auth::value_objects::hash_token;

        let Some(invite) = self
            .projects
            .find_pending_invite(&hash_token(raw_token))
            .await
            .map_err(|e| match e {
                StoreError::Conflict => AppError::Conflict("conflicting read".into()),
                StoreError::Storage(m) => AppError::Storage(m),
            })?
        else {
            return Ok(());
        };
        let me = self
            .auth
            .find_user_by_id(actor)
            .await
            .map_err(|e| match e {
                StoreError::Conflict => AppError::Conflict("conflicting lookup".into()),
                StoreError::Storage(m) => AppError::Storage(m),
            })?
            .ok_or(AppError::Unauthorized)?;
        if !me.email.as_str().eq_ignore_ascii_case(&invite.email) {
            return Err(AppError::Forbidden(
                "this invitation belongs to a different account".to_string(),
            ));
        }
        self.projects
            .decide_invite(&invite.id, false)
            .await
            .map_err(|e| match e {
                StoreError::Conflict => AppError::Conflict("conflicting update".into()),
                StoreError::Storage(m) => AppError::Storage(m),
            })?;

        self.audit
            .record(
                chrono::Utc::now(),
                &AuditEvent::new(
                    AuditAction::ProjectInviteDeclined,
                    Some(&actor.to_string()),
                    None,
                    Some(invite.project_id.as_str()),
                    format!("{} declined the team invitation", me.name),
                    None,
                ),
            )
            .await;
        Ok(())
    }

    /// Team roster for a project the caller belongs to, creator first.
    pub async fn list_members(
        &self,
        actor: UserId,
        project_id: &str,
    ) -> Result<Vec<crate::domain::projects::entities::ProjectMember>, AppError> {
        self.projects
            .get_project_access(actor, project_id)
            .await
            .map_err(|e| match e {
                StoreError::Conflict => AppError::Conflict("conflicting read".into()),
                StoreError::Storage(m) => AppError::Storage(m),
            })?
            .ok_or_else(|| AppError::NotFound("project not found".to_string()))?;
        Ok(self
            .projects
            .list_members(actor, project_id)
            .await
            .map_err(|e| match e {
                StoreError::Conflict => AppError::Conflict("conflicting read".into()),
                StoreError::Storage(m) => AppError::Storage(m),
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
