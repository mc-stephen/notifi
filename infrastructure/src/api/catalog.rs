//! Central route registry — every route the API exposes is declared here.
//!
//! Routes are grouped by **surface** ([`PROJECT_ROUTES`] — the product API
//! at the root — and [`USER_V1_ROUTES`] — the versioned dashboard API under
//! `/v1`). To add a route: declare it in the matching const, then mount it
//! in the surface's router (`project::router` / `user::v1_router`). Both
//! halves live in this file, so drift is visible immediately.

use serde::Serialize;


/// Which API surface a route belongs to.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Surface {
    /// Product API at the root (API-key authenticated, M6+).
    Project,
    /// Dashboard backend under `/v1` (session-cookie authenticated).
    UserV1,
    /// Ops/infrastructure endpoints outside both surfaces.
    Core,
}

impl Surface {
    pub fn as_str(self) -> &'static str {
        match self {
            Surface::Project => "project",
            Surface::UserV1 => "user_v1",
            Surface::Core => "core",
        }
    }
}

/// One exposed route, as listed in the catalog.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct RouteInfo {
    pub method: &'static str,
    pub path: &'static str,
    pub surface: Surface,
    /// Owning feature: `core` for ops routes, or the domain name
    /// (`auth`, `notifications`, ...) for feature routes.
    pub feature: &'static str,
    pub description: &'static str,
}

/// Product API routes (root). Empty until M2 lands the first product
/// endpoint; keep this list in lockstep with [`crate::api::project::router`].
pub const PROJECT_ROUTES: &[RouteInfo] = &[];

/// User/dashboard API routes (`/v1`). Mirrors [`crate::api::user::v1_router`].
pub const USER_V1_ROUTES: &[RouteInfo] = &[
    // -- auth feature ------------------------------------------------------
    RouteInfo {
        method: "POST",
        path: "/v1/auth/signup",
        surface: Surface::UserV1,
        feature: "auth",
        description: "create an account (+ verification token in dev mode)",
    },
    RouteInfo {
        method: "POST",
        path: "/v1/auth/login",
        surface: Surface::UserV1,
        feature: "auth",
        description: "sign in; sets the session_token cookie",
    },
    RouteInfo {
        method: "POST",
        path: "/v1/auth/logout",
        surface: Surface::UserV1,
        feature: "auth",
        description: "revoke the current session",
    },
    RouteInfo {
        method: "GET",
        path: "/v1/auth/me",
        surface: Surface::UserV1,
        feature: "auth",
        description: "current user from the session cookie",
    },
    RouteInfo {
        method: "POST",
        path: "/v1/auth/onboarding/complete",
        surface: Surface::UserV1,
        feature: "auth",
        description: "persist the first project (ends onboarding)",
    },
    RouteInfo {
        method: "GET",
        path: "/v1/auth/oauth/{provider}",
        surface: Surface::UserV1,
        feature: "auth",
        description: "start OAuth sign-in (302 to the provider consent screen)",
    },
    RouteInfo {
        method: "GET",
        path: "/v1/auth/oauth/{provider}/callback",
        surface: Surface::UserV1,
        feature: "auth",
        description: "OAuth redirect target; sets the session cookie",
    },
    RouteInfo {
        method: "POST",
        path: "/v1/auth/password/forgot",
        surface: Surface::UserV1,
        feature: "auth",
        description: "request a password reset (always 200)",
    },
    RouteInfo {
        method: "POST",
        path: "/v1/auth/password/reset",
        surface: Surface::UserV1,
        feature: "auth",
        description: "reset the password with a one-time token",
    },
    RouteInfo {
        method: "POST",
        path: "/v1/auth/verify-email",
        surface: Surface::UserV1,
        feature: "auth",
        description: "confirm the email via one-time token",
    },
    RouteInfo {
        method: "POST",
        path: "/v1/auth/verify-email/resend",
        surface: Surface::UserV1,
        feature: "auth",
        description: "re-issue the verification email (always 200)",
    },
];

/// Ops/infrastructure routes (absolute root, outside both surfaces).
pub const CORE_ROUTES: &[RouteInfo] = &[
    RouteInfo {
        method: "GET",
        path: "/",
        surface: Surface::Core,
        feature: "core",
        description: "service identification",
    },
    RouteInfo {
        method: "GET",
        path: "/healthz",
        surface: Surface::Core,
        feature: "core",
        description: "liveness",
    },
    RouteInfo {
        method: "GET",
        path: "/readyz",
        surface: Surface::Core,
        feature: "core",
        description: "readiness",
    },
    RouteInfo {
        method: "GET",
        path: "/routes",
        surface: Surface::Core,
        feature: "core",
        description: "this route catalog",
    },
];

/// Every route across all surfaces, for `GET /routes`.
pub fn all_routes() -> Vec<RouteInfo> {
    CORE_ROUTES
        .iter()
        .chain(PROJECT_ROUTES.iter())
        .chain(USER_V1_ROUTES.iter())
        .copied()
        .collect()
}
