# Notifi Server

Rust backend for the Notifi notification platform — a single server crate
(`src/`) plus independent channel plugin crates.

## Where to start

New to this codebase? Follow this order:

1. **This README** — layout map + commands (you are here).
2. **`docs/ARCHITECTURE.md`** — the full design: layers, boundaries,
   conventions, M0–M8 roadmap. *Read this before touching any code.*
3. **`src/`** — the server itself (`cargo run` from `infrastructure/`). Its
   modules are the map of how the app is put together:
   `api/` (HTTP surfaces) → `domain/` (business logic) → `ports/`
   (traits) → `infra/` (drivers).
4. **`Cargo.toml`** — package manifest + `[workspace] members`: the channel
   plugin crates.

Quick references: [`OAUTH_SETUP.md`](./OAUTH_SETUP.md) for GitHub/Google
app registration, [`COMMANDS.md`](./COMMANDS.md) for the full command list,
[`PROVIDERS.md`](./PROVIDERS.md) for provider registry & channel configs API.

Two reading habits make this codebase reviewable on sight:

- **Test names read as sentences.** Every behavioral claim below is proven
  by a test whose name says the behavior — `create_materializes_the_project_folder`,
  `purge_wipes_bucket_folder_first_then_rows`… jump from any claim here to
  the test and the code it exercises.
- **Story lives at the file top.** Every load-bearing module opens with a
  `//!` block: role, invariants, who calls it, which tests prove it. The same
  words render as browsable HTML too: `cargo doc --open -p server`.

## The flows, one page each

*Deep detail lives in the ARCHITECTURE sections pointed at; each walkthrough
is the route a reviewer walks when they open an unfamiliar file mid-flow.*

### The life of a project (§24)

1. **Create** — `POST /app/projects` → `ProjectService::create`
   (`domain/projects/services.rs`): the `projects` row lands with a unique
   stored slug (`kernel::config::project_slug`), then storage seeds
   `projects/{slug}/` with the code-owned defaults
   (`DefaultProjectLayout` through `materialize_project`, §18). Storage
   failure rolls the row back — a project never exists without its folder.
2. **Live** — the project serves whatever its `active` switch and its
   tombstone state allow (`get_project_access` freezes traffic on deletion).
3. **Soft-delete** — `DELETE /app/projects/{id}` tombstones the row
   (`deleted_at` + `purge_after`, 14 days); frozen for traffic, still
   visible to members so they can cancel or download.
4. **Cancel** — `POST /app/projects/{id}/deletion/cancel` fully undoes it.
5. **Export** — `GET /app/projects/{id}/export`, anytime, any member: one
   ZIP (manifest + data sections + the project files at real storage
   paths), capped to 3/day per project, API keys exported **without**
   `key_hash` (`domain/projects/export.rs`).
6. **Purge** — the worker sweep (`worker`, `ProjectService::purge_due`):
   folder keys under `projects/{slug}/` deleted **first**, then the row
   (15 FK cascades wipe every child table). Retry-safe on storage failure.

Routes/handlers: `api/routes/app/projects/`; SQL + fake stores:
`ports/projects_store.rs` (trait), `infra` (Postgres), `tests/common`
(fakes). Proven by `server/tests/projects_lifecycle.rs` end to end.

### The life of a backup (§23)

Self-contained block in `worker/src/backup.rs` on its **own clock** (never
the queue): daily `pg_dump --format=plain` streamed in 8 MiB parts into
the **same bucket as the config store**, under `backup/`, as one
restorable `.sql` object. Temp file deleted. **Nothing is ever
auto-deleted.** `NOTIFI_BACKUP_ENABLED=true` switches it on;
`cargo run -p worker -- --backup-now` runs one immediately.

### The life of a notification (§14, §16)

