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

export type ApprovalRequest = {
  id: string;
  kind: "create" | "remove";
  targetAdminId: string;
  targetName?: string | null;
  targetEmail?: string | null;
  requestedBy: string;
  requesterName?: string | null;
  status: "pending" | "approved" | "rejected";
  decidedAt?: string | null;
  createdAt: string;
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
