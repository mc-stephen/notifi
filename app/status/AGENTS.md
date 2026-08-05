# Notifi Status (app/status/)

Public status platform — Astro (SSG), static-first, minimal JS.

## Commands

```shell
npm run dev      # Dev server (port 4321)
npm run build    # Production build + sitemap (SSG)
npm run check    # astro check — TypeScript checks
npm run preview  # Preview the production build
```

## Conventions

- **Static by default.** Only files in `src/islands/` ship client JavaScript. Never add framework islands unless interactivity requires it.
- **Charts are server-rendered SVG** in `src/components/charts/` — no chart libraries, zero client JS.
- **Status states** are a fixed vocabulary (operational / degraded / partial_outage / major_outage / maintenance). State must always be conveyed by word + glyph, never color alone.
- **Data lives in `src/content/`** (Astro Content Collections). Page shapes mirror the public status API so M6 can swap in backend snapshots without touching pages.
- **Roll-up logic** lives in `src/lib/status.ts` — pure functions, unit-testable, shared conceptually with the Rust backend implementation.
- All timestamps UTC (RFC 3339); durations human-readable ("2h 14m").
- Canonical design: `DESIGN.md` at the repo root of this app.

## Reading order

`src/content.config.ts` → `src/lib/status.ts` → `src/layouts/` → `src/components/` → `src/pages/`.
