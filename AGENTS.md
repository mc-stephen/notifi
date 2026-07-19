# AGENTS — Notifi

## Layout

Two independent workspaces — no monorepo tooling:

| Directory | Stack | Purpose |
|-----------|-------|---------|
| `app/dashboard/` | Next.js 15, TypeScript, Tailwind v4, shadcn/v4 (Base UI) | NaaS Dashboard |
| `server/` | Rust, Cargo workspace (13 member crates) | Backend services |

---

## Dashboard (`app/dashboard/`)

- **Framework:** Next.js 15 App Router, React 19
- **Styling:** Tailwind CSS v4 — all theme values live in `src/app/globals.css` via `@theme inline`. No `tailwind.config.ts`.
- **UI kit:** shadcn/v4 with Base UI primitives (not Radix)
- **State:** Zustand stores in `src/store/`
- **Forms:** React Hook Form + Zod (resolver via `@hookform/resolvers`)
- **Charts:** Recharts
- **Virtual list:** `@tanstack/react-virtual`

### Commands (run from `app/dashboard/`)

```shell
npm run dev        # dev server (port 3000)
npm run build      # production build + typecheck
npm run lint       # ESLint via next lint
```

### Key files
- `src/app/globals.css` — Tailwind v4 palette, `@theme inline`, glass/card-lift utilities
- `src/app/(dashboard)/layout.tsx` — sidebar + topbar layout shell
- `src/store/` — Zustand stores (`tenant-store`, `environment-store`)
- `src/hooks/` — mock-data hooks (swap for real API later)
- `src/lib/types.ts` — all shared TS types
- `src/lib/constants.ts` — tenants, nav items, channel/status labels

---

## Server (`server/`)

Cargo workspace with `resolver = "2"` and 13 member crates:

| Crate | Role |
|-------|------|
| `main` | HTTP server entrypoint |
| `notification_core` | Shared traits/types (avoids circular deps) |
| `notification_channels/*` | Per-channel delivery implementations (email, sms, push, slack, telegram, webhook, etc.) |

### Commands (run from `server/`)
```shell
cargo build            # builds all workspace members
cargo test             # tests all workspace members
cargo run -p main      # start the HTTP server
```

### Config Architecture
Tenant configs are loaded at runtime from `server/configs/{brand}/{channel_name}/`. Each channel owns its config parsing via the `ChannelConfigLoader` trait in `notification_core`. Files are never compiled in — they're created at runtime (e.g. when a user registers a tenant).

Example layout:
```
server/configs/acme/email/config.json
server/configs/acme/fcm/service-account.json
server/configs/acme/slack/config.env
```

### Gotchas
- **Cyclic dependencies** between `main` and channel crates will fail. Define shared traits in `notification_core`, never import `main` from a channel crate.
- **Config parsing** lives in the channel crate, not in `notification_core`. The core only provides `ConfigResolver` for path resolution and `ConfigError` for error handling.
