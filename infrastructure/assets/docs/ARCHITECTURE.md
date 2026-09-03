# Notifi — Architecture

> Living document — kept in sync with the codebase. Every section references
> concrete file paths so any model (or human) can verify the claims.

---

## Table of Contents

1. [Workspace layout](#1-workspace-layout)
2. [Crate map](#2-crate-map)
3. [Module boundaries (single crate)](#3-module-boundaries-single-crate)
4. [API surfaces](#4-api-surfaces)
5. [Domain layer](#5-domain-layer)
6. [Ports layer](#6-ports-layer)
7. [Infrastructure layer](#7-infrastructure-layer)
8. [Presentation layer](#8-presentation-layer)
9. [Adding a new feature](#9-adding-a-new-feature)
10. [Error model](#10-error-model)
11. [Auth model](#11-auth-model)
12. [Testing strategy](#12-testing-strategy)
13. [Observability](#13-observability)
14. [Event-driven patterns](#14-event-driven-patterns)
15. [Shared kernel crates](#15-shared-kernel-crates)
16. [Channel plugin crates](#16-channel-plugin-crates)
17. [Configuration](#17-configuration)
18. [Runtime configuration](#18-runtime-configuration)
19. [Migration Strategy](#19-migration-strategy)
20. [Evolving Into Microservices](#20-evolving-into-microservices)
21. [Provider Registry](#21-provider-registry)
22. [Milestone Roadmap](#22-milestone-roadmap)

---

## 1. Workspace layout

```
notifi/                         # repo root
├── app/dashboard/              # Next.js 16 + React 19 frontend
├── infrastructure/             # Rust server (single crate + channel plugins)
│   ├── src/                    # server crate source
│   │   ├── api/                # HTTP presentation (axum handlers + routes)
│   │   ├── domain/             # Framework-free business logic
│   │   ├── ports/              # Trait contracts (AuthStore, etc.)
│   │   ├── infra/              # Concrete drivers (sqlx, reqwest, config, telemetry)
│   │   └── testing/            # Test utilities
│   ├── channels/               # Channel plugin crates
│   │   ├── email_channel/      # Email providers (SMTP, SendGrid, Resend, etc.)
│   │   ├── sms_channel/        # SMS providers (Twilio, Termii, etc.)
│   │   ├── push_channel/       # Push providers (FCM, APNS, OneSignal, etc.)
│   │   └── chat_channel/       # Chat providers (Slack, Telegram, etc.)
│   ├── crates/                 # Shared kernel crates
│   │   ├── core/               # Errors, ULID ids, events, outbox, config resolver
│   │   └── domain-ports/       # Trait-only contracts
│   └── assets/                 # Migrations, docs, configs
│       ├── docs/               # Architecture, commands, contracts
│       └── migrations/         # SQL migrations (0001_initial_schema.sql)
└── packages/                   # Shared tooling
```

## 2. Crate map

| Crate | Path | Purpose |
|-------|------|---------|
| `server` | `infrastructure/` | Main binary + lib — API, domain, infra |
| `notifi_core` | `crates/core/` | Shared kernel: errors, ULID, events, outbox |
| `domain-ports` | `crates/domain-ports/` | Trait-only contracts |
| `email_channel` | `channels/email_channel/` | Email providers |
| `sms_channel` | `channels/sms_channel/` | SMS providers |
| `push_channel` | `channels/push_channel/` | Push providers |
| `chat_channel` | `channels/chat_channel/` | Chat providers |

## 3. Module boundaries (single crate)

The server crate enforces layering through visibility:

| Module | Allowed dependencies | Purpose |
|--------|---------------------|---------|
| `api/` | axum, serde, domain, ports, infra | HTTP presentation only |
| `domain/` | none (framework-free) | Business logic, entities, value objects |
| `ports/` | domain only | Trait contracts (AuthStore, etc.) |
| `infra/` | sqlx, reqwest, domain, ports | Concrete driver implementations |

**Rule**: Keep axum/sqlx/reqwest out of `domain/` and `ports/`.

## 4. API surfaces

Two independent API surfaces:

| Surface | Route prefix | Auth | Purpose |
|---------|-------------|------|---------|
| **User API** | `/v1/*` | Session cookie (`CurrentUser`) | Dashboard backend |
| **Project API** | `/*` | API key (from M6) | Product API |

User API routes live in `src/api/user/`:
- `/v1/auth/*` — signup, login, logout, verify-email, password reset
- `/v1/providers` — provider registry (GET)
- `/v1/projects/{id}/channel-configs` — channel configs CRUD
- `/v1/projects/{id}/recipients` — recipients CRUD
- `/v1/projects/{id}/templates` — templates CRUD

## 5. Domain layer

Framework-free business logic. Entities, value objects, aggregate roots.

| Module | Contents |
|--------|----------|
| `domain/channels/` | Provider registry types (ProviderRegistry, ChannelDefinition, ProviderDefinition, ConfigField) |
| `domain/notifications/` | Notification aggregate, state machine |
| `domain/recipients/` | Recipient aggregate |
| `domain/templates/` | Template aggregate |

## 6. Ports layer

Trait contracts implemented by infrastructure:

| Trait | File | Purpose |
|-------|------|---------|
| `AuthStore` | `ports/auth_store.rs` | User/session persistence |
| `ChannelProviderStore` | `ports/channel_provider_store.rs` | Project provider config CRUD |

## 7. Infrastructure layer

Concrete driver implementations:

| Module | Purpose |
|--------|---------|
| `infra/config.rs` | `AppConfig` from env vars |
| `infra/db.rs` | Postgres pool, migrations, reset |
| `infra/redis.rs` | Redis connection |
| `infra/telemetry.rs` | Tracing, metrics |
| `infra/auth_repository_pg.rs` | AuthStore Postgres impl |
| `infra/channel_provider_repository_pg.rs` | ChannelProviderStore Postgres impl |

## 8. Presentation layer

Axum handlers and routes organized by API surface:

```
src/api/
├── mod.rs              # Shared middleware, error mapping
├── state.rs            # AppState (db, redis, auth, channel_providers)
├── catalog.rs          # GET /routes — route registry
├── user/               # User API (/v1/*)
│   ├── mod.rs          # v1_router()
│   ├── auth/           # /v1/auth/*
│   ├── providers/      # /v1/providers
│   └── channel_configs/ # /v1/projects/{id}/channel-configs
└── project/            # Project API (root, M2+)
```

## 9. Adding a new feature

1. Domain: add entities/aggregate in `src/domain/`
2. Ports: add trait in `src/ports/` (framework-free)
3. Infrastructure: implement trait in `src/infra/`
4. Presentation: add handlers + routes in `src/api/`
5. Wire into `src/api/state.rs` (AppState) and `src/api/catalog.rs`
6. Add integration tests in `tests/`

## 10. Error model

All errors map to RFC 9457 Problem Details. See `src/api/mod.rs`.

## 11. Auth model

Session-cookie based. See `src/api/user/auth/` and `src/ports/auth_store.rs`.

## 12. Testing strategy

- Unit tests in each module
- Integration tests in `tests/` (auth_api, recipients_api, templates_api)
- All tests run against SQLite in-memory (sqlx)
- CI gate: `cargo test --workspace --exclude web_channel`

## 13. Observability

- Tracing with correlation IDs
- `x-request-id` header on every response
- `/healthz`, `/readyz`, `/metrics` endpoints

## 14. Event-driven patterns

Outbox pattern for reliable event publishing. See `domain/` and `infra/`.

## 15. Shared kernel crates

- `crates/core`: errors, ULID, events, outbox, config resolver — framework-free
- `crates/domain-ports`: trait-only contracts — framework-free

## 16. Channel plugin crates

Each channel crate:
- Lives in `channels/{name}_channel/`
- Defines provider config structs
- Has a provider enum in `providers/mod.rs`
- Implements the channel's delivery trait

## 17. Configuration

Env vars with `NOTIFI_` prefix. See `src/infra/config.rs`.

## 18. Runtime configuration

Configs load dynamically from `{NOTIFI_CONFIG_ROOT}/brands/{brand}/config/{channel_name}/`.

## 19. Migration Strategy

- Single `assets/migrations/` set, numbered `0001_xxx.sql`, run at startup and via CLI
- Forward-only migrations; destructive changes done as expand → migrate → contract → collapse
- Current state: single `0001_initial_schema.sql` (consolidated dev-stage squash)

## 20. Evolving Into Microservices

Each domain is a crate with its own aggregate, outbox, schema namespace, event contracts, and README — extraction is mechanical.

## 21. Provider Registry

### Pattern

Each channel (email, sms, push, chat) has a `providers/` directory with one
file per provider. Each file defines a config struct. The channel's
`providers/mod.rs` enum lists all providers — this is the source of truth.

### Architecture

```
channels/email_channel/src/
├── providers/
│   ├── mod.rs           # EmailProvider enum + EmailChannel definition
│   ├── smtp.rs          # SmtpConfig struct
│   ├── sendgrid.rs      # SendGridConfig struct
│   ├── resend.rs        # ResendConfig struct
│   ├── aws_ses.rs       # AwsSesConfig struct
│   ├── postmark.rs      # PostmarkConfig struct
│   ├── mailgun.rs       # MailgunConfig struct
│   └── brevo.rs         # BrevoConfig struct
└── lib.rs               # Channel implementation
```

### Adding a new provider

1. **Create config file**: `providers/new_provider.rs` with a config struct:

```rust
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NewProviderConfig {
    pub api_key: String,
    pub region: Option<String>,
}
```

2. **Add enum variant** to `providers/mod.rs`:

```rust
#[derive(Debug, Clone)]
pub enum EmailProvider {
    Smtp(SmtpConfig),
    SendGrid(SendGridConfig),
    // ...
    NewProvider(NewProviderConfig),  // ← add this
}
```

3. **Add builder entry** in `src/api/user/providers/handlers.rs`:

```rust
fn build_new_provider() -> ProviderDefinition {
    ProviderDefinition {
        provider_id: "new_provider".to_string(),
        name: "New Provider".to_string(),
        scope: ProviderScope::Global,
        config_fields: vec![
            ConfigField { key: "api_key".to_string(), label: "API Key".to_string(), field_type: ConfigFieldType::Password, required: true },
        ],
        smtp_fallback: None,
    }
}
```

4. **Add to channel builder** in `build_email_channel()`:

```rust
fn build_email_channel() -> ChannelDefinition {
    ChannelDefinition {
        channel_id: "email".to_string(),
        channel_name: "Email".to_string(),
        providers: vec![
            // ...
            build_new_provider(),  // ← add this
        ],
    }
}
```

5. **Compiler forces handling**: If you miss a match arm in the delivery code,
   the compiler will catch it.

### API Endpoint

`GET /v1/providers` — returns the full provider registry:

```json
{
  "version": "1.0.0",
  "last_updated": "2026-09-03T00:00:00Z",
  "channels": [
    {
      "channel_id": "email",
      "channel_name": "Email",
      "providers": [
        {
          "provider_id": "smtp",
          "name": "SMTP (Generic)",
          "scope": "global",
          "config_fields": [
            { "key": "host", "label": "SMTP Host", "type": "text", "required": true },
            { "key": "port", "label": "SMTP Port", "type": "number", "required": true }
          ]
        }
      ]
    }
  ]
}
```

### Channel Configs CRUD

| Endpoint | Method | Purpose |
|----------|--------|---------|
| `/v1/projects/{id}/channel-configs` | GET | List project's provider configs |
| `/v1/projects/{id}/channel-configs` | POST | Connect a new provider |
| `/v1/projects/{id}/channel-configs/{config_id}` | PATCH | Update config |
| `/v1/projects/{id}/channel-configs/{config_id}` | DELETE | Disconnect provider |

---

## 22. Milestone Roadmap

| # | Milestone | Scope | Exit criteria |
|---|---|---|---|
| M0 ✅ | Foundations | Restructure workspace; `core` (errors, ULID, event trait, outbox writer, ConfigResolver); `infra/telemetry`; health/liveness; CI | Full workspace compiles & lints; `/healthz`, `/readyz`, `/metrics` |
| M1 ✅ | API Core | axum app core; middleware; Postgres pool + migrations; Redis client | A sample router mounts; error responses are RFC 9457; graceful shutdown |
| M2 | Notifications slice | `notifications` domain; outbox dispatcher + first worker | Create notification persists with outbox row; worker consumes; API tests green |
| M3 | Delivery | `DeliveryProvider`; channel crates implement it; provider registry, attempt records, retry policy, DLQ | Entire workspace compiles; one end-to-end send; retries + DLQ tested |
| M4 | Providers + Templates | `providers` domain (accounts, channel configs CRUD, secrets); `templates` (render, publish) | Configure SMTP provider + template via API; render verified |
| M5 | Recipients & Devices | `recipients` domain; delivery honors preferences | Preferenced delivery path tested |
| M6 | Platform | `projects` (API keys); `auth` (rate limits); `billing` (usage metering) | Authenticated sends; rate limit enforced; usage rows accrue |
| M7 | Growth | `webhooks` (outgoing); `schedules` (cron); analytics read models | Scheduled + webhook delivery end-to-end |
| M8 | Hardening | JWT sessions, observability dashboards, load/perf tests, docs, READMEs | Production readiness review pass |