An external send arrives at `/v1/*` (empty until M2) or a dashboard flow
fires; the domain turns it into a `SendRecord`, delivery goes through
`ports::DeliveryProvider` → the channel module for that medium
(`channels/{email,sms,push,chat,webhook}/…`), each channel resolving its
per-project config through the same `ConfigStore` the lifecycle uses. The
worker keeps sweeps ticking from its own clock (`worker::sweep_once`,
optionally RabbitMQ-driven). Provider identities, keys and the registry:
§21 + `channels/*/AUTHENTICATION.md`.

### Where tenant storage lives (§18)

Project channel configs + templates live under `projects/{slug}/…` in a
`ConfigStore` driver: the file system (root `NOTIFI_CONFIG_ROOT`) or the
R2 bucket when `NOTIFI_R2_*` is set (bucket = root, same keys). One
convention everywhere, cache (`cfg:` prefix, Redis) in between. Every
boot lays the top-level folders down (`projects/`, `backup/`) via
`build_ensured`; `--reset-config` wipes back to only those markers.

### What boot does (§17, lib.rs)

Config layering (defaults → optional `NOTIFI_CONFIG_FILE` → env
overrides) → telemetry → pool → optional Redis (sessions + config cache)
→ optional RabbitMQ → the config store driver pick (`build_ensured`) →
`compose()` wires the service graph → routers mount on three API
surfaces (`/`, `/app`, `/admin`; `/v1` when M2 arrives). Missing pieces
degrade explicitly rather than crash: no DB = auth answers 503, no mail
= auth flows ride dev tokens, no Redis = direct store reads.

## Layout

```
src/
├── main.rs           # binary entry point → server::run()
├── lib.rs            # module tree + run()/build_router()
├── api/              # HTTP presentation layer
│   ├── routes/       # app (`/app`), admin (`/admin`), external (`/v1`)
│   └── middleware/   # cors, request-id
├── channels/         # channel delivery modules (email, sms, push, web, …)
├── domain/           # business logic & models (auth, admin, billing, flows, …)
├── ports/            # trait contracts (AuthStore, DeliveryProvider, QueuePublisher, …)
├── kernel/           # framework-free kernel: errors, ULID ids, events, outbox, config
├── infra/            # concrete drivers: Postgres repos, OAuth client, config, telemetry
└── testing.rs        # in-memory fakes shared with integration tests
assets/               # server-owned data & docs
├── projects/         # tenant channel configs + templates (NOTIFI_CONFIG_ROOT)
├── docs/             # ARCHITECTURE.md, SERVER.md, this README, COMMANDS.md
└── migrations/       # sqlx migrations (0001_initial_schema.sql)
worker/               # background binary — RabbitMQ sweep-tick consumer
```

Other root files: `Cargo.toml` (package + workspace index), `rust-toolchain.toml`
(pinned toolchain), `.cargo/config.toml` (dev env: `NOTIFI_CONFIG_ROOT → ./assets`),
`target/` (build output, gitignored).

### API surfaces (see [`note.md`](./note.md))

| Surface | Base path | Auth | Purpose |
|---|---|---|---|
| **ops** | `/` | none | Health, readiness, route catalog |
| **external** | `/v1` | API keys (M6+) | The product API customers integrate with (empty until M2) |
| **app** | `/app` | session cookie | The user dashboard backend |
| **admin** | `/admin` | admin session + 2FA | Platform administration |

Route declarations live in `src/api/catalog.rs`; each surface's router is in
its own module (`src/api/routes/external/mod.rs`, `src/api/routes/app/mod.rs`, `src/api/routes/admin/mod.rs`).

## Commands

Run from this directory (`infrastructure/`). A full reference with explanations
lives in [`COMMANDS.md`](./COMMANDS.md).

```shell
cargo check --workspace
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cargo fmt --all --check
cargo run
```

## API endpoints

