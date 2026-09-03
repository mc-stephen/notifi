//! Notifi server — HTTP API, domain logic, and infrastructure in one crate.
//!
//! Layout (`docs/ARCHITECTURE.md` §2):
//! * [`api`] — axum presentation: two surfaces (project @ root, user @ `/v1`),
//!   middleware, error mapping.
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
        // (they emit actions) and the /v1/logs query surface.
        let audit = db.as_ref().map(|pool| {
            std::sync::Arc::new(domain::audit::AuditService::new(
                std::sync::Arc::new(infra::audit_repository_pg::PgAuditStore::new(pool.clone())),
            ))
        });

        // Auth is wired only when a database exists; its routes answer 503
        // otherwise (composition root wires the sqlx store into the service).
        let auth = db.as_ref().map(|pool| {
            std::sync::Arc::new(domain::auth::AuthService::new(
                std::sync::Arc::new(infra::PgAuthStore::new(pool.clone())),
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
            tracing::warn!("auth disabled (needs database); /v1/auth routes will answer 503");
        }
        if projects.is_none() {
            tracing::warn!("projects disabled (needs database); /v1/projects routes will answer 503");
        }
        if audit.is_none() {
            tracing::warn!("audit log disabled (needs database); /v1/logs routes will answer 503");
        }
        if recipients.is_none() {
            tracing::warn!(
                "recipients disabled (needs database); /v1/projects/{{project_id}}/recipients routes will answer 503"
            );
        }
        if templates.is_none() {
            tracing::warn!(
                "templates disabled (needs database); /v1/projects/{{project_id}}/templates routes will answer 503"
            );
        }
        if channel_providers.is_none() {
            tracing::warn!(
                "channel_providers disabled (needs database); /v1/projects/{{project_id}}/channel-configs routes will answer 503"
            );
        }
        if oauth.is_none() {
            tracing::warn!(
                "oauth disabled (no provider credentials); /v1/auth/oauth routes will answer 503"
            );
        }

        let app = api::build_router(
            api::AppState {
                db,
                redis: redis_conn,
                auth,
                oauth,
                projects,
                audit,
                recipients,
                templates,
                channel_providers,
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
