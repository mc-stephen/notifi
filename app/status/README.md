# Notifi Status

Public status platform for the [Notifi](https://notifi.dev) notification platform. Live at **https://status.notifi.dev**.

Communicates platform health, historical reliability, incident transparency, and operational maturity. Built with Astro — static-first, minimal JavaScript, dependency-free SVG charts.

## Commands

```shell
npm run dev      # Dev server (port 4321)
npm run build    # Static production build (SSG) + sitemap
npm run check    # astro check — type checking
npm run preview  # Preview production build locally
```

## Project Layout

| Path | Purpose |
|---|---|
| `src/content/` | Content collections: `components`, `incidents`, `maintenance`, `reports` |
| `src/lib/` | Pure logic: status roll-up, uptime series, formatting, feed builders |
| `src/layouts/` | `BaseLayout` (head/meta/theme) + `PageLayout` (nav/footer shell) |
| `src/components/ui/` | Reusable UI components (StatusBadge, HealthCard, charts…) |
| `src/islands/` | Client-side islands — the only shipped JS (subscribe, search, copy-link) |
| `src/pages/api/` | Generated static JSON endpoints (status API) |
| `src/pages/rss.xml.ts`, `atom.xml.ts` | Feed endpoints |

## Architecture

Full design: [`DESIGN.md`](./DESIGN.md). Milestone map (M0–M6):

- **M0** Scaffold + design system — *done*
- **M1** Content model + status roll-up
- **M2** Core pages (home, components, incidents, maintenance)
- **M3** SVG charts + uptime/performance
- **M4** Feeds + subscriptions + faq/about/reports
- **M5** Status API + SEO/OG + Lighthouse
- **M6** Live wiring to the Rust backend + webhooks

## Data

Pages are generated from committed JSON content collections. Milestone M6 replaces the collection loader with a snapshot generated from the Notifi backend (`infrastructure/`) without changing page code — page shapes already mirror the public API responses.
