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
   (`domain/projects/services.rs`): the user's ceiling (5 by default,
   admin-raised `project_limit`) is checked first — tombstones included,
   so delete-and-remake never dodges it, and the admin console's import
   rides the same guard. Then the `projects` row lands with a unique
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

**Team & ownership** — invites move by email (`POST …/members` →
`POST /app/invites/{token}/accept`), and the 2FA gate fires at accept
time. Ownership itself is exactly one account: it moves only through
`POST /app/projects/{id}/ownership` to an **accepted** member — invites
can never carry the `owner` role (`400`). One transaction re-crowns the
receiver, demotes the previous owner (and any legacy co-owner rows) to
`admin`, and slides `created_by` with it, so every owner-only power
(deletion, transfer) follows the marker (`ports/projects_store.rs::
transfer_ownership`). The previous owner stays on the team: they may
leave (`DELETE …/members/me` — the owner cannot until they transfer),
be kicked by the new owner (`DELETE …/members/{user_id}` — owner takes
anyone, admins take plain members only), or stay pro tem. Creation is
ceiling-gated: each user owns at most `project_limit` projects (5 by
default, admin-raised via `PATCH /admin/users/{id}/project-limit`,
audited `user.project_limit_changed`) — the same guard covers the
admin dashboard's import. Proven by `project_members_api.rs`,
`admin_users_api.rs::admin_sets_the_project_ceiling`, and
`projects_lifecycle.rs::the_project_ceiling_gates_create_until_transfer`.

Routes/handlers: `api/routes/app/projects/`; SQL + fake stores:
`ports/projects_store.rs` (trait), `infra` (Postgres), `tests/common`
(fakes). Proven by `server/tests/projects_lifecycle.rs` end to end.

### The life of a device token (§25)

