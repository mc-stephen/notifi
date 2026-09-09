import { defineMiddleware } from "astro:middleware";

const API_BASE = import.meta.env.PUBLIC_API_URL ?? "http://localhost:8080";

// Auth pages reachable without a session: login, recovery, and the 2FA
// code step (which carries its own pending credentials).
const PUBLIC_AUTH_PAGES = ["/login", "/password/forgot", "/password/reset", "/auth/verify"];

async function adminExists(): Promise<boolean> {
  try {
    const res = await fetch(`${API_BASE}/admin/status`, {
      headers: { "Content-Type": "application/json" },
    });
    if (!res.ok) return true;
    const data = await res.json();
    return data.adminExists ?? true;
  } catch {
    return true;
  }
}

type SessionState = { valid: true; totpEnabled: boolean } | { valid: false } | { valid: null };

// Verifies a session cookie against the backend. `valid: null` means the
// backend was unreachable — callers fail open so a down API doesn't lock
// anyone out; pages show their own errors in that case.
async function checkSession(session: string): Promise<SessionState> {
  try {
    const res = await fetch(`${API_BASE}/admin/me`, {
      headers: { Cookie: `admin_session=${session}` },
    });
    if (!res.ok) return { valid: false };
    const data = await res.json();
    return { valid: true, totpEnabled: data.totpEnabled ?? true };
  } catch {
    return { valid: null };
  }
}

export const onRequest = defineMiddleware(async (context, next) => {
  const path = new URL(context.url).pathname;
  let session = context.cookies.get("admin_session")?.value;
  let totpEnabled = true;

  if (session) {
    const state = await checkSession(session);
    if (state.valid === false) {
      // Drop dead sessions so stale cookies redirect to login instead of
      // rendering pages whose API calls then fail.
      context.cookies.delete("admin_session", { path: "/" });
      session = undefined;
    } else if (state.valid === true) {
      totpEnabled = state.totpEnabled;
    }
  }

  // Logged in: auth pages bounce to the app (2FA gate below may reroute).
  if (
    session &&
    (path === "/login" ||
      path === "/auth/setup" ||
      path === "/auth/verify" ||
      path.startsWith("/password/"))
  ) {
    return context.redirect("/overview");
  }

  // 2FA gate: an unverified session may only finish setup (or sign out via
  // the API + button on that page). Already-verified admins have no business
  // on the setup page either.
  if (session && !totpEnabled && path !== "/auth/totp") {
    return context.redirect("/auth/totp");
  }
  if (session && totpEnabled && path === "/auth/totp") {
    return context.redirect("/overview");
  }

  // No session: determine where to send the user
  if (!session) {
    const hasAdmin = await adminExists();

    // /auth/setup: only valid when no admin exists
    if (path === "/auth/setup") {
      if (hasAdmin) return context.redirect("/login");
      return next();
    }

    // Everything else (protected pages + public auth pages + /): route
    // based on admin status
    if (!hasAdmin) return context.redirect("/auth/setup");
    if (PUBLIC_AUTH_PAGES.includes(path)) return next();
    return context.redirect("/login");
  }

  return next();
});
