export type Environment = "production" | "development";

export type NotificationStatus =
  | "queued"
  | "sent"
  | "delivered"
  | "failed";

export type Channel =
  | "email"
  | "fcm"
  | "apns"
  | "sms"
  | "webpush";

export type ProviderStatus = "healthy" | "degraded" | "outage";

export interface Tenant {
  id: string;
  name: string;
  slug: string;
}

export interface KpiMetric {
  label: string;
  value: number;
  previousValue: number;
  change: number;
  isPercentage: boolean;
  icon: string;
}

export interface TimeSeriesPoint {
  timestamp: string;
  ingested: number;
  delivered: number;
}

export interface ChannelBreakdown {
  channel: Channel;
  volume: number;
  color: string;
}

export interface Provider {
  id: string;
  name: string;
  channel: Channel;
  status: ProviderStatus;
  icon: string;
}

export interface LogEntry {
  id: string;
  trackingId: string;
  subscriberId: string;
  channel: Channel;
  status: NotificationStatus;
  timestamp: string;
  metadata: Record<string, unknown>;
  executionSteps: ExecutionStep[];
  error?: string;
}

export interface ExecutionStep {
  label: string;
  offsetMs: number;
  status: "success" | "error" | "pending";
}

export interface Subscriber {
  id: string;
  email?: string;
  phone?: string;
  name?: string;
  createdAt: string;
  tokens: DeviceToken[];
}

export interface DeviceToken {
  id: string;
  channel: Channel;
  token: string;
  lastUsed: string;
  active: boolean;
}

export interface ChannelConfig {
  id: string;
  provider: string;
  channel: Channel;
  configured: boolean;
  configFields: ConfigField[];
}

export interface ConfigField {
  key: string;
  label: string;
  type: "text" | "password" | "email" | "file" | "select";
  required: boolean;
  placeholder?: string;
  options?: string[];
}

export interface ApiKey {
  id: string;
  prefix: string;
  maskedKey: string;
  createdAt: string;
  environment: Environment;
  active: boolean;
}

export interface Webhook {
  id: string;
  url: string;
  events: string[];
  active: boolean;
  createdAt: string;
}