| Endpoint | Surface | Purpose |
|---|---|---|
| `GET /` | ops | Service identification (name, version, milestone) |
| `GET /healthz` | ops | Liveness — the process is up |
| `GET /readyz` | ops | Readiness — Postgres + Redis reachable (503 problem doc otherwise) |
| `GET /routes` | ops | Route catalog — every endpoint across all surfaces |
| `/v1/*` | external | *(empty until M2 — product endpoints mount under /v1)* |
| `/app/auth/*` | app | Auth: signup, login, logout, me, OAuth, password flows, verify-email, onboarding-complete |
| `/app/providers` | app | Provider registry — all channels + providers + config fields (GET) |
| `/app/projects/{id}/channel-configs` | app | Channel configs CRUD (list, create, update, delete) |
| any other path | — | RFC 9457 `application/problem+json` 404 |

Every response carries a `x-request-id` header for log correlation.
Errors are RFC 9457 problem documents: `{type, title, status, detail, correlation_id}`.

See [`PROVIDERS.md`](./PROVIDERS.md) for the full provider registry & channel
configs API contract.

## Configuration

- Application config: `NOTIFI_HOST`, `NOTIFI_PORT`, `NOTIFI_LOG`,
  optional `NOTIFI_CONFIG_FILE` (JSON).
- Database: `NOTIFI_DATABASE_URL` (e.g. `postgres://user:pass@localhost:5432/notifi`).
  When set, the API connects and applies the `assets/migrations/` folder at boot.
- Redis: `NOTIFI_REDIS_URL` (e.g. `redis://:password@localhost:6379`).
- Auth: `NOTIFI_EXPOSE_DEV_TOKENS=true|false` — puts raw one-time tokens in
  auth responses (local dev until email delivery lands; the checked-in
  `.cargo/config.toml` enables it for `cargo run`/`cargo test`).
- Backups (worker): `NOTIFI_BACKUP_ENABLED=true` turns on the daily
  database dump — one plain `.sql` file per day (schema + data) streamed
  into the same bucket as the tenant config, under its own `backup/`
  folder. Reuses the `NOTIFI_R2_*` values; slot via `NOTIFI_BACKUP_AT`
  (default `03:00`), dump binary via `NOTIFI_BACKUP_PG_DUMP`, target via
  `NOTIFI_BACKUP_DB_URL` (defaults to `NOTIFI_DATABASE_URL`). Nothing is
  ever auto-deleted. One-shot test: `cargo run -p worker -- --backup-now`.
  Full story: ARCHITECTURE §23.
- Database and Redis are **optional**: without them the API still boots and
  serves `/healthz`; `/readyz` reports 503. `docker-compose.yml` provides both
  locally (edit the `CHANGE_ME` values first, then `docker compose up -d`).
  A **remote** database works too: put its URL in a gitignored `.env` file
  next to `Cargo.toml` (`NOTIFI_DATABASE_URL=...`) — loaded automatically at
  boot; real environment variables still win over file values.
- Telemetry filter: `NOTIFI_LOG` (falls back to `RUST_LOG`, then `info`).
- **Project assets** (tenant channel configs + templates) live in the
  config store, not in the repo. Keys under the convention root
  (relative to `infrastructure/`):

  ```text
  projects/{slug}/config/{channel}/...     channel configs
  projects/{slug}/templates/{name}/...     project-scoped templates
  ```

  The default layout is code (`kernel::config::DefaultProjectLayout`) — a
  project materializes its own `projects/{slug}/**` tree at creation. The
  root is overridable via `NOTIFI_CONFIG_ROOT` (`.cargo/config.toml`
  points dev at `./assets`), and all four `NOTIFI_R2_*` values switch to
  the Cloudflare R2 bucket, which then *is* the root. `--reset-config`
  wipes `projects/` back to only its folder markers; `--reset-db` chains it after the DB
  reset.

## Database reset

Nuke the database and re-apply migrations from scratch:

```shell
cargo run -- --reset-db    # drops public schema, re-applies 0001_initial_schema.sql, exits
cargo run                  # normal server start
```

> Use this when migration history is stale (e.g., after deleting migration files)
> or when you want a clean dev start without touching Docker volumes.
