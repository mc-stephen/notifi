export type AdminUserStatus = "active" | "suspended";

export type AdminAccount = {
  id: string;
  name: string;
  email: string;
  isSuperAdmin: boolean;
  status: "pending" | "active" | "suspended";
  totpEnabled: boolean;
  createdAt: string;
  lastLoginAt?: string | null;
};

export type AdminUser = {
  id: string;
  name: string;
  email: string;
  avatar?: string | null;
  emailVerified: boolean;
  status: AdminUserStatus;
  createdAt: string;
  lastLoginAt?: string | null;
};

export type AdminUserDetail = {
  id: string;
  name: string;
  email: string;
  avatar?: string | null;
  emailVerified: boolean;
  status: AdminUserStatus;
  createdAt: string;
  lastLoginAt?: string | null;
  projectCount: number;
  ticketTotal: number;
  ticketsOpen: number;
  notificationCount: number;
};

export type AdminProjectMember = {
  userId: string;
  name?: string | null;
  email?: string | null;
  role: string;
};

export type AdminProject = {
  id: string;
  name: string;
  slug: string;
  description?: string | null;
  environment: string;
  ownerId?: string | null;
  ownerName?: string | null;
  ownerEmail?: string | null;
  memberCount: number;
  createdAt: string;
  updatedAt: string;
};

export type AdminProjectDetail = AdminProject & {
  members: AdminProjectMember[];
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

export type BroadcastStatus = "scheduled" | "sending" | "sent" | "cancelled";

export type BroadcastAudience =
  | { kind: "all" }
  | { kind: "users"; userIds: string[] }
  | { kind: "projects"; projectIds: string[] };

export type AdminBroadcast = {
  id: string;
  adminEmail: string;
  notificationType: string;
  title: string;
  content: string;
  audienceKind?: string | null;
  channels: string[];
  status: BroadcastStatus;
  scheduledFor?: string | null;
  sentAt?: string | null;
  recipientCount: number;
  readCount: number;
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

// .==================================
// Pricing plans + subscriber history (admin /billing).
// Mock-backed for now; future Rust endpoints:
// GET /admin/billing/plans, /subscribers/summary, /subscribers/history
// .==================================

export type PlanCapability = {
  notificationsPerMonth: number | null;
  channels: string[];
  teamMembers: number | null;
  retentionDays: number | null;
  support: string;
  branding: boolean;
  apiCalls: number | null;
};

export type YearlyDiscount = {
  kind: "percent" | "fixed";
  // Percent (0–100) or fixed cents off the 12-month total.
  value: number;
};

export type BillingPlan = {
  id: string;
  name: string;
  priceCents: number | null;
  currency: string;
  // Present on local seeds; the API prices by priceCents and omits this.
  interval?: "month" | "year" | "custom";
  // Null = monthly only; set = user paying 12 months gets this cut.
  yearlyDiscount: YearlyDiscount | null;
  activeSubscribers: number;
  capabilities: PlanCapability;
  isActive: boolean;
};

export type SubscriberHistoryPoint = {
  date: string;
  total: number;
  byPlan: Record<string, number>;
};

export type PlanSubscriber = {
  subscriptionId: string;
  projectId: string;
  projectName: string;
  customerName: string | null;
  customerEmail: string | null;
  status: string;
  billingCycle: string;
  periodEnd: string;
};

export type AuditLogEntry = {
  id: string;
  userId?: string | null;
  adminId?: string | null;
  actorType: "user" | "admin" | "system";
  actorName?: string | null;
  eventType: string;
  message: string;
  projectId?: string | null;
  metadata?: Record<string, unknown> | null;
  occurredAt: string;
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

export type MetricData = {
  title: string;
  value: string | number;
  change?: number;
  changeLabel?: string;
};
