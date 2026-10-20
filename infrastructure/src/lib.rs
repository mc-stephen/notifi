//! Notifi server — HTTP API, domain logic, and infrastructure in one crate.
//!
//! Layout (`docs/ARCHITECTURE.md` §2):
//! * [`api`] — axum presentation: ops at root, external product API at
//!   `/v1`, dashboard backend at `/app`, administration at
//!   `/admin`; middleware, error mapping.
//! * [`domain`] — business logic and models, framework-free.
//! * [`ports`] — traits/contracts the domain depends on ([`ports::AuthStore`],
//!   [`ports::OAuthIdentityProvider`]); implemented by `infra`.
//! * [`infra`] — concrete drivers: Postgres repositories, OAuth HTTP client,
//!   config, telemetry, Redis.

pub mod api;
pub mod channels;
pub mod domain;
pub mod infra;
pub mod kernel;
pub mod ports;

use crate::infra::config::AppConfig;

/// Process entry point: config → database → redis → router → serve.
pub fn run() {
    let _ = run_inner();
}

fn run_inner() -> Result<(), String> {
    // Local secrets live in `infrastructure/.env` (gitignored); real env
    // vars still win over file values.
    dotenvy::dotenv().ok();

    let _guard = match infra::telemetry::init() {
        Ok(guard) => guard,
        Err(e) => {
            eprintln!("failed to initialize telemetry: {e}");
            return Err(e.to_string());
        }
    };

    let config = match AppConfig::from_env() {
        Ok(config) => config,
        Err(e) => {
            tracing::error!("invalid configuration: {e}");
            return Err(e);
        }
    };

    tracing::info!(
        host = %config.server.host,
        port = config.server.port,
        "api bootstrap complete"
    );

    let rt = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .expect("failed to build tokio runtime");

    rt.block_on(async move {
        let db = infra::db::connect(&config).await;
        let redis_conn = infra::redis::connect(&config);

        // Audit log listener: built from the same DB, shared by auth/projects
        // (they emit actions) and the /app/logs query surface.
        let audit = db.as_ref().map(|pool| {
            std::sync::Arc::new(domain::audit::AuditService::new(
                std::sync::Arc::new(infra::audit_repository_pg::PgAuditStore::new(pool.clone())),
            ))
        });

        // In-app notifications: personal, per-user. No project scope.
        // Built before auth so the auth service can capture it.
        let notifications = db.as_ref().map(|pool| {
            std::sync::Arc::new(domain::notifications::NotificationService::new(
                std::sync::Arc::new(infra::PgNotificationsStore::new(pool.clone())),
            ))
        });

        // System mail (transactional emails). Built only when SMTP is
        // configured; flows keep their dev behavior otherwise.
        let templates: Option<std::sync::Arc<dyn ports::SystemTemplates>> =
            Some(std::sync::Arc::new(infra::FileSystemTemplates::from_config(
                &config.mail,
            )));
        let mailer: Option<std::sync::Arc<dyn ports::SmtpMailer>> =
            config.mail.enabled().then(|| {
                let live = infra::LiveSmtpMailer::from_config(&config.mail)
                    .expect("mailer enabled without SMTP host");
                std::sync::Arc::new(live) as std::sync::Arc<dyn ports::SmtpMailer>
            });
        if mailer.is_some() {
            tracing::info!("system mail configured (verifying at boot)");
        }

        // SMTP tripwire: handshake + auth now, not at the first send. A
        // dead relay logs loud but never blocks boot (mail stays optional).
        if let Some(mailer) = mailer.as_ref() {
            match tokio::time::timeout(std::time::Duration::from_secs(5), mailer.verify()).await
            {
                Ok(Ok(true)) => {
                    tracing::info!("system mail verified (smtp handshake + auth ok)")
                }
                Ok(Ok(false)) => {
                    tracing::warn!("system mail CONFIGURED BUT BROKEN: relay refused NOOP")
                }
                Ok(Err(e)) => {
                    tracing::warn!(error = %e, "system mail CONFIGURED BUT BROKEN")
                }
                Err(_) => tracing::warn!("system mail check timed out after 5s"),
            }
        }

        // Auth is wired only when a database exists; its routes answer 503
        // otherwise (composition root wires the sqlx store into the service).
        let auth = db.as_ref().map(|pool| {
            let svc = domain::auth::AuthService::new(
                std::sync::Arc::new(infra::PgAuthStore::new(pool.clone())),
                config.auth.expose_dev_tokens,
                audit.clone().expect("audit service built with db"),
            );
            // Wire notifications if available (fire-and-forget on emit).
            let svc = if let Some(n) = notifications.clone() {
                svc.with_notifications(n)
            } else {
                svc
            };
            // Wire system mail the same way (fire-and-forget per email).
            let svc = match (mailer.clone(), templates.clone()) {
                (Some(m), Some(t)) => {
                    svc.with_system_mail(m, t, config.oauth.dashboard_url.clone())
                }
                _ => svc,
            };
            std::sync::Arc::new(svc)
        });

        // Admin service: separate from auth — platform managers only.
        let admin = db.as_ref().map(|pool| {
            let svc = domain::admin::AdminService::new(
                Box::new(infra::PgAdminStore::new(pool.clone())),
                config.auth.expose_dev_tokens,
                audit.clone().expect("audit service built with db"),
            );
            let svc = match (mailer.clone(), templates.clone()) {
                (Some(m), Some(t)) => svc.with_system_mail(m, t, config.mail.admin_url.clone()),
                _ => svc,
            };
            std::sync::Arc::new(svc)
        });

        // Projects slice: same store backing, separate service instance.
        let projects = db.as_ref().map(|pool| {
            std::sync::Arc::new(domain::projects::ProjectService::new(
                std::sync::Arc::new(infra::PgAuthStore::new(pool.clone())),
                audit.clone().expect("audit service built with db"),
            ))
        });

        // Project team management: 2FA gate flag + member invites. Needs
        // user lookup (TOTP status), so it holds both stores.
        let project_members = db.as_ref().map(|pool| {
            let store = std::sync::Arc::new(infra::PgAuthStore::new(pool.clone()));
            let svc = domain::projects::ProjectMembersService::new(
                store.clone(),
                store,
                audit.clone().expect("audit service built with db"),
            );
            let svc = match (mailer.clone(), templates.clone()) {
                (Some(m), Some(t)) => svc.with_system_mail(
                    m,
                    t,
                    config.oauth.dashboard_url.clone(),
                    config.auth.expose_dev_tokens,
                ),
                _ => svc,
            };
            std::sync::Arc::new(svc)
        });

        // Recipients slice: brand end-users, scoped to a project the caller
        // belongs to. Shares the audit service for create/delete events.
        let recipients = db.as_ref().map(|pool| {
            std::sync::Arc::new(domain::recipients::RecipientService::new(
                std::sync::Arc::new(infra::PgRecipientsStore::new(pool.clone())),
                audit.clone().expect("audit service built with db"),
            ))
        });

        // Templates slice: per-channel message definitions with attachments.
        let templates = db.as_ref().map(|pool| {
            std::sync::Arc::new(domain::templates::TemplateService::new(
                std::sync::Arc::new(infra::PgTemplatesStore::new(pool.clone())),
                audit.clone().expect("audit service built with db"),
            ))
        });

        // Project API keys + per-key IP allowlists. Record-only until the
        // external API lands; verification is exposed as a domain seam.
        let api_keys = db.as_ref().map(|pool| {
            std::sync::Arc::new(domain::api_keys::ApiKeyService::new(
                std::sync::Arc::new(infra::PgApiKeysStore::new(pool.clone())),
                audit.clone().expect("audit service built with db"),
            ))
        });

        // Channel providers: per-project provider configurations (API keys, secrets).
        let channel_providers: Option<std::sync::Arc<dyn ports::ChannelProviderStore + Send + Sync>> = db.as_ref().map(|pool| {
            std::sync::Arc::new(infra::PgChannelProviderStore::new(pool.clone())) as std::sync::Arc<dyn ports::ChannelProviderStore + Send + Sync>
        });

        // Support tickets: personal or project-scoped tickets with status.
        let tickets = db.as_ref().map(|pool| {
            std::sync::Arc::new(domain::support::TicketService::new(
                std::sync::Arc::new(infra::PgTicketsStore::new(pool.clone())),
                audit.clone().expect("audit service built with db"),
            ))
        });

        // Billing: plan catalog + per-project subscriptions (record-only;
        // no payment provider yet). New projects land on free via lazy
        // ensure on first subscription read.
        let billing = db.as_ref().map(|pool| {
            std::sync::Arc::new(domain::billing::BillingService::new(
                std::sync::Arc::new(infra::PgBillingStore::new(pool.clone())),
                audit.clone().expect("audit service built with db"),
            ))
        });

        // Flows: system default flow per project (seeded at creation).
        let flows = db.as_ref().map(|pool| {
            std::sync::Arc::new(domain::flows::FlowService::new(std::sync::Arc::new(
                infra::PgFlowStore::new(pool.clone()),
            )))
        });

        // Sends: record-first product sends executed through default flows.
        let sends = db.as_ref().map(|pool| {
            let channel_providers = channel_providers
                .clone()
                .expect("channel providers built with db");
            std::sync::Arc::new(domain::sends::SendService::new(
                std::sync::Arc::new(infra::PgSendStore::new(pool.clone())),
                flows.clone().expect("flows built with db"),
                channel_providers,
                std::sync::Arc::new(infra::delivery_adapters::DefaultAdapterFactory),
                std::sync::Arc::new(infra::ReqwestHttpRequester::new()),
            ))
        });

        // Flow schedules: timed runs of assigned flows. Built after sends
        // (execution records send rows) with the same provider wiring.
        let schedules = db.as_ref().map(|pool| {
            let channel_providers = channel_providers
                .clone()
                .expect("channel providers built with db");
            std::sync::Arc::new(domain::schedules::ScheduleService::new(
                std::sync::Arc::new(infra::PgScheduleStore::new(pool.clone())),
                std::sync::Arc::new(infra::PgFlowStore::new(pool.clone())),
                std::sync::Arc::new(infra::PgSendStore::new(pool.clone())),
                std::sync::Arc::new(infra::PgTemplatesStore::new(pool.clone())),
                channel_providers,
                std::sync::Arc::new(infra::delivery_adapters::DefaultAdapterFactory),
                std::sync::Arc::new(infra::ReqwestHttpRequester::new()),
            ))
        });

        // Inbound webhook fires: token-authenticated, sessionless.
        // Shares the send records, templates, and provider wiring.
        let inbound = db.as_ref().map(|pool| {
            let channel_providers = channel_providers
                .clone()
                .expect("channel providers built with db");
            std::sync::Arc::new(domain::inbound::InboundService::new(
                std::sync::Arc::new(infra::PgTriggerTokenStore::new(pool.clone())),
                std::sync::Arc::new(infra::PgSendStore::new(pool.clone())),
                std::sync::Arc::new(infra::PgTemplatesStore::new(pool.clone())),
                channel_providers,
                std::sync::Arc::new(infra::delivery_adapters::DefaultAdapterFactory),
                std::sync::Arc::new(infra::ReqwestHttpRequester::new()),
            ))
        });

        // Owner-managed webhooks: each creates + owns exactly one flow
        // (seeded with a locked webhook trigger). Shares the flow store
        // and the inbound token sync.
        let webhooks = db.as_ref().map(|pool| {
            std::sync::Arc::new(domain::webhooks::WebhookService::new(
                std::sync::Arc::new(infra::PgWebhookStore::new(pool.clone())),
                std::sync::Arc::new(infra::PgFlowStore::new(pool.clone())),
                inbound.clone().expect("inbound built with db"),
            ))
        });

        // Campaigns: flow-less message origins. Built after sends (rows
        // deliver through default flows) and recipients/templates.
        let campaigns = db.as_ref().map(|pool| {
            std::sync::Arc::new(domain::campaigns::CampaignService::new(
                std::sync::Arc::new(infra::PgCampaignStore::new(pool.clone())),
                std::sync::Arc::new(infra::PgSendStore::new(pool.clone())),
                sends.clone().expect("sends built with db"),
                std::sync::Arc::new(infra::PgRecipientsStore::new(pool.clone())),
                std::sync::Arc::new(infra::PgTemplatesStore::new(pool.clone())),
            ))
        });
        let admin_users = db.as_ref().map(|pool| {
            let auth_store = std::sync::Arc::new(infra::PgAuthStore::new(pool.clone()));
            std::sync::Arc::new(domain::admin::AdminUsersService::new(
                auth_store.clone(),
                auth_store,
                std::sync::Arc::new(infra::PgNotificationsStore::new(pool.clone())),
                std::sync::Arc::new(infra::PgTicketsStore::new(pool.clone())),
                audit.clone().expect("audit service built with db"),
            ))
        });

        // Admin project views: list/detail across all platform projects.
        let admin_projects = db.as_ref().map(|pool| {
            std::sync::Arc::new(domain::admin::AdminProjectsService::new(
                std::sync::Arc::new(infra::PgAuthStore::new(pool.clone())),
            ))
        });

        // Admin notification broadcasts (in-app): compose, history, scheduling.
        // Shares the auth + notifications stores with the services above.
        let admin_notifications = db.as_ref().map(|pool| {
            let auth_store = std::sync::Arc::new(infra::PgAuthStore::new(pool.clone()));
            std::sync::Arc::new(domain::admin::AdminNotificationsService::new(
                auth_store.clone(),
                auth_store,
                std::sync::Arc::new(infra::PgNotificationsStore::new(pool.clone())),
                audit.clone().expect("audit service built with db"),
            ))
        });

        // Provider tester: always available (validates credentials before saving).
        let provider_tester: std::sync::Arc<dyn ports::ProviderTester + Send + Sync> = std::sync::Arc::new(infra::ConfigProviderTester::new());

        // OAuth sign-in is wired when at least one provider has credentials.
        let github = match (
            config.oauth.github_client_id.clone(),
            config.oauth.github_client_secret.clone(),
        ) {
            (Some(client_id), Some(client_secret)) => {
                Some(infra::ProviderCredentials { client_id, client_secret })
            }
            _ => None,
        };
        let google = match (
            config.oauth.google_client_id.clone(),
            config.oauth.google_client_secret.clone(),
        ) {
            (Some(client_id), Some(client_secret)) => {
                Some(infra::ProviderCredentials { client_id, client_secret })
            }
            _ => None,
        };
        let oauth = if github.is_some() || google.is_some() {
            Some(std::sync::Arc::new(ports::OAuthRuntime {
                provider: infra::http_oauth_provider(
                    github,
                    google,
                    config.oauth.api_base_url.clone(),
                ),
                dashboard_url: config.oauth.dashboard_url.clone(),
            }))
        } else {
            None
        };

        if db.is_none() {
            tracing::warn!("database unavailable/disabled; readiness will report 503");
        }
        if redis_conn.is_none() {
            tracing::warn!("redis unavailable/disabled; readiness will report 503");
        }
        if auth.is_none() {
            tracing::warn!("auth disabled (needs database); /app/auth routes will answer 503");
        }
        if projects.is_none() {
            tracing::warn!("projects disabled (needs database); /app/projects routes will answer 503");
        }
        if audit.is_none() {
            tracing::warn!("audit log disabled (needs database); /app/logs routes will answer 503");
        }
        if recipients.is_none() {
            tracing::warn!(
                "recipients disabled (needs database); /app/projects/{{project_id}}/recipients routes will answer 503"
            );
        }
        if templates.is_none() {
            tracing::warn!(
                "templates disabled (needs database); /app/projects/{{project_id}}/templates routes will answer 503"
            );
        }
        if channel_providers.is_none() {
            tracing::warn!(
                "channel_providers disabled (needs database); /app/projects/{{project_id}}/channel-configs routes will answer 503"
            );
        }
        if tickets.is_none() {
            tracing::warn!(
                "support tickets disabled (needs database); /app/support/tickets routes will answer 503"
            );
        }
        if billing.is_none() {
            tracing::warn!(
                "billing disabled (needs database); /app/billing + /admin/billing routes will answer 503"
            );
        }
        if flows.is_none() || sends.is_none() {
            tracing::warn!(
                "flows/sends disabled (needs database); /v1/notifications + flow routes will answer 503"
            );
        }
        if schedules.is_none() {
            tracing::warn!(
                "schedules disabled (needs database); /app/schedules routes will answer 503"
            );
        }
        if webhooks.is_none() {
            tracing::warn!(
                "webhooks disabled (needs database); /app/webhooks routes will answer 503"
            );
        }
        if campaigns.is_none() {
            tracing::warn!(
                "campaigns disabled (needs database); /app/campaigns routes will answer 503"
            );
        }
        if admin_users.is_none() {
            tracing::warn!(
                "admin user management disabled (needs database); /admin/users routes will answer 503"
            );
        }
        if notifications.is_none() {
            tracing::warn!(
                "in-app notifications disabled (needs database); /app/notifications routes will answer 503"
            );
        }
        if oauth.is_none() {
            tracing::warn!(
                "oauth disabled (no provider credentials); /app/auth/oauth routes will answer 503"
            );
        }
        if admin_projects.is_none() {
            tracing::warn!(
                "admin project views disabled (needs database); /admin/projects routes will answer 503"
            );
        }
        if admin_notifications.is_none() {
            tracing::warn!(
                "admin notifications disabled (needs database); /admin/notifications routes will answer 503"
            );
        }

        // Cloned for the scheduled-broadcast worker below (AppState takes
        // ownership of the original).
        let admin_notifications_for_worker = admin_notifications.clone();
        // Same for the flow-schedule + campaign workers.
        let schedules_for_worker = schedules.clone();
        let campaigns_for_worker = campaigns.clone();

        let app = api::build_router(
            api::AppState {
                db,
                redis: redis_conn,
                auth,
                oauth,
                admin,
                admin_users,
                admin_projects,
                admin_notifications,
                projects,
                project_members,
                audit,
                recipients,
                api_keys,
                templates,
                channel_providers,
                tickets,
                billing,
                flows,
                sends,
                inbound,
                schedules,
                webhooks,
                campaigns,
                notifications,
                provider_tester,
            },
            &config,
        );

        let listener = match tokio::net::TcpListener::bind((
            config.server.host.as_str(),
            config.server.port,
        ))
        .await
        {
            Ok(listener) => listener,
            Err(e) => {
                tracing::error!(
                    host = %config.server.host,
                    port = config.server.port,
                    error = %e,
                    "failed to bind listener"
                );
                return;
            }
        };

        tracing::info!(
            host = %config.server.host,
            port = config.server.port,
            "api listening"
        );

        // Sweep dispatch: an external `worker` binary shares the load through
        // RabbitMQ when configured (one tick per interval); otherwise the
        // embedded loop below does the work. A failed tick publish degrades
        // to a local sweep so a RabbitMQ outage stalls nothing — the atomic
        // claims keep a concurrent worker from double-firing (a
        // claimed-but-unsent broadcast stays `sending` until an operator
        // re-queues it — see ARCHITECTURE.md).
        let rabbit = infra::rabbit::connect(&config).await;
        if let Some(hub) = rabbit {
            let admin = admin_notifications_for_worker.clone();
            let sched = schedules_for_worker.clone();
            let camps = campaigns_for_worker.clone();
            tokio::spawn(async move {
                let mut interval = tokio::time::interval(std::time::Duration::from_secs(
                    infra::rabbit::SWEEP_INTERVAL_SECS,
                ));
                loop {
                    interval.tick().await;
                    match hub.publish_sweep().await {
                        Ok(()) => {}
                        Err(e) => {
                            tracing::warn!(
                                error = %e,
                                "sweep tick failed; running degraded local sweep"
                            );
                            run_sweep_once(&admin, &sched, &camps).await;
                        }
                    }
                }
            });
        } else {
            tracing::info!("sweeps embedded (no rabbitmq configured)");
            let admin = admin_notifications_for_worker.clone();
            let sched = schedules_for_worker.clone();
            let camps = campaigns_for_worker.clone();
            if admin.is_some() || sched.is_some() || camps.is_some() {
                tokio::spawn(async move {
                    let mut interval = tokio::time::interval(std::time::Duration::from_secs(
                        infra::rabbit::SWEEP_INTERVAL_SECS,
                    ));
                    loop {
                        interval.tick().await;
                        run_sweep_once(&admin, &sched, &camps).await;
                    }
                });
            }
        }

        if let Err(e) = axum::serve(listener, app)
            .with_graceful_shutdown(shutdown_signal())
            .await
        {
            tracing::error!("server error: {e}");
        }

        tracing::info!("api shutdown complete");
    });

    Ok(())
}

