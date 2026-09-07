import { defineMiddleware } from "astro:middleware";

const API_BASE = import.meta.env.PUBLIC_API_URL ?? "http://localhost:8080";

const PROTECTED = [
  "/",
  "/overview",
  "/users",
  "/organizations",
  "/support",
  "/notifications",
  "/billing",
  "/platform",
  "/security",
  "/administration",
];

async function adminExists(): Promise<boolean> {
  try {
    const res = await fetch(`${API_BASE}/v1/admin/status`, {
      headers: { "Content-Type": "application/json" },
    });
    if (!res.ok) return true;
    const data = await res.json();
    return data.adminExists ?? true;
  } catch {
    return true;
  }
}

export const onRequest = defineMiddleware(async (context, next) => {
  const path = new URL(context.url).pathname;
  const session = context.cookies.get("session_token")?.value;

  // Already logged in: redirect away from auth pages
  if (path === "/login" || path === "/auth/setup") {
    if (session) return context.redirect("/overview");
  }

  // No session: determine where to send the user
  if (!session) {
    const hasAdmin = await adminExists();

    // /auth/setup: only valid when no admin exists
    if (path === "/auth/setup") {
      if (hasAdmin) return context.redirect("/login");
      return next();
    }

    // Everything else (protected pages + /login + /): route based on admin status
    if (!hasAdmin) return context.redirect("/auth/setup");
    if (path === "/login") return next();
    return context.redirect("/login");
  }

  return next();
});
