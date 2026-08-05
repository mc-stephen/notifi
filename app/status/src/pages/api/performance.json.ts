import { getMetrics } from "../../lib/data";
import { jsonResponse } from "../../lib/api";

export async function GET() {
  const metrics = await getMetrics();
  return jsonResponse({
    generated_at: metrics.generatedAt,
    metrics: {
      api_response_time_p95_ms: metrics.apiResponseTimeP95Ms,
      webhook_latency_p95_ms: metrics.webhookLatencyP95Ms,
      notification_processing_p95_ms: metrics.notificationProcessingP95Ms,
      queue_depth: metrics.queueDepth,
      worker_throughput_per_sec: metrics.workerThroughputPerSec,
      database_health: metrics.databaseHealth,
      cache_hit_rate_pct: metrics.cacheHitRatePct,
      delivery_success_rate_30d: metrics.deliverySuccessRate30d,
      regional_availability_pct: metrics.regionalAvailabilityPct,
    },
  });
}
