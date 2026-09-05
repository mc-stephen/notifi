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
