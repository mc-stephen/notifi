const API_BASE = import.meta.env.PUBLIC_API_URL ?? "http://localhost:8080";

async function request<T>(path: string, init?: RequestInit): Promise<T> {
  const res = await fetch(`${API_BASE}${path}`, {
    ...init,
    credentials: "include",
    headers: {
      "Content-Type": "application/json",
      ...init?.headers,
    },
  });
  if (!res.ok) {
    const body = await res.json().catch(() => null);
    const detail =
      body?.detail ?? body?.message ?? `${res.status} ${res.statusText}`;
    throw new ApiError(res.status, detail);
  }
  return res.json();
}

export class ApiError extends Error {
  status: number;
  constructor(status: number, detail: string) {
    super(detail);
    this.status = status;
  }
}

// ── Auth ──────────────────────────────────────────────────────────────────

export type AdminLoginResponse = {
  user_id?: string;
  requires_totp?: boolean;
  requires_totp_setup?: boolean;
};

export type AdminStatusResponse = {
  adminExists: boolean;
};

export type AdminMeResponse = {
  id: string;
  name: string;
  email: string;
  totpEnabled: boolean;
  isSuperAdmin: boolean;
  status: string;
};

export type BootstrapResponse = {
  userId: string;
  totpSecret: string;
  totpUri: string;
};

export type TotpSetupResponse = {
  totpSecret: string;
  totpUri: string;
};

export const auth = {
  adminStatus: () => request<AdminStatusResponse>("/admin/status"),

  bootstrap: (data: { name: string; email: string; password: string }) =>
    request<BootstrapResponse>("/admin/bootstrap", {
      method: "POST",
      body: JSON.stringify(data),
    }),

  login: (data: { email: string; password: string; totp_code?: string }) =>
    request<AdminLoginResponse>("/admin/login", {
      method: "POST",
      body: JSON.stringify(data),
    }),

  totpSetup: (regenerate = false) =>
    request<TotpSetupResponse>(
      `/admin/totp/setup${regenerate ? "?regenerate=true" : ""}`,
      { method: "POST", body: "{}" },
    ),

  totpVerify: (code: string) =>
    request<{ status: string }>("/admin/totp/verify", {
      method: "POST",
      body: JSON.stringify({ code }),
    }),

  forgotPassword: (email: string) =>
    request<{ status: string; resetToken?: string }>("/admin/password/forgot", {
      method: "POST",
      body: JSON.stringify({ email }),
    }),

  resetPassword: (token: string, password: string) =>
    request<{ status: string }>("/admin/password/reset", {
      method: "POST",
      body: JSON.stringify({ token, password }),
    }),

  me: () => request<AdminMeResponse>("/admin/me"),

  changePassword: (currentPassword: string, newPassword: string) =>
    request<{ status: string }>("/admin/password/change", {
      method: "POST",
      body: JSON.stringify({ currentPassword, newPassword }),
    }),

  logout: () =>
    request<{ status: string }>("/admin/logout", { method: "POST" }),
};

// ── Admin accounts ──────────────────────────────────────────────────────────

import type { AdminAccount } from "./types";
import type { AuditLogEntry } from "./types";

export const admins = {
  listAdmins: () => request<{ admins: AdminAccount[] }>("/admin/admins"),

  createAdmin: (data: { name: string; email: string; password: string }) =>
    request<{ admin: AdminAccount }>("/admin/admins", {
      method: "POST",
      body: JSON.stringify(data),
    }),

  removeAdmin: (id: string) =>
    request<{ status: string }>(
      `/admin/admins/${encodeURIComponent(id)}/remove`,
      { method: "POST" },
    ),

  setStatus: (id: string, status: "active" | "suspended") =>
    request<{ status: string }>(
      `/admin/admins/${encodeURIComponent(id)}/status`,
      { method: "PATCH", body: JSON.stringify({ status }) },
    ),
};

