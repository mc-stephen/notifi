import type { ProviderStatus, SystemHealth, MetricData } from "@/lib/types";

export const mockProviders: ProviderStatus[] = [
  {
    id: "prv_001",
    name: "SendGrid",
    channel: "email",
    status: "healthy",
    health: 99.9,
    successRate: 99.8,
    failureRate: 0.2,
    latencyMs: 120,
    region: "us-east-1",
    lastHealthCheck: "2025-09-04T12:00:00Z",
  },
  {
    id: "prv_002",
    name: "Twilio",
    channel: "sms",
    status: "healthy",
    health: 99.7,
    successRate: 99.5,
    failureRate: 0.5,
    latencyMs: 340,
    region: "us-east-1",
    lastHealthCheck: "2025-09-04T12:00:00Z",
  },
  {
    id: "prv_003",
    name: "FCM",
    channel: "push",
    status: "degraded",
    health: 95.2,
    successRate: 94.8,
    failureRate: 5.2,
    latencyMs: 890,
    region: "global",
    lastHealthCheck: "2025-09-04T12:00:00Z",
  },
  {
    id: "prv_004",
    name: "APNs",
    channel: "push",
    status: "healthy",
    health: 99.5,
    successRate: 99.3,
    failureRate: 0.7,
    latencyMs: 450,
    region: "us-west-2",
    lastHealthCheck: "2025-09-04T12:00:00Z",
  },
];

export const mockSystemHealth: SystemHealth[] = [
  {
    service: "API",
    status: "operational",
    latencyMs: 45,
    lastCheck: "2025-09-04T12:00:00Z",
  },
  {
    service: "Notification Engine",
    status: "operational",
    latencyMs: 12,
    lastCheck: "2025-09-04T12:00:00Z",
  },
  {
    service: "Queue",
    status: "operational",
    latencyMs: 3,
    lastCheck: "2025-09-04T12:00:00Z",
  },
  {
    service: "Workers",
    status: "operational",
    latencyMs: 8,
    lastCheck: "2025-09-04T12:00:00Z",
  },
  {
    service: "Database",
    status: "operational",
    latencyMs: 15,
    lastCheck: "2025-09-04T12:00:00Z",
  },
  {
    service: "Redis",
    status: "operational",
    latencyMs: 2,
    lastCheck: "2025-09-04T12:00:00Z",
  },
];

export const mockMetrics: MetricData[] = [
  { title: "Total Users", value: "12,847", change: 12.5, changeLabel: "vs last month" },
  { title: "Active Users", value: "8,234", change: 8.3, changeLabel: "vs last month" },
  { title: "Suspended Users", value: "23", change: -15.2, changeLabel: "vs last month" },
  { title: "Total Organizations", value: "1,245", change: 5.7, changeLabel: "vs last month" },
  { title: "Notifications Sent", value: "2.4M", change: 18.2, changeLabel: "vs last month" },
  { title: "Failed Notifications", value: "1,234", change: -8.5, changeLabel: "vs last month" },
  { title: "Open Support Tickets", value: "3", change: 0, changeLabel: "no change" },
];

// .==================================
// Per-range overview metrics (?range=24h|7d|30d|90d|12M).
// Titles and order must stay identical across ranges so
// switching never reorders the grid. 30d === mockMetrics.
// Future: GET /admin/overview/metrics?range=7d
// .==================================

export const RANGE_KEYS = ["24h", "7d", "30d", "90d", "12M"] as const;

export type RangeKey = (typeof RANGE_KEYS)[number];

export const RANGE_LABELS: Record<RangeKey, string> = {
  "24h": "Today",
  "7d": "7D",
  "30d": "30D",
  "90d": "90D",
  "12M": "12M",
};

export function parseRange(value: string | null): RangeKey {
  return (RANGE_KEYS as readonly string[]).includes(value ?? "") ? (value as RangeKey) : "30d";
}

export const mockMetricsByRange: Record<RangeKey, MetricData[]> = {
  "24h": [
    { title: "Total Users", value: "12,847", change: 0.3, changeLabel: "vs yesterday" },
    { title: "Active Users", value: "4,102", change: 2.1, changeLabel: "vs yesterday" },
    { title: "Suspended Users", value: "23", change: 0, changeLabel: "no change" },
    { title: "Total Organizations", value: "1,245", change: 0.2, changeLabel: "vs yesterday" },
    { title: "Notifications Sent", value: "84K", change: 3.4, changeLabel: "vs yesterday" },
    { title: "Failed Notifications", value: "41", change: -2.3, changeLabel: "vs yesterday" },
    { title: "Open Support Tickets", value: "3", change: 0, changeLabel: "no change" },
  ],
  "7d": [
    { title: "Total Users", value: "12,847", change: 2.8, changeLabel: "vs previous 7 days" },
    { title: "Active Users", value: "6,912", change: 4.6, changeLabel: "vs previous 7 days" },
    { title: "Suspended Users", value: "23", change: -4.1, changeLabel: "vs previous 7 days" },
    { title: "Total Organizations", value: "1,245", change: 1.3, changeLabel: "vs previous 7 days" },
    { title: "Notifications Sent", value: "571K", change: 6.9, changeLabel: "vs previous 7 days" },
    { title: "Failed Notifications", value: "302", change: -3.7, changeLabel: "vs previous 7 days" },
    { title: "Open Support Tickets", value: "3", change: 0, changeLabel: "no change" },
  ],
  "30d": mockMetrics,
  "90d": [
    { title: "Total Users", value: "12,847", change: 31.4, changeLabel: "vs previous 90 days" },
    { title: "Active Users", value: "9,530", change: 22.7, changeLabel: "vs previous 90 days" },
    { title: "Suspended Users", value: "23", change: -28.9, changeLabel: "vs previous 90 days" },
    { title: "Total Organizations", value: "1,245", change: 14.2, changeLabel: "vs previous 90 days" },
    { title: "Notifications Sent", value: "6.8M", change: 41.6, changeLabel: "vs previous 90 days" },
    { title: "Failed Notifications", value: "3,918", change: -12.4, changeLabel: "vs previous 90 days" },
    { title: "Open Support Tickets", value: "3", change: 0, changeLabel: "no change" },
  ],
  "12M": [
    { title: "Total Users", value: "12,847", change: 168.2, changeLabel: "vs previous 12 months" },
    { title: "Active Users", value: "10,214", change: 142.5, changeLabel: "vs previous 12 months" },
    { title: "Suspended Users", value: "23", change: -51.0, changeLabel: "vs previous 12 months" },
    { title: "Total Organizations", value: "1,245", change: 96.3, changeLabel: "vs previous 12 months" },
    { title: "Notifications Sent", value: "24.1M", change: 203.7, changeLabel: "vs previous 12 months" },
    { title: "Failed Notifications", value: "15,207", change: -22.8, changeLabel: "vs previous 12 months" },
    { title: "Open Support Tickets", value: "3", change: 0, changeLabel: "no change" },
  ],
};
