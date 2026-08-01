# AGENTS.md — Notifi

Dual-workspace repository containing an enterprise notification platform dashboard and Rust backend services.

## Repository Layout

| Workspace | Directory | Tech Stack |
|-----------|-----------|------------|
| **Dashboard** | `app/dashboard/` | Next.js 16 (App Router), React 19, Tailwind CSS v4, shadcn/v4 (Base UI) |
| **Server** | `infrastructure/` | Rust (Cargo workspace, 18 member crates) |

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
- **Auth (`/auth/*`)**: `app/auth/` — non-parenthesized directory preserving `/auth/` URL path (`/auth/login`, `/auth/signup`, `/auth/forgot-password`, `/auth/reset-password`, `/auth/verify-email`).
- **Onboarding (`/onboarding/*`)**: `app/onboarding/` — non-parenthesized directory preserving `/onboarding/` URL path (`/onboarding/welcome`, `/onboarding/use-case`, `/onboarding/organization`, `/onboarding/project`, `/onboarding/api-key`, `/onboarding/setup-channels`, `/onboarding/invite-team`, `/onboarding/success`).

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

## Backend Server (`infrastructure/`)

### Commands
```shell
cargo check --workspace --exclude web_channel     # Check workspace
cargo clippy --workspace --exclude web_channel --all-targets -- -D warnings
cargo test --workspace --exclude web_channel      # Run unit/integration tests
cargo run -p api                                  # Run main API executable
```

### Architecture & Workspace Quirks
- **Design Doc**: See `infrastructure/ARCHITECTURE.md` — the authoritative architecture (layers, domain template, conventions, M0–M8 roadmap).
- **Main Executable**: Package name is **`api`** (in `crates/api/`). Run with `cargo run -p api` (M1 adds the axum server; M0 is a bootstrap + SMTP smoke test).
- **Crate Architecture**: Workspace members are `crates/` (core, domain-ports, infra, api) plus 13 channel crates under `notification/channels/`. Channel crates must never depend on each other or on domains; they will adopt the `notifi_domain_ports::DeliveryProvider` trait in M3.
- **Framework-free Kernel**: `crates/core` (errors, ULID ids, events, outbox) and `crates/domain-ports` (traits) have no axum/sqlx/redis deps — keep it that way.
- **Known Build Issue**: `web_channel` currently has `web-push` API mismatch errors. Always include `--exclude web_channel` when running workspace commands (until M3 fixes it).
- **Runtime Tenant Config**: Configs load dynamically at runtime from `{NOTIFI_CONFIG_ROOT}/brands/{brand}/config/{channel_name}/` (root defaults to `configs`; local dev sets it to `assets` via `infrastructure/.cargo/config.toml`). Channels parse their own config; use `notifi_core::config::ConfigResolver::load_json` for new code. Brand templates live at `{NOTIFI_CONFIG_ROOT}/brands/{brand}/templates/{name}/`.
