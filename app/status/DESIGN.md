# Notifi Status Platform — Production Design Document

**Status domain:** `https://status.notifi.dev`
**Framework:** Astro (static-first, progressive enhancement)
**Status:** Design v1.0 — approved for implementation (M1+)

---

## 1. Executive Summary

The Notifi status platform is a public website that communicates the health, reliability, and operational maturity of the Notifi NPaaS (Notification Platform as a Service).

The platform's purpose is **customer confidence**. A developer deciding whether to route production notifications through Notifi will check the status page first. The page must instantly answer:

- Is Notifi operational right now?
- How reliable has Notifi been historically?
- When things break, how transparent is Notifi?
- Is Notifi operationally mature enough to trust with my traffic?

Design principles, in priority order:

1. **Truthfulness** — every number on the page is computed from real data. No marketing numbers.
2. **Calm** — no flashy animation, no panic colors, restrained motion. Readability first.
3. **Transparency** — incidents are documented with timeline, root cause, and postmortem. Silence is the enemy.
4. **Enterprise-grade** — precise typography, consistent semantics, polished empty/error states, dark + light themes.
5. **Fast** — the page must load fast even under heavy incident traffic. Static generation, minimal JavaScript, dependency-free SVG charts.

### Scope

| In scope (v1) | Out of scope (v1) |
|---|---|
| Public status pages, feeds, subscription forms | Slack/Discord bots (design-only) |
| Status API (JSON endpoints) | GraphQL API (designed, deferred) |
| Email subscriptions | Webhook subscriptions (designed, deferred) |
| RSS + Atom feeds | In-app status notifications in the dashboard |
| Content collection data layer | Live Rust backend status ingestion (M6+) |

---

## 2. Information Architecture

```
Status Platform (status.notifi.dev)
├── Home                          — global health, incidents, maintenance, uptime summary
├── Components                    — full component health dashboard
├── Incidents                     — incident list (open + historical)
│   └── Incident Detail           — timeline, updates, postmortem, impact
├── Scheduled Maintenance         — upcoming / in-progress / completed
│   └── Maintenance Detail        — schedule, progress updates, completion summary
├── Uptime History                — 24h / 7d / 30d / 90d / 1y / all-time
├── Performance                   — latency, throughput, queue, delivery metrics
├── Reports                       — historical reliability reports (monthly)
├── FAQ                           — status page itself explained
├── About Status                  — how we measure, SLO definitions
├── Subscription                  — email subscription, feed links, future webhook
├── Status API                    — public JSON endpoints
│   └── Status API Docs           — human-readable endpoint documentation
└── Machine-readable              — /rss.xml, /atom.xml, /api/status.json, ...
```

### Navigation strategy

Public status pages must be calm and minimal. Notifi status uses a **single flat primary nav** — no sidebar. Rationale:

- The page hierarchy is shallow (≤ 2 levels); a sidebar adds noise without value.
- The single job is conveying health; the homepage indicator already carries the primary signal.
- Deep pages (incident details) navigate via breadcrumbs, not sidebars.
- "Subscribe" is the one conversion action and is surfaced as a persistent button in the header.

Primary nav (desktop + mobile hamburger):

```
[◐ Notifi Status]   Overview · Components · Incidents · Maintenance · Uptime   [Subscribe]
```

Footer nav groups:

- **Status:** Overview, Components, Incidents, Scheduled Maintenance, Uptime History, Performance
- **Resources:** Reports, FAQ, About Status, Status API, Status API Docs
- **Feeds:** RSS, Atom, JSON API
- **Notifi:** Landing website (notifi.dev), Docs (docs.notifi.dev), Dashboard (dashboard.notifi.dev)

---

## 3. Page Hierarchy (Tree)

```
status.notifi.dev
├── /                              Home
├── /components                    Component health dashboard
├── /incidents/                    Incident index (open first, then historical, paginated)
├── /incidents/[slug]/             Incident detail
├── /maintenance/                  Maintenance index (upcoming / in-progress / completed tabs)
├── /maintenance/[slug]/           Maintenance detail
├── /uptime/                       Uptime history (range tabs: 24h, 7d, 30d, 90d, 1y, all)
├── /performance/                  Performance dashboards
├── /reports/                      Historical reports index
├── /reports/[slug]/               Individual reliability report
├── /faq/                          FAQ
├── /about/                        About this status page (measurement methodology)
├── /subscribe/                    Email subscription + feed preferences
├── /api/status.json               Global + component status (machine-readable)
├── /api/components.json           Component detail
├── /api/incidents.json            Incident list
├── /api/incidents/[id].json       Incident detail
├── /api/maintenance.json          Maintenance list
├── /api/uptime.json               Historical uptime series
├── /api/performance.json          Performance metrics
├── /rss.xml                       RSS feed (incidents + maintenance)
├── /atom.xml                      Atom feed
└── /404                           Custom 404 with live status
```

