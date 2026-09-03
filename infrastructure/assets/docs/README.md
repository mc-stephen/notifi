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

## Layout

```
src/
├── main.rs           # binary entry point → server::run()
├── lib.rs            # module tree + run()/build_router()
├── api/              # HTTP presentation layer
│   ├── project/      # PROJECT surface — product API at the root (API-key auth, M6+)
│   └── user/         # USER surface — dashboard backend under /v1 (session auth)
├── domain/           # business logic & models
│   ├── auth/
│   └── channels/     # provider registry types (ProviderRegistry, etc.)
├── ports/            # trait contracts (AuthStore, ChannelProviderStore, ...)
├── infra/            # concrete drivers: Postgres repos, OAuth client, config, telemetry
└── testing.rs        # in-memory fakes shared with integration tests
assets/               # server-owned data & docs
├── brands/           # tenant channel configs + templates (NOTIFI_CONFIG_ROOT)
├── docs/             # ARCHITECTURE.md, SERVER.md, this README, COMMANDS.md
└── migrations/       # sqlx migrations (0001_initial_schema.sql)
channels/             # channel plugin crates (email, sms, push, chat)
```

Other root files: `Cargo.toml` (package + workspace index), `rust-toolchain.toml`
(pinned toolchain), `.cargo/config.toml` (dev env: `NOTIFI_CONFIG_ROOT → ./assets`),
`target/` (build output, gitignored).

### Two API surfaces

| Surface | Base path | Auth | Purpose |
|---|---|---|---|
| **project** | `/` | API keys (M6+) | The product API customers integrate with — evolves independently |
| **user** | `/v1` | session cookie | The dashboard backend — versioned so `/v2` can later coexist with v1 |

Route declarations live in `src/api/catalog.rs`; each surface's router is in
its own module (`src/api/project/mod.rs`, `src/api/user/mod.rs`).

## Commands

Run from this directory (`infrastructure/`). A full reference with explanations
lives in [`COMMANDS.md`](./COMMANDS.md).

```shell
cargo check --workspace --exclude web_channel
cargo clippy --workspace --exclude web_channel --all-targets -- -D warnings
cargo test --workspace --exclude web_channel
cargo fmt --all --check
cargo run
```

> `web_channel` is excluded until M3 fixes its web-push API mismatch — see
> the TODO banner at the top of `channels/web_channel/src/lib.rs`.

## API endpoints

| Endpoint | Surface | Purpose |
|---|---|---|
| `GET /` | ops | Service identification (name, version, milestone) |
| `GET /healthz` | ops | Liveness — the process is up |
| `GET /readyz` | ops | Readiness — Postgres + Redis reachable (503 problem doc otherwise) |
| `GET /routes` | ops | Route catalog — every endpoint across all surfaces |
| project surface | project | *(empty until M2 — product endpoints mount at the root)* |
| `/v1/auth/*` | user/v1 | Auth: signup, login, logout, me, OAuth, password flows, verify-email, onboarding-complete |
| `/v1/providers` | user/v1 | Provider registry — all channels + providers + config fields (GET) |
| `/v1/projects/{id}/channel-configs` | user/v1 | Channel configs CRUD (list, create, update, delete) |
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
- Database and Redis are **optional**: without them the API still boots and
  serves `/healthz`; `/readyz` reports 503. `docker-compose.yml` provides both
  locally (edit the `CHANGE_ME` values first, then `docker compose up -d`).
  A **remote** database works too: put its URL in a gitignored `.env` file
  next to `Cargo.toml` (`NOTIFI_DATABASE_URL=...`) — loaded automatically at
  boot; real environment variables still win over file values.
- Telemetry filter: `NOTIFI_LOG` (falls back to `RUST_LOG`, then `info`).
- **Brand assets** (tenant channel configs + templates) live at
  `assets/brands/{brand}/` (relative to `infrastructure/`):

  ```text
  assets/brands/{brand}/config/{channel}/...      channel configs
  assets/brands/{brand}/templates/{name}/...      brand-scoped templates
  ```

  The config root is overridable via `NOTIFI_CONFIG_ROOT`.
  `infrastructure/.cargo/config.toml` sets it to `./assets` for local
  development, so `cargo run`/`cargo test` work without flags.

## Database reset

Nuke the database and re-apply migrations from scratch:

```shell
cargo run -- --reset-db    # drops public schema, re-applies 0001_initial_schema.sql, exits
cargo run                  # normal server start
```

> Use this when migration history is stale (e.g., after deleting migration files)
> or when you want a clean dev start without touching Docker volumes.
