/**
 * Uptime series generation — deterministic, derived purely from the
 * incident collection (design §11.4, §13 guardrails).
 *
 * - Availability per bucket = 100 − max incident penalty in that bucket.
 *   Penalty = overlap fraction × severity factor (critical 1.0, major 0.6,
 *   minor 0.25).
 * - Scheduled maintenance is excluded from availability (design §8.3).
 */
import type { Incident } from "./types";

export type UptimeRange = "24h" | "7d" | "30d" | "90d" | "1y" | "all";

export const UPTIME_RANGES: UptimeRange[] = ["24h", "7d", "30d", "90d", "1y", "all"];

export interface UptimePoint {
  t: string;
  availability: number;
  label: string;
}

const SEVERITY_FACTOR: Record<Incident["severity"], number> = {
  critical: 1.0,
  major: 0.6,
  minor: 0.25,
};

function bucketSpec(range: UptimeRange): { sizeMs: number; count: number } {
  const HOUR = 3_600_000;
  const DAY = 24 * HOUR;
  switch (range) {
    case "24h": return { sizeMs: HOUR, count: 24 };
    case "7d": return { sizeMs: HOUR, count: 7 * 24 };
    case "30d": return { sizeMs: DAY, count: 30 };
    case "90d": return { sizeMs: DAY, count: 90 };
    case "1y": return { sizeMs: 7 * DAY, count: 52 };
    case "all": return { sizeMs: 30 * DAY, count: 24 };
  }
}

function alignStart(ms: number, sizeMs: number): number {
  if (sizeMs >= 3_600_000) return Math.floor(ms / sizeMs) * sizeMs;
  return ms;
}

function windowFor(incident: Incident, now: Date): { start: number; end: number } {
  return {
    start: new Date(incident.startedAt).getTime(),
    end: incident.resolvedAt ? new Date(incident.resolvedAt).getTime() : now.getTime(),
  };
}

/** Availability for a single bucket, given all incident windows. */
function bucketAvailability(
  bucketStart: number,
  bucketEnd: number,
  incidents: Incident[],
  now: Date,
): number {
  let penalty = 0;
  for (const incident of incidents) {
    const w = windowFor(incident, now);
    const overlapStart = Math.max(bucketStart, w.start);
    const overlapEnd = Math.min(bucketEnd, w.end);
    if (overlapEnd <= overlapStart) continue;
    const fraction = (overlapEnd - overlapStart) / (bucketEnd - bucketStart);
    penalty = Math.max(penalty, fraction * 100 * SEVERITY_FACTOR[incident.severity]);
  }
  return Math.max(0, Math.round((100 - penalty) * 100) / 100);
}

const RANGE_LABELS: Record<UptimeRange, (d: Date) => string> = {
  "24h": (d) => `${String(d.getUTCHours()).padStart(2, "0")}:00`,
  "7d": (d) => {
    const days = ["Sun", "Mon", "Tue", "Wed", "Thu", "Fri", "Sat"];
    return `${days[d.getUTCDay()]} ${d.getUTCDate()}`;
  },
  "30d": (d) => `${d.getUTCMonth() + 1}/${d.getUTCDate()}`,
  "90d": (d) => `${d.getUTCMonth() + 1}/${d.getUTCDate()}`,
  "1y": (d) => {
    const weeks = ["W1", "W2", "W3", "W4", "W5"];
    return `${d.getUTCMonth() + 1}/${d.getUTCDate()}`;
  },
  all: (d) => {
    const months = ["Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec"];
    return months[d.getUTCMonth()];
  },
};

/** Full series for a range, oldest → newest. */
export function uptimeSeries(range: UptimeRange, incidents: Incident[], now: Date = new Date()): UptimePoint[] {
  const { sizeMs, count } = bucketSpec(range);
  const nowMs = now.getTime();
  const lastStart = alignStart(nowMs, sizeMs);
  const points: UptimePoint[] = [];

  for (let i = count - 1; i >= 0; i--) {
    const bucketStart = lastStart - i * sizeMs;
    const bucketEnd = bucketStart + sizeMs;
    const t = new Date(bucketStart);
    points.push({
      t: t.toISOString(),
      availability: bucketAvailability(bucketStart, bucketEnd, incidents, now),
      label: RANGE_LABELS[range](t),
    });
  }
  return points;
}

/** Mean availability over a range's buckets. */
export function rangeAvailability(range: UptimeRange, incidents: Incident[], now: Date = new Date()): number {
  const points = uptimeSeries(range, incidents, now);
  if (points.length === 0) return 100;
  const mean = points.reduce((acc, p) => acc + p.availability, 0) / points.length;
  return Math.round(mean * 100) / 100;
}

/** Total incident downtime (wall-clock minutes) inside a range's window. */
export function downtimeMinutes(range: UptimeRange, incidents: Incident[], now: Date = new Date()): number {
  const { sizeMs, count } = bucketSpec(range);
  const nowMs = now.getTime();
  const windowStart = alignStart(nowMs, sizeMs) - (count - 1) * sizeMs;
  const windowEnd = nowMs;
  let minutes = 0;
  for (const incident of incidents) {
    const w = windowFor(incident, now);
    const overlap = Math.max(0, Math.min(windowEnd, w.end) - Math.max(windowStart, w.start));
    minutes += overlap / 60000;
  }
  return Math.round(minutes);
}

export interface UptimeSummary {
  "24h": number;
  "7d": number;
  "30d": number;
  "90d": number;
  "1y": number;
  all: number;
}

/** Availability for every range, consistent with the series. */
export function uptimeSummary(incidents: Incident[], now: Date = new Date()): UptimeSummary {
  return {
    "24h": rangeAvailability("24h", incidents, now),
    "7d": rangeAvailability("7d", incidents, now),
    "30d": rangeAvailability("30d", incidents, now),
    "90d": rangeAvailability("90d", incidents, now),
    "1y": rangeAvailability("1y", incidents, now),
    all: rangeAvailability("all", incidents, now),
  };
}

/** Incidents per month, oldest → newest (incident frequency chart). */
export function incidentFrequency(incidents: Incident[], months = 12, now: Date = new Date()): { label: string; count: number }[] {
  const out: { label: string; count: number }[] = [];
  const names = ["Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec"];

  for (let i = months - 1; i >= 0; i--) {
    const d = new Date(Date.UTC(now.getUTCFullYear(), now.getUTCMonth() - i, 1));
    out.push({ label: names[d.getUTCMonth()], count: 0 });
  }

  for (const incident of incidents) {
    const d = new Date(incident.startedAt);
    const idx = months - 1 - ((now.getUTCFullYear() * 12 + now.getUTCMonth()) - (d.getUTCFullYear() * 12 + d.getUTCMonth()));
    if (idx >= 0 && idx < months) out[idx].count += 1;
  }
  return out;
}