### URL conventions

- Human-readable slugs: `/incidents/postgres-replication-lag/` (kebab-case, descriptive)
- Canonical URLs: every page emits `<link rel="canonical">` with the final URL (no trailing params)
- Ids in JSON endpoints only; pages never expose internal ids
- Content type suffixes: `.xml` (feeds), `.json` (API) — explicit, never content-negotiated (max cacheability)

---

## 4. Homepage Wireframe (Textual)

```
┌──────────────────────────────────────────────────────────────────────────┐
│ Header: [◐ Notifi Status]  Overview Components Incidents Maintenance      │
│         Uptime                                    [Subscribe]              │
├──────────────────────────────────────────────────────────────────────────┤
│                                                                            │
│  GLOBAL STATUS HERO                                                       │
│  ┌────────────────────────────────────────────────────────────────────┐  │
│  │  ● All Systems Operational                   99.98% uptime (90d)   │  │
│  │                                                                    │  │
│  │  [Operational summary sentence generated from component states]    │  │
│  │                                                                    │  │
│  │  [Uptime sparkline — 90d bar chart, dependency-free SVG]           │  │
│  └────────────────────────────────────────────────────────────────────┘  │
│                                                                            │
│  ACTIVE ALERTS (conditional — hidden when nothing active)                 │
│  ┌────────────────────────────────────────────────────────────────────┐  │
│  │  ● Active incidents [2]                    ▲ Upcoming maintenance  │  │
│  │  · Incident title — Investigating — 14m    · Maintenance title     │  │
│  │  · Incident title — Identified — 2h       · Starts in 3d, 02:00   │  │
│  └────────────────────────────────────────────────────────────────────┘  │
│                                                                            │
│  COMPONENT HEALTH OVERVIEW                                               │
│  ┌────────────────────────────────────────────────────────────────────┐  │
│  │  Public API        ● Operational   22ms    99.99%                  │  │
│  │  Notification API  ● Operational   18ms    99.99%                  │  │
│  │  Queues            ● Degraded       -       98.10%                 │  │
│  │  ... (18 rows, grouped by subsystem, link to /components)          │  │
│  └────────────────────────────────────────────────────────────────────┘  │
│                                                                            │
│  PERFORMANCE SUMMARY                                                     │
│  ┌────────────────────────────────────────────────────────────────────┐  │
│  │  API p95: 210ms        Delivery success: 99.92%                    │  │
│  │  Notification time: 1.2s    Regional availability: 100% (3/3)      │  │
│  └────────────────────────────────────────────────────────────────────┘  │
│                                                                            │
│  RECENT INCIDENTS & MAINTENANCE (two-column list, last 5 each)            │
│  │  RESOLVED INCIDENTS (14d)                    COMPLETED MAINTENANCE   │
│  │  · Title — Resolved — Jun 12                  · Title — Completed   │
│  │  · Title — Resolved — Jun 03                  · Title — Completed   │
│  └────────────────────────────────────────────────────────────────────┘  │
│                                                                            │
│  QUICK LINKS                                                              │
│  [Uptime history] [Performance] [RSS] [Atom] [Status API] [Subscribe]     │
│                                                                            │
│ Footer: nav groups + © 2026 Notifi · status.notifi.dev                    │
└──────────────────────────────────────────────────────────────────────────┘
```

Key homepage decisions:

- **Hero indicator is the page's center of gravity** — a large, semantic status dot + verb ("Operational" / "Degraded" / "Major Outage"), never color alone (see Accessibility, §15).
- Uptime percentage is displayed for a fixed, labeled window (90 days) with links to `/uptime` — never a floating "all-time" number that hides bad months.
- Active incidents are listed with status-phase chips; resolved items are condensed.
- All numbers are computed from content collections at build time.

---

## 5. Component Inventory

The status platform models 19 components, grouped by subsystem. Grouping prevents visual noise and enables roll-up (see §9).