// ── Support tickets (admin view) ────────────────────────────────────────────

import type { AdminTicket, AdminTicketMessage, AdminTicketStatus } from "./types";
import type { AdminUser, AdminUserDetail, AdminUserStatus } from "./types";
import type { AdminProject, AdminProjectDetail } from "./types";
import type { AdminBroadcast, BroadcastAudience } from "./types";

export type PageInfo = {
  page: number;
  perPage: number;
  total: number;
  totalPages: number;
};

export type TicketListResponse = {
  tickets: AdminTicket[];
} & PageInfo;

export const support = {
  listTickets: (params?: { status?: string; limit?: number; page?: number }) => {
    const search = new URLSearchParams();
    if (params?.status) search.set("status", params.status);
    if (params?.limit) search.set("limit", String(params.limit));
    if (params?.page) search.set("page", String(params.page));
    const query = search.toString();
    return request<TicketListResponse>(
      `/admin/support/tickets${query ? `?${query}` : ""}`,
    );
  },

  getTicket: (id: string) =>
    request<{ ticket: AdminTicket }>(
      `/admin/support/tickets/${encodeURIComponent(id)}`,
    ),

  listMessages: (id: string) =>
    request<{ messages: AdminTicketMessage[] }>(
      `/admin/support/tickets/${encodeURIComponent(id)}/messages`,
    ),

  sendReply: (id: string, body: string) =>
    request<{ message: AdminTicketMessage }>(
      `/admin/support/tickets/${encodeURIComponent(id)}/messages`,
      { method: "POST", body: JSON.stringify({ body }) },
    ),

  setStatus: (id: string, status: AdminTicketStatus) =>
    request<{ ticket: AdminTicket }>(
      `/admin/support/tickets/${encodeURIComponent(id)}`,
      { method: "PATCH", body: JSON.stringify({ status }) },
    ),
};

// ── Platform users (admin view) ─────────────────────────────────────────────

export type UserListResponse = {
  users: AdminUser[];
} & PageInfo;

export const users = {
  listUsers: (params?: { search?: string; status?: string; limit?: number; page?: number }) => {
    const search = new URLSearchParams();
    if (params?.search) search.set("search", params.search);
    if (params?.status) search.set("status", params.status);
    if (params?.limit) search.set("limit", String(params.limit));
    if (params?.page) search.set("page", String(params.page));
    const query = search.toString();
    return request<UserListResponse>(
      `/admin/users${query ? `?${query}` : ""}`,
    );
  },

  getUser: (id: string) =>
    request<{ user: AdminUserDetail }>(
      `/admin/users/${encodeURIComponent(id)}`,
    ),

  setStatus: (id: string, status: AdminUserStatus) =>
    request<{ user: AdminUserDetail }>(
      `/admin/users/${encodeURIComponent(id)}/status`,
      { method: "PATCH", body: JSON.stringify({ status }) },
    ),

  revokeSessions: (id: string) =>
    request<{ status: string }>(
      `/admin/users/${encodeURIComponent(id)}/sessions/revoke`,
      { method: "POST" },
    ),
};

// ── Platform projects (admin view) ──────────────────────────────────────────

export type ProjectListResponse = {
  projects: AdminProject[];
} & PageInfo;

export const projects = {
  listProjects: (params?: { search?: string; environment?: string; limit?: number; page?: number }) => {
    const search = new URLSearchParams();
    if (params?.search) search.set("search", params.search);
    if (params?.environment) search.set("environment", params.environment);
    if (params?.limit) search.set("limit", String(params.limit));
    if (params?.page) search.set("page", String(params.page));
    const query = search.toString();
    return request<ProjectListResponse>(
      `/admin/projects${query ? `?${query}` : ""}`,
    );
  },

  getProject: (id: string) =>
    request<{ project: AdminProjectDetail }>(
      `/admin/projects/${encodeURIComponent(id)}`,
    ),
};

