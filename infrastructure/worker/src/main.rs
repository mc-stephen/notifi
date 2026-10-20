//! Notifi worker — executes due flow schedules and due admin broadcasts.
//!
//! Transport: sweep ticks from RabbitMQ (`notifi.sweep`). When RabbitMQ is
//! unset or unreachable the worker sweeps on a local 30s interval instead,
//! so it is useful with plain `cargo run -p worker` and no extra services.
//!
//! Safe to run alongside the API: claims are atomic (`claim_due`), so a
//! second sweeper simply finds nothing due. A failed sweep is acked, not
//! requeued — the next tick retries within one interval, while a nack would
//! hot-loop on a down database.

use std::sync::Arc;
use std::time::Duration;

use lapin::options::BasicAckOptions;

use server::domain::admin::AdminNotificationsService;
use server::domain::campaigns::CampaignService;
use server::domain::flows::FlowService;
use server::domain::schedules::ScheduleService;
use server::domain::sends::SendService;
use server::infra::rabbit::{self, RabbitHub};

/// What a consume round ended with.
enum Round {
    Shutdown,
    Disconnected,
}

fn main() {
    let rt = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .expect("failed to build tokio runtime");
    rt.block_on(async {
        let code = run().await;
        if code != 0 {
            std::process::exit(code);
        }
    });
}

/// Runs one sweep of due broadcasts, due schedules, and campaigns.
async fn sweep_once(
    admin_notifications: &Arc<AdminNotificationsService>,
    schedules: &Arc<ScheduleService>,
    campaigns: &Arc<CampaignService>,
) {
    match admin_notifications.run_due_worker(chrono::Utc::now()).await {
        Ok(0) => {}
        Ok(n) => tracing::info!(sent = n, "scheduled notification worker sent broadcasts"),
        Err(e) => tracing::error!(error = %e, "scheduled notification worker failed"),
    }
    match schedules.run_due(chrono::Utc::now()).await {
        Ok(0) => {}
        Ok(n) => tracing::info!(ran = n, "schedule worker fired due schedules"),
        Err(e) => tracing::error!(error = %e, "schedule worker failed"),
    }
    match campaigns.tick(chrono::Utc::now()).await {
        Ok(0) => {}
        Ok(n) => tracing::info!(ran = n, "campaign worker processed campaigns"),
        Err(e) => tracing::error!(error = %e, "campaign worker failed"),
    }
}

/// Consumes ticks until shutdown or a transport error.
async fn consume_round(
    hub: &RabbitHub,
    admin_notifications: &Arc<AdminNotificationsService>,
    schedules: &Arc<ScheduleService>,
    campaigns: &Arc<CampaignService>,
) -> Round {
    let channel = match hub.declare().await {
        Ok(channel) => channel,
        Err(e) => {
            tracing::warn!(error = %e, "worker: rabbitmq declare failed");
            return Round::Disconnected;
        }
    };
    loop {
        tokio::select! {
            _ = shutdown_signal() => return Round::Shutdown,
            _ = tokio::time::sleep(Duration::from_secs(1)) => {}
        }
        match RabbitHub::poll_tick(&channel).await {
            Err(e) => {
                tracing::warn!(error = %e, "worker: rabbitmq poll failed");
                return Round::Disconnected;
            }
            Ok(None) => {}
            Ok(Some(tick)) => {
                sweep_once(admin_notifications, schedules, campaigns).await;
                if let Err(e) = tick.delivery.ack(BasicAckOptions::default()).await {
                    tracing::warn!(error = %e, "worker: tick ack failed");
                    return Round::Disconnected;
                }
            }
        }
    }
}

