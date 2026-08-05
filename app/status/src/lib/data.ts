/**
 * Collection loaders — the only place that touches Astro's content layer.
 * Returns plain domain objects (src/lib/types.ts) so pages, feeds, and
 * the status API share one access path. M6 swaps this module for a
 * backend snapshot loader without touching callers.
 */
import { getCollection } from "astro:content";
import type { Component, Incident, Maintenance, Report } from "./types";

export interface LatestMetrics {
  generatedAt: string;
  apiResponseTimeP95Ms: number;
  webhookLatencyP95Ms: number;
  notificationProcessingP95Ms: number;
  queueDepth: number;
  workerThroughputPerSec: number;
  databaseHealth: "healthy" | "degraded" | "down";
  cacheHitRatePct: number;
  deliverySuccessRate30d: number;
  regionalAvailabilityPct: number;
}

export async function getComponents(): Promise<Component[]> {
  const entries = await getCollection("components");
  return entries.map((e) => e.data);
}

export async function getIncidents(): Promise<Incident[]> {
  const entries = await getCollection("incidents");
  return entries.map((e) => e.data);
}

export async function getMaintenance(): Promise<Maintenance[]> {
  const entries = await getCollection("maintenance");
  return entries.map((e) => e.data);
}

export async function getReports(): Promise<Report[]> {
  const entries = await getCollection("reports");
  return entries.map((e) => e.data);
}

export async function getIncidentBySlug(slug: string): Promise<Incident | undefined> {
  const incidents = await getIncidents();
  return incidents.find((i) => i.slug === slug);
}

export async function getMaintenanceBySlug(slug: string): Promise<Maintenance | undefined> {
  const maintenance = await getMaintenance();
  return maintenance.find((m) => m.slug === slug);
}

export async function getReportBySlug(slug: string): Promise<Report | undefined> {
  const reports = await getReports();
  return reports.find((r) => r.slug === slug);
}

export async function getMetrics(): Promise<LatestMetrics> {
  const entries = await getCollection("metrics");
  const latest = entries[0]?.data;
  return {
    generatedAt: latest.generatedAt,
    ...latest.metrics,
  };
}
