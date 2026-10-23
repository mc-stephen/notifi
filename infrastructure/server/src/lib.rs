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
//! * [`compose`] — the composition root: builds the shared service graph
//!   exactly once for both entry points (the API process and the worker).

pub mod api;
pub mod channels;
pub mod compose;
pub mod domain;
pub mod infra;
pub mod kernel;
pub mod ports;

/// Test-only support utilities (local HTTP stub for provider controller
/// tests). Not part of any runtime binary — gated away outside test builds.
#[cfg(test)]
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

        // System mail (transactional emails). Built only when SMTP is
        // configured; flows keep their dev behavior otherwise.
        let system_templates: Option<std::sync::Arc<dyn ports::SystemTemplates>> = Some(
            std::sync::Arc::new(infra::FileSystemTemplates::from_config(&config.mail)),
        );
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
            match tokio::time::timeout(std::time::Duration::from_secs(5), mailer.verify()).await {
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

        // Tenant project config storage: the factory picks the driver and
        // logs the choice (R2 when NOTIFI_R2_* is all set, filesystem
        // otherwise), so ops flags and boot can never pick differently.
        // Boot also lays the top-level folders down (`projects/`) so the
        // storage layout exists before the first project arrives.
        // Read-through config cache when Redis is reachable: warm keys
        // answer at Redis speed and the store behind sees nothing, so
        // R2 request counts (and per-delivery latency) stay near zero.
        // Writes invalidate through the wrapper, never fail the send path.
        // Built before `compose` so the project service (creation seeding,
        // deletion purge) shares this exact instance — cache invalidation
        // only works if every writer goes through the same wrapper.
        let config_store: std::sync::Arc<dyn ports::ConfigStore + Send + Sync> =
            infra::config_store_factory::build_ensured(&config)
                .await
                .expect("config store builds");
        let config_store: std::sync::Arc<dyn ports::ConfigStore + Send + Sync> =
            match &redis_conn {
                Some(client) => match client.clone().get_connection_manager().await {
                    Ok(conn) => {
                        tracing::info!(
                            ttl_secs = infra::config_cache::CONFIG_CACHE_TTL.as_secs(),
                            "config cache: redis"
                        );
                        std::sync::Arc::new(infra::CachedConfigStore::new(
                            config_store,
                            std::sync::Arc::new(infra::RedisCacheBackend::new(conn)),
                        ))
                    }
                    Err(e) => {
                        tracing::warn!(error = %e, "config cache unavailable; reads hit the store directly");
                        config_store
                    }
                },
                None => {
                    tracing::info!("config cache: none (redis unset); reads hit the store directly");
                    config_store
                }
            };

        // Composition root: every database-backed service, built exactly
        // once — the worker binary calls the same `compose`, so a new
        // service wires in one place for both processes. Mail/system
        // templates come from this entry point (the API verifies SMTP at
        // boot; the worker passes None — no sweep service consumes mail).
        let graph = db.as_ref().map(|pool| {
            compose::compose(
                pool.clone(),
                &config,
                mailer.clone(),
                system_templates.clone(),
                config_store.clone(),
            )
        });

        // Provider tester: always available (validates credentials before saving).
        let provider_tester: std::sync::Arc<dyn ports::ProviderTester + Send + Sync> =
            std::sync::Arc::new(infra::ConfigProviderTester::new());

        // OAuth sign-in is wired when at least one provider has credentials.
        let github = match (
            config.oauth.github_client_id.clone(),
            config.oauth.github_client_secret.clone(),
        ) {
            (Some(client_id), Some(client_secret)) => Some(infra::ProviderCredentials {
                client_id,
                client_secret,
            }),
            _ => None,
        };
        let google = match (
            config.oauth.google_client_id.clone(),
            config.oauth.google_client_secret.clone(),
        ) {
            (Some(client_id), Some(client_secret)) => Some(infra::ProviderCredentials {
                client_id,
                client_secret,
            }),
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
        if oauth.is_none() {
            tracing::warn!(
                "oauth disabled (no provider credentials); /app/auth/oauth routes will answer 503"
            );
        }
        // AppState takes Option fields; the graph (built only with a
        // database) fills them all at once.
        let services = graph.as_ref();
        let app = api::build_router(
            api::AppState {
                db,
                redis: redis_conn,
                auth: services.map(|s| s.auth.clone()),
                oauth,
                admin: services.map(|s| s.admin.clone()),
                admin_users: services.map(|s| s.admin_users.clone()),
                admin_projects: services.map(|s| s.admin_projects.clone()),
                admin_notifications: services.map(|s| s.admin_notifications.clone()),
                projects: services.map(|s| s.projects.clone()),
                project_exports: services.map(|s| s.project_exports.clone()),
                project_members: services.map(|s| s.project_members.clone()),
                audit: services.map(|s| s.audit.clone()),
                recipients: services.map(|s| s.recipients.clone()),
                api_keys: services.map(|s| s.api_keys.clone()),
                templates: services.map(|s| s.templates.clone()),
                channel_providers: services.map(|s| s.channel_providers.clone()),
                tickets: services.map(|s| s.tickets.clone()),
                billing: services.map(|s| s.billing.clone()),
                flows: services.map(|s| s.flows.clone()),
                sends: services.map(|s| s.sends.clone()),
                inbound: services.map(|s| s.inbound.clone()),
                schedules: services.map(|s| s.schedules.clone()),
                webhooks: services.map(|s| s.webhooks.clone()),
                campaigns: services.map(|s| s.campaigns.clone()),
                notifications: services.map(|s| s.notifications.clone()),
                provider_tester,
                config_store,
            },
            &config,
        );

        let listener =
            match tokio::net::TcpListener::bind((config.server.host.as_str(), config.server.port))
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

        // Sweep dispatch: when RabbitMQ is configured the API publishes one
        // tick per interval to the durable `notifi.sweep` queue (the worker
        // binary sweeps on its own clock and consumes those ticks for extra
        // sweeps); without RabbitMQ the embedded loop below does the work.
        // A failed tick publish degrades to a local sweep so a RabbitMQ
        // outage stalls nothing — the atomic claims keep a concurrent
        // worker from double-firing (a claimed-but-unsent broadcast stays
        // `sending` until an operator re-queues it — see ARCHITECTURE.md).
        let rabbit = infra::rabbit::connect(&config).await;
        if let Some(hub) = rabbit {
            let admin = services.map(|s| s.admin_notifications.clone());
            let sched = services.map(|s| s.schedules.clone());
            let camps = services.map(|s| s.campaigns.clone());
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
            let admin = services.map(|s| s.admin_notifications.clone());
            let sched = services.map(|s| s.schedules.clone());
            let camps = services.map(|s| s.campaigns.clone());
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