| # | Subsystem | Component | Health source (conceptual) |
|---|---|---|---|
| 1 | Platform | Public API | REST API availability + p95 latency |
| 2 | Platform | Dashboard | Web UI availability |
| 3 | Platform | Authentication | Login/token issuance success rate |
| 4 | Platform | Notification API | Notification ingest endpoint |
| 5 | Platform | Queues | Queue depth vs. healthy threshold |
| 6 | Platform | Workers | Consumer lag, processing throughput |
| 7 | Data | PostgreSQL | Replication lag, connection health |
| 8 | Data | Redis | Cache hit rate, memory pressure |
| 9 | Platform | Analytics | Event pipeline health |
| 10 | Platform | Templates | Template render pipeline |
| 11 | Platform | Webhooks | Outbound webhook delivery success |
| 12 | Providers | Email Providers | Aggregate delivery success + provider status |
| 13 | Providers | SMS Providers | Aggregate delivery success + provider status |
| 14 | Providers | Push Providers | FCM/APNS/Web Push aggregate status |
| 15 | Platform | Documentation | Docs site availability |
| 16 | Platform | Landing Website | Marketing site availability |
| 17 | Platform | Billing | Billing API + payment processor status |
| 18 | Platform | Storage | Object storage availability, upload success |
| 19 | Platform | SDK Downloads | Package registry + SDK artifact delivery |

### Per-component displayed data

| Field | Description | Source |
|---|---|---|
| Operational Status | One of 5 states (§9) | Roll-up of probes/incidents |
| Current State | Free-form detail ("Elevated latency", "Provider degradation") | Latest incident/probe |
| Latency / Response Time | p50 + p95, last hour | Synthetic probe |
| Availability | % over selected window (7/30/90d) | Probe history |
| Recent Changes | List of incidents + maintenance affecting it (last 30d) | Incident collection |
| Dependencies | Upstream components whose state affects this one | Explicit graph in data model |
| Last Updated | Probe timestamp or incident update time | Data layer |

### Data-layer dependency graph (partial, illustrative)

```
Public API ──→ Authentication, PostgreSQL, Redis
Notification API ──→ Public API, Queues
Queues ──→ Redis, Workers
Workers ──→ PostgreSQL, Email Providers, SMS Providers, Push Providers, Storage
Dashboard ──→ Public API, Analytics, Billing
Webhooks ──→ Workers, PostgreSQL
```

The graph is stored as `dependencies: string[]` per component and used to display "This component depends on: X, Y" and to suppress misleading "Operational" badges when an upstream dependency is down.

---

## 6. Reusable Design System

### 6.1 Status semantics (the core vocabulary)

Five states, with strict color + label + description pairing. **State is always conveyed by a word, a glyph, and color.**

| State | Label | Glyph | Color (light/dark) | Description pattern |
|---|---|---|---|---|
| `operational` | Operational | ● filled | `#22c55e` green | "Receiving and processing traffic normally." |
| `degraded` | Degraded Performance | ◑ half | `#f59e0b` amber | "Available with elevated latency or errors." |
| `partial_outage` | Partial Outage | ◕ | `#f97316` orange | "Some functions or regions are unavailable." |
| `major_outage` | Major Outage | ○ | `#ef4444` red | "The component is substantially unavailable." |
| `maintenance` | Under Maintenance | ◌ | `#3b82f6` blue | "Temporarily taken offline for maintenance." |

Rules:

- **Never use color alone.** Each state has a distinct word and glyph; the glyph is conveyed to screen readers via aria-label.
- Degraded vs. partial outage: *degraded* = slow/erroring but usable; *partial outage* = hard failure of a subset (region, function); *major outage* = hard failure of most/all.
- Ambers/reds are used sparingly — a calm page keeps most of the surface green.
- Loading/unknown state: a neutral pulsing dot with "Checking…" (client-only; SSG pages never block on it).

### 6.2 Typography

- UI font: `Inter` (system font stack fallback, self-hosted woff2 — no external requests).
- Mono: `JetBrains Mono` or `ui-monospace` for timestamps, latency values, API examples.
- Scale (rem): `12 / 13 / 14 / 16 / 20 / 24 / 32 / 40`; hero headline 32–40.
- Max text measure: 68ch. Line-height: 1.5 body, 1.2 headings.
- Numbers use `font-variant-numeric: tabular-nums` everywhere (uptime %, latencies, dates) to prevent jitter in tables/charts.

### 6.3 Color & themes

- CSS custom properties (`--bg`, `--fg`, `--muted`, `--border`, `--accent`, status colors) defined per theme.
- **Light theme (default):** near-white background `#fafafa`, near-black text `#18181b`, green accent derived from status color.
- **Dark theme:** `#09090b` background, `#fafafa` text; status colors brightened (≥ 4.5:1 contrast against surface, verified against WCAG).
- Theme resolved at build time from Astro `prefers-color-scheme` hint? **No** — SSG cannot know the client's preference. Instead: inline `<script>` in `<head>` reads `localStorage.theme ?? matchMedia(prefers-color-scheme)` and sets `data-theme` before first paint (progressive enhancement, ~300 bytes, no flash, no hydration).
- High-contrast mode: `prefers-contrast: more` increases border weight and darkens status colors.
- Focus rings: 2px accent outline, `:focus-visible` only.

