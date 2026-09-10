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
pub mod domain;
pub mod infra;
pub mod ports;
pub mod testing;

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
            std::sync::Arc::new(svc)
        });

        // Admin service: separate from auth — platform managers only.
        let admin = db.as_ref().map(|pool| {
            std::sync::Arc::new(domain::admin::AdminService::new(
                Box::new(infra::PgAdminStore::new(pool.clone())),
                config.auth.expose_dev_tokens,
                audit.clone().expect("audit service built with db"),
            ))
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
            std::sync::Arc::new(domain::projects::ProjectMembersService::new(
                store.clone(),
                store,
                audit.clone().expect("audit service built with db"),
            ))
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

        // Admin user management: list/stats/suspend across platform users.
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
                templates,
                channel_providers,
                tickets,
                billing,
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

        // Scheduled-broadcast worker: drains due broadcasts on an interval.
        // Runs only when the admin notifications service is wired; stops
        // with the process (a claimed-but-unsent row stays `sending` until
        // an operator re-queues it — see ARCHITECTURE.md).
        if let Some(notifications) = admin_notifications_for_worker.clone() {
            tokio::spawn(async move {
                let mut interval =
                    tokio::time::interval(std::time::Duration::from_secs(30));
                loop {
                    interval.tick().await;
                    match notifications
                        .run_due_worker(chrono::Utc::now())
                        .await
                    {
                        Ok(0) => {}
                        Ok(n) => tracing::info!(
                            sent = n,
                            "scheduled notification worker sent broadcasts"
                        ),
                        Err(e) => tracing::error!(
                            error = %e,
                            "scheduled notification worker failed"
                        ),
                    }
                }
            });
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
