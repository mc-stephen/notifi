//! Central route registry — every route the API exposes is declared here.
//!
//! Routes are grouped by **surface** ([`PROJECT_ROUTES`] — the external
//! product API under `/v1` — [`APP_ROUTES`] — the user dashboard backend
//! under `/app` — and [`ADMIN_ROUTES`] — platform administration under
//! `/admin`). To add a route: declare it in the matching const, then
//! mount it in the surface's router (`project::router` / `user::app_router` /
//! `user::admin_router`). All parts live in this file, so drift is visible
//! immediately.

use serde::Serialize;


/// Which API surface a route belongs to.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Surface {
    /// External product API under `/v1` (API-key authenticated, M6+).
    Project,
    /// User dashboard backend under `/app` (session-cookie authenticated).
    App,
    /// Platform administration under `/admin` (admin session + 2FA).
    Admin,
    /// Ops/infrastructure endpoints outside all surfaces.
    Core,
}

impl Surface {
    pub fn as_str(self) -> &'static str {
        match self {
            Surface::Project => "project",
            Surface::App => "app",
            Surface::Admin => "admin",
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

/// External product API routes (`/v1`). Empty until M2 lands the first
/// product endpoint; keep this list in lockstep with
/// [`crate::api::project::router`].
pub const PROJECT_ROUTES: &[RouteInfo] = &[];

/// User dashboard backend (`/app`). Mirrors
/// [`crate::api::user::app_router`].
pub const APP_ROUTES: &[RouteInfo] = &[
    // -- auth feature ------------------------------------------------------
    RouteInfo {
        method: "POST",
        path: "/app/auth/signup",
        surface: Surface::App,
        feature: "auth",
        description: "create an account (+ verification token in dev mode)",
    },
    RouteInfo {
        method: "POST",
        path: "/app/auth/login",
        surface: Surface::App,
        feature: "auth",
        description: "sign in; sets the session_token cookie",
    },
    RouteInfo {
        method: "POST",
        path: "/app/auth/logout",
        surface: Surface::App,
        feature: "auth",
        description: "revoke the current session",
    },
    RouteInfo {
        method: "GET",
        path: "/app/auth/me",
        surface: Surface::App,
        feature: "auth",
        description: "current user from the session cookie",
    },
    RouteInfo {
        method: "POST",
        path: "/app/auth/onboarding/complete",
        surface: Surface::App,
        feature: "auth",
        description: "persist the first project (ends onboarding)",
    },
    RouteInfo {
        method: "GET",
        path: "/app/auth/oauth/{provider}",
        surface: Surface::App,
        feature: "auth",
        description: "start OAuth sign-in (302 to the provider consent screen)",
    },
    RouteInfo {
        method: "GET",
        path: "/app/auth/oauth/{provider}/callback",
        surface: Surface::App,
        feature: "auth",
        description: "OAuth redirect target; sets the session cookie",
    },
    RouteInfo {
        method: "POST",
        path: "/app/auth/password/forgot",
        surface: Surface::App,
        feature: "auth",
        description: "request a password reset (always 200)",
    },
    RouteInfo {
        method: "POST",
        path: "/app/auth/password/reset",
        surface: Surface::App,
        feature: "auth",
        description: "reset the password with a one-time token",
    },
    RouteInfo {
        method: "POST",
        path: "/app/auth/verify-email",
        surface: Surface::App,
        feature: "auth",
        description: "confirm the email via one-time token",
    },
    RouteInfo {
        method: "POST",
        path: "/app/auth/verify-email/resend",
        surface: Surface::App,
        feature: "auth",
        description: "re-issue the verification email (always 200)",
    },
    // -- projects feature ----------------------------------------------------
    RouteInfo {
        method: "GET",
        path: "/app/projects",
        surface: Surface::App,
        feature: "projects",
        description: "list projects the user owns or belongs to",
    },
    RouteInfo {
        method: "POST",
        path: "/app/projects",
        surface: Surface::App,
        feature: "projects",
        description: "create a project owned by the user",
    },
    RouteInfo {
        method: "PATCH",
        path: "/app/projects/{id}/environment",
        surface: Surface::App,
        feature: "projects",
        description: "switch the project-level environment gate",
    },
    // -- logs feature -------------------------------------------------------
    RouteInfo {
        method: "GET",
        path: "/app/logs",
        surface: Surface::App,
        feature: "logs",
        description: "audit-log entries the user may see (their own + project)",
    },
    // -- recipients feature ---------------------------------------------------
    RouteInfo {
        method: "POST",
        path: "/app/projects/{id}/recipients",
        surface: Surface::App,
        feature: "recipients",
        description: "create a recipient (brand end-user) in a project",
    },
    RouteInfo {
        method: "GET",
        path: "/app/projects/{id}/recipients",
        surface: Surface::App,
        feature: "recipients",
        description: "list recipients in a project, newest first",
    },
    RouteInfo {
        method: "GET",
        path: "/app/projects/{id}/recipients/{recipient_id}",
        surface: Surface::App,
        feature: "recipients",
        description: "fetch a single recipient",
    },
    RouteInfo {
        method: "PATCH",
        path: "/app/projects/{id}/recipients/{recipient_id}",
        surface: Surface::App,
        feature: "recipients",
        description: "update a recipient's name/contacts",
    },
    RouteInfo {
        method: "DELETE",
        path: "/app/projects/{id}/recipients/{recipient_id}",
        surface: Surface::App,
        feature: "recipients",
        description: "soft-delete a recipient",
    },
    // -- templates feature ----------------------------------------------------
    RouteInfo {
        method: "POST",
        path: "/app/projects/{id}/templates",
        surface: Surface::App,
        feature: "templates",
        description: "create a message template (per-channel content + attachments)",
    },
    RouteInfo {
        method: "GET",
        path: "/app/projects/{id}/templates",
        surface: Surface::App,
        feature: "templates",
        description: "list templates in a project, newest first",
    },
    RouteInfo {
        method: "GET",
        path: "/app/projects/{id}/templates/{template_id}",
        surface: Surface::App,
        feature: "templates",
        description: "fetch a single template with its attachments",
    },
    RouteInfo {
        method: "PATCH",
        path: "/app/projects/{id}/templates/{template_id}",
        surface: Surface::App,
        feature: "templates",
        description: "update a template's content and/or attachments",
    },
    RouteInfo {
        method: "DELETE",
        path: "/app/projects/{id}/templates/{template_id}",
        surface: Surface::App,
        feature: "templates",
        description: "soft-delete a template",
    },
    // -- providers feature ----------------------------------------------------
    RouteInfo {
        method: "GET",
        path: "/app/providers",
        surface: Surface::App,
        feature: "providers",
        description: "full provider registry (all channels + providers + config_fields)",
    },
    // -- channel-configs feature -----------------------------------------------
    RouteInfo {
        method: "GET",
        path: "/app/projects/{id}/channel-configs",
        surface: Surface::App,
        feature: "channel_configs",
        description: "list connected providers for a project",
    },
    RouteInfo {
        method: "POST",
        path: "/app/projects/{id}/channel-configs",
        surface: Surface::App,
        feature: "channel_configs",
        description: "connect a provider (store API keys + SMTP fallback)",
    },
    RouteInfo {
        method: "PATCH",
        path: "/app/projects/{id}/channel-configs/{config_id}",
        surface: Surface::App,
        feature: "channel_configs",
        description: "update a provider's configuration",
    },
    RouteInfo {
        method: "DELETE",
        path: "/app/projects/{id}/channel-configs/{config_id}",
        surface: Surface::App,
        feature: "channel_configs",
        description: "disconnect a provider",
    },
    // -- support feature ----------------------------------------------------
    RouteInfo {
        method: "POST",
        path: "/app/support/tickets",
        surface: Surface::App,
        feature: "support",
        description: "submit a support ticket (personal or project-scoped)",
    },
    RouteInfo {
        method: "GET",
        path: "/app/support/tickets",
        surface: Surface::App,
        feature: "support",
        description: "list tickets visible to the caller (personal + project)",
    },
    RouteInfo {
        method: "GET",
        path: "/app/support/tickets/{ticket_id}",
        surface: Surface::App,
        feature: "support",
        description: "fetch a single support ticket",
    },
    RouteInfo {
        method: "GET",
        path: "/app/support/tickets/{ticket_id}/messages",
        surface: Surface::App,
        feature: "support",
        description: "list the conversation thread for a ticket",
    },
    RouteInfo {
        method: "POST",
        path: "/app/support/tickets/{ticket_id}/messages",
        surface: Surface::App,
        feature: "support",
        description: "send a reply on a ticket",
    },
    // -- notifications feature ------------------------------------------------
    RouteInfo {
        method: "GET",
        path: "/app/notifications",
        surface: Surface::App,
        feature: "notifications",
        description: "list in-app notifications (newest first)",
    },
    RouteInfo {
        method: "GET",
        path: "/app/notifications/count",
        surface: Surface::App,
        feature: "notifications",
        description: "unread notification count for the bell badge",
    },
    RouteInfo {
        method: "GET",
        path: "/app/notifications/{notification_id}",
        surface: Surface::App,
        feature: "notifications",
        description: "fetch a single notification",
    },
    RouteInfo {
        method: "PATCH",
        path: "/app/notifications/{notification_id}/read",
        surface: Surface::App,
        feature: "notifications",
        description: "mark a notification read/unread",
    },
    RouteInfo {
        method: "PATCH",
        path: "/app/notifications/read-all",
        surface: Surface::App,
        feature: "notifications",
        description: "mark all notifications as read",
    },
    RouteInfo {
        method: "DELETE",
        path: "/app/notifications/{notification_id}",
        surface: Surface::App,
        feature: "notifications",
        description: "soft-delete a notification",
    },
];

/// Platform administration (`/admin`). Mirrors
/// [`crate::api::user::admin_router`].
pub const ADMIN_ROUTES: &[RouteInfo] = &[
    // -- admin accounts -----------------------------------------------------
    RouteInfo {
        method: "GET",
        path: "/admin/status",
        surface: Surface::Admin,
        feature: "admin",
        description: "whether any admin account exists (first-run check)",
    },
    RouteInfo {
        method: "POST",
        path: "/admin/bootstrap",
        surface: Surface::Admin,
        feature: "admin",
        description: "create the first admin account (+ TOTP secret)",
    },
    RouteInfo {
        method: "POST",
        path: "/admin/login",
        surface: Surface::Admin,
        feature: "admin",
        description: "sign in; sets the session_token cookie",
    },
    RouteInfo {
        method: "POST",
        path: "/admin/logout",
        surface: Surface::Admin,
        feature: "admin",
        description: "revoke the current admin session",
    },
    RouteInfo {
        method: "GET",
        path: "/admin/me",
        surface: Surface::Admin,
        feature: "admin",
        description: "fetch the authenticated admin (session check)",
    },
    RouteInfo {
        method: "POST",
        path: "/admin/password/forgot",
        surface: Surface::Admin,
        feature: "admin",
        description: "request an admin password-reset token (always 200)",
    },
    RouteInfo {
        method: "POST",
        path: "/admin/password/reset",
        surface: Surface::Admin,
        feature: "admin",
        description: "reset an admin password with a one-time token",
    },
    RouteInfo {
        method: "POST",
        path: "/admin/totp/setup",
        surface: Surface::Admin,
        feature: "admin",
        description: "get-or-create the TOTP secret (?regenerate=true rotates)",
    },
    RouteInfo {
        method: "POST",
        path: "/admin/totp/verify",
        surface: Surface::Admin,
        feature: "admin",
        description: "verify a TOTP code and enable 2FA",
    },
    // -- admin accounts + governance ----------------------------------------
    RouteInfo {
        method: "POST",
        path: "/admin/password/change",
        surface: Surface::Admin,
        feature: "admin",
        description: "change the caller's own password (rotates sessions)",
    },
    RouteInfo {
        method: "GET",
        path: "/admin/admins",
        surface: Surface::Admin,
        feature: "admin",
        description: "list all admin accounts",
    },
    RouteInfo {
        method: "POST",
        path: "/admin/admins",
        surface: Surface::Admin,
        feature: "admin",
        description: "create another admin (pending unless created by super admin)",
    },
    RouteInfo {
        method: "POST",
        path: "/admin/admins/{admin_id}/remove",
        surface: Surface::Admin,
        feature: "admin",
        description: "request (or, for super admin, apply) an admin removal",
    },
    RouteInfo {
        method: "GET",
        path: "/admin/approvals",
        surface: Surface::Admin,
        feature: "admin",
        description: "list approval requests (super admin only)",
    },
    RouteInfo {
        method: "POST",
        path: "/admin/approvals/{request_id}/approve",
        surface: Surface::Admin,
        feature: "admin",
        description: "approve a request (super admin only)",
    },
    RouteInfo {
        method: "POST",
        path: "/admin/approvals/{request_id}/reject",
        surface: Surface::Admin,
        feature: "admin",
        description: "reject a request (super admin only)",
    },
    // -- admin users feature ------------------------------------------------
    RouteInfo {
        method: "GET",
        path: "/admin/users",
        surface: Surface::Admin,
        feature: "users",
        description: "list platform users with search/status filters (admin view)",
    },
    RouteInfo {
        method: "GET",
        path: "/admin/users/{user_id}",
        surface: Surface::Admin,
        feature: "users",
        description: "fetch one platform user with stats (admin view)",
    },
    RouteInfo {
        method: "PATCH",
        path: "/admin/users/{user_id}/status",
        surface: Surface::Admin,
        feature: "users",
        description: "suspend or restore a platform user",
    },
    RouteInfo {
        method: "POST",
        path: "/admin/users/{user_id}/sessions/revoke",
        surface: Surface::Admin,
        feature: "users",
        description: "revoke all sessions for a platform user",
    },
    // -- admin projects feature ---------------------------------------------
    RouteInfo {
        method: "GET",
        path: "/admin/projects",
        surface: Surface::Admin,
        feature: "projects",
        description: "list all platform projects with owner info (admin view)",
    },
    RouteInfo {
        method: "GET",
        path: "/admin/projects/{project_id}",
        surface: Surface::Admin,
        feature: "projects",
        description: "fetch one platform project with owner + members (admin view)",
    },
    // -- admin support feature ----------------------------------------------
    RouteInfo {
        method: "GET",
        path: "/admin/support/tickets",
        surface: Surface::Admin,
        feature: "support",
        description: "list all support tickets (admin view)",
    },
    RouteInfo {
        method: "GET",
        path: "/admin/support/tickets/{ticket_id}",
        surface: Surface::Admin,
        feature: "support",
        description: "fetch a single support ticket (admin view)",
    },
    RouteInfo {
        method: "GET",
        path: "/admin/support/tickets/{ticket_id}/messages",
        surface: Surface::Admin,
        feature: "support",
        description: "list the conversation thread for a ticket (admin view)",
    },
    RouteInfo {
        method: "POST",
        path: "/admin/support/tickets/{ticket_id}/messages",
        surface: Surface::Admin,
        feature: "support",
        description: "send a support reply on a ticket",
    },
    RouteInfo {
        method: "PATCH",
        path: "/admin/support/tickets/{ticket_id}",
        surface: Surface::Admin,
        feature: "support",
        description: "set a ticket's status",
    },
    // -- admin notifications feature ----------------------------------------
    RouteInfo {
        method: "POST",
        path: "/admin/notifications",
        surface: Surface::Admin,
        feature: "notifications",
        description: "broadcast now, or schedule with sendAt",
    },
    RouteInfo {
        method: "GET",
        path: "/admin/notifications",
        surface: Surface::Admin,
        feature: "notifications",
        description: "broadcast history (newest first)",
    },
    RouteInfo {
        method: "GET",
        path: "/admin/notifications/{broadcast_id}",
        surface: Surface::Admin,
        feature: "notifications",
        description: "one broadcast with read stats",
    },
    RouteInfo {
        method: "DELETE",
        path: "/admin/notifications/scheduled/{broadcast_id}",
        surface: Surface::Admin,
        feature: "notifications",
        description: "cancel a scheduled broadcast",
    },
];

/// Ops/infrastructure routes (absolute root, outside all surfaces).
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
        .chain(APP_ROUTES.iter())
        .chain(ADMIN_ROUTES.iter())
        .copied()
        .collect()
}