### 6.4 Motion

- All animations gated by `@media (prefers-reduced-motion: no-preference)`.
- Allowed: 150–250ms opacity/transform fades, 0.5s ease-out value transitions on charts.
- **Forbidden:** parallax, marquees, looped pulses (except the loading dot, max 2 iterations), page transitions.
- Zero animation on status changes themselves — the page is truthful, not dramatic.

### 6.5 Component library inventory

| Component | Rendering | Notes |
|---|---|---|
| `StatusBadge` | Static | Pill: glyph + label; `size` variants |
| `HealthCard` | Static | Component row/card with all §5 fields |
| `IncidentCard` | Static | Title, phase chip, start time, impact line |
| `IncidentTimeline` | Static | Ordered update list with vertical rail, phase markers |
| `ComponentGrid` | Static | 19 components grouped by subsystem, adaptive columns |
| `MetricCard` | Static | Label + big number + trend + sparkline |
| `UptimeChart` | Server SVG | Bar/heatmap series, range prop (24h…all) |
| `LatencyChart` | Server SVG | Area/line series, p50/p95 bands |
| `MaintenanceCard` | Static | Title, window (start→end, timezone), state chip |
| `SubscriptionForm` | Island | Small client island; validates, posts, shows success |
| `RSSCard` | Static | Feed-type links (RSS/Atom) + subscribe button |
| `HistoryCalendar` | Server SVG | 90d day-dot matrix with tooltips (title attr) |
| `AnnouncementBanner` | Static | Optional top banner (incident/major notice) |
| `Search` | Island | Client-side index of incidents (filter-as-you-type) |
| `Filters` | Static/island | Phase/severity/component filter chips |
| `Pagination` | Static | Prev/next + page links, incident index |
| `Tabs` | Static | Range tabs (uptime), phase tabs (maintenance) |
| `EmptyState` | Static | "No incidents in this period" + illustration dot |
| `ErrorState` | Static | "This page could not be generated" + status link |
| `LoadingState` | Static-first | Skeleton only inside islands; SSG pages never show it |

### 6.6 Status badges & indicator anatomy

```
● OPERATIONAL      [dot] [LABEL]            — badge
◐ Degraded         [half-filled dot]        — badge
◕ Partial Outage   [three-quarter dot]      — badge
○ Major Outage     [ring dot]               — badge
◌ Maintenance      [dashed-ring dot]        — badge
```

The dot is 8px (badge) / 20px (hero). All glyphs are drawn with pure CSS/`<svg>` — no icon font dependency.

---

## 7. Incident Lifecycle & User Experience

### 7.1 Lifecycle states

```
Investigating ──► Identified ──► Monitoring ──► Resolved
      │               │              │
      └──(escalate: severity up)────┘
```

- **Investigating:** symptoms known, cause unknown. Page shows "We are investigating…".
- **Identified:** root cause or cause class found. Page shows what was found + next step.
- **Monitoring:** fix deployed, observing. Page shows expected resolution window.
- **Resolved:** confirmed recovered + monitored for N minutes. Final update closes the incident.

Every transition is a new entry on the timeline. An incident may skip states (Investigating → Resolved) but **every state change must produce a timeline entry with a timestamp, author, details, and affected components**.

### 7.2 Incident detail page structure

```
┌──────────────────────────────────────────────────────────────┐
│ Breadcrumb: Overview / Incidents / [title]                   │
│                                                              │
│  [MAJOR OUTAGE]  (severity chip + resolved chip)             │
│  Postgres Replication Lag Causing Notification Delays        │
│  Resolved — Jun 12, 2026 · 2h 14m duration                   │
│                                                              │
│  Affected: Queues, Workers, Notification API                 │
│  Customer impact: Up to 8 minutes of delivery delay on SMS   │
│                                                              │
│  Share: [Copy link]  [RSS]  [Atom]  [Embeddable? — future]   │
│                                                              │
│  TIMELINE                                                    │
│  ─ Investigated  Jun 12 09:14 UTC  SRE@Notifi               │
│  ─ Identified    Jun 12 09:41 UTC  "Replication lag > 90s…" │
│  ─ Monitoring    Jun 12 11:02 UTC  "Failover complete…"     │
│  ─ Resolved      Jun 12 11:28 UTC  "Verified recovered…"    │
│                                                              │
│  ROOT CAUSE ANALYSIS                                         │
│  Root cause · Mitigation · Monitoring · Postmortem link      │
│  (shown in severity order; minor incidents may omit)         │
│                                                              │
│  Impact summary · Duration breakdown · Uptime effect         │
└──────────────────────────────────────────────────────────────┘
```

### 7.3 Incident content model (per the prompt)

