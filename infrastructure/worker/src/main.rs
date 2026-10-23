//! Notifi worker — executes due flow schedules, due admin broadcasts, and
//! due campaign drains.
//!
//! Triggering: the worker owns its own clock — a local sweep every 30s —
//! plus, when RabbitMQ is configured, ticks from the durable `notifi.sweep`
//! queue trigger extra sweeps between intervals. Ticks are disposable
//! triggers; the database stays the source of truth through atomic claims
//! (`claim_due`), so local sweeps, tick sweeps, the API's degraded sweeps,
//! and extra worker processes can all overlap without ever double-running
//! a job.
//!
//! Also living here: the self-contained [`backup`] block — a daily
//! database dump-and-upload on its own clock (never the queue). Off
//! unless `NOTIFI_BACKUP_ENABLED` is switched on; `cargo run -p worker --
//! --backup-now` runs one dump immediately.
//!
//! Useful with plain `cargo run -p worker` and no extra services: RabbitMQ
//! being unset, down, or never published-to changes nothing — the local
//! interval keeps sweeping.

use std::sync::Arc;
use std::time::Duration;

use lapin::options::BasicAckOptions;

mod backup;

use server::compose::ServiceGraph;
use server::infra::config::AppConfig;
use server::infra::rabbit::{self, RabbitHub};

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

    // One-shot backup: dump + upload + exit, no sweeps, no queue.
    if std::env::args().any(|a| a == "--backup-now") {
        return backup::one_shot(&config).await;
    }

    let pool = match server::infra::db::connect(&config).await {
        Some(pool) => pool,
        None => {
            eprintln!("worker: database unavailable (NOTIFI_DATABASE_URL); nothing to sweep");
            return 1;
        }
    };

    // One composition root with the API (`server::compose::compose`) — the
    // worker shares the brain, only the trigger differs. Mail/system
    // templates stay `None`: no sweep service consumes them. Project
    // storage rides in via the same factory (the project purge wipes
    // folders through it); cacheless is fine — this process never reads
    // tenant configs for delivery, so there is nothing to keep warm.
    // `build_ensured` also lays the bucket's top-level folders down
    // (`projects/`, `backup/`) so the layout exists even before the
    // server ever starts.
    let config_store: std::sync::Arc<dyn server::ports::ConfigStore + Send + Sync> =
        server::infra::config_store_factory::build_ensured(&config)
            .await
            .expect("config store builds");
    let graph = Arc::new(server::compose::compose(
        pool,
        &config,
        None,
        None,
        config_store,
    ));

    tracing::info!("worker started");

    // Immediate sweep at boot so work never waits a full interval.
    sweep_once(&graph).await;

    // Database backups: the self-contained block on its own clock. Off
    // unless NOTIFI_BACKUP_ENABLED is on — `from_env` itself logs why
    // when the pieces it got were incomplete.
    if let Some(settings) = backup::BackupSettings::from_env(&config) {
        tokio::spawn(backup::schedule(settings));
    } else if !std::env::args().any(|a| a == "--backup-now") {
        tracing::info!("worker: database backups off (NOTIFI_BACKUP_ENABLED not on)");
    }

    // RabbitMQ (optional): a tick consumer runs alongside the local clock.
    // The first connect decides whether the task spawns at all — a worker
    // without a broker logs nothing about rabbit afterwards.
    if let Some(hub) = tokio::time::timeout(Duration::from_secs(10), rabbit::connect(&config))
        .await
        .ok()
        .flatten()
    {
        tracing::info!("worker: consuming sweep ticks from rabbitmq alongside the local interval");
        tokio::spawn(tick_consumer(Some(hub), graph.clone(), config.clone()));
    }

    // The worker's own clock: never depends on a configured queue.
    loop {
        tokio::select! {
            _ = shutdown_signal() => break,
            _ = tokio::time::sleep(Duration::from_secs(rabbit::SWEEP_INTERVAL_SECS)) => {
                sweep_once(&graph).await;
            }
        }
    }

    tracing::info!("worker shutdown complete");
    0
}

/// Runs one sweep of due broadcasts, due schedules, and campaign drains.
async fn sweep_once(graph: &ServiceGraph) {
    match graph
        .admin_notifications
        .run_due_worker(chrono::Utc::now())
        .await
    {
        Ok(0) => {}
        Ok(n) => tracing::info!(sent = n, "scheduled notification worker sent broadcasts"),
        Err(e) => tracing::error!(error = %e, "scheduled notification worker failed"),
    }
    match graph.schedules.run_due(chrono::Utc::now()).await {
        Ok(0) => {}
        Ok(n) => tracing::info!(ran = n, "schedule worker fired due schedules"),
        Err(e) => tracing::error!(error = %e, "schedule worker failed"),
    }
    match graph.campaigns.tick(chrono::Utc::now()).await {
        Ok(0) => {}
        Ok(n) => tracing::info!(ran = n, "campaign worker processed campaigns"),
        Err(e) => tracing::error!(error = %e, "campaign worker failed"),
    }
    // Project deletion purge: expired tombstones lose their project folder
    // first (retry-able), then the row (cascades wipe the children).
    match graph.projects.purge_due(chrono::Utc::now()).await {
        Ok(0) => {}
        Ok(n) => tracing::info!(purged = n, "deleted projects fully erased"),
        Err(e) => tracing::error!(error = %e, "project purge worker failed"),
    }
}

/// Consumes sweep ticks forever, reconnecting after transport failures with
/// a 5s backoff and a 10s connect cap. Each tick triggers an extra sweep;
/// an unacked tick from a crashed worker is redelivered, and the atomic
/// claim makes that a harmless repeat.
///
/// Spawned only when the first connect succeeds, so unset RabbitMQ logs
/// nothing here.
async fn tick_consumer(mut hub: Option<RabbitHub>, graph: Arc<ServiceGraph>, config: AppConfig) {
    loop {
        if hub.is_none() {
            tokio::time::sleep(Duration::from_secs(5)).await;
            hub = tokio::time::timeout(Duration::from_secs(10), rabbit::connect(&config))
                .await
                .ok()
                .flatten();
            if hub.is_none() {
                // Still unset or unreachable — the local clock covers for
                // it; try again after the next backoff.
                continue;
            }
        }
        let channel = match hub.as_ref().expect("hub present").declare().await {
            Ok(channel) => channel,
            Err(e) => {
                tracing::warn!(error = %e, "worker: rabbitmq declare failed");
                hub = None;
                continue;
            }
        };
        match RabbitHub::poll_tick(&channel).await {
            Err(e) => {
                tracing::warn!(error = %e, "worker: rabbitmq poll failed");
                hub = None;
            }
            Ok(None) => {
                tokio::time::sleep(Duration::from_secs(1)).await;
            }
            Ok(Some(tick)) => {
                sweep_once(&graph).await;
                if let Err(e) = tick.delivery.ack(BasicAckOptions::default()).await {
                    tracing::warn!(error = %e, "worker: tick ack failed");
                    hub = None;
                }
            }
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
