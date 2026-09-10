# Notifi Admin (app/admin-dashboard/)

Internal control plane — Astro (`output: server`, node adapter) + React islands. No public signup; session cookie `admin_session`, all pages SSR behind `middleware.ts`.

## Commands

Run from `app/admin-dashboard/`:

```shell
npm run dev      # Dev server (Astro default :4321 — collides with landing/status, pass --port to run two)
npm run build    # Production build (SSR, no type gate)
npm run check    # astro check — TypeScript checks
```

Backend base URL comes from `PUBLIC_API_URL` (`.env` defaults to Rust server `http://localhost:8080`).

## Conventions

- **Mock data lives in `src/data/`** (`billing.ts`, `platform.ts`, `audit-logs.ts`) and is seed-only — except the plan catalog, which is live via the `billing` client in `src/lib/api.ts` (`GET/POST/PATCH/DELETE /admin/billing/plans`). Typed clients live in `src/lib/api.ts`; client-side stores in `src/lib/*.ts` (e.g. `plans.ts`: zod validation, API payloads, change events). Never scatter hardcoded data in pages — keep the seam so Rust `GET/PATCH/POST /admin/*` endpoints can replace the store without UI rewrites.
- **SSR fetch pattern:** pages fetch `PUBLIC_API_URL` with the `admin_session` cookie header; handle 401 (session expired) and 503 (API down) distinctly. Exemplar: `src/pages/notifications/index.astro`.
- **Tabs:** vanilla `.tab-btn` + `.tab-content` + `localStorage` persistence, same script as `notifications/index.astro`. Don't add a tab library.
- **Modals:** fixed-overlay dialog + backdrop + Escape close, inline error/success boxes (not toasts). Destructive or high-impact actions need the shared confirm dialog with consequences spelled out. Exemplar: `src/pages/administration/admins.astro`.
- **Page scripts** may import from `lib/` and `data/` (processed by Astro). Scope DOM ids per page.
- **Charts:** `recharts` islands (`client:load`, see `src/components/billing/`). Keep islands small; static content stays Astro HTML.
- **Minimal code, no bloat.** Wrap explanatory comment blocks like this (repo convention):
  `.==================================` / msg / `.==================================`
- Billing plans have an active/archived lifecycle (`isActive` in `lib/types.ts`, rules in `lib/plans.ts`): plans with subscribers are archived, never hard-deleted; renewals onto archived plans are refused. Preserve this when wiring the backend.
- Subscriber history (`GET /admin/billing/subscribers/history`) is replayed server-side from project births + billing audit events — no ledger table. Dormant cancelled subs keep their plan until first read triggers the lazy revert; the replay uses the same rule.
- `npm run check` may report diagnostics in pages you didn't touch — confirm they're pre-existing (grep the file list) before "fixing" them.

## Reading order

`src/lib/api.ts` → `src/lib/types.ts` → `src/data/` → `src/pages/<domain>/index.astro` → `src/layouts/AdminLayout.astro` (owns the sidebar nav; `lib/constants.ts` `NAV_STRUCTURE` mirrors it).

Spec for admin behavior (roles, permissions, auditability): `app/admin.md` at repo root.
