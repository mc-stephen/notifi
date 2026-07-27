import { NextResponse } from "next/server";
import type { NextRequest } from "next/server";

// Paths that don't require authentication
const PUBLIC_PATHS = ["/auth", "/onboarding"];

// Paths that should always be accessible
const EXEMPT_PATHS = ["/api", "/_next", "/favicon.ico"];

export function middleware(request: NextRequest) {
  const { pathname } = request.nextUrl;

  // Skip middleware for exempt paths
  if (EXEMPT_PATHS.some((path) => pathname.startsWith(path))) {
    return NextResponse.next();
  }

  // Check if path is public (auth/onboarding)
  const isPublicPath = PUBLIC_PATHS.some((path) => pathname.startsWith(path));

  // Check for session token (mock cookie for now)
  const sessionToken = request.cookies.get("session_token")?.value;

  // For now (mock data), always set a session cookie and allow access
  // In production, this would check the actual session
  const isAuthenticated = !!sessionToken || true; // Always true for mock

  // If not authenticated and trying to access protected route
  if (!isAuthenticated && !isPublicPath) {
    const loginUrl = new URL("/auth/login", request.url);
    loginUrl.searchParams.set("redirect", pathname);
    return NextResponse.redirect(loginUrl);
  }

  // If authenticated and trying to access auth pages, redirect to dashboard
  if (isAuthenticated && isPublicPath && pathname !== "/auth/login") {
    // Allow signup, forgot-password, reset-password, verify-email
    const allowedAuthPaths = [
      "/auth/signup",
      "/auth/forgot-password",
      "/auth/reset-password",
      "/auth/verify-email",
    ];

    if (!allowedAuthPaths.some((path) => pathname.startsWith(path))) {
      return NextResponse.redirect(new URL("/", request.url));
    }
  }

  // Set mock session cookie for development
  const response = NextResponse.next();
  if (!sessionToken) {
    response.cookies.set("session_token", "mock_session_token", {
      httpOnly: true,
      secure: process.env.NODE_ENV === "production",
      sameSite: "lax",
      maxAge: 7 * 24 * 60 * 60, // 7 days
      path: "/",
    });
  }

  return response;
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
