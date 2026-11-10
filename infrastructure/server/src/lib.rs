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

/// Process entry point: config → the four boot gates (Postgres, Redis,
/// SMTP-when-enabled, the object store) → router → serve. An incomplete
/// system aborts with the reason; nothing limps on degraded.
pub fn run() {
    if let Err(e) = run_inner() {
        eprintln!("{e}");
        std::process::exit(1);
    }
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

    let boot: Result<(), String> = rt.block_on(async move {
        // Gate 1 — Postgres: connection AND schema. Not a single honest
        // action is possible without it.
        let db = Some(
            infra::db::connect(&config).await.ok_or_else(|| {
                "postgres boot failed: NOTIFI_DATABASE_URL unset, unreachable, or migrations failed (the specific error is logged above)".to_string()
            })?,
        );

        // Gate 2 — Redis: the URL must parse and the machine must answer.
        let redis_client = infra::redis::connect(&config)?;
        let redis_conn = match redis_client.clone().get_connection_manager().await {
            Ok(conn) => conn,
            Err(e) => {
                return Err(format!(
                    "redis boot failed: {e} — the machine must be reachable (NOTIFI_REDIS_URL)"
                ));
            }
        };

        let mailer: Option<std::sync::Arc<dyn ports::SmtpMailer>> =
            config.mail.enabled().then(|| {
                let live = infra::LiveSmtpMailer::from_config(&config.mail)
                    .expect("mailer enabled without SMTP host");
                std::sync::Arc::new(live) as std::sync::Arc<dyn ports::SmtpMailer>
            });
        if mailer.is_some() {
            tracing::info!("system mail configured (verifying at boot)");
        }

        // Gate 3 — SMTP, only when mail is switched on (unset mail is a
        // legitimate choice; dev-token mode carries the flows): the
        // relay must answer the handshake, or boot aborts.
        if let Some(mailer) = mailer.as_ref() {
            match tokio::time::timeout(std::time::Duration::from_secs(5), mailer.verify()).await {
                Ok(Ok(true)) => {
                    tracing::info!("system mail verified (smtp handshake + auth ok)")
                }
                Ok(Ok(false)) => {
                    return Err("smtp boot failed: relay refused the NOOP probe".to_string());
                }
                Ok(Err(e)) => return Err(format!("smtp boot failed: {e}")),
                Err(_) => {
                    return Err("smtp boot failed: the verify probe timed out after 5s".to_string())
                }
            }
        }

        // Tenant project config storage: the factory demands the full
        // `NOTIFI_R2_*` quartet and refuses otherwise — R2-or-nothing,
        // no fallback driver. Boot lays the top-level folders down
        // (`projects/`, `backup/`, `users/`) so the layout exists before
        // the first project arrives. Read-through config cache: warm
        // keys answer at Redis speed and the store behind sees nothing,
        // so R2 request counts (and per-delivery latency) stay near
        // zero. Writes invalidate through the wrapper, never fail the
        // send path. Built before `compose` so the project service
        // (creation seeding, deletion purge) shares this exact instance
        // — cache invalidation only works if every writer goes through
        // the same wrapper.
        let config_store: std::sync::Arc<dyn ports::ConfigStore + Send + Sync> =
            infra::config_store_factory::build_ensured(&config)
                .await
                .map_err(|e| format!("object store boot failed: {e}"))?;

        // System mail (transactional emails): bodies live in the store's
        // `templates/` folder — put there by ops, the server never
        // writes them. Boot reads the folder into forever-keys in Redis
        // (`cfg:templates/…`, no expiry); renders answer from the cache;
        // POST /admin/templates/warm re-reads after an edit in the
        // bucket. (A warm miss still falls back to direct store reads —
        // Redis being required, a miss is an empty folder, not a cache
        // outage.)
        let template_cache: std::sync::Arc<dyn infra::config_cache::CacheBackend + Send + Sync> =
            std::sync::Arc::new(
                infra::RedisCacheBackend::new(redis_conn.clone()).without_ttl(),
            );
        tracing::info!("system template cache: redis (forever keys)");
        let system_templates: Option<std::sync::Arc<dyn ports::SystemTemplates>> = {
            let reader =
                infra::StoreSystemTemplates::new(config_store.clone(), Some(template_cache));
            match reader.warm_from_store().await {
                Ok(count) => tracing::info!(warmed = count, "system templates warm"),
                Err(e) => tracing::warn!(error = %e, "system templates unavailable; renders will read the store directly"),
            }
            Some(std::sync::Arc::new(reader))
        };

        let config_store: std::sync::Arc<dyn ports::ConfigStore + Send + Sync> =
            std::sync::Arc::new(infra::CachedConfigStore::new(
                config_store,
                std::sync::Arc::new(infra::RedisCacheBackend::new(redis_conn)),
            ));
        tracing::info!(
            ttl_secs = infra::config_cache::CONFIG_CACHE_TTL.as_secs(),
            "config cache: redis"
        );

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

        // Gate 4 — OAuth: sign-in cannot be half-configured or absent.
        // A missing runtime would promise login buttons that answer 503;
        // a half-paired provider is a broken config. Both abort here.
        fn oauth_pair(
            client_id: Option<String>,
            client_secret: Option<String>,
            id_env: &str,
            secret_env: &str,
            provider: &str,
        ) -> Result<Option<infra::ProviderCredentials>, String> {
            match (client_id, client_secret) {
                (Some(client_id), Some(client_secret)) => Ok(Some(infra::ProviderCredentials {
                    client_id,
                    client_secret,
                })),
                (None, None) => Ok(None),
                (Some(_), None) => Err(format!(
                    "oauth boot failed: {provider} credentials half-paired — {id_env} set without {secret_env}"
                )),
                (None, Some(_)) => Err(format!(
                    "oauth boot failed: {provider} credentials half-paired — {secret_env} set without {id_env}"
                )),
            }
        }
        let github = oauth_pair(
            config.oauth.github_client_id.clone(),
            config.oauth.github_client_secret.clone(),
            "NOTIFI_OAUTH_GITHUB_CLIENT_ID",
            "NOTIFI_OAUTH_GITHUB_CLIENT_SECRET",
            "github",
        )?;
        let google = oauth_pair(
            config.oauth.google_client_id.clone(),
            config.oauth.google_client_secret.clone(),
            "NOTIFI_OAUTH_GOOGLE_CLIENT_ID",
            "NOTIFI_OAUTH_GOOGLE_CLIENT_SECRET",
            "google",
        )?;
        if github.is_none() && google.is_none() {
            return Err("oauth boot failed: no provider configured — OAuth sign-in is a boot requirement; set NOTIFI_OAUTH_GITHUB_CLIENT_ID/_CLIENT_SECRET and/or NOTIFI_OAUTH_GOOGLE_CLIENT_ID/_CLIENT_SECRET".to_string());
        }
        let oauth = Some(std::sync::Arc::new(ports::OAuthRuntime {
            provider: infra::http_oauth_provider(
                github,
                google,
                config.oauth.api_base_url.clone(),
            ),
            dashboard_url: config.oauth.dashboard_url.clone(),
        }));

        // AppState takes Option fields; the graph (built only with a
        // database) fills them all at once.
        let gateway = db.as_ref().map(|pool| {
            std::sync::Arc::new(infra::gateway_store::PgGatewayStore::new(pool.clone()))
                as std::sync::Arc<dyn ports::gateway::GatewayStore + Send + Sync>
        });
        let services = graph.as_ref();
        let app = api::build_router(
            api::AppState {
                db,
                redis: Some(redis_client),
                auth: services.map(|s| s.auth.clone()),
                oauth,
                admin: services.map(|s| s.admin.clone()),
                admin_users: services.map(|s| s.admin_users.clone()),
                admin_projects: services.map(|s| s.admin_projects.clone()),
                admin_notifications: services.map(|s| s.admin_notifications.clone()),
                projects: services.map(|s| s.projects.clone()),
                project_exports: services.map(|s| s.project_exports.clone()),
                project_imports: services.map(|s| s.project_imports.clone()),
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
                campaigns: services.map(|s| s.campaigns.clone()),
                notifications: services.map(|s| s.notifications.clone()),
                gateway,
                provider_tester,
                config_store,
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
                return Err(format!(
                    "bind failed on {}:{}: {e} — the port is taken or the address is unavailable",
                    config.server.host, config.server.port
                ));
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
            tokio::spawn(async move {
                let mut interval = tokio::time::interval(
                    std::time::Duration::from_secs(infra::rabbit::SWEEP_INTERVAL_SECS),
                );
                loop {
                    interval.tick().await;
                    run_sweep_once(&admin, &sched, &camps).await;
                }
            });
        }

        // ConnectInfo feeds the inbound webhook IP allow-list (peer
        // address extraction on `POST /v1/hooks/{token}`).
        if let Err(e) = axum::serve(
            listener,
            app.into_make_service_with_connect_info::<std::net::SocketAddr>(),
        )
        .with_graceful_shutdown(shutdown_signal())
        .await
        {
            tracing::error!("server error: {e}");
        }

        tracing::info!("api shutdown complete");
        Ok(())
    });

    if let Err(e) = boot {
        tracing::error!("boot refused: {e}");
        eprintln!("boot refused: {e}");
        std::process::exit(1);
    }

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
