/**
 * API serializers — shapes mirror the public status API exactly
 * (design §10). Fields are snake_case; ids are slugs.
 */
import type { Component, Incident, Maintenance } from "./types";
import type { ComponentHealth, GlobalHealth, SubsystemHealth } from "./status";

export const API_CACHE_CONTROL = "public, max-age=60, stale-while-revalidate=300";

export function jsonResponse(data: unknown): Response {
  return new Response(JSON.stringify(data, null, 2), {
    headers: { "Content-Type": "application/json; charset=utf-8", "Cache-Control": API_CACHE_CONTROL },
  });
}

export function serializeComponentState(c: ComponentHealth) {
  return {
    id: c.slug,
    name: c.name,
    subsystem: c.subsystem,
    group: c.group,
    state: c.state,
    latency: { p50_ms: c.probe.latency.p50, p95_ms: c.probe.latency.p95 },
    availability: { "7d": c.probe.availability["7d"], "30d": c.probe.availability["30d"], "90d": c.probe.availability["90d"] },
    dependencies: c.dependencies,
    maintenance_scheduled: c.maintenanceScheduled ?? false,
    last_updated: c.lastUpdated,
    open_incident: c.incident ? { id: c.incident.slug, title: c.incident.title, severity: c.incident.severity, status: c.incident.status } : null,
  };
}

export function serializeSubsystem(s: SubsystemHealth) {
  return {
    subsystem: s.subsystem,
    state: s.state,
    component_count: s.components.length,
  };
}

export function serializeStatus(global: GlobalHealth, uptime90d: number) {
  return {
    generated_at: new Date().toISOString(),
    state: global.state,
    uptime_90d: uptime90d,
    subsystems: global.subsystems.map(serializeSubsystem),
    summary: {
      open_incidents: global.components.filter((c) => c.incident).length,
      maintenance_scheduled: global.components.filter((c) => c.maintenanceScheduled).length,
      maintenance_in_progress: global.components.filter((c) => c.maintenance).length,
    },
    url: "https://status.notifi.dev/",
  };
}

export function serializeIncidentSummary(i: Incident, site: string) {
  return {
    id: i.slug,
    slug: i.slug,
    title: i.title,
    severity: i.severity,
    status: i.status,
    started_at: i.startedAt,
    resolved_at: i.resolvedAt ?? null,
    affected_components: i.affectedComponents,
    url: `${site}/incidents/${i.slug}/`,
  };
}

export function serializeIncidentDetail(i: Incident, site: string) {
  return {
    ...serializeIncidentSummary(i, site),
    customer_impact: i.customerImpact ?? null,
    investigation: i.investigation ?? null,
    root_cause: i.rootCause ?? null,
    mitigation: i.mitigation ?? null,
    monitoring: i.monitoring ?? null,
    resolution: i.resolution ?? null,
    postmortem_url: i.postmortemSlug ? `${site}/reports/${i.postmortemSlug}/` : null,
    updates: i.updates.map((u) => ({
      timestamp: u.timestamp,
      author: u.author,
      state: u.state,
      details: u.details,
      affected_components: u.affectedComponents ?? [],
      customer_impact: u.customerImpact ?? null,
    })),
  };
}

export function serializeMaintenance(m: Maintenance, site: string) {
  return {
    id: m.slug,
    slug: m.slug,
    title: m.title,
    purpose: m.purpose,
    expected_impact: m.expectedImpact,
    affected_components: m.affectedComponents,
    start_time: m.startTime,
    end_time: m.endTime,
    status: m.status,
    timezone: "UTC",
    completion_summary: m.completionSummary ?? null,
    url: `${site}/maintenance/${m.slug}/`,
    updates: m.updates.map((u) => ({
      timestamp: u.timestamp,
      author: u.author,
      state: u.state,
      details: u.details,
    })),
  };
}

export function serializeComponent(c: Component) {
  return {
    id: c.slug,
    name: c.name,
    subsystem: c.subsystem,
    group: c.group,
    probe_state: c.probe.state,
    latency: { p50_ms: c.probe.latency.p50, p95_ms: c.probe.latency.p95 },
    availability: { "7d": c.probe.availability["7d"], "30d": c.probe.availability["30d"], "90d": c.probe.availability["90d"] },
    dependencies: c.dependencies,
    last_updated: c.lastUpdated,
  };
}
