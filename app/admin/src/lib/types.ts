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

export type AdminTicketStatus = "open" | "in_progress" | "resolved" | "closed";

export type AdminTicket = {
  id: string;
  projectId?: string | null;
  subject: string;
  description: string;
  category: string;
  priority: string;
  status: AdminTicketStatus;
  customerId: string;
  customerName: string;
  customerEmail: string;
  createdAt: string;
  updatedAt: string;
};

export type AdminTicketMessage = {
  id: string;
  ticketId: string;
  author: "customer" | "support";
  authorName?: string | null;
  body: string;
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
