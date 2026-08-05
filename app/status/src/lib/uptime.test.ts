import { describe, expect, it } from "vitest";
import { downtimeMinutes, incidentFrequency, rangeAvailability, uptimeSeries, uptimeSummary } from "./uptime";
import type { Incident } from "./types";

const NOW = new Date("2026-08-01T15:00:00Z");

function incident(overrides: Partial<Incident> = {}): Incident {
  return {
    slug: "i",
    title: "Incident",
    severity: "minor",
    status: "resolved",
    startedAt: "2026-07-20T10:00:00Z",
    resolvedAt: "2026-07-20T11:00:00Z",
    affectedComponents: ["x"],
    updates: [],
    ...overrides,
  };
}

describe("uptimeSeries", () => {
  it("produces the expected bucket counts", () => {
    expect(uptimeSeries("24h", [], NOW)).toHaveLength(24);
    expect(uptimeSeries("7d", [], NOW)).toHaveLength(168);
    expect(uptimeSeries("30d", [], NOW)).toHaveLength(30);
    expect(uptimeSeries("90d", [], NOW)).toHaveLength(90);
    expect(uptimeSeries("1y", [], NOW)).toHaveLength(52);
    expect(uptimeSeries("all", [], NOW)).toHaveLength(24);
  });

  it("is 100% when there are no incidents", () => {
    expect(uptimeSeries("30d", [], NOW).every((p) => p.availability === 100)).toBe(true);
  });

  it("is oldest → newest", () => {
    const points = uptimeSeries("30d", [], NOW);
    expect(new Date(points[0].t).getTime()).toBeLessThan(new Date(points[1].t).getTime());
    expect(new Date(points[points.length - 1].t).getTime()).toBeLessThanOrEqual(NOW.getTime());
  });

  it("penalizes buckets overlapping an incident by severity factor", () => {
    const fullDay = incident({
      severity: "critical",
      startedAt: "2026-07-01T00:00:00Z",
      resolvedAt: "2026-07-02T00:00:00Z",
    });
    const points = uptimeSeries("90d", [fullDay], NOW);
    const bad = points.find((p) => p.availability < 100);
    expect(bad).toBeDefined();
    expect(bad!.availability).toBe(0);
    const avg = rangeAvailability("90d", [fullDay], NOW);
    expect(avg).toBeCloseTo(98.89, 1); // 89/90 buckets at 100%
  });

  it("minor incidents apply a smaller penalty than critical ones", () => {
    const oneHour = { startedAt: "2026-07-31T12:00:00Z", resolvedAt: "2026-07-31T13:00:00Z" };
    const minor = incident({ severity: "minor", ...oneHour });
    const critical = incident({ severity: "critical", ...oneHour });
    const hitMinor = uptimeSeries("7d", [minor], NOW).find((p) => p.availability < 100);
    const hitCritical = uptimeSeries("7d", [critical], NOW).find((p) => p.availability < 100);
    expect(hitMinor!.availability).toBeGreaterThan(hitCritical!.availability);
  });
});

describe("uptimeSummary", () => {
  it("is consistent with rangeAvailability for every range", () => {
    const incidents = [
      incident({
        severity: "major",
        startedAt: "2026-06-12T09:14:00Z",
        resolvedAt: "2026-06-12T11:28:00Z",
      }),
    ];
    const summary = uptimeSummary(incidents, NOW);
    for (const range of ["24h", "7d", "30d", "90d", "1y", "all"] as const) {
      expect(summary[range]).toBe(rangeAvailability(range, incidents, NOW));
    }
  });
});

describe("downtimeMinutes", () => {
  it("counts only the overlap inside the range window", () => {
    const now = new Date("2026-08-01T15:00:00Z");
    const twoHour = incident({
      severity: "critical",
      startedAt: "2026-07-29T16:00:00Z", // outside 24h window, inside 7d
      resolvedAt: "2026-07-29T18:00:00Z",
    });
    expect(downtimeMinutes("24h", [twoHour], now)).toBe(0);
    expect(downtimeMinutes("7d", [twoHour], now)).toBe(120);
  });

  it("counts partial overlap at the window edge", () => {
    const now = new Date("2026-08-01T15:00:00Z");
    const ongoing = incident({
      severity: "minor",
      status: "identified",
      startedAt: "2026-08-01T14:30:00Z",
      resolvedAt: undefined,
    });
    expect(downtimeMinutes("24h", [ongoing], now)).toBe(30);
  });
});

describe("incidentFrequency", () => {
  it("counts incidents per month, oldest → newest", () => {
    const incidents = [
      incident({ slug: "a", startedAt: "2026-07-05T00:00:00Z" }),
      incident({ slug: "b", startedAt: "2026-07-20T00:00:00Z" }),
      incident({ slug: "c", startedAt: "2026-05-10T00:00:00Z" }),
    ];
    const freq = incidentFrequency(incidents, 6, NOW);
    expect(freq).toHaveLength(6);
    expect(freq[4].count).toBe(2); // July
    expect(freq[2].count).toBe(1); // May
  });
});
