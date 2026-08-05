import { defineCollection, z } from "astro:content";
import { glob } from "astro/loaders";

const componentSchema = z.object({
  slug: z.string(),
  name: z.string(),
  subsystem: z.enum(["platform", "data", "providers", "web"]),
  group: z.string(),
  dependencies: z.array(z.string()).default([]),
  probe: z.object({
    state: z.enum(["operational", "degraded", "partial_outage", "major_outage"]),
    latency: z.object({ p50: z.number(), p95: z.number() }),
    availability: z.object({
      "7d": z.number(),
      "30d": z.number(),
      "90d": z.number(),
    }),
  }),
  lastUpdated: z.string(),
});

const incidentUpdateSchema = z.object({
  timestamp: z.string(),
  author: z.string(),
  state: z.enum(["investigating", "identified", "monitoring", "resolved"]),
  details: z.string(),
  affectedComponents: z.array(z.string()).optional(),
  customerImpact: z.string().optional(),
});

const incidentSchema = z.object({
  slug: z.string(),
  title: z.string(),
  severity: z.enum(["minor", "major", "critical"]),
  status: z.enum(["investigating", "identified", "monitoring", "resolved"]),
  startedAt: z.string(),
  resolvedAt: z.string().optional(),
  affectedComponents: z.array(z.string()).default([]),
  customerImpact: z.string().optional(),
  investigation: z.string().optional(),
  rootCause: z.string().optional(),
  mitigation: z.string().optional(),
  monitoring: z.string().optional(),
  resolution: z.string().optional(),
  postmortemSlug: z.string().optional(),
  updates: z.array(incidentUpdateSchema).default([]),
});

const maintenanceUpdateSchema = z.object({
  timestamp: z.string(),
  author: z.string(),
  state: z.enum(["scheduled", "in_progress", "completed"]),
  details: z.string(),
});

const maintenanceSchema = z.object({
  slug: z.string(),
  title: z.string(),
  purpose: z.string(),
  expectedImpact: z.string(),
  affectedComponents: z.array(z.string()).default([]),
  startTime: z.string(),
  endTime: z.string(),
  status: z.enum(["scheduled", "in_progress", "completed"]),
  updates: z.array(maintenanceUpdateSchema).default([]),
  completionSummary: z.string().optional(),
});

const reportSchema = z.object({
  slug: z.string(),
  month: z.string(),
  summary: z.string(),
  availability: z.object({
    platform: z.number(),
    data: z.number(),
    providers: z.number(),
    web: z.number(),
  }),
  globalAvailability: z.number(),
  incidents: z.number(),
  criticalIncidents: z.number(),
  meanTimeToResolve: z.string(),
  highlights: z.array(z.string()).default([]),
});

const metricsSchema = z.object({
  generatedAt: z.string(),
  metrics: z.object({
    apiResponseTimeP95Ms: z.number(),
    webhookLatencyP95Ms: z.number(),
    notificationProcessingP95Ms: z.number(),
    queueDepth: z.number(),
    workerThroughputPerSec: z.number(),
    databaseHealth: z.enum(["healthy", "degraded", "down"]),
    cacheHitRatePct: z.number(),
    deliverySuccessRate30d: z.number(),
    regionalAvailabilityPct: z.number(),
  }),
});

const components = defineCollection({
  loader: glob({ pattern: "**/*.json", base: "./src/content/components" }),
  schema: componentSchema,
});

const incidents = defineCollection({
  loader: glob({ pattern: "**/*.json", base: "./src/content/incidents" }),
  schema: incidentSchema,
});

const maintenance = defineCollection({
  loader: glob({ pattern: "**/*.json", base: "./src/content/maintenance" }),
  schema: maintenanceSchema,
});

const reports = defineCollection({
  loader: glob({ pattern: "**/*.json", base: "./src/content/reports" }),
  schema: reportSchema,
});

const metrics = defineCollection({
  loader: glob({ pattern: "**/*.json", base: "./src/content/metrics" }),
  schema: metricsSchema,
});

export { componentSchema, incidentSchema, maintenanceSchema, reportSchema };
export const collections = { components, incidents, maintenance, reports, metrics };
