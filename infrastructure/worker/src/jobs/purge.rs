//! Job: project-deletion purge — expired tombstones lose their project
//! folder first (retry-able), then the row (cascades wipe the children).

use chrono::Utc;

use server::compose::ServiceGraph;

pub async fn run(graph: &ServiceGraph) {
    match graph.projects.purge_due(Utc::now()).await {
        Ok(0) => {}
        Ok(n) => tracing::info!(purged = n, "deleted projects fully erased"),
        Err(e) => tracing::error!(error = %e, "project purge worker failed"),
    }
}
