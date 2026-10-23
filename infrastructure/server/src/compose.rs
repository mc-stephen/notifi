//! The composition root: every database-backed service, built exactly once.
//!
//! Both entry points — the API process ([`crate::run`]) and the `worker`
//! binary — call [`compose`], so a new service is wired in one place and
//! shows up in both. Config-only wiring (system mail, OAuth credentials,
//! the provider tester) stays with each entry point; the mail pieces are
//! passed in because several services bake them in.

use std::sync::Arc;

use sqlx::PgPool;

use crate::domain;
use crate::infra::config::AppConfig;
use crate::{infra, ports};

/// The full service graph, built against one pool.
///
/// Mirrors the `Option<Arc<..>>` set in [`crate::api::AppState`] minus the
/// config-only entries (`db`, `redis`, `oauth`, `provider_tester`). The API
/// maps these into `Some`s for AppState; the worker reaches for the three
/// sweep services (`admin_notifications`, `schedules`, `campaigns`).
pub struct ServiceGraph {
    pub audit: Arc<domain::audit::AuditService>,
    pub notifications: Arc<domain::notifications::NotificationService>,
    pub auth: Arc<domain::auth::AuthService>,
    pub admin: Arc<domain::admin::AdminService>,
    pub admin_users: Arc<domain::admin::AdminUsersService>,
    pub admin_projects: Arc<domain::admin::AdminProjectsService>,
    pub admin_notifications: Arc<domain::admin::AdminNotificationsService>,
    pub projects: Arc<domain::projects::ProjectService>,
    pub project_exports: Arc<domain::projects::ProjectExportService>,
    pub project_members: Arc<domain::projects::ProjectMembersService>,
    pub recipients: Arc<domain::recipients::RecipientService>,
    pub templates: Arc<domain::templates::TemplateService>,
    pub api_keys: Arc<domain::api_keys::ApiKeyService>,
    pub channel_providers: Arc<dyn ports::ChannelProviderStore + Send + Sync>,
    pub tickets: Arc<domain::support::TicketService>,
    pub billing: Arc<domain::billing::BillingService>,
    pub flows: Arc<domain::flows::FlowService>,
    pub sends: Arc<domain::sends::SendService>,
    pub schedules: Arc<domain::schedules::ScheduleService>,
    pub inbound: Arc<domain::inbound::InboundService>,
    pub webhooks: Arc<domain::webhooks::WebhookService>,
    pub campaigns: Arc<domain::campaigns::CampaignService>,
}

