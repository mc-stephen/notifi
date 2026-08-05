/**
 * Plain, framework-free domain types for the status platform.
 * Content collections (src/content.config.ts) validate against these shapes;
 * src/lib logic operates on them without any Astro dependency.
 */

export type ComponentState =
  | "operational"
  | "degraded"
  | "partial_outage"
  | "major_outage"
  | "maintenance";

export type IncidentSeverity = "minor" | "major" | "critical";

export type IncidentPhase =
  | "investigating"
  | "identified"
  | "monitoring"
  | "resolved";

export type MaintenanceState = "scheduled" | "in_progress" | "completed";

export type Subsystem = "platform" | "data" | "providers" | "web";

export interface Component {
  slug: string;
  name: string;
  subsystem: Subsystem;
  group: string;
  dependencies: string[];
  probe: {
    state: Exclude<ComponentState, "maintenance">;
    latency: { p50: number; p95: number };
    availability: { "7d": number; "30d": number; "90d": number };
  };
  lastUpdated: string;
}

export interface IncidentUpdate {
  timestamp: string;
  author: string;
  state: IncidentPhase;
  details: string;
  affectedComponents?: string[];
  customerImpact?: string;
}

export interface Incident {
  slug: string;
  title: string;
  severity: IncidentSeverity;
  status: IncidentPhase;
  startedAt: string;
  resolvedAt?: string;
  affectedComponents: string[];
  customerImpact?: string;
  investigation?: string;
  rootCause?: string;
  mitigation?: string;
  monitoring?: string;
  resolution?: string;
  postmortemSlug?: string;
  updates: IncidentUpdate[];
}

export interface MaintenanceUpdate {
  timestamp: string;
  author: string;
  state: MaintenanceState;
  details: string;
}

export interface Maintenance {
  slug: string;
  title: string;
  purpose: string;
  expectedImpact: string;
  affectedComponents: string[];
  startTime: string;
  endTime: string;
  status: MaintenanceState;
  updates: MaintenanceUpdate[];
  completionSummary?: string;
}

export interface Report {
  slug: string;
  month: string;
  summary: string;
  availability: Partial<Record<Subsystem, number>>;
  globalAvailability: number;
  incidents: number;
  criticalIncidents: number;
  meanTimeToResolve: string;
  highlights: string[];
}

/** Ordered severity rank for roll-up comparisons. */
export const SEVERITY_RANK: Record<IncidentSeverity, number> = {
  minor: 1,
  major: 2,
  critical: 3,
};

export const STATE_RANK: Record<ComponentState, number> = {
  operational: 0,
  maintenance: 0,
  degraded: 1,
  partial_outage: 2,
  major_outage: 3,
};

/** Severity → component state mapping when an incident is open. */
export const SEVERITY_TO_STATE: Record<IncidentSeverity, ComponentState> = {
  minor: "degraded",
  major: "partial_outage",
  critical: "major_outage",
};

/** Human labels — always shown next to the glyph; never color alone. */
export const STATE_LABELS: Record<ComponentState, string> = {
  operational: "Operational",
  degraded: "Degraded Performance",
  partial_outage: "Partial Outage",
  major_outage: "Major Outage",
  maintenance: "Under Maintenance",
};

export const PHASE_LABELS: Record<IncidentPhase, string> = {
  investigating: "Investigating",
  identified: "Identified",
  monitoring: "Monitoring",
  resolved: "Resolved",
};

export const MAINTENANCE_LABELS: Record<MaintenanceState, string> = {
  scheduled: "Upcoming",
  in_progress: "In Progress",
  completed: "Completed",
};

export const SUBSYSTEM_LABELS: Record<Subsystem, string> = {
  platform: "Platform",
  data: "Data",
  providers: "Providers",
  web: "Web",
};

/** Subscription scopes offered by the subscribe form (design §status subscriptions). */
export const SUBSCRIPTION_SCOPES = [
  { value: "all", label: "All incidents & maintenance" },
  { value: "critical", label: "Critical incidents only" },
  { value: "maintenance", label: "Maintenance only" },
  { value: "components", label: "Specific components (choose below)" },
] as const;
