/**
 * Typed fetch client for the Notifi API (Rust backend).
 *
 * - Base URL comes from NEXT_PUBLIC_API_URL (see lib/env.ts, .env.local).
 * - credentials: "include" so the httpOnly session_token cookie flows
 *   between dashboard and API.
 * - Errors are thrown as ApiError built from RFC 9457 problem documents
 *   ({ type, title, status, detail, correlation_id }) — read `.message`.
 *
 * Usage:
 *   const { user, session } = await api<LoginResponse>("/v1/auth/login", {
 *     method: "POST",
 *     body: JSON.stringify({ email, password, rememberMe }),
 *   });
 */

import { env } from "./env";

export class ApiError extends Error {
  readonly status: number;
  readonly correlationId?: string;

  constructor(status: number, message: string, correlationId?: string) {
    super(message);
    this.name = "ApiError";
    this.status = status;
    this.correlationId = correlationId;
  }
}

type ProblemDocument = {
  title?: string;
  detail?: string;
  correlation_id?: string | null;
};

export async function api<T>(path: string, init?: RequestInit): Promise<T> {
  const res = await fetch(`${env.apiBase}${path}`, {
    credentials: "include",
    ...init,
    headers: { "content-type": "application/json", ...init?.headers },
  });

  if (!res.ok) {
    let problem: ProblemDocument = {};
    try {
      problem = await res.json();
    } catch {
      // non-JSON error body — fall back to status text
    }
    throw new ApiError(
      res.status,
      problem.detail ?? problem.title ?? res.statusText,
      problem.correlation_id ?? undefined
    );
  }

  if (res.status === 204) return undefined as T;
  const text = await res.text();
  return text ? JSON.parse(text) : (undefined as T);
}
