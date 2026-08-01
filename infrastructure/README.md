# Notifi Server

Rust backend for the Notifi notification platform — a modular monolith
implemented as a Cargo workspace.

See [`ARCHITECTURE.md`](ARCHITECTURE.md) for the full design (layers, domain
template, conventions, roadmap).

## Workspace Layout

```
crates/
├── api/               # HTTP API binary (composition root; axum from M1)
├── core/              # notifi-core: errors, ULID ids, events, outbox, config
├── domain-ports/      # trait-only contracts (DeliveryProvider, EventBus, ...)
├── infra/
│   ├── config/        # layered NOTIFI_* env/file configuration
│   └── telemetry/     # tracing/logging bootstrap
└── adapters/          # (M3) channel adapters move here
notification/channels/ # 13 channel crates (email, sms, fcm, apns, ...)
migrations/            # sqlx migrations (from M1)
```

## Commands

Run from this directory (`infrastructure/`):

```shell
cargo check --workspace --exclude web_channel
cargo clippy --workspace --exclude web_channel --all-targets -- -D warnings
cargo test --workspace --exclude web_channel
cargo fmt --all --check
cargo run -p api
```

> `web_channel` is excluded until M3 fixes its web-push 0.9 API mismatch.

## Configuration

- Application config: `NOTIFI_HOST`, `NOTIFI_PORT`, `NOTIFI_LOG`,
  optional `NOTIFI_CONFIG_FILE` (JSON).
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
