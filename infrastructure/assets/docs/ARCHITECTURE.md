# Notifi Server — Architecture

> Status: **Target-state design**. Companion to `SERVER.md` (requirements).
> Applies to the Rust workspace rooted at `infrastructure/`.
> NOTE: the physical layout was simplified from a multi-crate workspace to a
> single server crate + channel plugins — see §2 for the current tree. The
> layering rules and boundaries below still apply, enforced by modules.

## 1. High-Level System Architecture

A **Modular Monolith**: one deployable unit, many crates, compiler-enforced boundaries.

```
┌──────────────────────────────────────────────────────────────────────────┐
│  api (HTTP)              workers (queue consumers)        ← binaries      │
│  composition root: mount routers, wire dependencies                       │
├──────────────────────────────────────────────────────────────────────────┤
│  domains/*    projects  auth  notifications  delivery  recipients        │
│               providers  templates  webhooks  schedules  analytics        │
│               billing                                                    │
│               each = vertical slice with mini clean architecture inside  │
├──────────────────────────────────────────────────────────────────────────┤
│  core (errors, IDs, events, outbox, config)   domain-ports (traits only) │
├──────────────────────────────────────────────────────────────────────────┤
│  infra/*      postgres  redis  queue  telemetry  config                  │
│  adapters/    channels/* (email, sms, fcm, apns, web-push, ...)          │
└──────────────────────────────────────────────────────────────────────────┘
```

Key properties:

- **Features own their full HTTP slice.** A domain exposed externally carries its own `presentation/` submodule (routes, handlers, DTOs). The `api` binary only mounts routers; it contains no business logic.
- **Cross-domain communication is event-driven.** Modules never import each other. They publish domain events through a **transactional outbox** (Postgres) → **pgmq** queue → `workers` → subscribed domain handlers.
- **The 13 existing channel crates become delivery adapters.** They implement one port trait (`DeliveryProvider`) from `domain-ports`; they never depend on each other or on any domain.
- **Dependencies always point inward.** `core`/`domain-ports` are framework-free. Domains are framework-free. Only `infra/*`, adapters, and the binaries touch concrete libraries (axum, sqlx, redis, reqwest).

### Tech stack

| Concern | Choice |
|---|---|
| HTTP | axum (tokio-native, tower middleware) |
| Async runtime | tokio (multi-thread) |
| Database | PostgreSQL via sqlx (compile-time checked queries) |
| Migrations | sqlx (single `assets/migrations/` set) |
| Cache / rate limit / pub-sub | Redis |
| Queue / workers | pgmq (enqueue atomically with the DB transaction) |
| Telemetry | tracing + tracing-opentelemetry + Prometheus metrics |
| Config | Layered: defaults ← `NOTIFI_CONFIG_FILE` ← `NOTIFI_*` env |
| IDs | ULID (newtyped per aggregate) |
| Errors | thiserror; RFC 9457 problem details at the HTTP edge |

---

## 2. Layout

The server is a **single crate at `infrastructure/`** (`src/`), with
independent channel plugins as small crates beside it. Architectural
boundaries are enforced by Rust's module system + visibility, not by crate
count.

