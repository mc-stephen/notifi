# Notifi Server — Command Reference

Every command runs from the `infrastructure/` directory.

> **Why `--exclude web_channel`?** The web channel crate has a web-push 0.9
> API mismatch that is fixed in M3. Until then it breaks any workspace-wide
> build, so every workspace command excludes it.

## 1. Build & check

```shell
# Fast compile check (no codegen) — the "does it compile?" gate
cargo check --workspace --exclude web_channel

# Single crate (faster feedback while iterating)
cargo check -p api
cargo check -p notifi_core

# Full build (production binary)
cargo build --workspace --exclude web_channel
```

## 2. Lint (clippy) — must be zero warnings

```shell
cargo clippy --workspace --exclude web_channel --all-targets -- -D warnings
```

> `-D warnings` turns every warning into an error. CI fails on this command.

## 3. Format

```shell
cargo fmt --all                    # format the code
cargo fmt --all --check            # check only (CI gate — fails if not formatted)
```

## 4. Tests

```shell
# All crates (unit + integration tests)
cargo test --workspace --exclude web_channel

# One crate only
cargo test -p api

# One test by name
cargo test -p api healthz
```

## 5. Run the API

```shell
cargo run -p api
```

Boots on `127.0.0.1:8080` by default. Override with env vars:

```shell
NOTIFI_HOST=0.0.0.0 NOTIFI_PORT=9000 NOTIFI_LOG=debug cargo run -p api
```

Optional external services (without them the API still boots; `/readyz` → 503):

```shell
NOTIFI_DATABASE_URL=postgres://user:pass@localhost:5432/notifi \
NOTIFI_REDIS_URL=redis://:password@localhost:6379 \
cargo run -p api
```

> Migrations in `migrations/` apply automatically at boot when the database
> connection succeeds.

## 6. Local database & Redis (docker)

```shell
# Edit docker-compose.yml first: replace the CHANGE_ME values
docker compose up -d            # start Postgres (with pgmq) + Redis
docker compose ps               # status
docker compose logs -f postgres # follow logs
docker compose down             # stop (keeps data in volumes)
docker compose down -v          # stop AND wipe data (fresh start)
```

## 7. Try the API (curl)

```shell
curl -s http://127.0.0.1:8080/                    # service info
curl -s -i http://127.0.0.1:8080/healthz          # liveness → 200
curl -s -i http://127.0.0.1:8080/readyz           # readiness → 200 or 503
curl -s -i http://127.0.0.1:8080/does-not-exist   # RFC 9457 problem doc (404)
```

Every response includes an `x-request-id` header for log correlation.

## 8. Graceful shutdown

`Ctrl-C` (SIGINT) or `kill -TERM <pid>` stops the server cleanly — in-flight
requests finish, logs say `api shutdown complete`.

## 9. Quick triage

| Symptom | Likely cause / fix |
|---|---|
| `failed to load manifest for workspace member` | run from `infrastructure/`, not `crates/api/` |
| `web_channel` compile errors | expected until M3 — keep using `--exclude web_channel` |
| `/readyz` → 503 | `docker compose up -d` + set `NOTIFI_DATABASE_URL` / `NOTIFI_REDIS_URL` |
| migration warnings about checksums | migrations already applied — normal |
| bind error on port | something else uses 8080 → `NOTIFI_PORT=XXXX cargo run -p api` |
