# AGENTS.md — Notifi

Notification-platform-as-a-service monorepo: customer dashboard, internal admin console, marketing/docs/status sites, Rust backend.

## Repository Layout

| Workspace | Directory | Tech Stack | Dev port |
|-----------|-----------|------------|----------|
| **Dashboard** | `app/dashboard/user/` | Next.js 16 (App Router), React 19, Tailwind v4, shadcn/v4 (Base UI) | 3000 |
| **Admin** | `app/dashboard/admin/` | Astro (SSR) + React islands | 4321* |
| **Landing** | `app/landing/` | Astro (SSG) | 4321* |
| **Docs** | `app/documentation/` | Next.js + Fumadocs | 3000* |
| **Status** | `app/status/` | Astro (SSG), static-first | 4321* |
| **Server** | `infrastructure/` (code in `server/`, `worker/`) | Rust (workspace: `server` + `worker` crates) | 8080 |
| **SDKs / shared** | `sdks/`, `shared/` | Spec docs only (no code yet) / brand tokens | — |

*\*Astro defaults to 4321, Next.js to 3000 — pass `--port` when running two of the same kind. Run commands from the workspace subdirectory.*

Per-app details: `app/dashboard/admin/AGENTS.md`, `app/status/AGENTS.md`, `app/dashboard/user/AGENTS.md`.

---

## Dashboard (`app/dashboard/user/`)

### Commands
```shell
npm run dev      # Dev server (port 3000)
npm run build    # Production build & TypeScript check
npm run lint     # ESLint checks
```

### Route Structure
- **Dashboard (`/`)**: `app/(dashboard)/` — route group for top-level pages (`/`, `/settings`, `/profile`, `/billing`).
- **Auth (`/auth/*`)**: `app/auth/` — non-parenthesized directory preserving `/auth/` URL path (`/auth/login`, `/auth/signup`, `/auth/password/forgot`, `/auth/password/reset`, `/auth/verify-email`).
- **Onboarding (`/onboarding/*`)**: `app/onboarding/` — non-parenthesized directory preserving `/onboarding/` URL path (`/onboarding/welcome`, `/onboarding/use-case`, `/onboarding/project`, `/onboarding/setup-channels`, `/onboarding/invite-team`, `/onboarding/success`).

*Note: Route groups with parentheses (like `(auth)`) strip the folder name from the URL path. Regular folders without parentheses preserve the path segment. Do NOT place auth or onboarding in parenthesized route groups if URL path prefixes are required.*

### File Structure & Paths
- **No `src/` directory**: All code lives directly under `app/dashboard/user/`.
  - Global CSS: `app/globals.css` (Tailwind v4 with `@theme inline`)
  - App Router: `app/(dashboard)/`, `app/auth/`, `app/onboarding/`, and `app/layout.tsx`
  - Components: `components/ui/` (shadcn/v4 Base UI) and `components/custom/`
  - Hooks: `hooks/`
  - State: `store/` (Zustand)
  - Types & Constants: `lib/types.ts`, `lib/constants.ts`

### Framework & Library Conventions
- **Route Params**: Route `params` in Next.js 16 are Promises. Use `use(params)` in Client Components (`"use client"`) or `await params` in Server Components.
- **Base UI Primitives**: shadcn/v4 components use `@base-ui/react` primitives. Use `render` prop instead of Radix `asChild` (e.g., `<DropdownMenuTrigger render={<Button ... />} />`).
- **Select Handler Typing**: Base UI `Select` `onValueChange` passes `(value: string | null, eventDetails)`. Wrap state setters: `onValueChange={(v) => v && setSelected(v)}`.
- **React 19 & Hydration Rules**:
  - Do NOT call impure functions (`Math.random()`, `Date.now()`) directly inside component render functions.
  - Do NOT invoke synchronous `setState` inside `useEffect` (triggers React Compiler errors).
  - Use `useSyncExternalStore` or client mount checks to defer theme providers and prevent SSR hydration mismatch warnings.