| Field | Required | Notes |
|---|---|---|
| Title | ✓ | Human, factual: "SMS Delivery Delays in US-East" |
| Severity | ✓ | `minor` / `major` / `critical` (chips: informational→minor→major→critical) |
| Status | ✓ | One of the 4 phases (or `resolved`) |
| Affected components | ✓ | Component slugs |
| Timeline (updates) | ✓ | Array of updates (§7.4) |
| Investigation | ✓ (resolved) | What was examined |
| Root cause | ✓ (resolved, major+) | Plain-language + technical appendix |
| Mitigation | ✓ (resolved, major+) | What was done + who |
| Monitoring | ✓ (resolved) | How recovery was confirmed |
| Resolution | ✓ | Final statement + timestamp |
| Postmortem | o | Link to `/reports/[slug]` for critical incidents |
| Customer impact | ✓ | Quantified ("X% of messages delayed < 5m") |
| Duration | computed | Resolution − start; displayed as "2h 14m" |
| Share link | ✓ | Canonical absolute URL; copy button (island) |
| RSS/Atom | generated | Every incident appears in feeds while open + at close |

### 7.4 Timeline update fields

| Field | Notes |
|---|---|
| Timestamp | RFC 3339 UTC; rendered local with explicit "UTC" hint |
| Author | Human name + role ("Sarah, SRE") — never anonymous |
| Details | 1–3 sentences, plain language first, technical addendum |
| Affected components | Can be amended mid-incident |
| Customer impact | Optional per update; aggregate on final |

### 7.5 Incident UX rules

- The homepage only shows **open** incidents; closed ones live under Incidents/Recent.
- Incident pages are **append-only** — updates never edited in place (they are, but not silently: show "Edited" marker).
- RSS/Atom push: new update → new `<item>` (not a rewrite) so feed readers get push per update.
- Every incident page links to RSS filtered to that incident (`/rss.xml?incident=slug`).

---

## 8. Maintenance Workflow

### 8.1 States

| State | Window | Page behavior |
|---|---|---|
| `scheduled` (Upcoming) | start in future | Shown on homepage "Upcoming maintenance" + `/maintenance` |
| `in_progress` | start ≤ now ≤ end | Component shows `maintenance` badge; homepage alert |
| `completed` | now > end | Moved to Completed; summary posted |

### 8.2 Maintenance event fields

| Field | Notes |
|---|---|
| Title | "PostgreSQL Minor Version Upgrade" |
| Purpose | Why — expected outcome |
| Expected impact | Service behavior during window ("up to 2 min API 5xx") |
| Affected components | Component slugs |
| Start time / End time | RFC 3339 UTC |
| Timezone | Displayed explicitly ("UTC — 02:00" or "All times UTC") |
| Progress updates | Optional `in_progress` updates (same model as incident updates) |
| Completion summary | Final update: what was done, measured impact |

### 8.3 Maintenance UX rules

- **Announce early:** published ≥ 72h before start for major maintenance; ≥ 24h otherwise.
- Upcoming maintenance gets a banner on the homepage ("Planned maintenance affects Queues on Jun 20, 02:00–04:00 UTC").
- Maintenance does **not** count against uptime percentage when marked as scheduled (documented in `/about`).
- Feed items marked `category: maintenance` so subscribers can filter.
- If a window is missed/overrun, the event becomes `completed` with a "window exceeded" flag and an incident may be created.

---

## 9. Component Health Model

### 9.1 State machine

Each component resolves to exactly one of the five states (§6.1) via this algorithm (per window/range):

```
1. If an open major outage affects it          → major_outage
2. If an open partial outage affects it        → partial_outage
3. If scheduled/in-progress maintenance        → maintenance (in-progress) / scheduled flag
4. Else if degraded incident or probe breach   → degraded
5. Else                                        → operational
```

Priority: incident state > maintenance state > probe health > default operational. This makes the page explainable: **every non-operational badge links to the incident or maintenance that caused it.**

### 9.2 Roll-up

- **Component state** rolls up from: open incidents affecting it + probe health (latency/error thresholds) + dependency state.
- **Subsystem state** (Platform / Data / Providers) = worst state of its members.
- **Global state** = worst of all subsystems — except Provider subsystems, which degrade the global state only to `degraded` (provider incidents are operational events, not platform outages). This nuance goes in `/about`.

### 9.3 Health signals (conceptual, M6 wiring)

| Signal | Thresholds (example) |
|---|---|
| API availability | 5xx rate > 1% for 5m → degraded; > 5% → partial |
| Latency | p95 > 500ms for 10m → degraded |
| Queue depth | > 10× healthy for 10m → degraded |
| Delivery success | < 99% over 5m → degraded |
| Provider status | Provider-reported incident → degraded |

