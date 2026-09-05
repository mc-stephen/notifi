export type AdminUser = {
  id: string;
  name: string;
  email: string;
  avatar?: string;
  status: "active" | "restricted" | "suspended" | "banned" | "deleted";
  organization?: string;
  role: "super_admin" | "admin" | "support" | "billing" | "operations" | "readonly";
  plan: string;
  createdAt: string;
  lastActiveAt: string;
  notificationCount: number;
  billingStatus: "current" | "past_due" | "cancelled";
};

export type Organization = {
  id: string;
  name: string;
  slug: string;
  plan: string;
  memberCount: number;
  projectCount: number;
  status: "active" | "restricted" | "suspended";
  createdAt: string;
  monthlyNotifications: number;
};

export type Ticket = {
  id: string;
  subject: string;
  description: string;
  status: "open" | "pending" | "waiting" | "resolved" | "closed";
  priority: "low" | "normal" | "high" | "urgent";
  customerName: string;
  customerEmail: string;
  organization?: string;
  assignedAdmin?: string;
  createdAt: string;
  updatedAt: string;
  messageCount: number;
};

export type TicketMessage = {
  id: string;
  ticketId: string;
  author: "customer" | "admin";
  authorName: string;
  body: string;
  isInternalNote: boolean;
  createdAt: string;
};

export type AdminNotification = {
  id: string;
  channel: string;
  status: "queued" | "sent" | "delivered" | "failed";
  subject?: string;
  recipientEmail: string;
  organization?: string;
  provider?: string;
  sentAt?: string;
  deliveredAt?: string;
  failureReason?: string;
  createdAt: string;
};

export type Subscription = {
  id: string;
  customerName: string;
  customerEmail: string;
  plan: string;
  status: "active" | "past_due" | "cancelled" | "trialing";
  amount: number;
  currency: string;
  startDate: string;
  renewalDate: string;
  usagePercent: number;
};

export type Payment = {
  id: string;
  customerName: string;
  amount: number;
  currency: string;
  status: "succeeded" | "failed" | "pending" | "refunded";
  paymentMethod: string;
  createdAt: string;
  failureReason?: string;
};

export type Invoice = {
  id: string;
  customerName: string;
  amount: number;
  currency: string;
  status: "paid" | "open" | "void" | "uncollectible";
  period: string;
  createdAt: string;
  paidAt?: string;
};

export type AuditLogEntry = {
  id: string;
  actor: string;
  actorEmail: string;
  action: string;
  target: string;
  targetType: string;
  timestamp: string;
  ipAddress?: string;
  result: "success" | "failure";
  metadata?: Record<string, unknown>;
};

export type ProviderStatus = {
  id: string;
  name: string;
  channel: string;
  status: "healthy" | "degraded" | "down" | "unknown";
  health: number;
  successRate: number;
  failureRate: number;
  latencyMs: number;
  region: string;
  lastHealthCheck: string;
};

export type SystemHealth = {
  service: string;
  status: "operational" | "degraded" | "down" | "unknown";
  latencyMs?: number;
  lastCheck: string;
};

export type AdminUser2 = {
  id: string;
  name: string;
  email: string;
  role: "super_admin" | "admin" | "support" | "billing" | "operations" | "readonly";
  status: "active" | "disabled";
  lastLoginAt?: string;
  createdAt: string;
};

export type MetricData = {
  title: string;
  value: string | number;
  change?: number;
  changeLabel?: string;
};
