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

export type LoginResponse = {
  totpRequired?: boolean;
  user: { id: string; name: string; email: string; role: string; avatar?: string };
  session: { token: string; expiresAt: string; onboardingCompleted: boolean };
};

export type AdminStatusResponse = {
  adminExists: boolean;
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
  adminStatus: () => request<AdminStatusResponse>("/v1/admin/status"),

  bootstrap: (data: { name: string; email: string; password: string }) =>
    request<BootstrapResponse>("/v1/admin/bootstrap", {
      method: "POST",
      body: JSON.stringify(data),
    }),

  login: (data: { email: string; password: string; rememberMe?: boolean }) =>
    request<LoginResponse>("/v1/auth/login", {
      method: "POST",
      body: JSON.stringify(data),
    }),

  totpSetup: () =>
    request<TotpSetupResponse>("/v1/admin/totp/setup", { method: "POST" }),

  totpVerify: (code: string) =>
    request<{ status: string }>("/v1/admin/totp/verify", {
      method: "POST",
      body: JSON.stringify({ code }),
    }),

  me: () =>
    request<{ user: LoginResponse["user"]; onboardingCompleted: boolean }>(
      "/v1/auth/me",
    ),

  logout: () =>
    request<{ status: string }>("/v1/admin/logout", { method: "POST" }),
};

// ── Support tickets (admin view) ────────────────────────────────────────────

import type { AdminTicket, AdminTicketMessage, AdminTicketStatus } from "./types";

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
      `/v1/admin/support/tickets${query ? `?${query}` : ""}`,
    );
  },

  getTicket: (id: string) =>
    request<{ ticket: AdminTicket }>(
      `/v1/admin/support/tickets/${encodeURIComponent(id)}`,
    ),

  listMessages: (id: string) =>
    request<{ messages: AdminTicketMessage[] }>(
      `/v1/admin/support/tickets/${encodeURIComponent(id)}/messages`,
    ),

  sendReply: (id: string, body: string) =>
    request<{ message: AdminTicketMessage }>(
      `/v1/admin/support/tickets/${encodeURIComponent(id)}/messages`,
      { method: "POST", body: JSON.stringify({ body }) },
    ),

  setStatus: (id: string, status: AdminTicketStatus) =>
    request<{ ticket: AdminTicket }>(
      `/v1/admin/support/tickets/${encodeURIComponent(id)}`,
      { method: "PATCH", body: JSON.stringify({ status }) },
    ),
};
