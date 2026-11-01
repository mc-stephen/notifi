//! Job: due admin broadcasts (scheduled notifications).
//!
//! Every sweep tick claims what is due through atomic claims — the
//! database stays the source of truth — sends each broadcast to its
//! audience and records stats, so overlapping sweepers (local clock,
//! rabbit ticks, extra worker processes, the API's degraded sweeps)
//! never double-send.

use chrono::Utc;

use server::compose::ServiceGraph;

pub async fn run(graph: &ServiceGraph) {
    match graph.admin_notifications.run_due_worker(Utc::now()).await {
        Ok(0) => {}
        Ok(n) => tracing::info!(sent = n, "scheduled notification worker sent broadcasts"),
        Err(e) => tracing::error!(error = %e, "scheduled notification worker failed"),
    }
}