---

## 10. Public Status API Specification (High Level)

Base: `https://status.notifi.dev/api/`. All endpoints are static JSON generated at build time (M6+: cached behind the Rust backend). No auth, no rate limits by design — but 60s CDN caching and `Cache-Control: public, max-age=60`.

| Endpoint | Response |
|---|---|
| `GET /api/status.json` | Global state, per-subsystem state, uptime 90d, summary counts |
| `GET /api/components.json` | All 18 components: id, name, subsystem, state, latency p50/p95, availability, last_updated, dependencies |
| `GET /api/incidents.json?status=open&severity=critical` | Incident list (id, slug, title, severity, status, started_at, resolved_at, affected_components, url) |
| `GET /api/incidents/{id}.json` | Full incident: all §7.3 fields + timeline updates |
| `GET /api/maintenance.json?state=upcoming` | Maintenance list |
| `GET /api/uptime.json?range=30d` | Series of availability per unit (hour/day) per component group + total |
| `GET /api/performance.json` | Latest metrics: api_p95, webhook_latency_p95, processing_time_p95, queue_depth, worker_throughput, delivery_success_rate, db_health, cache_hit_rate, regional_availability |
| `GET /rss.xml` / `GET /atom.xml` | Feeds (see §16) |

**Error shape:** `{"error": "not_found", "detail": "..."}` — 404 for unknown ids only (static generation makes 500s impossible).

**Future (documented, not built):** `GET /graphql` with the same resource types; `GET /api/subscribe` webhook registry.

---

## 11. Data Model Assumptions

Astro Content Collections, versioned JSON (in `src/content/`). Future-proofing: shapes mirror the API responses 1:1 so M6 wiring swaps the loader, not the pages.

### 11.1 `Component`

```ts
{
  slug: "queues",
  name: "Queues",
  subsystem: "platform",            // platform | data | providers | web
  group: "Internal Infrastructure", // for display grouping
  dependencies: ["redis", "workers"],
  state: "operational",             // denormalized from incidents + probes
  latency: { p50: 18, p95: 42 },    // ms, last hour
  availability: { "7d": 100, "30d": 99.98, "90d": 99.98 },  // percent
  lastUpdated: "2026-08-01T09:00:00Z",
}
```

### 11.2 `Incident`

```ts
{
  slug: "postgres-replication-lag",
  title: "Postgres Replication Lag Causing Notification Delays",
  severity: "major",                          // minor | major | critical
  status: "resolved",                         // investigating | identified | monitoring | resolved
  startedAt: "...", resolvedAt: "...",        // ISO 8601 UTC
  affectedComponents: ["queues", "workers"],
  customerImpact: "Up to 8 min delivery delay...",
  investigation: "...", rootCause: "...",
  mitigation: "...", monitoring: "...", resolution: "...",
  postmortemSlug: "2026-06-postgres-lag",     // optional
  updates: [{                                  // timeline
    timestamp, author, state,                 // state = phase at time of update
    details, affectedComponents?, customerImpact?,
  }],
}
```

### 11.3 `Maintenance`

```ts
{
  slug: "postgres-minor-upgrade",
  title: "...", purpose: "...", expectedImpact: "...",
  affectedComponents: [...],
  startTime: "...", endTime: "...",
  status: "scheduled",                        // scheduled | in_progress | completed
  updates: [{ timestamp, author, state, details }],
  completionSummary?: "...",
}
```

### 11.4 Time-series (generated, derived)

- `uptimeSeries` — per range: `{ range, unit, points: [{ t, availability, latency? , maintenance?: bool }] }`
- `uptimeSummary` — `{ "24h": 99.99, "7d": 100, "30d": 99.98, "90d": 99.98, "1y": 99.97, "all": 99.96 }`
- `incidentFrequency` — incidents per month (derived from Incident collection)

### 11.5 Derived-value rules (truthfulness guardrails)

- Availability excludes scheduled maintenance windows (§8.3) and counts only within the window.
- All-time uptime is computed from component inception dates, documented per component.
- Duration = `resolvedAt − startedAt`; if an incident is still open, show "ongoing" (never a fake end).

---

## 12. Astro Project Structure (app/status/)

