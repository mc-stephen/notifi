export type Plan = "free" | "starter" | "pro" | "enterprise";

export type Project = {
  id: string;
  name: string;
  slug: string;
  description?: string;
  environment: Environment;
  createdAt: string;
};

export type Environment = "development" | "production";

export type TeamMember = {
  id: string;
  userId: string;
  name: string;
  email: string;
  avatar?: string;
  role: Role;
  lastActiveAt: string;
};

export type Role = "owner" | "admin" | "developer" | "viewer" | "billing";

export type InAppNotificationType =
  | "team_add"
  | "team_remove"
  | "role_change"
  | "provider_add"
  | "provider_delete"
  | "api_key_created"
  | "api_key_revoked"
  | "project_created"
  | "billing_change"
  | "system";

export type InAppNotification = {
  id: string;
  type: InAppNotificationType;
  title: string;
  message: string;
  actorName?: string;
  projectName: string;
  read: boolean;
  createdAt: string;
};

export type NotificationSetting = {
  key: string;
  label: string;
  description: string;
  email: boolean;
  inApp: boolean;
};

export type NotificationSettingsCategory = {
  id: string;
  label: string;
  description: string;
  settings: NotificationSetting[];
};

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
  /** The brand's in-house targeting key, unique within the project. */
  userId: string;
  name: string;
  /** Flexible contact blob (email, phone, device/push ids, ...). */
  contacts?: Record<string, unknown>;
  createdAt: string;
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

/// A notification template. Contains one per-channel content representation
/// (subject, html, text, sms, push title/body...) stored as a flexible JSON
/// `content` blob, plus metadata-only attachments.
export type Template = {
  id: string;
  projectId: string;
  name: string;
  description?: string;
  channel: string;
  content: TemplateContent;
  version: number;
  attachments: TemplateAttachment[];
  createdAt: string;
  updatedAt: string;
};

/// Flexible per-channel content. Keys vary by channel; the UI reads/writes
/// well-known keys (subject, html, text, sms, push:{title,body}) via helpers.
export type TemplateContent = Record<string, unknown>;

export type TemplateAttachment = {
  id: string;
  name: string;
  mimeType: string;
  sizeBytes: number;
  url: string;
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

/// A connected messaging provider (e.g. SendGrid, Twilio, FCM).
export type MessageProvider = {
  id: string;
  projectId: string;
  name: string;
  type: NotificationChannel;
  connected: boolean;
  health: HealthStatus;
  latencyMs: number;
  successRate: number;
  region: string;
  config: Record<string, unknown>;
  quotaUsed: number;
  quotaLimit: number;
  createdAt: string;
};

/// A logical channel with provider routing (ordered by priority).
export type Channel = {
  id: string;
  projectId: string;
  type: NotificationChannel;
  enabled: boolean;
  providerIds: string[];
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

// ---------------------------------------------------------------------------
// Support tickets
// ---------------------------------------------------------------------------

export type TicketStatus = "open" | "in_progress" | "resolved" | "closed";

export type SupportTicket = {
  id: string;
  projectId?: string | null;
  subject: string;
  category: string;
  priority: string;
  description: string;
  status: TicketStatus;
  createdAt: string;
  updatedAt: string;
};

export type TicketMessageAuthor = "customer" | "support";

export type TicketMessage = {
  id: string;
  ticketId: string;
  author: TicketMessageAuthor;
  body: string;
  createdAt: string;
};