```
infrastructure/                 # server crate root (package `server`)
├── Cargo.toml                  # [package] server  +  [workspace] = channels/*
├── rust-toolchain.toml
├── assets/                     # server-owned data & docs
│   ├── brands/                 # tenant channel configs + templates
│   ├── docs/                   # ARCHITECTURE.md (this doc), SERVER.md, README, ...
│   └── migrations/             # numbered sqlx migrations (domain-namespaced tables)
├── docker-compose.yml          # local Postgres (pgmq) + Redis
├── .cargo/config.toml          # dev env: NOTIFI_CONFIG_ROOT → ./assets
├── channels/                   # channel plugin crates (email, sms, fcm, ...)
│   ├── email_channel/
│   └── ...                     # one crate per delivery protocol; depend only
│                               # on ports (M3), never on each other or domains
└── src/
    ├── main.rs                 # binary entry point → server::run()
    ├── lib.rs                  # module tree; run() bootstrap; build_router()
    ├── api/                    # HTTP presentation layer
    │   ├── project/            # PROJECT surface: product API at the root
    │   │                       #   (API-key auth M6+; endpoints from M2)
    │   ├── user/               # USER surface: dashboard backend under /v1
    │   │   └── auth/           #   handlers/dto/routes for the auth slice
    │   ├── catalog.rs          # route registry per surface (GET /routes)
    │   ├── state.rs            # AppState shared by handlers
    │   ├── error.rs            # RFC 9457 problem rendering
    │   └── middleware/         # CORS, request-id, tracing
    ├── domain/
    │   └── auth/               # entities, errors, services, value objects
    ├── ports/                  # trait contracts (AuthStore, OAuthIdentityProvider,
    │                           # DeliveryProvider via crates/domain-ports)
    ├── infra/                  # sqlx repos, OAuth HTTP client, config, telemetry, redis
    └── testing.rs              # in-memory fakes shared with integration tests
crates/
├── core/                       # notifi-core: errors, ULID ids, events, outbox, config
└── domain-ports/               # trait-only contracts shared with channels (M3)

Note: tenant channel configs and brand templates live in
`infrastructure/assets/brands/{brand}/{config,templates}/...`; local dev
points `NOTIFI_CONFIG_ROOT` at `infrastructure/assets` via
`.cargo/config.toml` (see §18).

### API surfaces

Two independent surfaces, composed in `src/api/mod.rs::build_router`:

| Surface | Base | Auth | Consumers |
|---|---|---|---|
| **project** | `/` | API keys (M6+) | Customer backends integrating Notifi |
| **user/v1** | `/v1` | session cookie | The Notifi dashboard |

- Surfaces share ops routes (`/`, `/healthz`, `/readyz`, `/routes`) but
  nothing else: product logic changes must never affect user/dashboard
  logic or vice versa.
- The user surface is versioned by path prefix. Breaking changes ship as
  `/v2` mounted beside `/v1`; v1 keeps its contract.
- Every route declares its surface in `src/api/catalog.rs`, which drives
  `GET /routes`.

**Migration from the old tree:** the former `crates/{api,domains/*,infra/*}`
were folded into `src/`; `core` and `domain-ports` remain as tiny
framework-free crates because the channel plugins will consume them in M3.
The 13 channel crates moved from `crates/adapters/channels/*` to
`channels/*`.

---

## 3. Canonical Domain Crate Template (mini clean architecture)

Every domain follows this anatomy; optional folders are omitted where a feature
does not need them (e.g., no HTTP surface → no `presentation/`).

```
notifications/
├── Cargo.toml
├── README.md                      # purpose, responsibilities, rules, events,
│                                  # public interfaces, dependencies, expansion
└── src/
    ├── lib.rs                     # public surface only (re-exports + wiring)
    │
    ├── presentation/              # ← depends only on application
    │   ├── mod.rs
    │   ├── routes.rs              # axum Router (mounted by the api binary)
    │   ├── handlers.rs            # thin: parse DTO → call service → map errors
    │   ├── dto/                   # request/response shapes (serde)
    │   │   ├── mod.rs
    │   │   ├── create_notification.rs
    │   │   └── list_notifications.rs
    │   └── responses.rs           # problem-details response mapping (RFC 9457)
    │
    ├── application/               # ← depends on domain only; framework-free
    │   ├── mod.rs
    │   ├── commands.rs            # command objects (CreateNotification, Enqueue)
    │   ├── queries.rs             # paginated read queries
    │   ├── services.rs            # orchestrate command → domain → repo → outbox
    │   └── events.rs              # inbound event subscriptions; outbound events
    │
    ├── domain/                    # ← depends on nothing but notifi-core
    │   ├── mod.rs
    │   ├── entities.rs            # aggregate + state machine
    │   ├── value_objects.rs       # NotificationId(Ulid), Status, ProviderRef
    │   ├── repository.rs          # repository trait (port)
    │   ├── policies.rs            # business rules (retry, channel selection)
    │   ├── specifications.rs      # query filter predicates
    │   └── errors.rs              # thiserror enum → core::ApiError
    │
    ├── infrastructure/            # ← implements domain ports (private)
    │   ├── mod.rs
    │   ├── repository_pg.rs       # sqlx impl of domain::repository
    │   ├── outbox.rs              # atomic event writes with aggregate TX
    │   └── providers.rs           # concrete DeliveryProvider wiring
    │
    └── tests/                     # feature tests + factories (public API only)
```

Rules:

- `lib.rs` never exports `infrastructure/` — no caller can reach SQL.
- `domain/` and `application/` compile without framework crates (enforced by
  dependency lists + clippy: no `axum`/`sqlx`/`reqwest` imports outside
  `presentation/` and `infrastructure/`).
- Each folder declares `mod.rs` (classic 2018+ module style, unambiguous).
- Crate-level integration tests live in `tests/` at the crate root.

### When each file exists

| File | Exists when |
|---|---|
| `presentation/routes.rs` | Feature exposes HTTP endpoints |
| `presentation/dto/` | Request/response shapes differ from domain objects |
| `application/events.rs` | Feature emits or consumes domain events |
| `domain/specifications.rs` | Feature needs reusable query predicates |
| `domain/policies.rs` | Feature has non-trivial business rules beyond the entity |
| `infrastructure/outbox.rs` | Feature writes aggregates to Postgres |
| `infrastructure/providers.rs` | Feature dispatches through delivery adapters |
| `tests/` | Always (even a unit-test-only feature) |

---

## 4. Layer Responsibilities

| Layer | Responsibility | Allowed deps |
|---|---|---|
| `api` (binary) | Mount routers, wire dependencies, middleware chain, health, shutdown | everything |
| `workers` (binary) | Queue consumer runtime, worker registry, retries, DLQ, graceful shutdown | domains, infra-queue |
| `presentation/` | Translate HTTP ↔ application; validation of input shape; error mapping | application, axum, serde |
| `application/` | Orchestrate use cases (commands/queries/services); transactions; event publishing | domain, core, ports |
| `domain/` | Entities, value objects, invariants, state machine, ports (repository traits), errors | core only |
| `infrastructure/` | Concrete persistence, queue writes, provider wiring | domain (ports), sqlx/redis |
| `infra/*` | Shared concrete infrastructure: pool, clients, telemetry | core, ports |
| `adapters/channels/*` | One delivery protocol each (SMTP, FCM, APNs, ...); config load per tenant | ports, core |
| `core` | DomainError/ApiError, ULID newtypes, DomainEvent trait, outbox writer, ConfigResolver | nothing |

## 5. Dependency Direction

```
core  ←  domain-ports
core  ←  domains/*           (domain + application layers)
domain-ports  ←  infra/*, adapters/channels/*
domains/*  ←  api, workers
infra/*  ←  api, workers     (composition root wires concrete impls into domains)
```

- Domain code **never** imports `infra/*` or adapters. It depends on traits.
- Adapters **never** import domains or each other (mirrors the existing
  `notification_core` rule in AGENTS.md).
- No circular dependencies: enforced by crate imports at compile time.

## 6. Inter-Module Communication

1. Application service runs a use case inside one Postgres transaction:
   aggregate rows **+ outbox rows** commit together (no dual-write).
2. A dispatcher (in `workers` or a dedicated task) reads outbox rows and
   publishes them as pgmq messages on topic `domain.events.<type>`.
3. Worker handlers subscribed to those topics call the target domain's
   `application/events.rs` handlers, which react and may emit new events.
4. Synchronous cross-module reads (e.g., "project exists") go through the
   consumer domain's application query API — never through another domain's
   tables.

This keeps modules decoupled: a domain reacts to `notification.failed` without
knowing who produced it.

## 7. Module Boundaries

- **Crate = boundary.** All cross-crate access requires `pub` items; private
  modules (`infrastructure/`, internals) are unreachable.
- **One aggregate per domain.** `notifications` owns the notification lifecycle;
  `delivery` owns send attempts and provider dispatch; `providers` owns
  credentials and channel configs. `notifications` never touches a provider
  directly — it emits `notification.queued` and `delivery` reacts.
- **Schema namespaces.** Tables are prefixed by domain: `notification_*`,
  `delivery_*`, `provider_*`, ... Domains may not reference foreign domains'
  tables except through their query APIs.

## 8. Feature Boundaries

| Feature (domain) | Owns |
|---|---|
| projects | projects, env-scoped credentials (dev/prod gate), per-project membership, user folders |
| auth | API keys (hashed), sessions, members, permissions |
| notifications | notification aggregate, state machine, queuing |
| delivery | attempts, provider registry, retry policy, DLQ |
| recipients | recipients, devices (FCM/APNs/web-push tokens), channel preferences |
| providers | provider accounts (SMTP creds, Twilio, FCM project, ...), channel configs |
| templates | template versions, rendering (handlebars), publish lifecycle |
| webhooks | outgoing delivery of customer-configured webhooks |
| schedules | cron/one-off scheduled sends |
| analytics | delivery event read models, per-project counters |
| billing | plans, usage metering, invoices (par org, not per account) |

## 9. Adding a New Feature

1. `cargo new crates/domains/<name> --lib`; depend on `notifi-core` (+
   `notifi-domain-ports` if it sends or consumes events).
2. Build `domain/` first: entities, value objects, errors, repository trait.
3. Add `application/` services/commands/queries and `events.rs`.
4. Add `infrastructure/repository_pg.rs` + `outbox.rs`; register a migration.
5. Add `presentation/` if the feature has HTTP routes.
6. In `api`: mount `name::presentation::routes::router()`. In `workers`:
   register event subscriptions (if any).
7. Write `tests/`; write `README.md`; add a `migrations/` file.
8. **No other domain changes.** If another domain needs the new feature's
   data, expose a query; never import its internals.

## 10. Naming Conventions

- Crates: `notifi-<thing>` → package `notifi_<thing>`; binaries `api`, `workers`.
- Files/dirs: `snake_case`; module tree mirrors the anatomy above
  (`routes.rs`, `repository_pg.rs`, `outbox.rs`).
- IDs: newtypes `XId(Ulid)` (`NotificationId`, `ProjectId`); fields `id`, `org_id`.
- Events: `<domain>.<past_tense_verb>` — `notification.created`,
  `notification.queued`, `notification.delivered`, `notification.failed`,
  `notification.retried`, `organization.created`, `provider.connected`,
  `template.published`, `recipient.created`, `webhook.delivered`.
- Commands: `XCommand` (`CreateNotificationCommand`); queries `XQuery`
  (`ListNotificationsQuery`); services `XService` (`NotificationService`).
- Errors: `XError` per domain.
- Tests: `tests/<feature>_<scenario>.rs`, factories in `tests/factories.rs`.

## 11. Coding Conventions

- `#![forbid(unsafe_code)]` in every crate.
- No `unwrap`/`expect`/`panic` in library code; use `Result` and typed errors.
  Binaries may `expect` only in `main` for startup config.
- Newtype patterns for IDs and primitive-sense values (`Status`, `AttemptCount`).
- Builder pattern for multi-field command construction; `From` impls for DTO ↔
  command mapping (`mapper` role lives in `presentation/dto`).
- Typestate only where it clearly eliminates invalid states (e.g., a notification
  `Queued` cannot be `sent` without an attempt).
- Avoid `Arc<Mutex<_>>`; prefer owned data, `tokio::sync` primitives, and
  immutable reads.
- Keep modules small (~200 lines); split along the anatomy, not by habit.
- Explicit `pub` surface in `lib.rs`; document every re-export.
- No comments describing *what*; doc comments (`///`) on public items only.

## 12. Module README Template

```markdown
# notifi-<domain>

## Purpose
## Responsibilities
## Business rules          (the invariants this domain enforces)
## Dependencies            (crates + other domains consumed via events/queries)
## Public interface        (services, routes, commands, queries)
## Events                  (emitted / subscribed)
## Future expansion
## Examples
```

## 13. Event Conventions

- Written to the outbox **inside the aggregate's transaction**; one producer per
  aggregate; immutable payloads (JSON, schema-versioned `v` field).
- Naming: `<domain>.<past_tense>`; payload carries `aggregate_id`, `org_id`,
  `occurred_at` (ULID-embedded timestamp), `correlation_id`.
- Consumers acknowledge after processing; failures retry per policy; persistent
  failures go to DLQ.
- `Notification` lifecycle events are canonical:
  `created → queued → sending → delivered | failed → retried → dead_lettered`.

## 14. Error Conventions

- One `thiserror` enum per domain (`NotificationError`, `DeliveryError`, ...)
  implementing `Into<core::ApiError>`.
- `core::ApiError` carries `(status, type_url, title, detail, instance,
  correlation_id)` and serializes to RFC 9457 `application/problem+json`.
- Provider errors are wrapped at the adapter boundary into `DeliveryError`
  with a `kind` (auth, rate_limited, invalid_recipient, timeout, permanent) —
  the only thing the core needs to decide retry policy.
- Validation errors are a list (per-field) in `detail`; never `400` for
  domain violations that are really `409` (state machine) or `404` (aggregate
  not found).

## 15. Testing Conventions

- **Unit:** inline `#[cfg(test)]` in domain modules (state machine, policies).
- **Feature tests:** `tests/` in the domain crate, exercising application
  services against real Postgres/Redis via `testcontainers`.
- **API tests:** `tower::ServiceExt::oneshot` against the mounted router —
  full request → problem-details response assertions.
- **Worker tests:** in-memory queue implementation of the port; assert
  retry counts, DLQ routing, event re-emission.
- **Factories:** `tests/factories.rs` per crate; fixtures under
  `tests/fixtures/`. Snapshot stable outputs with `insta` where useful.
- Gates: `cargo clippy --workspace -- -D warnings`, `cargo test --workspace`
  (plus `--exclude web_channel` until M3 fixes it — then never).

## 16. Repository Conventions

- Trait per aggregate in `domain/repository.rs`
  (`NotificationRepository`, `AttemptRepository`, ...): `Send + Sync`, async,
  returns domain errors, never leaks sqlx types.
- sqlx impl in `infrastructure/repository_pg.rs`, bound to `PgPool` passed at
  construction (composition root).
- Aggregates carry `version` for optimistic locking (`UPDATE ... WHERE version = $x`
  → `409` on conflict).
- Soft deletes: `deleted_at` on tenant-owned aggregates; hard delete only for
  pure logs.
- Read models in `analytics` are denormalized and rebuilt from events — never
  written by writers.
- Audit: append-only `{domain}_{table}_audit` rows written in the same
  transaction for mutation operations.

## 17. Worker Conventions

- `workers` owns the runtime: queue manager (pgmq consumer groups), a registry
  `(topic, handler)` supplied by each domain at startup, per-topic concurrency
  limits, graceful shutdown via `tokio_util::CancellationToken`.
- Retry policy per topic: exponential backoff + jitter, configurable max
  attempts; after max → DLQ topic + `delivery.failed`/`job.dead_lettered`.
- Priority: separate pgmq queue per priority tier (`notifications.high` vs
  `notifications.low`); high-priority consumers pull first.
- Every worker emits metrics (`jobs.processed`, `jobs.retries`,
  `jobs.dlq_total`) and logs structured with `correlation_id`.

## 18. Configuration Conventions

- Layered loading (in `infra/config`): compiled defaults ← optional config
  file (`NOTIFI_CONFIG_FILE`) ← environment `NOTIFI_*`. Never secrets in
  defaults or committed files.
- One `AppConfig` struct composed of per-domain `XConfig` sections; each
  domain exposes `fn from_env`/`From<AppConfig>` for its own slice.
- Tenant channel configs follow the canonical brand-assets layout
  `{NOTIFI_CONFIG_ROOT}/brands/{brand}/config/{channel}/` (templates at
  `.../brands/{brand}/templates/{name}/`), loaded through
  `core::ConfigResolver` (`channel_dir`/`template_dir`/`load_json`) honoring
  `NOTIFI_CONFIG_ROOT` (default `configs`; local dev sets it to
  `infrastructure/assets` via `infrastructure/.cargo/config.toml`,
  `value = "./assets"` resolved relative to that config file) — this is the
  `ChannelConfigLoader` trait from `domain-ports`, adopted by all adapters.
- Secrets: env vars or mounted secret files only (SMTP password, Twilio auth,
  service accounts, VAPID keys).

## 19. Migration Strategy

- Single `assets/migrations/` set, numbered `0001_xxx.sql`, run at startup and via
  CLI (`sqlx migrate`).
- Tables namespaced by domain prefix; schema-per-domain (Postgres schemas
  `notifications`, `delivery`, ...) adopted if extraction looms.
- Forward-only migrations; destructive changes done as expand → migrate →
  contract → collapse.
- Outbox table: `event_outbox(id, aggregate_type, aggregate_id, event_type,
  payload, status, created_at, published_at)` with a partial index on
  `published_at IS NULL`.

## 20. Evolving Into Microservices

Because each domain is a crate with its own aggregate, outbox, schema
namespace, event contracts, and README, extraction is mechanical:

1. **delivery + adapters** (first, when push volume demands): copy crate +
   migrations, run the `workers` runtime as its own deployment, expose
   `DeliveryProvider` over gRPC for the monolith.
2. **analytics**: promote read-model tables + consumers into a read-model
   service subscribing to the event topics (pgmq → stream bridge).
3. **billing / webhooks / schedules**: same pattern; the outbox is the
   handoff, pgmq topics become the message bus, in-process ports become
   gRPC shims generated from `domain-ports` trait signatures.

No rewrites: the ports are already the network boundaries.

---

## 21. Milestone Roadmap

| # | Milestone | Scope | Exit criteria |
|---|---|---|---|
| M0 ✅ | Foundations | Restructure workspace into `crates/`; `core` (errors, ULID, event trait, outbox writer, ConfigResolver); `infra/telemetry`; health/liveness; CI (check + clippy -D warnings + test) | Full workspace compiles & lints (with `--exclude web_channel`); `/healthz`, `/readyz`, `/metrics` |
| M1 ✅ | API Core | axum app core in `api`: middleware (trace id, request id, logging, CORS, problem-details mapping); Postgres pool + migrations runner; Redis client | A sample router mounts; error responses are RFC 9457; graceful shutdown |
| M2 | Notifications slice | `notifications` domain: aggregate + state machine (`created→queued`), repository_pg, service, `POST /v1/notifications`; outbox dispatcher + first worker | Create notification persists with outbox row; worker consumes `notification.created`; API tests green |
| M3 | Delivery | `domain-ports::DeliveryProvider`; convert all 13 channel crates (email, sms, android/fcm, ios, macos, web, webhook, whatsapp, telegram, slack, discord, window, linux) to implement it; **fix web_channel against web-push 0.9**; provider registry, attempt records, retry policy, DLQ; `delivered/failed/retried` events | Entire workspace compiles; one end-to-end send through an adapter; retries + DLQ tested |
| M4 | Providers + Templates | `providers` domain (accounts, channel configs CRUD, secrets); `templates` (render, publish) | Configure SMTP provider + template via API; render verified |
| M5 | Recipients & Devices | `recipients` domain: recipients, devices, per-recipient channel preferences; delivery honors preferences | Preferenced delivery path tested |
| M6 | Platform | `projects` (projects + env-gated API keys), `auth` (rate limits in Redis), `billing` (usage metering, per-account plans) | Authenticated sends; rate limit enforced; usage rows accrue |
| M7 | Growth | `webhooks` (outgoing), `schedules` (cron), `analytics` read models | Scheduled + webhook delivery end-to-end |
| M8 | Hardening | JWT sessions, observability dashboards, load/perf tests, docs, READMEs complete; extraction drills documented | Production readiness review pass |
