import { NextResponse } from "next/server";
import type { NextRequest } from "next/server";

// Paths that don't require authentication
const PUBLIC_PATHS = ["/auth", "/onboarding"];

// Paths that should always be accessible
const EXEMPT_PATHS = ["/api", "/_next", "/favicon.ico"];

export function proxy(request: NextRequest) {
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

  // If not authenticated and trying to access protected route
  if (!isAuthenticated && !isPublicPath) {
    const loginUrl = new URL("/auth/login", request.url);
    loginUrl.searchParams.set("redirect", pathname);
    return NextResponse.redirect(loginUrl);
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