```
app/status/
├── astro.config.mjs            # SSG output, sitemap + mdx integrations
├── package.json                # astro, @astrojs/mdx, @astrojs/sitemap, @astrojs/rss
├── tsconfig.json
├── public/
│   ├── favicon.svg             # status-dot favicon (green by default, theme-aware)
│   ├── robots.txt              # allow all; disallow /api/*? no — APIs are public
│   └── og/                     # per-page OG image templates (static, no SSR)
├── src/
│   ├── content/
│   │   ├── config.ts           # zod schemas for all collections
│   │   ├── components/*.json   # 18 component records
│   │   ├── incidents/*.json    # incident records (slug.json)
│   │   ├── maintenance/*.json  # maintenance records
│   │   └── reports/*.json      # monthly reliability reports
│   ├── lib/
│   │   ├── status.ts           # state roll-up algorithm (§9) — pure functions
│   │   ├── uptime.ts           # series generation + summarization
│   │   ├── format.ts           # dates, durations, percentages, ordinal suffixes
│   │   └── feeds.ts            # RSS/Atom item builders (shared with API)
│   ├── layouts/
│   │   ├── BaseLayout.astro    # html head: meta, OG, canonical, JSON-LD, theme script
│   │   ├── PageLayout.astro    # header/nav/footer shell + breadcrumbs slot
│   │   └── DocsLayout.astro    # for API docs pages (long-form content)
│   ├── components/
│   │   ├── ui/                 # §6.5 inventory (StatusBadge, HealthCard, ...)
│   │   └── charts/             # UptimeChart.astro, LatencyChart.astro,
│   │                           # HistoryCalendar.astro, Sparkline.astro (pure SVG)
│   ├── islands/                # client islands (only these ship JS)
│   │   ├── SubscriptionForm.tsx    # email subscribe
│   │   ├── Search.tsx              # incident search
│   │   └── CopyLink.tsx            # share button
│   ├── pages/
│   │   ├── index.astro
│   │   ├── components.astro
│   │   ├── incidents/
│   │   │   ├── index.astro
│   │   │   └── [slug].astro        # getStaticPaths from collection
│   │   ├── maintenance/
│   │   │   ├── index.astro
│   │   │   └── [slug].astro
│   │   ├── uptime.astro
│   │   ├── performance.astro
│   │   ├── reports/index.astro + [slug].astro
│   │   ├── faq.astro
│   │   ├── about.astro
│   │   ├── subscribe.astro
│   │   ├── api/*.json.ts          # 6 JSON endpoints + XML feeds
│   │   ├── rss.xml.ts
│   │   ├── atom.xml.ts
│   │   └── 404.astro
│   └── styles/global.css          # tokens, themes, typography, motion
└── README.md
```

### Structure rationale

- **Everything static except three islands.** Charts are server-rendered SVG components — zero client JS, ideal LCP, printer-friendly.
- **Content collections give type safety and codegen**; `getStaticPaths` derives all incident/maintenance/report pages.
- **`lib/status.ts` is the single source of truth for the roll-up algorithm**, shared conceptually with the M6 Rust implementation.
- **JSON API endpoints are generated pages** (`*.json.ts`) — same build pipeline, trivially cacheable, no server.
- Islands are framework-free-by-default: SubscriptionForm/Search/CopyLink can be vanilla TS + Preact only if interactivity grows.

---

## 13. SEO Strategy

1. **Semantic HTML:** single `h1` per page, ordered `h2/h3`, `nav`, `main`, `article`, `time` elements with `datetime`, `figure` for charts, `address` in footer.
2. **Meta + social:** title `Notifi Status — {page}`, description per page, canonical URL, `og:type=website`, per-page OG images generated at build (static PNGs with state color), Twitter `summary_large_image`.
3. **Structured data (JSON-LD):**
   - Every page: `WebSite` + `Organization` + `Service` (Name: Notifi Status; provider: Notifi).
   - Homepage: `Service` with `serviceType: "Notification Platform"` and `offers` availability.
   - Incident pages: `Article`/`Report` schema with `datePublished`/`dateModified`, `about` → components.
   - FAQ: `FAQPage` schema.
   - Uptime numbers via `aggregateRating`-free custom `SoftwareApplication`? No — keep to `Service`; don't misuse review schemas.
4. **Feeds & APIs indexed:** RSS/Atom referenced via `<link rel="alternate" type="application/rss+xml">`; `sitemap-index.xml` via `@astrojs/sitemap`.
5. **URLs:** kebab-case human-readable; canonical strips query params; `x-default` hreflang; trailing-slash policy fixed once (`never`).
6. **Core Web Vitals targets:** LCP < 2.0s, CLS < 0.05, INP < 200ms. Achieved via SSG, inline CSS, `<img loading=lazy>` for below-fold, no render-blocking JS (theme script is inline, 300B).
7. **Speed under incident load:** CDN edge caching, immutable asset hashing, zero runtime origin requests on read paths — the page works even if Notifi itself is down.
8. **robots.txt:** allow all; `sitemap` entry. Status pages should be indexable — an outage page in Google is a trust signal.

---

## 14. Accessibility Recommendations