- **next-themes Warning**: `providers.tsx` includes a dev-only `console.error` filter to suppress the React 19 "Encountered a script tag" false positive from `next-themes` script injection.

---

## Other Frontends

Astro apps expose `dev` / `build` / `check` / `preview`; docs exposes `dev` / `build` / `lint`. Run from the workspace subdirectory.

- **Admin** (`app/dashboard/admin/`) — internal console, Astro SSR + React islands, talks to the Rust server via `PUBLIC_API_URL`. See `app/dashboard/admin/AGENTS.md` for data/modal/tab conventions.
- **Status** (`app/status/`) — public status page, Astro SSG, zero client JS (server-rendered SVG charts only). See `app/status/AGENTS.md`.
- **Landing** (`app/landing/`) — marketing site, Astro SSG. Canonical design in `DESIGN.md`.
- **Docs** (`app/documentation/`) — Next.js + Fumadocs, content in `content/`.

Brand source of truth for all of them: `shared/identity/` (colors, typography, logos). `sdks/` holds spec docs only — no code yet.

---

## CI & Formatting

`.github/workflows/server.yml` covers `infrastructure/**` only (no frontend CI): `cargo check`, `cargo clippy -- -D warnings`, `cargo test`, plus **`cargo fmt --all --check` — run `cargo fmt` before pushing Rust changes.**

---

## Product Rules & Spec Pointers

- Root `Readme.md`: billing is **per project, not per account**; users capped at 10 orgs; "provider" pages register vendors, "channel" pages configure routing onto them. Honor these when touching billing/channel code.
- `app/*.md` are the original build prompts — treat `app/admin.md` (roles, permissions, auditability) as spec when adding admin features.

---

## API Contract Docs

- **Auth**: `app/dashboard/user/app/auth/API_CONTRACT.md` — expected request/response shapes for login, signup, OAuth, forgot/reset password, verify email.
- **Onboarding**: `app/dashboard/user/app/onboarding/API_CONTRACT.md` — data collected per step and proposed endpoints (flow is currently client-side only).

**Rule for any AI model or developer**: these files document the frontend's expected API contract. If you change an auth or onboarding page's inputs/outputs, validation, redirects, or the matching backend implementation, you MUST update the relevant contract file in the same change. (Dashboard pages have no contract file yet.)

---

## Backend Server (`infrastructure/`)

The server is a **single crate** rooted at `infrastructure/` (`src/{api,channels,domain,kernel,ports,infra}`; test fakes live under `tests/`). Channel plugins live as modules under `src/channels/` grouped by channel family — `email/`, `sms/`, `push/`, `chat/` (slack, telegram, whatsapp, discord, ms-teams as providers), and `device/` (android, ios, macos, linux, window, and web as device flavors sharing one destination class) — plus `webhook/`. API layout (see root `Readme.md`): ops at `/`, external product API at `/v1/*` (empty until M2; API-key auth from M6), user dashboard backend at `/app/*`, administration at `/admin/*`.

### Commands

Run from `infrastructure/`:

```shell
cargo check --workspace     # Check workspace
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace      # Run unit/integration tests
cargo run -p server         # Run main server executable (use -p: two binaries)
```

### Architecture & Quirks

