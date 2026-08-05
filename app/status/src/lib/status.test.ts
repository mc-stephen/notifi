import { describe, expect, it } from "vitest";
import {
  activeIncidentFor,
  isIncidentOpen,
  openIncidents,
  resolveComponentHealth,
  resolveGlobalHealth,
} from "./status";
import type { Component, Incident, Maintenance } from "./types";

const NOW = new Date("2026-08-01T15:00:00Z");

function component(overrides: Partial<Component> = {}): Component {
  return {
    slug: "test-component",
    name: "Test Component",
    subsystem: "platform",
    group: "Platform",
    dependencies: [],
    probe: {
      state: "operational",
      latency: { p50: 10, p95: 30 },
      availability: { "7d": 100, "30d": 99.9, "90d": 99.9 },
    },
    lastUpdated: "2026-08-01T14:00:00Z",
    ...overrides,
  };
}

function incident(overrides: Partial<Incident> = {}): Incident {
  return {
    slug: "test-incident",
    title: "Test Incident",
    severity: "minor",
    status: "investigating",
    startedAt: "2026-08-01T09:00:00Z",
    affectedComponents: ["test-component"],
    updates: [],
    ...overrides,
  };
}

function maintenance(overrides: Partial<Maintenance> = {}): Maintenance {
  return {
    slug: "test-maintenance",
    title: "Test Maintenance",
    purpose: "Test",
    expectedImpact: "None",
    affectedComponents: ["test-component"],
    startTime: "2026-08-01T10:00:00Z",
    endTime: "2026-08-01T12:00:00Z",
    status: "scheduled",
    updates: [],
    ...overrides,
  };
}

describe("isIncidentOpen / openIncidents", () => {
  it("treats resolved incidents as closed", () => {
    expect(isIncidentOpen(incident({ status: "resolved" }))).toBe(false);
    expect(isIncidentOpen(incident({ status: "identified" }))).toBe(true);
  });

  it("sorts open incidents newest first", () => {
    const a = incident({ slug: "a", startedAt: "2026-08-01T08:00:00Z" });
    const b = incident({ slug: "b", startedAt: "2026-08-01T10:00:00Z" });
    const c = incident({ slug: "c", status: "resolved" });
    expect(openIncidents([a, b, c], NOW).map((i) => i.slug)).toEqual(["b", "a"]);
  });
});

describe("activeIncidentFor", () => {
  it("returns the most severe open incident affecting the component", () => {
    const minor = incident({ slug: "minor", severity: "minor" });
    const critical = incident({ slug: "critical", severity: "critical", startedAt: "2026-08-01T08:00:00Z" });
    expect(activeIncidentFor(component(), [minor, critical], NOW)?.slug).toBe("critical");
  });

  it("ignores incidents affecting other components", () => {
    const other = incident({ affectedComponents: ["other-component"] });
    expect(activeIncidentFor(component(), [other], NOW)).toBeUndefined();
  });

  it("ignores resolved incidents", () => {
    const resolved = incident({ status: "resolved" });
    expect(activeIncidentFor(component(), [resolved], NOW)).toBeUndefined();
  });
});

describe("resolveComponentHealth", () => {
  it("maps severity to state: critical → major_outage", () => {
    const health = resolveComponentHealth(component(), [incident({ severity: "critical" })], [], NOW);
    expect(health.state).toBe("major_outage");
    expect(health.incident?.slug).toBe("test-incident");
  });

  it("maps severity to state: major → partial_outage, minor → degraded", () => {
    expect(resolveComponentHealth(component(), [incident({ severity: "major" })], [], NOW).state).toBe("partial_outage");
    expect(resolveComponentHealth(component(), [incident({ severity: "minor" })], [], NOW).state).toBe("degraded");
  });

  it("in-progress maintenance wins over probe state", () => {
    const health = resolveComponentHealth(
      component(),
      [],
      [maintenance({ status: "in_progress" })],
      NOW,
    );
    expect(health.state).toBe("maintenance");
    expect(health.maintenance?.slug).toBe("test-maintenance");
  });

  it("incidents win over in-progress maintenance", () => {
    const health = resolveComponentHealth(
      component(),
      [incident({ severity: "critical" })],
      [maintenance({ status: "in_progress" })],
      NOW,
    );
    expect(health.state).toBe("major_outage");
  });

  it("falls back to probe state, and flags future maintenance without changing state", () => {
    const degraded = resolveComponentHealth(component({ probe: { ...component().probe, state: "degraded" } }), [], [], NOW);
    expect(degraded.state).toBe("degraded");

    const scheduled = resolveComponentHealth(component(), [], [maintenance()], NOW);
    expect(scheduled.state).toBe("operational");
    expect(scheduled.maintenanceScheduled).toBe(true);
  });
});

describe("resolveGlobalHealth", () => {
  it("global state is the worst of all subsystems", () => {
    const c1 = component({ slug: "a", subsystem: "platform" });
    const c2 = component({ slug: "b", subsystem: "data", probe: { ...component().probe, state: "partial_outage" } });
    const global = resolveGlobalHealth([c1, c2], [], [], NOW);
    expect(global.state).toBe("partial_outage");
  });

  it("provider subsystem degradation caps the global state at degraded", () => {
    const c1 = component({ slug: "a", subsystem: "platform" });
    const c2 = component({ slug: "b", subsystem: "providers", probe: { ...component().probe, state: "major_outage" } });
    const global = resolveGlobalHealth([c1, c2], [], [], NOW);
    expect(global.state).toBe("degraded");
  });

  it("platform outage dominates provider outage", () => {
    const c1 = component({ slug: "a", subsystem: "platform", probe: { ...component().probe, state: "major_outage" } });
    const c2 = component({ slug: "b", subsystem: "providers", probe: { ...component().probe, state: "major_outage" } });
    const global = resolveGlobalHealth([c1, c2], [], [], NOW);
    expect(global.state).toBe("major_outage");
  });

  it("in-progress maintenance does not degrade the global indicator", () => {
    const c1 = component({ slug: "a", subsystem: "platform" });
    const global = resolveGlobalHealth([c1], [], [maintenance({ status: "in_progress" })], NOW);
    expect(global.state).toBe("operational");
  });

  it("groups components into subsystems", () => {
    const c1 = component({ slug: "a", subsystem: "platform" });
    const c2 = component({ slug: "b", subsystem: "providers" });
    const global = resolveGlobalHealth([c1, c2], [], [], NOW);
    expect(global.subsystems).toHaveLength(2);
    expect(global.subsystems.find((s) => s.subsystem === "providers")?.components).toHaveLength(1);
  });
});