| Area | Implementation |
|---|---|
| Keyboard | All interactive elements reachable; visible focus ring; islands (search, subscribe) full keyboard flows; skip-to-content link |
| Screen readers | Status dot: `role="img" aria-label="Operational"` — never rely on color; tables have `caption`/`scope`; timeline is an ordered `<ol>` with semantic headings per update |
| ARIA | `aria-live="polite"` on the global status hero; `aria-current` on nav; tabs use proper `tablist/tab/tabpanel` semantics |
| Contrast | All text ≥ 4.5:1 (AA); status colors ≥ 3:1 against surface; `prefers-contrast: more` boosts borders |
| Reduced motion | All animation disabled under `prefers-reduced-motion` (§6.4) |
| Responsive | Mobile-first; nav collapses to hamburger < 768px; component grid 1→2→3 columns; charts reflow (bars maintain min touch width); no horizontal scroll on ≤ 360px viewport |
| Reading order | Matches visual order; `dir="ltr"`; lang attribute `en` |
| Focus states | High-contrast 2px ring; never removed without replacement |

---

## 15. Future Integrations

### 15.1 RSS / Atom (M4 — built into v1)

- `rss.xml` / `atom.xml`: every incident update + maintenance announcement = one entry.
- `category` tags: `incident`, `maintenance`, `severity:{minor|major|critical}` for subscriber-side filtering.
- Per-incident filtered feeds: `/rss.xml?incident=slug`.

### 15.2 Email subscriptions (M4)

- Form on homepage + `/subscribe` (island): email + preference (all / critical only / components / maintenance).
- Preference model: `{ email, scope: "all" | "critical" | ["components..."] | "maintenance", verified }`
- Double opt-in by design. Storage: JSON content collection (v1) → M6 Rust persistence + background sender.
- Verification token → signed URL → `/subscribe/verify?token=…` page.

### 15.3 Webhooks (M5, designed)

- `POST /api/subscriptions` (Rust, M6): `{ url, events: ["incident.opened","incident.updated","incident.resolved","maintenance.scheduled","maintenance.completed"] }`
- Payloads mirror API JSON shapes; HMAC-signed with a per-subscription secret (`X-Notifi-Signature`).
- Retries: 3× exponential backoff; dead-letter after failure.

### 15.4 Slack / Discord (future)

- Slash-command / bot: `!status` returns `status.notifi.dev/api/status.json` rendered as a message with emoji state; incident updates pushed to a `#incidents` channel via the same webhook pipeline.
- Component subscription parity with email scopes.

---

## 16. Milestone-Based Implementation Plan

| Milestone | Scope | Deliverable |
|---|---|---|
| **M0** | Scaffold | `create astro` in `app/status/`, integrations (mdx, sitemap), theme tokens, BaseLayout, nav shell, favicon, robots, sitemap, README |
| **M1** | Content & model | Zod schemas; component collection (18); `lib/status.ts` roll-up; status helper tests |
| **M2** | Core pages | Homepage, Components, Incidents index + detail, Maintenance index + detail (all real data) |
| **M3** | Charts & uptime | UptimeChart, LatencyChart, HistoryCalendar, Sparkline (SVG); `/uptime`, `/performance`; uptime generation lib + tests |
| **M4** | Feeds & subscribe | RSS + Atom, `/subscribe` + email form island, per-incident feeds, `/faq`, `/about`, `/reports` |
| **M5** | Status API & docs | `api/*.json.ts` endpoints, Status API docs pages, JSON-LD, OG images, SEO audit (Lighthouse ≥ 95 all categories) |
| **M6** | Live wiring | Rust status ingestion → generated JSON snapshot (or on-demand with CDN cache); webhook subscriptions; a11y + reduced-motion audit |

**Exit criteria for M2 (the "is it real?" checkpoint):** homepage renders true roll-up from 18 components, incident detail renders a full lifecycle with timeline, all numbers consistent with the data model.

---

## Appendix A — Brand Consistency

- Status site mirrors the dashboard's aesthetic: same Tailwind v4 token philosophy, Inter, restrained grays with a single accent.
- Status dot favicon: green when operational; the hero indicator color in a ring when not (theme-aware, static per state).
- The word "Notifi" always capitalized; product name "Notifi" never "notifi".

## Appendix B — Measurement Methodology (to be published at /about)

1. Probes run every 60s from 3+ regions (future: every 30s).
2. Latency percentiles are computed over 5-minute buckets.
3. Availability = (window − downtime) / window, where downtime = time spent in `partial_outage`/`major_outage` for that component; `degraded` counts as 100% availability but is reported in performance metrics.
4. Scheduled maintenance is excluded from availability windows (§8.3).
5. All times UTC; all durations human-readable (e.g., "2h 14m").