// ── Notification broadcasts (admin view) ────────────────────────────────────

export type BroadcastListResponse = {
  broadcasts: AdminBroadcast[];
  page: number;
  perPage: number;
  total: number;
  totalPages: number;
};

export const notifications = {
  broadcast: (data: {
    title: string;
    content: string;
    notificationType?: string;
    channels?: string[];
    audience: BroadcastAudience;
    scheduledFor?: string;
  }) =>
    request<{ broadcast: { id: string; recipientCount: number; scheduled: boolean } }>(
      "/admin/notifications",
      { method: "POST", body: JSON.stringify(data) },
    ),

  listBroadcasts: (params?: { status?: string; type?: string; limit?: number; page?: number }) => {
    const search = new URLSearchParams();
    if (params?.status) search.set("status", params.status);
    if (params?.type) search.set("type", params.type);
    if (params?.limit) search.set("limit", String(params.limit));
    if (params?.page) search.set("page", String(params.page));
    const query = search.toString();
    return request<BroadcastListResponse>(
      `/admin/notifications${query ? `?${query}` : ""}`,
    );
  },

  getBroadcast: (id: string) =>
    request<{ broadcast: AdminBroadcast }>(
      `/admin/notifications/${encodeURIComponent(id)}`,
    ),

  cancelScheduled: (id: string) =>
    request<{ status: string }>(
      `/admin/notifications/scheduled/${encodeURIComponent(id)}`,
      { method: "DELETE" },
    ),
};

// ── Audit logs (admin view) ─────────────────────────────────────────────────

export type AuditLogListResponse = {
  logs: AuditLogEntry[];
  page: number;
  perPage: number;
  total: number;
  totalPages: number;
};

export const audit = {
  listLogs: (params?: {
    search?: string;
    eventType?: string;
    actorType?: string;
    actorId?: string;
    projectId?: string;
    limit?: number;
    page?: number;
  }) => {
    const search = new URLSearchParams();
    if (params?.search) search.set("search", params.search);
    if (params?.eventType) search.set("eventType", params.eventType);
    if (params?.actorType) search.set("actorType", params.actorType);
    if (params?.actorId) search.set("actorId", params.actorId);
    if (params?.projectId) search.set("projectId", params.projectId);
    if (params?.limit) search.set("limit", String(params.limit));
    if (params?.page) search.set("page", String(params.page));
    const query = search.toString();
    return request<AuditLogListResponse>(
      `/admin/logs${query ? `?${query}` : ""}`,
    );
  },
};

// ── Billing plans (admin view — the live catalog) ───────────────────────────

import type { BillingPlan, PlanSubscriber, SubscriberHistoryPoint } from "./types";
import type { PlanPayload } from "./plans";

export const billing = {
  listPlans: () => request<{ plans: BillingPlan[] }>("/admin/billing/plans"),

  createPlan: (payload: PlanPayload) =>
    request<{ plan: BillingPlan }>("/admin/billing/plans", {
      method: "POST",
      body: JSON.stringify(payload),
    }),

  updatePlan: (id: string, payload: Partial<PlanPayload> & { isActive?: boolean }) =>
    request<{ plan: BillingPlan }>(
      `/admin/billing/plans/${encodeURIComponent(id)}`,
      { method: "PATCH", body: JSON.stringify(payload) },
    ),

  deletePlan: (id: string) =>
    request<{ status: string }>(
      `/admin/billing/plans/${encodeURIComponent(id)}`,
      { method: "DELETE" },
    ),

  listSubscribers: (id: string) =>
    request<{ subscriptions: PlanSubscriber[] }>(
      `/admin/billing/plans/${encodeURIComponent(id)}/subscriptions`,
    ),

  subscriberHistory: (months = 12) =>
    request<{ points: SubscriberHistoryPoint[] }>(
      `/admin/billing/subscribers/history?months=${months}`,
    ),
};
