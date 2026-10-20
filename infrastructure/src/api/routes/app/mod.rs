//! User dashboard backend (`/app`). New dashboard features mount
//! inside [`app_router`]. Platform administration lives in
//! [`crate::api::routes::admin`].

pub mod api_keys;
pub mod auth;
pub mod billing;
pub mod campaigns;
pub mod channel_configs;
pub mod flows;
pub mod invites;
pub mod logs;
pub mod notifications;
pub mod projects;
pub mod providers;
pub mod recipients;
pub mod schedules;
pub mod sends;
pub mod support;
pub mod templates;
pub mod webhooks;

use axum::Router;

use crate::api::state::AppState;

/// The `/app` router: user-dashboard backend (auth, projects, support,
/// notifications, logs). Nests feature routers under their path segments.
pub fn app_router(state: &AppState) -> Router<AppState> {
    // Auth routes pull their service from a scoped Extension; without it
    // (no database) they answer 503 problem documents.
    let auth_routes = match state.auth.clone() {
        Some(service) => auth::routes::router().layer(axum::Extension(service)),
        None => auth::routes::router(),
    };
    // The OAuth runtime is optional independently of the service; without
    // it (no provider credentials) the /oauth routes answer 503. Layered
    // conditionally so the extension carries the bare `Arc<OAuthRuntime>`
    // the handlers extract — NOT `Extension<Option<Arc<...>>>`, which would
    // silently never match.
    let auth_routes = match state.oauth.clone() {
        Some(runtime) => auth_routes.layer(axum::Extension(runtime)),
        None => auth_routes,
    };

    let mut project_routes = projects::routes::router();
    // Project routes require session auth via `CurrentUser`, which needs the
    // auth service extension.
    if let Some(auth_service) = state.auth.clone() {
        project_routes = project_routes.layer(axum::Extension(auth_service));
    }
    if let Some(service) = state.projects.clone() {
        project_routes = project_routes.layer(axum::Extension(service));
    }
    if let Some(service) = state.project_members.clone() {
        project_routes = project_routes.layer(axum::Extension(service));
    }

    let mut log_routes = logs::routes::router();
    if let Some(auth_service) = state.auth.clone() {
        log_routes = log_routes.layer(axum::Extension(auth_service));
    }
    if let Some(service) = state.audit.clone() {
        log_routes = log_routes.layer(axum::Extension(service));
    }

    let mut recipient_routes = recipients::routes::router();
    if let Some(auth_service) = state.auth.clone() {
        recipient_routes = recipient_routes.layer(axum::Extension(auth_service));
    }
    if let Some(service) = state.recipients.clone() {
        recipient_routes = recipient_routes.layer(axum::Extension(service));
    }

    let mut template_routes = templates::routes::router();
    if let Some(auth_service) = state.auth.clone() {
        template_routes = template_routes.layer(axum::Extension(auth_service));
    }
    if let Some(service) = state.templates.clone() {
        template_routes = template_routes.layer(axum::Extension(service));
    }

    let provider_routes = providers::routes::routes();

    let mut channel_config_routes = channel_configs::routes::routes();
    if let Some(auth_service) = state.auth.clone() {
        channel_config_routes = channel_config_routes.layer(axum::Extension(auth_service));
    }
    if let Some(store) = state.channel_providers.clone() {
        channel_config_routes = channel_config_routes.layer(axum::Extension(store));
    }
    if let Some(svc) = state.notifications.clone() {
        channel_config_routes = channel_config_routes.layer(axum::Extension(svc));
    }
    channel_config_routes =
        channel_config_routes.layer(axum::Extension(state.provider_tester.clone()));

    let mut support_routes = support::routes::router();
    if let Some(auth_service) = state.auth.clone() {
        support_routes = support_routes.layer(axum::Extension(auth_service));
    }
    if let Some(service) = state.tickets.clone() {
        support_routes = support_routes.layer(axum::Extension(service));
    }

    let mut notification_routes = notifications::routes::router();
    if let Some(auth_service) = state.auth.clone() {
        notification_routes = notification_routes.layer(axum::Extension(auth_service));
    }
    if let Some(service) = state.notifications.clone() {
        notification_routes = notification_routes.layer(axum::Extension(service));
    }

    let mut billing_routes = billing::routes::router();
    if let Some(auth_service) = state.auth.clone() {
        billing_routes = billing_routes.layer(axum::Extension(auth_service));
    }
    if let Some(service) = state.billing.clone() {
        billing_routes = billing_routes.layer(axum::Extension(service));
    }

    let mut flows_routes = flows::routes::router();
    if let Some(auth_service) = state.auth.clone() {
        flows_routes = flows_routes.layer(axum::Extension(auth_service));
    }
    if let Some(service) = state.flows.clone() {
        flows_routes = flows_routes.layer(axum::Extension(service));
    }
    if let Some(service) = state.schedules.clone() {
        flows_routes = flows_routes.layer(axum::Extension(service));
    }
    if let Some(service) = state.inbound.clone() {
        flows_routes = flows_routes.layer(axum::Extension(service));
    }
    if let Some(service) = state.webhooks.clone() {
        flows_routes = flows_routes.layer(axum::Extension(service));
    }

    let mut sends_routes = sends::routes::router();
    if let Some(auth_service) = state.auth.clone() {
        sends_routes = sends_routes.layer(axum::Extension(auth_service));
    }
    if let Some(service) = state.sends.clone() {
        sends_routes = sends_routes.layer(axum::Extension(service));
    }

    let mut schedules_routes = schedules::routes::router();
    if let Some(auth_service) = state.auth.clone() {
        schedules_routes = schedules_routes.layer(axum::Extension(auth_service));
    }
    if let Some(service) = state.schedules.clone() {
        schedules_routes = schedules_routes.layer(axum::Extension(service));
    }

    let mut webhook_routes = webhooks::routes::router();
    if let Some(auth_service) = state.auth.clone() {
        webhook_routes = webhook_routes.layer(axum::Extension(auth_service));
    }
    if let Some(service) = state.webhooks.clone() {
        webhook_routes = webhook_routes.layer(axum::Extension(service));
    }

    let mut campaign_routes = campaigns::routes::router();
    if let Some(auth_service) = state.auth.clone() {
        campaign_routes = campaign_routes.layer(axum::Extension(auth_service));
    }
    if let Some(service) = state.campaigns.clone() {
        campaign_routes = campaign_routes.layer(axum::Extension(service));
    }

    let mut invite_routes = invites::routes::router();
    if let Some(auth_service) = state.auth.clone() {
        invite_routes = invite_routes.layer(axum::Extension(auth_service));
    }
    if let Some(service) = state.project_members.clone() {
        invite_routes = invite_routes.layer(axum::Extension(service));
    }

    let mut api_key_routes = api_keys::routes::router();
    if let Some(auth_service) = state.auth.clone() {
        api_key_routes = api_key_routes.layer(axum::Extension(auth_service));
    }
    if let Some(service) = state.api_keys.clone() {
        api_key_routes = api_key_routes.layer(axum::Extension(service));
    }

    Router::new()
        .nest("/auth", auth_routes)
        .nest("/providers", provider_routes)
        .nest("/projects", project_routes)
        .nest("/projects/{project_id}/recipients", recipient_routes)
        .nest("/projects/{project_id}/templates", template_routes)
        .nest("/projects/{project_id}/api-keys", api_key_routes)
        .nest(
            "/projects/{project_id}/channel-configs",
            channel_config_routes,
        )
        .nest("/support", support_routes)
        .nest("/notifications", notification_routes)
        .nest("/billing", billing_routes)
        .nest("/flows", flows_routes)
        .nest("/sends", sends_routes)
        .nest("/schedules", schedules_routes)
        .nest("/webhooks", webhook_routes)
        .nest("/campaigns", campaign_routes)
        .nest("/invites", invite_routes)
        .nest("/logs", log_routes)
}
