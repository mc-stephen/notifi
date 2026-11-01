//! Job: due flow schedules — the calendar the schedules engine fires on.
//!
//! `ScheduleService::run_due` claims due entries atomically, so the
//! same overlap guards as every other job apply: the database is the
//! source of truth and the sweep is just a trigger.

use chrono::Utc;

use server::compose::ServiceGraph;

pub async fn run(graph: &ServiceGraph) {
    match graph.schedules.run_due(Utc::now()).await {
        Ok(0) => {}
        Ok(n) => tracing::info!(ran = n, "schedule worker fired due schedules"),
        Err(e) => tracing::error!(error = %e, "schedule worker failed"),
    }
}
