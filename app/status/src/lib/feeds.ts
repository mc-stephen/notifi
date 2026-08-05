/**
 * Feed entry builders — shared by the RSS and Atom endpoints.
 *
 * Rules (design §15.1):
 * - Every incident update is an entry (append-only; feed readers get a
 *   push per update, never a rewrite).
 * - Every maintenance announcement + progress update is an entry.
 * - Entries carry `category` terms: incident | maintenance,
 *   severity:minor|major|critical, and component slugs.
 */
import { formatDateTime, incidentDuration } from "./format";
import type { Incident, Maintenance } from "./types";

export interface FeedEntry {
  /** Stable unique id: incident/update timestamp. */
  guid: string;
  title: string;
  /** Absolute URL to the incident/maintenance page. */
  link: string;
  pubDate: string;
  updatedAt: string;
  author: string;
  categories: string[];
  content: string;
}

export function buildFeedEntries(
  incidents: Incident[],
  maintenance: Maintenance[],
  site: string,
  now: Date = new Date(),
): FeedEntry[] {
  const entries: FeedEntry[] = [];

  for (const incident of incidents) {
    const link = `${site}/incidents/${incident.slug}/`;
    for (const update of incident.updates) {
      entries.push({
        guid: `${incident.slug}-${update.timestamp}`,
        title: `${incident.title} — ${update.state}`,
        link,
        pubDate: update.timestamp,
        updatedAt: update.timestamp,
        author: update.author,
        categories: [
          "incident",
          `severity:${incident.severity}`,
          `status:${incident.status}`,
          ...incident.affectedComponents.map((c) => `component:${c}`),
        ],
        content: [
          `Status: ${incident.status} · Severity: ${incident.severity}`,
          update.details,
          incident.customerImpact ? `Customer impact: ${incident.customerImpact}` : "",
          incident.resolvedAt ? `Duration: ${incidentDuration(incident, now)}` : "",
        ]
          .filter(Boolean)
          .join("\n"),
      });
    }
  }

  for (const event of maintenance) {
    const link = `${site}/maintenance/${event.slug}/`;
    for (const update of event.updates) {
      entries.push({
        guid: `${event.slug}-${update.timestamp}`,
        title: `${event.title} — ${update.state}`,
        link,
        pubDate: update.timestamp,
        updatedAt: update.timestamp,
        author: update.author,
        categories: ["maintenance", `mstate:${event.status}`, ...event.affectedComponents.map((c) => `component:${c}`)],
        content: [
          `Status: ${event.status}`,
          update.details,
          `Window: ${formatDateTime(event.startTime)} → ${formatDateTime(event.endTime)} UTC`,
          event.completionSummary ? `Completion summary: ${event.completionSummary}` : "",
        ]
          .filter(Boolean)
          .join("\n"),
      });
    }
  }

  return entries.sort((a, b) => b.pubDate.localeCompare(a.pubDate));
}

export function filterEntries(entries: FeedEntry[], incidentSlug?: string | null): FeedEntry[] {
  if (!incidentSlug) return entries;
  return entries.filter((e) => e.link.endsWith(`/incidents/${incidentSlug}/`));
}