- **Design Doc**: See `infrastructure/server/assets/docs/ARCHITECTURE.md` — layering rules, conventions, M0–M8 roadmap. §2 documents the current single-crate layout (the doc's earlier multi-crate history is noted at its top).
- **Module boundaries** (single crate, enforced by visibility): `server/src/api/` = HTTP presentation only · `server/src/domain/` = framework-free business logic · `server/src/kernel/` = framework-free shared kernel (errors, ULID ids, events, outbox, config) · `server/src/ports/` = trait contracts (`AuthStore`, `OAuthIdentityProvider`, `DeliveryProvider`, `QueuePublisher`, …) implemented by `infra/` · `server/src/infra/` = concrete drivers (sqlx, reqwest, config, telemetry) · `server/src/channels/` = channel delivery modules. Keep axum/sqlx/reqwest out of `domain/` and `ports/`.
- **API surfaces**: route registry per surface lives in `server/src/api/catalog.rs` (drives `GET /routes`). The three audience route surfaces (`app`, `admin`, `external`) live under `server/src/api/routes/`; product endpoints mount in `server/src/api/routes/external/mod.rs` under `/v1` — empty until M2 lands product endpoints. Dashboard endpoints live under `server/src/api/routes/app/` and mount at `/app/*`; admin endpoints live under `server/src/api/routes/admin/` and mount at `/admin/*`. Cross-cutting middleware stays at `server/src/api/middleware/`.
- **Shared kernel**: `server/src/kernel/` (errors, ULID ids, events, outbox, config resolver) — folded from the old `crates/core` crate; `server/src/ports/` now also includes `channel`, `delivery`, `event_bus`, `queue` from the old `crates/domain-ports`. All framework-free.
- **Known Build Issue**: none — `web_channel` was fixed (web-push 0.9.5 API migration) and compiles as part of the workspace.
- **Database backups**: self-contained block in `infrastructure/worker/src/backup.rs` on its own clock (never the queue) — daily `pg_dump --format=plain` streamed in 8 MiB parts to a dedicated R2 bucket as one plain-text `.sql` object, temp file deleted, **nothing ever auto-deleted**. On only when `NOTIFI_BACKUP_R2_BUCKET` is set (creds reuse the `NOTIFI_R2_*` trio; slot `NOTIFI_BACKUP_AT` default `03:00`; binary `NOTIFI_BACKUP_PG_DUMP`; target `NOTIFI_BACKUP_DB_URL` else `NOTIFI_DATABASE_URL`). `cargo run -p worker -- --backup-now` = one dump-and-upload now. Boot-dumps immediately when today's backup is missing. Hosts need the Postgres client tools.
- **Runtime Tenant Config**: brand configs + template assets address through `ports::ConfigStore` — keys relative to the convention root (`brands/{brand}/config/{channel}/…`, `brands/{brand}/templates/{name}/…`), named by `crate::kernel::config::ConfigResolver::channel_key` / `template_key`. **The default layout is code**: `kernel::config::DefaultBrandLayout` owns the default file set (per-channel config + starter templates) and `ConfigResolver::brand_slug` slugifies project names (`"The Team"` → `the_team`) — storage holds no defaults; a fresh brand materializes via the port's `materialize_brand()` when its project is created, and `--reset-config` (also chained after a successful `--reset-db`) wipes `brands/` back to empty through `delete_raw`. Root: `NOTIFI_CONFIG_ROOT` (defaults to `configs`; local dev sets it to `infrastructure/server/assets` via `infrastructure/.cargo/config.toml`). Driver picks live in `infra::config_store_factory`, shared by boot and ops: file store by default, Cloudflare R2 object store when all four `NOTIFI_R2_ACCOUNT_ID/_ACCESS_KEY_ID/_SECRET_ACCESS_KEY/_BUCKET` are set — the bucket is the root, same keys either way. `server/tests/config_store_r2.rs -- --ignored` runs the live reset-to-empty and the `report_brands_tree` bucket look. Reads and writes ride a Redis read-through cache (`infra/config_cache.rs`, `cfg:` namespace, put/delete invalidate; direct store calls when Redis is unset).
- **Layout**: server-owned data and docs live under `infrastructure/server/assets/` (`docs/`, `migrations/`); brand config storage is a backend store, not repo files.
- **Migration gotcha**: `sqlx::migrate!` embeds at compile time but a *new* migration file alone doesn't trigger a rebuild — after adding one, force recompilation (e.g. `touch server/src/lib.rs`) or the running binary silently keeps the old set. Mismatched Rust/SQL integer types surface only at runtime as decode errors, so keep `INT8`/`BIGINT` in sync with `i64`.
