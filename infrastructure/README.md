# Notifi Server

Rust backend for the Notifi notification platform — a modular monolith
implemented as a Cargo workspace.

## Where to start

New to this codebase? Follow this order:

1. **This README** — workspace map + commands (you are here).
2. **`docs/ARCHITECTURE.md`** — the full design: layers, domain template,
   conventions, M0–M8 roadmap. *Read this before touching any code.*
3. **`crates/api/`** — the HTTP API binary. This is the runnable entry point
   of the server (`cargo run -p api`); everything else is a library it wires
   together at startup.
4. **`Cargo.toml`** — the workspace index: its `[workspace] members` list is
   the map of every crate in the project.

## Workspace Layout

```
crates/
├── api/               # HTTP API binary (composition root; axum from M1)
├── core/              # notifi-core: errors, ULID ids, events, outbox, config
├── domain-ports/      # trait-only contracts (DeliveryProvider, EventBus, ...)
├── infra/
│   ├── config/        # layered NOTIFI_* env/file configuration
│   └── telemetry/     # tracing/logging bootstrap
└── adapters/
    └── channels/      # 13 channel crates (email, sms, fcm, apns, ...)
docs/                  # ARCHITECTURE.md (design), SERVER.md (requirements)
migrations/            # sqlx migrations (PostgreSQL schemas)
```

Other root files: `Cargo.toml` (workspace index), `rust-toolchain.toml`
(pinned toolchain), `.cargo/config.toml` (dev env: `NOTIFI_CONFIG_ROOT`),
`target/` (build output, gitignored).

## Commands

Run from this directory (`infrastructure/`). A full reference with explanations
lives in [`COMMANDS.md`](./COMMANDS.md).

```shell
cargo check --workspace --exclude web_channel
cargo clippy --workspace --exclude web_channel --all-targets -- -D warnings
cargo test --workspace --exclude web_channel
cargo fmt --all --check
cargo run -p api
```

> `web_channel` is excluded until M3 fixes its web-push 0.9 API mismatch.

## API endpoints (M1)

| Endpoint | Purpose |
|---|---|
| `GET /` | Service identification (name, version, milestone) |
| `GET /healthz` | Liveness — the process is up |
| `GET /readyz` | Readiness — Postgres + Redis reachable (503 problem doc otherwise) |
| any other path | RFC 9457 `application/problem+json` 404 |

Every response carries a `x-request-id` header for log correlation.
Errors are RFC 9457 problem documents: `{type, title, status, detail, correlation_id}`.

## Configuration

- Application config: `NOTIFI_HOST`, `NOTIFI_PORT`, `NOTIFI_LOG`,
  optional `NOTIFI_CONFIG_FILE` (JSON).
- Database: `NOTIFI_DATABASE_URL` (e.g. `postgres://user:pass@localhost:5432/notifi`).
  When set, the API connects and applies the `migrations/` folder at boot.
- Redis: `NOTIFI_REDIS_URL` (e.g. `redis://:password@localhost:6379`).
- Database and Redis are **optional**: without them the API still boots and
  serves `/healthz`; `/readyz` reports 503. `docker-compose.yml` provides both
  (edit the `CHANGE_ME` values first, then `docker compose up -d`).
- Telemetry filter: `NOTIFI_LOG` (falls back to `RUST_LOG`, then `info`).
- **Brand assets** (tenant channel configs + templates) live at
  `assets/brands/{brand}/`:

  ```text
  assets/brands/{brand}/config/{channel}/...      channel configs
  assets/brands/{brand}/templates/{name}/...      brand-scoped templates
  ```

  The config root is overridable via `NOTIFI_CONFIG_ROOT`.
  `infrastructure/.cargo/config.toml` sets it to `assets` for local
  development, so `cargo run`/`cargo test` work without flags.
