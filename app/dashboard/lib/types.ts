export type Organization = {
  id: string;
  name: string;
  slug: string;
  logo?: string;
  plan: Plan;
  createdAt: string;
};

export type Plan = "free" | "starter" | "pro" | "enterprise";

export type Project = {
  id: string;
  orgId: string;
  name: string;
  slug: string;
  description?: string;
  createdAt: string;
};

export type Environment = "development" | "staging" | "production";

export type TeamMember = {
  id: string;
  orgId: string;
  userId: string;
  name: string;
  email: string;
  avatar?: string;
  role: Role;
  lastActiveAt: string;
};

export type Role = "owner" | "admin" | "developer" | "viewer" | "billing";

export type NotificationChannel =
  | "email"
  | "sms"
  | "push-android"
  | "push-ios"
  | "web-push"
  | "linux"
  | "macos"
  | "rcs"
  | "whatsapp"
  | "slack"
  | "discord"
  | "teams"
  | "telegram"
  | "webhook";

export type NotificationStatus =
  | "queued"
  | "processing"
  | "sent"
  | "delivered"
  | "opened"
  | "clicked"
  | "failed"
  | "cancelled"
  | "retrying";

export type NotificationPriority = "low" | "normal" | "high" | "urgent";

export type Notification = {
  id: string;
  projectId: string;
  templateId?: string;
  recipientId: string;
  channel: NotificationChannel;
  status: NotificationStatus;
  priority: NotificationPriority;
  subject?: string;
  body: string;
  metadata?: Record<string, unknown>;
  sentAt?: string;
  deliveredAt?: string;
  openedAt?: string;
  clickedAt?: string;
  failedAt?: string;
  failureReason?: string;
  retryCount: number;
  providerId?: string;
  providerResponse?: Record<string, unknown>;
  createdAt: string;
};

export type Recipient = {
  id: string;
  projectId: string;
  email?: string;
  phone?: string;
  name: string;
  attributes?: Record<string, string>;
  tags?: string[];
  segments?: string[];
  language?: string;
  timezone?: string;
  createdAt: string;
  lastActiveAt?: string;
  devices?: Device[];
};

export type DevicePlatform =
  | "android"
  | "ios"
  | "ipados"
  | "macos"
  | "linux"
  | "windows"
  | "browser";

export type DeviceStatus = "active" | "inactive" | "expired";

export type Device = {
  id: string;
  recipientId: string;
  platform: DevicePlatform;
  token: string;
  provider: string;
  appVersion?: string;
  platformVersion?: string;
  status: DeviceStatus;
  lastActiveAt?: string;
  expiresAt?: string;
};

export type Template = {
  id: string;
  projectId: string;
  name: string;
  folder?: string;
  category?: string;
  channel: NotificationChannel;
  subject?: string;
  body: string;
  variables: TemplateVariable[];
  version: number;
  isDraft: boolean;
  locale?: string;
  createdAt: string;
  updatedAt: string;
};

export type TemplateVariable = {
  name: string;
  type: "string" | "number" | "boolean" | "date" | "json";
  required: boolean;
  defaultValue?: string;
  description?: string;
};

export type Campaign = {
  id: string;
  projectId: string;
  name: string;
  templateId: string;
  status: CampaignStatus;
  channel: NotificationChannel;
  recipientCount: number;
  sentCount: number;
  deliveredCount: number;
  failedCount: number;
  scheduledAt?: string;
  startedAt?: string;
  completedAt?: string;
  createdAt: string;
};

export type CampaignStatus =
  | "draft"
  | "scheduled"
  | "running"
  | "paused"
  | "completed"
  | "cancelled";

export type Schedule = {
  id: string;
  projectId: string;
  notificationId: string;
  cron: string;
  timezone: string;
  active: boolean;
  nextRunAt?: string;
  lastRunAt?: string;
  createdAt: string;
};

export type ChannelConfig = {
  id: string;
  projectId: string;
  type: NotificationChannel;
  enabled: boolean;
  config: Record<string, unknown>;
};

export type ProviderType =
  | "email"
  | "sms"
  | "push-android"
  | "push-ios"
  | "web-push"
  | "webhook";

export type Provider = {
  id: string;
  channelId: string;
  type: ProviderType;
  name: string;
  enabled: boolean;
  health: HealthStatus;
  latencyMs: number;
  successRate: number;
  priority: number;
  fallbackId?: string;
  region?: string;
  quotaUsed: number;
  quotaLimit: number;
  createdAt: string;
};

export type HealthStatus = "healthy" | "degraded" | "down" | "unknown";

export type Webhook = {
  id: string;
  projectId: string;
  url: string;
  events: string[];
  signingSecret: string;
  enabled: boolean;
  lastTriggeredAt?: string;
  successRate: number;
  createdAt: string;
};

export type ApiKey = {
  id: string;
  projectId: string;
  name: string;
  prefix: string;
  environment: Environment;
  permissions: string[];
  scopes: string[];
  expiresAt?: string;
  lastUsedAt?: string;
  rateLimit?: number;
  enabled: boolean;
  createdAt: string;
};

export type EventType =
  | "queued"
  | "worker_assigned"
  | "provider_selected"
  | "sent"
  | "delivered"
  | "opened"
  | "clicked"
  | "failed"
  | "retried"
  | "cancelled";

export type Event = {
  id: string;
  notificationId: string;
  type: EventType;
  timestamp: string;
  metadata?: Record<string, unknown>;
  provider?: string;
  requestId?: string;
  correlationId?: string;
  diagnosticInfo?: Record<string, unknown>;
};

export type LogEntry = {
  id: string;
  projectId: string;
  level: "debug" | "info" | "warn" | "error";
  message: string;
  source: string;
  timestamp: string;
  metadata?: Record<string, unknown>;
};

export type ChartDataPoint = {
  date: string;
  value: number;
  label?: string;
};

export type MetricCard = {
  title: string;
  value: string | number;
  change?: number;
  changeLabel?: string;
  icon?: React.ComponentType<{ className?: string }>;
};