Tokens live on the recipient: the `contacts` blob carries a `devices`
array — `{destination, vendor?, token, label?}` per app install
(`destination` = OS family android/apple/linux/web/window; `vendor` = who
delivers, the dev's call; `label` = dashboard display name). Rotate or
remove = the project dev PUTs the fresh blob — the recipients update swaps
`contacts` wholesale, so there's no token endpoint and `recipient.updated`
audit covers every change. Delivery adapters (M5+) walk the array per
`vendor`. Proven by `device_blob_tests` in `domain/recipients/services.rs`.

### The life of a system email (§18)

The server's own emails (verify, resets, notices, invites) render from
`templates/{name}.html` in the config store — the R2 bucket in
deployment, put there by ops (the server never writes templates).
Boot reads the folder into the forever-cache — Redis keys with no
expiry (direct store reads when Redis is unset) — and every render
answers from the cache; a cold name does one lazy fetch. Subjects
are code (one template, one purpose — `.subject.txt` is gone),
plaintext is tag-stripped, `{{vars}}` fill at send time. Edit a
body in the bucket → `POST /admin/templates/warm` (a restart does
the same). Proven by `system_templates::tests`.

### The life of a backup (§23)

Self-contained block in `worker/src/jobs/db_backup.rs` on its **own clock** (never
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
The worker's sweep loop calls one job file each, under `jobs/`
(`broadcasts.rs`, `schedules.rs`, `campaigns.rs`, `purge.rs`), from its
own clock, optionally RabbitMQ-driven. Provider identities, keys and
the registry: §21 + `channels/*/AUTHENTICATION.md`.

### Where tenant storage lives (§18)

Project channel configs + templates live under `projects/{slug}/…` in the
R2-backed `ConfigStore` (the bucket IS the root; keys are store-relative).
One convention everywhere, cache (`cfg:` prefix, Redis) in between. Every
boot lays the top-level folders down (`projects/`, `backup/`, `users/`) via
`build_ensured`; `--reset` wipes `projects/` back to only those markers
(its store half — the DB comes first, the `cfg:` cache drop third).

### What boot does (§17, lib.rs)

Config layering (defaults → optional `NOTIFI_CONFIG_FILE` → env
overrides) → telemetry → then the four boot gates, each one a hard
requirement where it is switched on: Postgres (connection + migrations),
Redis (reachable), SMTP (only when mail is on — unset mail is a choice),
the object store (the full `NOTIFI_R2_*` quartet, `build_ensured`) →
OAuth credentials (some provider fully paired — absent or half-paired
refuses boot) → `compose()` wires the service graph → routers mount on
three API surfaces (`/`, `/app`, `/admin`; `/v1` when M2 arrives). An
incomplete system aborts with the reason instead of limping on.

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
├── projects/         # runtime tree in the R2 bucket; a test's file store lands here only
├── docs/             # ARCHITECTURE.md, SERVER.md, this README, COMMANDS.md
└── migrations/       # sqlx migration (0001: the complete schema — dev-stage squash)
worker/               # main.rs (wiring) + jobs/ — one file per job
                      # (broadcasts, schedules, campaigns, purge, db_backup)
```

Other root files: `Cargo.toml` (package + workspace index), `rust-toolchain.toml`
(pinned toolchain), `.cargo/config.toml` (dev env: `NOTIFI_EXPOSE_DEV_TOKENS`),
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
| `/app/auth/*` | app | Auth: signup, login, logout, me, OAuth, password flows, verify-email, onboarding-complete; the avatar trio (`POST`/`GET`/`DELETE` `/profile/avatar`) rides the `users/` folder — one image per user (`users/{id}/avatar.{ext}`, jpeg/png/webp ≤ 2 MiB; also 302-redirects external OAuth-imported URLs) |
| `/app/providers` | app | Provider registry — all channels + providers + config fields (GET) |
| `/app/projects/{id}/channel-configs` | app | Channel configs CRUD (list, create, update, delete) |
| `/app/projects/{id}/recipients` | app | Recipients CRUD — the contacts blob carries the `devices` token array (§25) |
| `/app/projects` (POST) | app | Create — ceiling-gated (`project_limit`, default 5, admin-raised; imports ride the same guard) |
| `/app/projects/{id}/members` · `{id}/ownership` | app | Team: invites → accept/decline (`/app/invites/{token}/*`), roster; ownership transfer to an accepted member (owner only, atomic); self-leave; owner/admin removal |
| any other path | — | RFC 9457 `application/problem+json` 404 |

Every response carries a `x-request-id` header for log correlation.
Errors are RFC 9457 problem documents: `{type, title, status, detail, correlation_id}`.

See [`PROVIDERS.md`](./PROVIDERS.md) for the full provider registry & channel
configs API contract.

## Configuration

- Application config: `NOTIFI_HOST`, `NOTIFI_PORT`, `NOTIFI_LOG`,
  optional `NOTIFI_CONFIG_FILE` (JSON).
- Database: `NOTIFI_DATABASE_URL` (e.g. `postgres://user:pass@localhost:5432/notifi`).
  **Required**: the server connects and applies the `assets/migrations/` folder
  at boot, and refuses to start without it.
- Redis: `NOTIFI_REDIS_URL` (e.g. `redis://:password@localhost:6379`). **Required**
  the same way — sessions and the config cache ride it.
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
- Database and Redis are **required for boot** — an incomplete system exits
  with the reason. `docker-compose.yml` provides both locally (edit the
  `CHANGE_ME` values first, then `docker compose up -d`). A **remote**
  database works too: put its URL in a gitignored `.env` file next to
  `Cargo.toml` (`NOTIFI_DATABASE_URL=...`) — loaded automatically at
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
  store IS the Cloudflare R2 bucket (all four `NOTIFI_R2_*` values are a
  boot requirement — R2-or-nothing, no fallback driver). `--reset` (one
  clean-start flag) drops the DB and then — only on success — wipes
  `projects/` back to only its folder markers, and finally drops the
  Redis `cfg:` cache namespace best-effort (nothing durable rides it).

## Database reset

Nuke the database and re-apply migrations from scratch (then chain the
projects-folder wipe when the DB half succeeded):

```shell
cargo run -- --reset    # DB re-applies, projects/ wiped back to markers, cfg: cache dropped, exits
cargo run                  # normal server start
```

> Use this when migration history is stale (e.g., after deleting migration files)
> or when you want a clean dev start without touching Docker volumes.
