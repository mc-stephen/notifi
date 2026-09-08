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

  logout: () =>
    request<{ status: string }>("/admin/logout", { method: "POST" }),
};

// ── Support tickets (admin view) ────────────────────────────────────────────

import type { AdminTicket, AdminTicketMessage, AdminTicketStatus } from "./types";
import type { AdminUser, AdminUserDetail, AdminUserStatus } from "./types";

export type TicketListResponse = {
  tickets: AdminTicket[];
  hasMore: boolean;
};

export const support = {
  listTickets: (params?: { status?: string; limit?: number; before?: string }) => {
    const search = new URLSearchParams();
    if (params?.status) search.set("status", params.status);
    if (params?.limit) search.set("limit", String(params.limit));
    if (params?.before) search.set("before", params.before);
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
  hasMore: boolean;
};

export const users = {
  listUsers: (params?: { search?: string; status?: string; limit?: number; before?: string }) => {
    const search = new URLSearchParams();
    if (params?.search) search.set("search", params.search);
    if (params?.status) search.set("status", params.status);
    if (params?.limit) search.set("limit", String(params.limit));
    if (params?.before) search.set("before", params.before);
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