/// Runs one sweep of due broadcasts + due schedules + campaigns. Shared by
/// the embedded loop and the degraded tick path above.
async fn run_sweep_once(
    admin_notifications: &Option<std::sync::Arc<domain::admin::AdminNotificationsService>>,
    schedules: &Option<std::sync::Arc<domain::schedules::ScheduleService>>,
    campaigns: &Option<std::sync::Arc<domain::campaigns::CampaignService>>,
) {
    if let Some(notifications) = admin_notifications {
        match notifications.run_due_worker(chrono::Utc::now()).await {
            Ok(0) => {}
            Ok(n) => tracing::info!(sent = n, "scheduled notification worker sent broadcasts"),
            Err(e) => tracing::error!(error = %e, "scheduled notification worker failed"),
        }
    }
    if let Some(schedules) = schedules {
        match schedules.run_due(chrono::Utc::now()).await {
            Ok(0) => {}
            Ok(n) => tracing::info!(ran = n, "schedule worker fired due schedules"),
            Err(e) => tracing::error!(error = %e, "schedule worker failed"),
        }
    }
    if let Some(campaigns) = campaigns {
        match campaigns.tick(chrono::Utc::now()).await {
            Ok(0) => {}
            Ok(n) => tracing::info!(ran = n, "campaign worker processed campaigns"),
            Err(e) => tracing::error!(error = %e, "campaign worker failed"),
        }
    }
}

/// Resolves on SIGINT (Ctrl-C) or SIGTERM; triggers graceful shutdown.
async fn shutdown_signal() {
    let ctrl_c = async {
        tokio::signal::ctrl_c()
            .await
            .expect("failed to install ctrl-c handler");
    };

    #[cfg(unix)]
    let terminate = async {
        tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
            .expect("failed to install SIGTERM handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {},
        _ = terminate => {},
    }
    tracing::info!("shutdown signal received");
}
