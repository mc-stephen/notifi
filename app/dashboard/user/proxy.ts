import { NextResponse } from "next/server";
import type { NextRequest } from "next/server";

// Paths that don't require authentication
const PUBLIC_PATHS = ["/auth", "/onboarding"];

// Paths that should always be accessible
const EXEMPT_PATHS = ["/api", "/_next", "/favicon.ico"];

// Onboarding is only reachable mid-flow (valid session, project missing),
// so unlike other public paths its cookie is *validated*, not just read.
const VALIDATED_PATHS = ["/onboarding"];

const API_BASE =
  process.env.NEXT_PUBLIC_API_URL ?? "https://api.notifi.dev";

function loginRedirect(request: NextRequest, pathname: string) {
  const loginUrl = new URL("/auth/login", request.url);
  loginUrl.searchParams.set("next", pathname);
  return NextResponse.redirect(loginUrl);
}

/// Validates the session cookie against the API (`GET /app/auth/me`).
/// Explicitly invalid (401/403) → false. Network/API failure → true
/// (fail-open: a down API shouldn't lock users out; the client layout
/// guard still bounces them once it can reach the API).
async function hasValidSession(request: NextRequest): Promise<boolean> {
  const cookie = request.headers.get("cookie");
  if (!cookie) return false;
  try {
    const res = await fetch(`${API_BASE}/app/auth/me`, {
      headers: { cookie },
      signal: AbortSignal.timeout(3000),
    });
    return res.ok;
  } catch {
    return true;
  }
}

export async function proxy(request: NextRequest) {
  const { pathname } = request.nextUrl;

  // Skip middleware for exempt paths
  if (EXEMPT_PATHS.some((path) => pathname.startsWith(path))) {
    return NextResponse.next();
  }

  // Check if path is public (auth/onboarding)
  const isPublicPath = PUBLIC_PATHS.some((path) => pathname.startsWith(path));

  // Check for session token (mock cookie)
  const sessionToken = request.cookies.get("session_token")?.value;

  const isAuthenticated = !!sessionToken;

  // If not authenticated and trying to access protected route.
  // `next` matches the login/signup return contract (see auth pages).
  if (!isAuthenticated && !isPublicPath) {
    return loginRedirect(request, pathname);
  }

  // Onboarding without a *valid* session goes back to login — the flow
  // is only entered with a session behind it.
  if (VALIDATED_PATHS.some((path) => pathname.startsWith(path))) {
    if (!(await hasValidSession(request))) {
      return loginRedirect(request, pathname);
    }
  }

  // If authenticated and trying to access auth pages, redirect to dashboard
  if (isAuthenticated && pathname.startsWith("/auth")) {
    // Allow signup, password/forgot, password/reset, verify-email
    const allowedAuthPaths = [
      "/auth/signup",
      "/auth/password/forgot",
      "/auth/password/reset",
      "/auth/verify-email",
    ];

    if (!allowedAuthPaths.some((path) => pathname.startsWith(path))) {
      return NextResponse.redirect(new URL("/", request.url));
    }
  }

  return NextResponse.next();
}

export const config = {
  matcher: [
    /*
     * Match all request paths except:
     * - api (API routes)
     * - _next/static (static files)
     * - _next/image (image optimization files)
     * - favicon.ico (favicon file)
     */
    "/((?!api|_next/static|_next/image|favicon.ico).*)",
  ],
};