/// Builds the full service graph against `pool`.
///
/// `mailer` / `system_templates` are built by the entry point (the API
/// verifies SMTP at boot before calling this; the worker passes `None` —
/// none of its sweep services consume mail). `config_store` likewise
/// arrives pre-built so both processes agree on one storage decision:
/// the API passes its count-wrapped store (writes invalidate the cache),
/// the worker a plain driver — its purge path needs no cache coherence
/// (a stale key only lives out its TTL, and creation re-invalidates).
pub fn compose(
    pool: PgPool,
    config: &AppConfig,
    mailer: Option<Arc<dyn ports::SmtpMailer>>,
    system_templates: Option<Arc<dyn ports::SystemTemplates>>,
    config_store: Arc<dyn ports::ConfigStore + Send + Sync>,
) -> ServiceGraph {
    // Audit log listener: built from the same DB, shared by auth/projects
    // (they emit actions) and the /app/logs query surface.
    let audit = Arc::new(domain::audit::AuditService::new(Arc::new(
        infra::audit_repository_pg::PgAuditStore::new(pool.clone()),
    )));

    // In-app notifications: personal, per-user, no project scope. Built
    // before auth so the auth service can capture it.
    let notifications = Arc::new(domain::notifications::NotificationService::new(Arc::new(
        infra::PgNotificationsStore::new(pool.clone()),
    )));

    // Auth.
    let auth = domain::auth::AuthService::new(
        Arc::new(infra::PgAuthStore::new(pool.clone())),
        config.auth.expose_dev_tokens,
        audit.clone(),
    )
    .with_notifications(notifications.clone());
    let auth = match (mailer.clone(), system_templates.clone()) {
        (Some(m), Some(t)) => auth.with_system_mail(m, t, config.oauth.dashboard_url.clone()),
        _ => auth,
    };
    let auth = Arc::new(auth);

    // Admin service: separate from auth — platform managers only.
    let admin = domain::admin::AdminService::new(
        Box::new(infra::PgAdminStore::new(pool.clone())),
        config.auth.expose_dev_tokens,
        audit.clone(),
    );
    let admin = match (mailer.clone(), system_templates.clone()) {
        (Some(m), Some(t)) => admin.with_system_mail(m, t, config.mail.admin_url.clone()),
        _ => admin,
    };
    let admin = Arc::new(admin);

    // Projects slice: same store backing, separate service instance.
    // Brand storage rides in so creation seeds the bucket folder, the
    // deletion purge wipes it, and exports can list the files. The same
    // (cache-wrapped on the API side) instance is shared by the export
    // service so every read/write sees one storage view.
    let export_config_store = Arc::clone(&config_store);
    let projects = Arc::new(
        domain::projects::ProjectService::new(
            Arc::new(infra::PgAuthStore::new(pool.clone())),
            audit.clone(),
        )
        .with_config_store(config_store),
    );

    // Project team management: 2FA gate flag + member invites. Needs user
    // lookup (TOTP status), so it holds both stores.
    let member_store = Arc::new(infra::PgAuthStore::new(pool.clone()));
    let project_members = domain::projects::ProjectMembersService::new(
        member_store.clone(),
        member_store,
        audit.clone(),
    );
    let project_members = match (mailer.clone(), system_templates.clone()) {
        (Some(m), Some(t)) => project_members.with_system_mail(
            m,
            t,
            config.oauth.dashboard_url.clone(),
            config.auth.expose_dev_tokens,
        ),
        _ => project_members,
    };
    let project_members = Arc::new(project_members);

    // Recipients slice: brand end-users, scoped to a project the caller
    // belongs to. Shares the audit service for create/delete events.
    let recipients = Arc::new(domain::recipients::RecipientService::new(
        Arc::new(infra::PgRecipientsStore::new(pool.clone())),
        audit.clone(),
    ));

    // Templates slice: per-channel message definitions with attachments.
    let templates = Arc::new(domain::templates::TemplateService::new(
        Arc::new(infra::PgTemplatesStore::new(pool.clone())),
        audit.clone(),
    ));

    // Project API keys + per-key IP allowlists. Record-only until the
    // external API lands; verification is exposed as a domain seam.
    let api_keys = Arc::new(domain::api_keys::ApiKeyService::new(
        Arc::new(infra::PgApiKeysStore::new(pool.clone())),
        audit.clone(),
    ));

    // Channel providers: per-project provider configurations (API keys, secrets).
    let channel_providers: Arc<dyn ports::ChannelProviderStore + Send + Sync> =
        Arc::new(infra::PgChannelProviderStore::new(pool.clone()));

    // Support tickets: personal or project-scoped tickets with status.
    let tickets = Arc::new(domain::support::TicketService::new(
        Arc::new(infra::PgTicketsStore::new(pool.clone())),
        audit.clone(),
    ));

    // Billing: plan catalog + per-project subscriptions (record-only; no
    // payment provider yet). New projects land on free via lazy ensure on
    // first subscription read.
    let billing = Arc::new(domain::billing::BillingService::new(
        Arc::new(infra::PgBillingStore::new(pool.clone())),
        audit.clone(),
    ));

    // Flows: system default flow per project (seeded at creation).
    let flows = Arc::new(domain::flows::FlowService::new(Arc::new(
        infra::PgFlowStore::new(pool.clone()),
    )));

    // Sends: record-first product sends executed through default flows.
    let sends = Arc::new(domain::sends::SendService::new(
        Arc::new(infra::PgSendStore::new(pool.clone())),
        flows.clone(),
        channel_providers.clone(),
        Arc::new(infra::delivery_adapters::DefaultAdapterFactory),
        Arc::new(infra::ReqwestHttpRequester::new()),
    ));

    // Flow schedules: timed runs of assigned flows. Built after sends
    // (execution records send rows) with the same provider wiring.
    let schedules = Arc::new(domain::schedules::ScheduleService::new(
        Arc::new(infra::PgScheduleStore::new(pool.clone())),
        Arc::new(infra::PgFlowStore::new(pool.clone())),
        Arc::new(infra::PgSendStore::new(pool.clone())),
        Arc::new(infra::PgTemplatesStore::new(pool.clone())),
        channel_providers.clone(),
        Arc::new(infra::delivery_adapters::DefaultAdapterFactory),
        Arc::new(infra::ReqwestHttpRequester::new()),
    ));

    // Inbound webhook fires: token-authenticated, sessionless. Shares the
    // send records, templates, and provider wiring.
    let inbound = Arc::new(domain::inbound::InboundService::new(
        Arc::new(infra::PgTriggerTokenStore::new(pool.clone())),
        Arc::new(infra::PgSendStore::new(pool.clone())),
        Arc::new(infra::PgTemplatesStore::new(pool.clone())),
        channel_providers.clone(),
        Arc::new(infra::delivery_adapters::DefaultAdapterFactory),
        Arc::new(infra::ReqwestHttpRequester::new()),
    ));

    // Owner-managed webhooks: each creates + owns exactly one flow (seeded
    // with a locked webhook trigger). Shares the flow store and the
    // inbound token sync.
    let webhooks = Arc::new(domain::webhooks::WebhookService::new(
        Arc::new(infra::PgWebhookStore::new(pool.clone())),
        Arc::new(infra::PgFlowStore::new(pool.clone())),
        inbound.clone(),
    ));

    // Data exports: the project-download service. Stores only — reads are
    // already tombstone-tolerant where needed; rate limiting and the audit
    // stay with ProjectService::guard_export.
    let project_exports = Arc::new(
        domain::projects::ProjectExportService::new(
            Arc::new(infra::PgAuthStore::new(pool.clone())),
            Arc::new(infra::PgTemplatesStore::new(pool.clone())),
            Arc::new(infra::PgApiKeysStore::new(pool.clone())),
            Arc::new(infra::PgSendStore::new(pool.clone())),
            Arc::new(infra::PgRecipientsStore::new(pool.clone())),
            Arc::new(infra::PgWebhookStore::new(pool.clone())),
            Arc::new(infra::PgFlowStore::new(pool.clone())),
        )
        .with_config_store(export_config_store),
    );

    // Campaigns: flow-less message origins. Built after sends (rows deliver
    // through default flows) and recipients/templates.
    let campaigns = Arc::new(domain::campaigns::CampaignService::new(
        Arc::new(infra::PgCampaignStore::new(pool.clone())),
        Arc::new(infra::PgSendStore::new(pool.clone())),
        sends.clone(),
        Arc::new(infra::PgRecipientsStore::new(pool.clone())),
        Arc::new(infra::PgTemplatesStore::new(pool.clone())),
    ));

    // Admin user management (list/stats/suspend). Holds the user store
    // twice (view + mutate).
    let admin_user_store = Arc::new(infra::PgAuthStore::new(pool.clone()));
    let admin_users = Arc::new(domain::admin::AdminUsersService::new(
        admin_user_store.clone(),
        admin_user_store,
        Arc::new(infra::PgNotificationsStore::new(pool.clone())),
        Arc::new(infra::PgTicketsStore::new(pool.clone())),
        audit.clone(),
    ));

    // Admin project views: list/detail across all platform projects.
    let admin_projects = Arc::new(domain::admin::AdminProjectsService::new(Arc::new(
        infra::PgAuthStore::new(pool.clone()),
    )));

    // Admin notification broadcasts (in-app): compose, history, scheduling.
    // Shares the auth + notifications stores with the services above.
    let admin_auth_store = Arc::new(infra::PgAuthStore::new(pool.clone()));
    let admin_notifications = Arc::new(domain::admin::AdminNotificationsService::new(
        admin_auth_store.clone(),
        admin_auth_store,
        Arc::new(infra::PgNotificationsStore::new(pool.clone())),
        audit.clone(),
    ));

    ServiceGraph {
        audit,
        notifications,
        auth,
        admin,
        admin_users,
        admin_projects,
        admin_notifications,
        projects,
        project_exports,
        project_members,
        recipients,
        templates,
        api_keys,
        channel_providers,
        tickets,
        billing,
        flows,
        sends,
        schedules,
        inbound,
        webhooks,
        campaigns,
    }
}
