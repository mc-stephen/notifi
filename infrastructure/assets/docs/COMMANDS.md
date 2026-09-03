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
cargo check
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
cargo test

# One test by name
cargo test --test auth_api healthz
```

## 5. Run the API

```shell
cargo run
```

Boots on `127.0.0.1:8080` by default. Override with env vars:

```shell
NOTIFI_HOST=0.0.0.0 NOTIFI_PORT=9000 NOTIFI_LOG=debug cargo run
```

Optional external services (without them the API still boots; `/readyz` → 503):

```shell
NOTIFI_DATABASE_URL=postgres://user:pass@localhost:5432/notifi \
NOTIFI_REDIS_URL=redis://:password@localhost:6379 \
cargo run
```

> Migrations in `assets/migrations/` apply automatically at boot when the database
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
curl -s http://127.0.0.1:8080/routes | python3 -m json.tool   # route catalog
curl -s -i http://127.0.0.1:8080/does-not-exist   # RFC 9457 problem doc (404)
```

Auth flow (needs a database; dev mode returns one-time tokens in responses):

```shell
# 1. signup → 201, note verificationToken from the response
curl -s -X POST http://127.0.0.1:8080/v1/auth/signup \
  -H 'content-type: application/json' \
  -d '{"name":"Jane","email":"jane@example.com","password":"Sup3rSecret!"}'

# 2. verify email with that token
curl -s -X POST http://127.0.0.1:8080/v1/auth/verify-email \
  -H 'content-type: application/json' -d '{"token":"<verificationToken>"}'

# 3. login → sets the session_token cookie; session.token is also in the body
curl -s -c /tmp/notifi-cookies.txt -X POST http://127.0.0.1:8080/v1/auth/login \
  -H 'content-type: application/json' \
  -d '{"email":"jane@example.com","password":"Sup3rSecret!","rememberMe":true}'

# 4. current user via cookie
curl -s -b /tmp/notifi-cookies.txt http://127.0.0.1:8080/v1/auth/me
```

Every response includes an `x-request-id` header for log correlation.

## 8. Graceful shutdown

`Ctrl-C` (SIGINT) or `kill -TERM <pid>` stops the server cleanly — in-flight
requests finish, logs say `api shutdown complete`.

## 9. Quick triage

| Symptom | Likely cause / fix |
|---|---|
| `failed to load manifest for workspace member` | run from `infrastructure/` root |
| `web_channel` compile errors | expected until M3 — keep using `--exclude web_channel` |
| `/readyz` → 503 | set `NOTIFI_DATABASE_URL` / `NOTIFI_REDIS_URL` (or a gitignored `.env`) |
| `received fatal alert: HandshakeFailure` | server's TLS rejected the handshake — try `sslmode=disable` in the URL and fix the server's TLS config (traffic is then unencrypted!) |
| migration warnings about checksums | migrations already applied — normal |
| bind error on port | something else uses 8080 → `NOTIFI_PORT=XXXX cargo run` |

## 10. Database reset

Nuke the database and re-apply migrations from scratch:

```shell
cargo run -- --reset-db    # drops public schema, re-applies 0001_initial_schema.sql, exits
cargo run                  # normal server start
```

> Use this when migration history is stale (e.g., after deleting migration files)
> or when you want a clean dev start without touching Docker volumes.

### When to use

| Scenario | Command |
|---|---|
| Fresh dev start, no data to keep | `cargo run -- --reset-db` |
| Migration history mismatch | `cargo run -- --reset-db` |
| Want to wipe Docker volumes too | `docker compose down -v && cargo run -- --reset-db` |
