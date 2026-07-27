# AGENTS.md — Notifi

Dual-workspace repository containing an enterprise notification platform dashboard and Rust backend services.

## Repository Layout

| Workspace | Directory | Tech Stack |
|-----------|-----------|------------|
| **Dashboard** | `app/dashboard/` | Next.js 16 (App Router), React 19, Tailwind CSS v4, shadcn/v4 (Base UI) |
| **Server** | `server/` | Rust (Cargo workspace, 15 member crates) |

*Note: Workspaces are independent — run commands from their respective directories.*

---

## Dashboard (`app/dashboard/`)

### Commands
```shell
npm run dev      # Dev server (port 3000)
npm run build    # Production build & TypeScript check
npm run lint     # ESLint checks
```

### File Structure & Paths
- **No `src/` directory**: All code lives directly under `app/dashboard/`.
  - Global CSS: `app/globals.css` (Tailwind v4 with `@theme inline`)
  - App Router: `app/(dashboard)/` and `app/layout.tsx`
  - Components: `components/ui/` (shadcn/v4 Base UI) and `components/custom/`
  - Hooks: `hooks/`
  - State: `store/` (Zustand)
  - Types & Constants: `lib/types.ts`, `lib/constants.ts`

### Framework & Library Conventions
- **Route Params**: Route `params` in Next.js 16 are Promises. Use `use(params)` in Client Components (`"use client"`) or `await params` in Server Components.
- **Base UI Primitives**: shadcn/v4 components use `@base-ui/react` primitives. Use `render` prop instead of Radix `asChild` (e.g. `<DropdownMenuTrigger render={<Button ... />} />`).
- **React 19 & Hydration Rules**:
  - Do NOT call impure functions (`Math.random()`, `Date.now()`) directly inside component render functions.
  - Do NOT invoke synchronous `setState` inside `useEffect` (triggers React Compiler errors).
  - Use `useSyncExternalStore` or client mount checks to defer theme providers and prevent SSR hydration mismatch warnings.

---

## Backend Server (`server/`)

### Commands
```shell
cargo check                     # Check workspace
cargo test                      # Run unit/integration tests
cargo run -p server             # Run main HTTP server executable
cargo test -p email_channel     # Test a specific member crate
```

### Workspace Structure & Package Names
- Main executable package name is **`server`** (located in `server/main/`). Run with `cargo run -p server` (not `main`).
- Shared traits: `notification_core` (`server/notification/core`).
- Channel implementations: `notification/channels/*` (`email_channel`, `sms_channel`, `slack_channel`, etc.).

### Build & Test Quirk
- `web_channel` currently has `web-push` API mismatch compile errors. To check or test the rest of the workspace:
  ```shell
  cargo check --workspace --exclude web_channel
  cargo test --workspace --exclude web_channel
  ```

### Architecture & Config Gotchas
- **No Cyclic Dependencies**: Channel crates depend on `notification_core`, never on `server` or each other.
- **Runtime Tenant Config**: Configs are loaded at runtime from `server/configs/{brand}/{channel_name}/`. Each channel crate parses its own config via `ChannelConfigLoader`.
