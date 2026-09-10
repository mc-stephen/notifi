# AGENTS.md — Notifi

Notification-platform-as-a-service monorepo: customer dashboard, internal admin console, marketing/docs/status sites, Rust backend.

## Repository Layout

| Workspace | Directory | Tech Stack | Dev port |
|-----------|-----------|------------|----------|
| **Dashboard** | `app/user-dashboard/` | Next.js 16 (App Router), React 19, Tailwind v4, shadcn/v4 (Base UI) | 3000 |
| **Admin** | `app/admin-dashboard/` | Astro (SSR) + React islands | 4321* |
| **Landing** | `app/landing/` | Astro (SSG) | 4321* |
| **Docs** | `app/documentation/` | Next.js + Fumadocs | 3000* |
| **Status** | `app/status/` | Astro (SSG), static-first | 4321* |
| **Server** | `infrastructure/` | Rust (single `server` crate + channel plugin crates) | 8080 |
| **SDKs / shared** | `sdks/`, `shared/` | Spec docs only (no code yet) / brand tokens | — |

*\*Astro defaults to 4321, Next.js to 3000 — pass `--port` when running two of the same kind. Run commands from the workspace subdirectory.*

Per-app details: `app/admin-dashboard/AGENTS.md`, `app/status/AGENTS.md`, `app/user-dashboard/AGENTS.md`.

---

## Dashboard (`app/user-dashboard/`)

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
- **No `src/` directory**: All code lives directly under `app/user-dashboard/`.
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

- **Admin** (`app/admin-dashboard/`) — internal console, Astro SSR + React islands, talks to the Rust server via `PUBLIC_API_URL`. See `app/admin-dashboard/AGENTS.md` for data/modal/tab conventions.
- **Status** (`app/status/`) — public status page, Astro SSG, zero client JS (server-rendered SVG charts only). See `app/status/AGENTS.md`.
- **Landing** (`app/landing/`) — marketing site, Astro SSG. Canonical design in `DESIGN.md`.
- **Docs** (`app/documentation/`) — Next.js + Fumadocs, content in `content/`.

Brand source of truth for all of them: `shared/identity/` (colors, typography, logos). `sdks/` holds spec docs only — no code yet.

---

## CI & Formatting

`.github/workflows/server.yml` covers `infrastructure/**` only (no frontend CI): `cargo check`, `cargo clippy -- -D warnings`, `cargo test`, plus **`cargo fmt --all --check` — run `cargo fmt` before pushing Rust changes.** All workspace commands take `--exclude web_channel` (see Backend section).

---

## Product Rules & Spec Pointers

- Root `Readme.md`: billing is **per project, not per account**; users capped at 10 orgs; "provider" pages register vendors, "channel" pages configure routing onto them. Honor these when touching billing/channel code.
- `app/*.md` are the original build prompts — treat `app/admin.md` (roles, permissions, auditability) as spec when adding admin features.

---

## API Contract Docs

- **Auth**: `app/user-dashboard/app/auth/API_CONTRACT.md` — expected request/response shapes for login, signup, OAuth, forgot/reset password, verify email.
- **Onboarding**: `app/user-dashboard/app/onboarding/API_CONTRACT.md` — data collected per step and proposed endpoints (flow is currently client-side only).

**Rule for any AI model or developer**: these files document the frontend's expected API contract. If you change an auth or onboarding page's inputs/outputs, validation, redirects, or the matching backend implementation, you MUST update the relevant contract file in the same change. (Dashboard pages have no contract file yet.)

---

## Backend Server (`infrastructure/`)

The server is a **single crate** rooted at `infrastructure/` (`src/{api,domain,ports,infra,testing}`), plus independent channel plugin crates in `infrastructure/channels/*`. API layout (see root `Readme.md`): ops at `/`, external product API at `/v1/*` (empty until M2; API-key auth from M6), user dashboard backend at `/app/*`, administration at `/admin/*`.

### Commands

Run from `infrastructure/`:

```shell
cargo check --workspace --exclude web_channel     # Check workspace
cargo clippy --workspace --exclude web_channel --all-targets -- -D warnings
cargo test --workspace --exclude web_channel      # Run unit/integration tests
cargo run                                         # Run main server executable
```

### Architecture & Quirks

- **Design Doc**: See `infrastructure/assets/docs/ARCHITECTURE.md` — layering rules, conventions, M0–M8 roadmap. §2 documents the current single-crate layout (the doc's earlier multi-crate history is noted at its top).
- **Module boundaries** (single crate, enforced by visibility): `src/api/` = HTTP presentation only · `src/domain/` = framework-free business logic · `src/ports/` = trait contracts (`AuthStore`, `OAuthIdentityProvider`) implemented by `infra/` · `src/infra/` = concrete drivers (sqlx, reqwest, config, telemetry). Keep axum/sqlx/reqwest out of `domain/` and `ports/`.
- **API surfaces**: route registry per surface lives in `src/api/catalog.rs` (drives `GET /routes`). Product endpoints mount in `src/api/project/mod.rs` under `/v1` — empty until M2 lands product endpoints. Dashboard endpoints live under `src/api/user/` and mount at `/app/*`; admin endpoints mount at `/admin/*`.
- **Shared kernel crates**: `crates/core` (errors, ULID ids, events, outbox, config resolver) and `crates/domain-ports` (trait-only contracts) stay framework-free — keep them that way; the channel plugins will consume them in M3.
- **Known Build Issue**: `web_channel` is currently broken (web-push API mismatch). Always include `--exclude web_channel` when running workspace commands. A fix-checklist TODO sits at the top of `channels/web_channel/src/lib.rs`.
- **Runtime Tenant Config**: Configs load dynamically at runtime from `{NOTIFI_CONFIG_ROOT}/brands/{brand}/config/{channel_name}/` (root defaults to `configs`; local dev sets it to `infrastructure/assets` via `infrastructure/.cargo/config.toml`). Channels parse their own config; use `notifi_core::config::ConfigResolver::load_json` for new code. Brand templates live at `{NOTIFI_CONFIG_ROOT}/brands/{brand}/templates/{name}/`.
- **Layout**: server-owned data and docs live under `infrastructure/assets/` (`brands/`, `docs/`, `migrations/`); migrations apply from `assets/migrations/` at boot.
- **Migration gotcha**: `sqlx::migrate!` embeds at compile time but a *new* migration file alone doesn't trigger a rebuild — after adding one, force recompilation (e.g. `touch src/lib.rs`) or the running binary silently keeps the old set. Mismatched Rust/SQL integer types surface only at runtime as decode errors, so keep `INT8`/`BIGINT` in sync with `i64`.
