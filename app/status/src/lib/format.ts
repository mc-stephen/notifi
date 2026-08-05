/**
 * Formatting helpers. All timestamps are UTC (RFC 3339) at rest;
 * rendering is always UTC-labeled (never ambiguous local time).
 */

const MONTHS = [
  "Jan", "Feb", "Mar", "Apr", "May", "Jun",
  "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
];

/** "Jun 12, 2026" */
export function formatDate(iso: string): string {
  const d = new Date(iso);
  return `${MONTHS[d.getUTCMonth()]} ${d.getUTCDate()}, ${d.getUTCFullYear()}`;
}

/** "Jun 12, 2026 · 09:14 UTC" */
export function formatDateTime(iso: string): string {
  const d = new Date(iso);
  const hh = String(d.getUTCHours()).padStart(2, "0");
  const mm = String(d.getUTCMinutes()).padStart(2, "0");
  return `${formatDate(iso)} · ${hh}:${mm} UTC`;
}

/** "09:14 UTC" */
export function formatTime(iso: string): string {
  const d = new Date(iso);
  const hh = String(d.getUTCHours()).padStart(2, "0");
  const mm = String(d.getUTCMinutes()).padStart(2, "0");
  return `${hh}:${mm} UTC`;
}

/** "2h 14m", "45m", "3d 2h", "> 90d" — human durations. */
export function formatDuration(ms: number): string {
  if (!Number.isFinite(ms) || ms < 0) return "—";
  const minutes = Math.round(ms / 60000);
  if (minutes < 1) return "< 1m";
  if (minutes < 60) return `${minutes}m`;
  const hours = Math.floor(minutes / 60);
  const remMin = minutes % 60;
  if (hours < 24) return remMin ? `${hours}h ${remMin}m` : `${hours}h`;
  const days = Math.floor(hours / 24);
  const remHours = hours % 24;
  if (days < 90) return remHours ? `${days}d ${remHours}h` : `${days}d`;
  return `> 90d`;
}

/** "99.98%" — exactly two decimals, tabular-nums friendly. */
export function formatPercent(value: number): string {
  return `${value.toFixed(2)}%`;
}

/** "1,234" */
export function formatNumber(value: number): string {
  return new Intl.NumberFormat("en-US").format(value);
}

/** "14m ago" */
export function timeAgo(iso: string, now: Date = new Date()): string {
  const diff = now.getTime() - new Date(iso).getTime();
  if (diff < 0) return timeUntil(iso, now);
  const minutes = Math.floor(diff / 60000);
  if (minutes < 1) return "just now";
  if (minutes < 60) return `${minutes}m ago`;
  const hours = Math.floor(minutes / 60);
  if (hours < 24) return `${hours}h ago`;
  const days = Math.floor(hours / 24);
  if (days < 30) return `${days}d ago`;
  return formatDate(iso);
}

/** "in 3d", "in 2h", "in 45m" */
export function timeUntil(iso: string, now: Date = new Date()): string {
  const diff = new Date(iso).getTime() - now.getTime();
  if (diff <= 0) return "now";
  const minutes = Math.floor(diff / 60000);
  if (minutes < 60) return `in ${minutes}m`;
  const hours = Math.floor(minutes / 60);
  if (hours < 48) return `in ${hours}h`;
  const days = Math.round(hours / 24);
  return `in ${days}d`;
}

/** Incident duration from start → (now | resolvedAt). */
export function incidentDuration(incident: { startedAt: string; resolvedAt?: string }, now: Date = new Date()): string {
  const end = incident.resolvedAt ?? now.toISOString();
  return formatDuration(new Date(end).getTime() - new Date(incident.startedAt).getTime());
}