async fn run() -> i32 {
    dotenvy::dotenv().ok();

    let _guard = match server::infra::telemetry::init() {
        Ok(guard) => guard,
        Err(e) => {
            eprintln!("failed to initialize telemetry: {e}");
            return 1;
        }
    };

    let config = match server::infra::config::AppConfig::from_env() {
        Ok(config) => config,
        Err(e) => {
            eprintln!("invalid configuration: {e}");
            return 1;
        }
    };

    let pool = match server::infra::db::connect(&config).await {
        Some(pool) => pool,
        None => {
            eprintln!("worker: database unavailable (NOTIFI_DATABASE_URL); nothing to sweep");
            return 1;
        }
    };

    // Same wiring as the API composition root (`server::run`) — the worker
    // shares the brain, only the trigger differs (ticks vs embedded loops).
    let audit = Arc::new(server::domain::audit::AuditService::new(Arc::new(
        server::infra::audit_repository_pg::PgAuditStore::new(pool.clone()),
    )));
    let auth_store = Arc::new(server::infra::PgAuthStore::new(pool.clone()));
    let admin_notifications = Arc::new(AdminNotificationsService::new(
        auth_store.clone(),
        auth_store,
        Arc::new(server::infra::PgNotificationsStore::new(pool.clone())),
        audit.clone(),
    ));
    let channel_providers: Arc<dyn server::ports::ChannelProviderStore> =
        Arc::new(server::infra::PgChannelProviderStore::new(pool.clone()));
    let flow_store: Arc<dyn server::ports::flow_store::FlowStore> =
        Arc::new(server::infra::PgFlowStore::new(pool.clone()));
    let flows = Arc::new(FlowService::new(flow_store.clone()));
    let send_store = Arc::new(server::infra::PgSendStore::new(pool.clone()));
    let sends = Arc::new(SendService::new(
        send_store.clone(),
        flows.clone(),
        channel_providers,
        Arc::new(server::infra::delivery_adapters::DefaultAdapterFactory),
        Arc::new(server::infra::ReqwestHttpRequester::new()),
    ));
    let schedules = Arc::new(ScheduleService::new(
        Arc::new(server::infra::PgScheduleStore::new(pool.clone())),
        flow_store,
        send_store.clone(),
        Arc::new(server::infra::PgTemplatesStore::new(pool.clone())),
        Arc::new(server::infra::PgChannelProviderStore::new(pool.clone())),
        Arc::new(server::infra::delivery_adapters::DefaultAdapterFactory),
        Arc::new(server::infra::ReqwestHttpRequester::new()),
    ));
    let campaigns = Arc::new(CampaignService::new(
        Arc::new(server::infra::PgCampaignStore::new(pool.clone())),
        send_store.clone(),
        sends.clone(),
        Arc::new(server::infra::PgRecipientsStore::new(pool.clone())),
        Arc::new(server::infra::PgTemplatesStore::new(pool.clone())),
    ));

    tracing::info!("worker started");

    // Immediate sweep at boot so work never waits a full interval.
    sweep_once(&admin_notifications, &schedules, &campaigns).await;

    loop {
        // Reconnect each round: instant `None` when unconfigured, 10s cap
        // when RabbitMQ is down — then the local interval covers for it.
        let hub = tokio::time::timeout(Duration::from_secs(10), rabbit::connect(&config))
            .await
            .ok()
            .flatten();
        match hub {
            Some(hub) => {
                tracing::info!("worker: consuming sweep ticks");
                match consume_round(&hub, &admin_notifications, &schedules, &campaigns).await {
                    Round::Shutdown => break,
                    Round::Disconnected => {
                        tokio::time::sleep(Duration::from_secs(5)).await;
                    }
                }
            }
            None => {
                tracing::warn!("worker: no rabbitmq; sweeping on a local interval");
                sweep_once(&admin_notifications, &schedules, &campaigns).await;
                tokio::select! {
                    _ = shutdown_signal() => break,
                    _ = tokio::time::sleep(Duration::from_secs(rabbit::SWEEP_INTERVAL_SECS)) => {}
                }
            }
        }
    }

    tracing::info!("worker shutdown complete");
    0
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
