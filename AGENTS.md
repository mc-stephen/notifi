# AGENTS.md — Notifi

Dual-workspace repository containing an enterprise notification platform dashboard and Rust backend services.

## Repository Layout

| Workspace | Directory | Tech Stack |
|-----------|-----------|------------|
| **Dashboard** | `app/dashboard/` | Next.js 16 (App Router), React 19, Tailwind CSS v4, shadcn/v4 (Base UI) |
| **Server** | `server/` | Rust (Cargo workspace, 15 member crates) |

*Run commands from workspace subdirectories (`app/dashboard/` or `server/`).*

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

## Backend Server (`server/`)

### Commands
```shell
cargo check --workspace --exclude web_channel     # Check workspace
cargo test --workspace --exclude web_channel      # Run unit/integration tests
cargo run -p server                                # Run main HTTP server executable
```

### Architecture & Workspace Quirks
- **Main Executable**: Package name is **`server`** (in `server/main/`). Run with `cargo run -p server` (not `main`).
- **Crate Architecture**: 15 member crates. Channel crates depend on `notification_core` (`server/notification/core`), never on `server` or each other.
- **Known Build Issue**: `web_channel` currently has `web-push` API mismatch errors. Always include `--exclude web_channel` when running workspace commands.
- **Runtime Tenant Config**: Configs load dynamically at runtime from `server/configs/{brand}/{channel_name}/`. Each channel crate parses its own config via `ChannelConfigLoader`.
