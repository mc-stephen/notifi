//! Job: campaign drains — ticking live audiences through their send
//! steps, exactly one `tick` per sweep whose claiming is atomic.

use chrono::Utc;

use server::compose::ServiceGraph;

pub async fn run(graph: &ServiceGraph) {
    match graph.campaigns.tick(Utc::now()).await {
        Ok(0) => {}
        Ok(n) => tracing::info!(ran = n, "campaign worker processed campaigns"),
        Err(e) => tracing::error!(error = %e, "campaign worker failed"),
    }
}
