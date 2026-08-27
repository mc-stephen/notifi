# AGENTS.md — Notifi

Dual-workspace repository containing an enterprise notification platform dashboard and Rust backend services.

## Repository Layout

| Workspace | Directory | Tech Stack |
|-----------|-----------|------------|
| **Dashboard** | `app/dashboard/` | Next.js 16 (App Router), React 19, Tailwind CSS v4, shadcn/v4 (Base UI) |
| **Server** | `infrastructure/` | Rust (single `server` crate + channel plugin crates) |

*Run commands from workspace subdirectories (`app/dashboard/` or `infrastructure/`).*

---

## Dashboard (`app/dashboard/`)

### Commands
```shell
npm run dev      # Dev server (port 3000)
npm run build    # Production build & TypeScript check
npm run lint     # ESLint checks
```

### Route Structure
- **Dashboard (`/`)**: `app/(dashboard)/` — route group for top-level pages (`/`, `/settings`, `/profile`, `/billing`).
- **Auth (`/auth/*`)**: `app/auth/` — non-parenthesized directory preserving `/auth/` URL path (`/auth/login`, `/auth/signup`, `/auth/password/forgot`, `/auth/password/reset`, `/auth/verify-email`).
- **Onboarding (`/onboarding/*`)**: `app/onboarding/` — non-parenthesized directory preserving `/onboarding/` URL path (`/onboarding/welcome`, `/onboarding/use-case`, `/onboarding/organization`, `/onboarding/project`, `/onboarding/setup-channels`, `/onboarding/invite-team`, `/onboarding/success`).

*Note: Route groups with parentheses (like `(auth)`) strip the folder name from the URL path. Regular folders without parentheses preserve the path segment. Do NOT place auth or onboarding in parenthesized route groups if URL path prefixes are required.*

### File Structure & Paths
- **No `src/` directory**: All code lives directly under `app/dashboard/`.
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

## API Contract Docs

- **Auth**: `app/dashboard/app/auth/API_CONTRACT.md` — expected request/response shapes for login, signup, OAuth, forgot/reset password, verify email.
- **Onboarding**: `app/dashboard/app/onboarding/API_CONTRACT.md` — data collected per step and proposed endpoints (flow is currently client-side only).

**Rule for any AI model or developer**: these files document the frontend's expected API contract. If you change an auth or onboarding page's inputs/outputs, validation, redirects, or the matching backend implementation, you MUST update the relevant contract file in the same change. (Dashboard pages have no contract file yet.)

---

## Backend Server (`infrastructure/`)

The server is a **single crate** rooted at `infrastructure/` (`src/{api,domain,ports,infra,testing}`), plus independent channel plugin crates in `infrastructure/channels/*`. Two independent API surfaces: **project** (product API at the root; API-key auth from M6) and **user** (dashboard backend under `/v1`, versioned).

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
- **API surfaces**: route registry per surface lives in `src/api/catalog.rs` (drives `GET /routes`). Project endpoints mount in `src/api/project/mod.rs` — empty until M2 lands product endpoints. Dashboard endpoints live under `src/api/user/` and mount at `/v1/auth/*` today.
- **Shared kernel crates**: `crates/core` (errors, ULID ids, events, outbox, config resolver) and `crates/domain-ports` (trait-only contracts) stay framework-free — keep them that way; the channel plugins will consume them in M3.
- **Known Build Issue**: `web_channel` is currently broken (web-push API mismatch). Always include `--exclude web_channel` when running workspace commands. A fix-checklist TODO sits at the top of `channels/web_channel/src/lib.rs`.
- **Runtime Tenant Config**: Configs load dynamically at runtime from `{NOTIFI_CONFIG_ROOT}/brands/{brand}/config/{channel_name}/` (root defaults to `configs`; local dev sets it to `infrastructure/assets` via `infrastructure/.cargo/config.toml`). Channels parse their own config; use `notifi_core::config::ConfigResolver::load_json` for new code. Brand templates live at `{NOTIFI_CONFIG_ROOT}/brands/{brand}/templates/{name}/`.
- **Layout**: server-owned data and docs live under `infrastructure/assets/` (`brands/`, `docs/`, `migrations/`); migrations apply from `assets/migrations/` at boot.
