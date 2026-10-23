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
├── app/dashboard/user/          # Next.js 16 + React 19 frontend
├── infrastructure/             # Rust workspace (server + worker crates)
│   ├── server/                 # server crate — API binary + shared library
│   │   ├── src/                # server crate source
│   │   │   ├── api/                # HTTP presentation (axum handlers + routes)
│   │   │   │   ├── routes/         # Audience route surfaces (app, admin, external)
│   │   │   │   ├── middleware/     # Cross-cutting middleware (cors, request_id)
│   │   │   │   └── catalog.rs      # GET /routes — route registry
│   │   │   ├── domain/             # Framework-free business logic
│   │   │   ├── ports/              # Trait contracts (AuthStore, DeliveryProvider, QueuePublisher, …)
│   │   │   ├── kernel/             # Shared kernel: errors, ULID, events, outbox, config resolver
│   │   │   ├── channels/           # Channel delivery modules (email, sms, push, …)
│   │   │   ├── infra/              # Concrete drivers (sqlx, reqwest, config, telemetry)
│   │   │   └── testing/            # Test utilities
│   │   └── assets/                 # Migrations, docs, configs
│   │       ├── docs/               # Architecture, commands, contracts
│   │       └── migrations/         # SQL migrations (0001_initial_schema.sql)
│   └── worker/                 # worker crate — background sweeps + queue reader
└── packages/                   # Shared tooling
```

## 2. Crate map

| Crate | Path | Purpose |
|-------|------|---------|
| `server` | `infrastructure/server/` | Main binary + lib — API, domain, infra, kernel, ports, channels |
| `worker` | `infrastructure/worker/` | Background binary — self-timed sweeps, opportunistic RabbitMQ ticks |

## 3. Module boundaries (single crate)

The server crate enforces layering through visibility:

| Module | Allowed dependencies | Purpose |
|--------|---------------------|---------|
| `api/` | axum, serde, domain, ports, kernel, infra | HTTP presentation only |
| `domain/` | none (framework-free) | Business logic, entities, value objects |
| `kernel/` | none (framework-free) | Shared kernel — errors, ULID ids, events, outbox, config resolver |
| `ports/` | domain, kernel only | Trait contracts (AuthStore, DeliveryProvider, QueuePublisher, …) |
| `infra/` | sqlx, reqwest, domain, ports, kernel | Concrete driver implementations |
| `channels/` | domain, ports, kernel | Channel delivery modules (email, sms, push, …) |

**Rule**: Keep axum/sqlx/reqwest out of `domain/`, `kernel/`, and `ports/`.

## 4. API surfaces

Four API surfaces (see root `note.md`):

| Surface | Route prefix | Auth | Purpose |
|---------|-------------|------|---------|
| **Ops** | `/*` | none | Health, readiness, route catalog |
| **External API** | `/v1/*` | API key (from M6) | Product API (empty until M2) |
| **App API** | `/app/*` | Session cookie (`CurrentUser`) | User dashboard backend |
| **Admin API** | `/admin/*` | Admin session + 2FA (`CurrentAdmin`) | Platform administration |

App API routes live in `src/api/routes/app/`:
- `/app/auth/*` — signup, login, logout, verify-email, password reset, OAuth
- `/app/providers` — provider registry (GET)
- `/app/projects/{id}/channel-configs` — channel configs CRUD
- `/app/projects/{id}/recipients` — recipients CRUD
- `/app/projects/{id}/templates` — templates CRUD
- `/app/support/*`, `/app/notifications/*`, `/app/logs` — tickets, in-app notifications, audit log

Admin API routes live in `src/api/routes/admin/`:
- `/admin/*` — status, bootstrap, login, logout, me, password reset, TOTP
- `/admin/users/*` — platform user list/detail/suspend
- `/admin/support/*` — all tickets, replies, status changes

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
├── middleware/         # cors, request_id
├── routes/
│   ├── app/            # App API (/app/*)
│   │   ├── mod.rs      # app_router()
│   │   ├── auth/       # /app/auth/*
│   │   ├── providers/  # /app/providers
│   │   └── channel_configs/ # /app/projects/{id}/channel-configs
│   ├── admin/          # Admin API (/admin/*)
│   │   ├── mod.rs      # admin_router()
│   │   └── users/      # /admin/users/*
│   └── external/       # External product API (/v1)
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

Session-cookie based. See `src/api/routes/app/auth/` and `src/ports/auth_store.rs`.

## 12. Testing strategy

- Unit tests in each module
- Integration tests in `tests/` (auth_api, recipients_api, templates_api)
- All integration tests run against the local Postgres DB (see `COMMANDS.md` §6)
- CI gate: `cargo test --workspace`

## 13. Observability

- Tracing with correlation IDs
- `x-request-id` header on every response
- `/healthz`, `/readyz`, `/metrics` endpoints

## 14. Event-driven patterns

Outbox pattern for reliable event publishing. See `domain/` and `infra/`.

Sweeps (due flow schedules, due admin broadcasts, due campaign drains):
while `NOTIFI_RABBITMQ_URL` is unset the API runs them embedded on a 30s
interval; once it is set, the API publishes a tick per interval to the
durable `notifi.sweep` queue (`src/infra/rabbit.rs`) — with a degraded local
sweep only when a publish fails. The `worker` binary (`infrastructure/worker/`)
always sweeps on its own 30s local interval and, when connected, consumes
those ticks for extra sweeps in between. Ticks are disposable triggers;
atomic `claim_due` in Postgres stays the source of truth, so a lost tick,
overlapping sweeps, or a second sweeper can never double-send.

Both processes build their services from one composition root
(`src/compose.rs`): the API maps the graph into AppState, the worker takes
its three sweep services from the same graph — new services wire once.

## 15. Shared kernel

- `src/kernel/`: errors, ULID ids, events, outbox, config resolver — framework-free (folded from the old `crates/core` crate)
- `src/ports/`: trait-only contracts — framework-free (includes the old `crates/domain-ports` modules: `channel`, `delivery`, `event_bus`, `queue`)

## 16. Channel modules

Channel modules group by channel family; the provider lives inside its
family, never as its own `{name}_channel`:
- Lives in `src/channels/{family}_channel/` (email, sms, push, chat,
  device, webhook) — e.g. `chat_channel/providers/slack.rs`, and one
  `device_channel/providers/` tree holds android, ios, macos, linux,
  window, and web as provider flavors
- Defines provider config structs in the provider file
- Has a provider enum in `providers/mod.rs` (the family's source of
  truth); providers whose flows permit it also carry controller structs
  implementing the family trait plus tests (stub requests + env-gated
  live sends)

## 17. Configuration

Env vars with `NOTIFI_` prefix. See `src/infra/config.rs`.

## 18. Runtime configuration

Brand channel configs and template assets load through the [`ports::ConfigStore`] port (§6). Keys are relative to the convention root; kernel key-builders (`ConfigResolver::channel_key` / `template_key`) share one home with the path helpers, so keys and paths stay one convention:

```text
brands/{brand}/config/{channel_name}/{file_name}
brands/{brand}/templates/{template_name}/{file_name}
```

Driver selection at boot (`AppState.config_store`, always available):

* `infra/file_config_store.rs` — baseline; reads `{NOTIFI_CONFIG_ROOT}` (dev points it at `server/assets` via `.cargo/config.toml`).
* `infra/r2_config_store.rs` — Cloudflare R2 (S3-compatible, zero egress); active when all four `NOTIFI_R2_ACCOUNT_ID / _ACCESS_KEY_ID / _SECRET_ACCESS_KEY / _BUCKET` are set. The bucket **is** the root: the same key addresses either driver, so the folder scheme never becomes a special case.

`server/tests/config_store_r2.rs` seeds the local `assets/brands/**` tree into the bucket and probes the live roundtrip (`--ignored`); `config_store_file.rs` covers the file driver against kernel-derived keys.

Caching: `infra/config_cache.rs` wraps the active store in a Redis read-through (`infra::RedisCacheBackend` behind the same `NOTIFI_REDIS_URL`; `cfg:` key namespace, 10 min TTL, invalidate-on-write). When Redis is unset the store passes through unchanged — caching is composition, never a port concern. `server/tests/config_cache_redis.rs` covers the live roundtrips (gated on `NOTIFI_TEST_REDIS_URL`).

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
src/channels/email_channel/
├── providers/
│   ├── mod.rs           # EmailProvider enum + EmailChannel definition
│   ├── smtp.rs          # SmtpConfig struct
│   ├── sendgrid.rs      # SendGridConfig struct
│   ├── resend.rs        # ResendConfig struct
│   ├── aws_ses.rs       # AwsSesConfig struct
│   ├── postmark.rs      # PostmarkConfig struct
│   ├── mailgun.rs       # MailgunConfig struct
│   └── brevo.rs         # BrevoConfig struct
└── mod.rs               # Channel implementation
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

3. **Add builder entry** in `src/api/routes/app/providers/handlers.rs`:

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

`GET /app/providers` — returns the full provider registry:

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
| `/app/projects/{id}/channel-configs` | GET | List project's provider configs |
| `/app/projects/{id}/channel-configs` | POST | Connect a new provider |
| `/app/projects/{id}/channel-configs/{config_id}` | PATCH | Update config |
| `/app/projects/{id}/channel-configs/{config_id}` | DELETE | Disconnect provider |

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
| M-flow | Flow execution engine | Flow definition store + versioning; full edge-traversal evaluation (conditions, AI transforms); scheduled-send claim worker; retries/backoff/DLQ + attempt records; idempotency keys; custom-flow server sync | Priority/fallback/conditional/AI sends end-to-end with traces |
| M8 | Hardening | JWT sessions, observability dashboards, load/perf tests, docs, READMEs | Production readiness review pass |

> **Phase A shipped (flows v0):** `POST /v1/notifications` (instant, record-first) executes the project's default flow inline — priority walk with fallback, completion marking, 409 without a default. System flows persist server-side (`flows` table); custom flows stay client-side. Conditions/AI nodes pass through unevaluated; scheduled `send_at`, retries, and per-attempt history are M-flow work.

> **Schedules shipped (flow scheduler v1):** `schedules` table + `ScheduleService` (once/recurring-interval timing, no cron) with a 30s claim-and-run worker mirroring the broadcast worker. A schedule assigns one flow plus its own execution payload; assigned flows hide from `GET /app/flows` (managed via Schedules, builder still reachable by id); runs record send rows so deliveries show them. API at `/app/schedules` (CRUD, run-now); dashboard page rebuilt on it.

> **Triggers v1 + HTTP steps + inbound webhooks:** trigger nodes carry a type (`event` webhook-inbound chooser in the builder); `HTTP Request` action nodes fire once per run (log-only failures, `http.status` condition field for If-branching); `POST /v1/hooks/{token}` fires a flow with caller-wins/fallback resolution, backed by the `flow_trigger_tokens` lookup synced on flow saves.
