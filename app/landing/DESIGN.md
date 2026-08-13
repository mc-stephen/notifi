# Notifi Landing Website — Production Design Document

**Domain:** `https://notifi.dev`
**Framework:** Astro (SSG, static-first, islands-only JS)
**Design language:** [`app/design-language.md`](../design-language.md) — the Master Design Language is authoritative for all tokens, type, motion, and states. This document references it and never overrides it.
**Status:** Design v1.0 — approved for implementation (M0+)

---

## 1. Executive Summary

The Notifi landing website is the front door of a Notification Platform as a Service (NPaaS): one API that delivers through email, SMS, FCM, APNS, web push, Linux, macOS, and RCS — with enterprise reliability and analytics.

The site's purpose is **conversion with credibility**. A developer evaluating notification infrastructure decides in minutes whether Notifi is worth integrating. The page must:

1. Capture attention within 5 seconds (hero + live flow).
2. Explain the product within 15 seconds (one API, every channel).
3. Build trust before asking for signup (architecture, security, status, testimonials).
4. Convert developers and enterprise customers (free tier + contact sales).

The emotional journey drives the section order:

```
Nav (Confidence) → Hero (Curiosity) → Architecture (Technical Trust) → Channels (Capability)
→ DX (Excitement) → Analytics (Enterprise Confidence) → Testimonials (Trust)
→ Pricing (Low Risk) → CTA (Motivation) → Footer (Professionalism)
```

### Scope

| In scope (v1) | Out of scope (v1) |
|---|---|
| Homepage + supporting pages (pricing, changelog, blog, security, SDKs, 404) | Interactive playground/console (design only) |
| Dark-first premium marketing design | Dashboard live embeds (deferred to M6) |
| Content collections for all copy | i18n (architecture supports it; content deferred) |
| Newsletter capture | Auth flows (dashboard handles them) |
| SEO, OG, structured data, feeds | A/B testing tooling |

### Success metrics

- Lighthouse ≥ 95 on all four CWV dimensions (desktop and mobile)
- First CTA above the fold on all viewports
- Zero layout shift; LCP < 2.0s on 4G
- 100% keyboard operable; AA contrast verified

---

## 2. Information Architecture

```
notifi.dev
├── /                        Homepage (conversion engine)
├── /pricing                 Pricing — free / pro / enterprise
├── /changelog               Product changelog (content collection)
├── /blog                    Engineering blog index
│   └── /blog/[slug]/        Post
├── /security                Security & compliance detail
├── /sdk                     SDKs, quickstart, package installs
├── /enterprise              Enterprise sales landing
├── /contact-sales           Sales contact form
├── /docs                    → docs.notifi.dev (external)
├── /status                  → status.notifi.dev (external)
├── /dashboard               → dashboard.notifi.dev (external)
├── /legal/privacy           Privacy policy
├── /legal/terms             Terms of service
├── /newsletter              Newsletter confirmation
├── /rss.xml                 Blog feed
└── /404                     Custom 404 (product-first)
```

### Navigation strategy

One marketing site, three destinations that must be one click away at all times: **Docs, Status, Dashboard**. Everything else hangs off the mega menu. The header carries a persistent "Start building free" primary CTA.

Footer groups:

- **Product:** Features, Channels, Architecture, Pricing, Changelog, Security, SDKs
- **Developers:** Docs, API Reference, Status, Guides, Blog
- **Company:** About, Enterprise, Contact Sales, Brand
- **Legal:** Privacy, Terms
- **Status row:** All systems operational link (live status dot, static snapshot)

---

## 3. Page Hierarchy

```
notifi.dev
├── /                              Homepage
├── /pricing                       Pricing (3 tiers + enterprise)
├── /changelog/                    Changelog index (grouped by release)
├── /blog/                         Blog index
│   └── /blog/[slug]/              Article
├── /security                      Security & compliance
├── /sdk/                          SDK overview + install snippets
├── /enterprise                    Enterprise page
├── /contact-sales                 Sales form
├── /newsletter                    Confirmation
├── /legal/privacy, /legal/terms   Legal
├── /rss.xml                       Blog feed
└── /404                           Custom 404
```

URL conventions: kebab-case slugs, no trailing slash (`trailingSlash: "never"`), canonical on every page, human-readable blog slugs (`/blog/notification-delivery-guarantees/`).

---

## 4. Homepage Wireframe (Textual)

```
┌────────────────────────────────────────────────────────────────┐
│ NAV (sticky, 64px)                                             │
│ [◐ Notifi]  Products▾  Developers▾  Resources▾  Pricing        │
│                                        Docs  Status  [Start free]│
├────────────────────────────────────────────────────────────────┤
│ HERO (min-height 92vh)                                         │
│  eyebrow: ONE API · EVERY CHANNEL                              │
│  H1: One API. Every notification. Every platform.             │
│  sub: Send email, SMS, push, and more from a single endpoint — │
│       with retries, analytics, and enterprise reliability.     │
│  [Start building free] [Read the docs]                         │
│  meta: No credit card · 10k notifications/mo free              │
│  VISUAL: animated delivery flow (API→queue→workers→channels)   │
│          + floating code window (curl/JS POST)                 │
│          + live "delivered" toast cards                        │
│  BG: violet radial washes + faint grid + dot field             │
├────────────────────────────────────────────────────────────────┤
│ TRUST BAR: "Powering notifications at" — 6 placeholder logos   │
├────────────────────────────────────────────────────────────────┤
│ ARCHITECTURE (sticky column + SVG diagram)                     │
│  "Designed like production infrastructure"                     │
│  Ingest → Outbox → Queue → Workers → Providers → Deliver      │
│  per-stage captions: reliability, retries, backpressure        │
├────────────────────────────────────────────────────────────────┤
│ CHANNELS — "One integration. Every channel."                   │
│  8 cards: Email, SMS, Android Push, Apple Push, Web Push,      │
│           Linux, macOS, RCS (+ "Coming soon: Slack/Discord/…") │
├────────────────────────────────────────────────────────────────┤
│ WHY DEVELOPERS LOVE IT — 3 cards                              │
│  Predictable delivery · First-class retries · Honest analytics │
├────────────────────────────────────────────────────────────────┤
│ INTERACTIVE API EXAMPLE (tabs)                                 │
│  JS / Python / Go / cURL  →  send → response JSON + timeline   │
│  "delivered in 412ms across 3 channels"                        │
├────────────────────────────────────────────────────────────────┤
│ SDK SUPPORT — language chips (JS, TS, Python, Go, Rust, …)     │
├────────────────────────────────────────────────────────────────┤
│ ANALYTICS PREVIEW (dashboard mock, server-rendered)            │
│  animated counters: delivered, open rate, channel split,       │
│  latency p95 · "Every send is measurable"                      │
├────────────────────────────────────────────────────────────────┤
│ TEMPLATE SYSTEM — template cards + rendered preview            │
├────────────────────────────────────────────────────────────────┤
│ SCHEDULING — timezone-aware schedule visualization             │
├────────────────────────────────────────────────────────────────┤
│ PROVIDER ECOSYSTEM — provider cards + automatic failover       │
├────────────────────────────────────────────────────────────────┤
│ ENTERPRISE — 4 features: SSO/SAML, RBAC, audit logs, SLA       │
├────────────────────────────────────────────────────────────────┤
│ PERFORMANCE — animated counters (99.99% delivery, p95, …)      │
├────────────────────────────────────────────────────────────────┤
│ SECURITY — encryption, keys, compliance (SOC 2 in progress)    │
├────────────────────────────────────────────────────────────────┤
│ TESTIMONIALS — 3 quotes + role chips                           │
├────────────────────────────────────────────────────────────────┤
│ PRICING PREVIEW — 3 cards (Free / Pro / Enterprise)            │
├────────────────────────────────────────────────────────────────┤
│ FAQ — accordion, 6 questions                                   │
├────────────────────────────────────────────────────────────────┤
│ CTA — "Ship your first notification in minutes" + email input  │
├────────────────────────────────────────────────────────────────┤
│ FOOTER (5 columns + status row)                                │
└────────────────────────────────────────────────────────────────┘
```

---

## 5. Every Section in Order — Goal & Emotion

| # | Section | Goal | Emotion | Conversion point |
|---|---|---|---|---|
| 1 | Announcement banner | Announce product news without blocking | Anticipation | → changelog |
| 2 | Sticky navigation | Orient; never lose the CTA | Confidence | → signup / docs |
| 3 | Hero | Explain product in 15s; prove it moves | Curiosity | → signup (primary), docs (secondary) |
| 4 | Trust bar | Social proof before detail | Assumed trust | — |
| 5 | Architecture overview | Prove engineering depth honestly | Technical trust | → docs/architecture |
| 6 | Supported channels | Show breadth; trigger "does it do mine?" | Capability | → docs/channels |
| 7 | Why developers love it | Differentiate on DX | Resonance | → docs |
| 8 | Interactive API example | Show how easy sending is | Excitement | → docs/quickstart |
| 9 | SDK support | Confirm the stack fits | Capability | → /sdk |
| 10 | Analytics preview | Show the operational layer | Enterprise confidence | → dashboard demo |
| 11 | Template system | Show content flexibility | Capability | → docs/templates |
| 12 | Scheduling | Show timezone-aware delivery | Capability | → docs/scheduling |
| 13 | Provider ecosystem | Show vendor freedom + failover | Enterprise confidence | → docs/providers |
| 14 | Enterprise features | Answer security/scale questions | Enterprise confidence | → /enterprise |
| 15 | Performance metrics | Quantify reliability (truthful placeholders) | Technical trust | → status |
| 16 | Security | Answer compliance questions | Trust | → /security |
| 17 | Testimonials | Third-party validation | Trust | — |
| 18 | Pricing preview | Lower commitment anxiety | Low risk | → /pricing |
| 19 | FAQ | Remove last objections | Low risk | → pricing / contact |
| 20 | CTA | Final motivation | Motivation | → signup |
| 21 | Footer | Professional close; everything findable | Professionalism | → docs / status / dashboard |

### Per-section content notes

**Hero copy (final candidates):**
- H1: `One API. Every notification. Every platform.` — with "notification" and "platform" set in a violet-to-indigo gradient accent.
- Sub: `Send email, SMS, push, and desktop notifications from a single endpoint — with automatic retries, delivery analytics, and the reliability your production traffic deserves.`
- Microcopy under CTAs: `Free forever for 10,000 notifications / month. No credit card.`

**Channels section:** 8 primary channel cards + one "More coming" card listing Slack, Discord, Telegram, WhatsApp, Teams. Each card: line icon, name, one-line capability, status chip (Available / Coming soon).

**Why developers love it (3 cards):**
1. *Predictable delivery* — idempotent API, deterministic retries, per-channel backoff.
2. *A retry policy that makes sense* — exponential backoff with jitter, dead-letter visibility.
3. *Honest analytics* — every send produces an event; no vanity numbers.

**Enterprise (4 features):** SSO/SAML · Role-based access control · Audit logs · Uptime SLA.

**Performance metrics (placeholder-labeled, M6 wires real data):** `99.99% delivery SLA` · `p95 send latency 412ms` · `2.4B notifications delivered` · `180 countries reached`. Each carries a footnote: *sample figures — production data ships with your account*.

**Security:** TLS 1.3 in transit, AES-256 at rest, per-project API keys, key rotation, SOC 2 Type II in progress (honest), GDPR-ready.

**Testimonials (3):** placeholder personas with realistic role chips (Staff Engineer, Mobile Lead, CTO) — content collection, swapped at launch for real customers.

---

## 6. Navigation Structure

Desktop (≥1024px):

```
[◐ Notifi]  Products ▾   Developers ▾   Resources ▾   Pricing   [Docs] [Status] [Start building free]
```

- Center: Products, Developers, Resources (mega menus), Pricing (link).
- Right: Docs (secondary), Status (ghost), Start building free (primary).
- Below 1024px: hamburger → full-screen drawer with all groups expanded (accordion) + CTA block.
- Header: sticky, 64px, `backdrop-blur` only when scrolled, border-bottom appears on scroll. Reduced-motion: no blur transition, instant.

---

## 7. Mega Menu Structure

### Products (2 columns + feature tile)

```
Notifications            Channels
├── API Reference   →    ├── Email            ├── Web Push
├── SDKs           →     ├── SMS             ├── Linux
├── Templates      →     ├── Android (FCM)   ├── macOS
├── Scheduling     →     ├── Apple (APNS)    ├── RCS
└── Providers      →     └── …more coming    └── Slack/Discord/… (coming)
[Feature tile: "New — Channel templates" → changelog]
```

### Developers (3 groups)

```
Getting Started          Guides              Reference
├── Quickstart       →   ├── Retries & delivery    ├── API Reference
├── Authentication   →   ├── Rate limits           ├── SDK Docs
├── Environments     →   ├── Webhooks              ├── Errors
└── CLI              →   └── Migrations            └── Changelog
```

### Resources (2 groups)

```
Learn                        Company
├── Blog                  →  ├── Pricing
├── FAQ                   →  ├── Security
├── Status                →  ├── Enterprise
└── Support               →  └── Contact Sales
```

Mega menu behavior: disclosure on hover (desktop) + focus/click (keyboard), 150ms fade + 4px rise, `aria-expanded`, Escape closes, closes on outside click, full keyboard navigation, `role="navigation"` region label per menu.

---

## 8. Component Inventory

### Layout & chrome
| Component | Notes |
|---|---|
| AnnouncementBanner | Dismissible, session-only; slides down from top edge |
| Header (sticky) | 64px; scroll-aware border + blur |
| MegaMenu | Products / Developers / Resources |
| MobileDrawer | Full-screen, accordion groups |
| Footer | 5 columns + status row + newsletter |
| SkipLink | First focusable element |
| ThemeToggle | `<html data-theme>` inline script, no flash |
| LiveStatusPill | Static snapshot of status.notifi.dev API + link |

### Section components
| Component | Notes |
|---|---|
| Hero | Headline, sub, dual CTA, meta line, flow diagram, floating code window |
| TrustBar | Logo placeholders (grayscale, 60% opacity, hover 100%) |
| ArchitectureDiagram | Server-rendered SVG, stage captions, hover stage highlight |
| ChannelCard | Icon, name, capability, status chip |
| ChannelGrid | 8 cards + coming-soon card |
| FeatureCard | 3-col: Why developers love it |
| ApiExample | Tabbed code (JS/Python/Go/cURL) + response + timeline |
| SdkChips | Language chips with install version |
| AnalyticsPreview | Dashboard mock: counters, channel-split bars, latency sparkline |
| TemplateCard | Template name + rendered preview + tags |
| ScheduleVisual | Week grid with timezone selector (static illustration) |
| ProviderCard | Provider + role (primary/failover) + status |
| EnterpriseFeature | 4 bento tiles |
| MetricCard | Animated counter (CSS-only, reduced-motion safe) |
| SecurityRow | Icon + statement pairs |
| TestimonialCard | Quote, name, role chip |
| PricingCard | 3 tiers, popular badge, feature list, CTA |
| FaqAccordion | Native `<details>` enhanced, or disclosure pattern |
| CtaSection | Headline + email input + button |
| NewsletterForm | Email + consent checkbox + success state |

### UI primitives (reuse from design language §12)
Button (5 variants × 3 sizes), Card, Badge/Chip, CodeWindow (filename bar + copy), Tabs, Accordion, Input, Icon (lucide-style inline SVG), Skeleton.

---

## 9. Design System Specification

The landing site consumes the Master Design Language (§4–§14 of `app/design-language.md`) without modification. Marketing-specific allowances:

- Radius: `--radius-2xl`/`--radius-3xl` on hero visuals and showcase cards (allowed marketing-only tokens).
- Shadows: `--shadow-lg` + `--shadow-glow` reserved for hero and primary CTA.
- Brand gradient: violet→indigo permitted only on hero headline accent and CTA section background wash (alpha ≤ 0.12).
- Grid: marketing container `80rem`, gutters 24px, 12 columns.
- Every component must exist in both themes; dark is default and first-drawn.

---

## 10. Color Palette Recommendations (Landing Expression)

From §4 of the design language, applied to marketing:

| Role | Dark (default) | Light |
|---|---|---|
| Canvas | `--color-canvas` dark: `oklch(0.14 0.008 280)` | light: `oklch(0.985 0.002 280)` |
| Hero wash | radial `oklch(0.515 0.235 280 / 0.14)` top-center, fading at 60% height | `oklch(0.515 0.235 280 / 0.06)` |
| Headline accent | gradient `oklch(0.68 0.19 280) → oklch(0.72 0.17 310)` | `oklch(0.515 0.235 280) → oklch(0.455 0.215 310)` |
| Primary CTA | `--color-primary-400` fill, white text, glow | `--color-primary-500` fill, white text |
| Success (delivered chips) | `oklch(0.80 0.16 150)` text, 12% fill | `oklch(0.50 0.14 150)` text, 10% fill |
| Code windows | `--color-surface-sunken` | `--color-surface-sunken` |

Ratios: violet 10–20% of viewport on marketing; neutral scale carries the rest. Semantic colors only for state glyphs and status chips.

---

## 11. Typography Recommendations

- Families: Geist (display/body), Geist Mono (code, metrics, eyebrows). From design language §5.
- Hero H1: Display tier — `clamp(40px, 6vw, 88px)`, 700, `-0.03em`, line-height 1.05. Headline breaks at natural phrase boundaries ("Every notification." on its own line at ≥768px).
- Section H2: 28–32px, 700, `-0.02em`; eyebrow above in mono 12px uppercase `0.08em` spacing, violet-400 (dark) / primary-600 (light).
- Body: 16–17px, `--measure: 68ch`; captions 12px mono for code-adjacent labels.
- Metrics and counters: Geist Mono, tabular figures, 28–40px in cards.
- Code: Geist Mono 13.5px, syntax palette per §12.5 of the design language.

---

## 12. Motion & Animation Guidelines

Per-section choreography (design language §9 tokens):

| Section | Animation | Duration / easing |
|---|---|---|
| Hero | Staggered entrance: eyebrow → H1 → sub → CTAs (40ms steps); flow diagram starts after 600ms; floating code window fades in at 400ms | 400ms, `--ease-out` |
| Delivery flow | Packet dot travels API → … → Delivered; pauses 300ms at each node; loops after 2.4s rest | 600ms segments, linear |
| Trust bar | Logos fade in staggered after hero | 250ms |
| Section reveals | On-scroll fade + rise 12px (IntersectionObserver island, once) | 400ms, 80ms stagger |
| Channel cards | Hover: border shift + shadow one step; icon stroke brightens | 150ms `ease-out` |
| API example | Tab switch: content crossfade 150ms; "sending…" → "delivered" status sequence 900ms | 150/400ms |
| Counters | CSS counter/JS-free tabular count to final value on reveal | 600–900ms, `--ease-out` |
| Accordion | Height 250ms; chevron 150ms | 250ms |
| CTA | Primary CTA glow pulse (subtle, 2.4s loop) — dark mode only | — |

Rules: no element animates longer than 900ms except the hero flow loop; at most one looping animation per viewport; every entrance has exactly one trigger (scroll or load), never both. Reduced motion: all transforms removed, counters render final values, flow renders static with arrow markers (§9.4).

---

## 13. Responsive Behavior per Section

| Section | <640 | 640–1023 | 1024–1439 | ≥1440 (ultrawide) |
|---|---|---|---|---|
| Header | Hamburger; CTA icon-only (→) | Hamburger; CTA label | Full nav + mega menus | Same |
| Hero | Stacked; flow diagram below CTAs; H1 40px | Diagram right (55/45 split) or below | 55/45 split, diagram right | Diagram scales to 64rem max, centered |
| Trust bar | 2-col logo grid | 3-col | 6-col row | 6-col row, wider |
| Architecture | Diagram horizontal-scroll; captions below | Diagram vertical; sticky caption column | Sticky left column (40%) + diagram right | Same |
| Channels | 1-col | 2-col | 4-col grid | 4-col grid, 80rem container |
| Why devs love it | 1-col | 3-col (compact) | 3-col | Same |
| API example | Tabs scrollable; code window full-width | Same | Code + response side-by-side | Same |
| SDK chips | Wrap, 3 per row | 6 per row | Single row | Same |
| Analytics preview | Stacked counters, then chart | 2-col | Dashboard mock full-bleed | Same |
| Templates / Scheduling / Providers | 1-col stacks | 2-col | 3-col | Same |
| Enterprise | 1-col | 2×2 bento | 2×2 bento | Same |
| Performance | 1-col counters | 2-col | 4-col | Same |
| Testimonials | 1-col | 3-col | 3-col | Same |
| Pricing | 1-col; Pro featured full-width | 3-col | 3-col | Same |
| FAQ | Full-width | 68ch centered | Same | Same |
| CTA / Footer | Stacked groups | Footer 2-col grid | 5-col footer | Same |

Every section: `space-16` (64px) vertical rhythm on mobile, `space-20/24` on desktop; ultrawide caps content at 80rem with centered alignment.

---

## 14. Astro Folder Structure (with Explanations)

```
app/landing/
├── astro.config.mjs         # SSG, site: https://notifi.dev, trailingSlash: "never",
│                            # integrations: sitemap, mdx (blog), image; compressHTML on
├── content.config.ts        # Content collection schemas (Zod): channels, features,
│                            # testimonials, faqs, pricing, changelog, blog, navigation
├── public/                  # Static: favicon (logo-icon.svg), og images, robots.txt,
│                            # site.webmanifest, .well-known/ (security.txt, llms.txt)
└── src/
    ├── layouts/             # BaseLayout (head, meta, theme script, SEO, OG, JSON-LD)
    │                        # MarketingLayout (header + footer shell, skip link)
    │                        # PageLayout (docs-style inner pages: legal, security)
    ├── components/
    │   ├── ui/              # Reusable primitives: Button, Card, Badge, CodeWindow,
    │   │                    # Tabs, Accordion, Input, Icon, Skeleton, Logo, ThemeToggle
    │   ├── sections/        # One file per homepage section (Hero.astro, Channels.astro,
    │   │                    # Faq.astro, Footer.astro …) — composable via props
    │   ├── diagrams/        # Server-rendered SVG: FlowDiagram, ArchitectureDiagram,
    │   │                    # ScheduleVisual, ChannelIcons (zero JS by default)
    │   └── mock/            # Dashboard mock: AnalyticsMock, MetricCard, Sparkline
    ├── islands/             # The ONLY client JS: ThemeToggle, RevealObserver,
    │                        # FaqAccordion (if not native), NewsletterForm,
    │                        # CodeCopy, TabPanel (interactive API example)
    ├── content/             # Astro Content Collections (content.config.ts schemas):
    │   ├── channels/        # channel YAML: name, icon, status, blurb
    │   ├── features/        # "why developers love it" + enterprise + security entries
    │   ├── testimonials/    # quote, name, role (swapped at launch)
    │   ├── faqs/            # question, answer (JSON-LD FAQPage generated)
    │   ├── pricing/         # tiers, features, popular flag
    │   ├── changelog/       # dated releases (also feeds /changelog)
    │   └── blog/            # MDX posts (drafts, published flags, OG images)
    ├── lib/                 # Pure TS: content queries, seo.ts (metadata builders),
    │                        # schema.ts (JSON-LD), utils.ts (dates, classNames)
    ├── assets/              # svg icons (lucide-style inline), brand logos, fonts
    ├── styles/              # global.css — imports design-language tokens as CSS
    │                        # custom properties (dark default, light via [data-theme])
    └── pages/
        ├── index.astro      # Homepage — composes sections in order (wireframe §4)
        ├── pricing.astro    # /pricing
        ├── changelog.astro  # /changelog (+ static pagination)
        ├── security.astro   # /security
        ├── sdk.astro        # /sdk
        ├── enterprise.astro # /enterprise
        ├── contact-sales.astro
        ├── newsletter.astro
        ├── blog/index.astro + blog/[slug].astro
        ├── legal/privacy.astro, legal/terms.astro
        ├── 404.astro
        └── rss.xml.ts       # blog feed (sitemap integration covers sitemap.xml)
```

**Directory purposes (prompt requirement):**

- `src/layouts/` — page shells: head/meta/theme/SEO (Base), marketing chrome (Marketing), minimal inner-page chrome (Page).
- `src/components/ui/` — framework-agnostic presentational primitives shared across all pages.
- `src/components/sections/` — one Astro component per homepage section; the homepage is a composition, enabling reordering and reuse on subpages.
- `src/components/diagrams/` — server-rendered SVG, the product visualization layer.
- `src/islands/` — the only place client JS may live; each island is tiny, lazy, and optional for SSR correctness.
- `src/content/` — all copy as content collections so non-code changes never touch components; enables i18n later via `getCollection` locale.
- `src/lib/` — pure functions; unit-testable, no framework imports (mirrors `app/status/src/lib` convention).
- `src/assets/` — icons and imagery with build-time optimization.
- `src/styles/` — the single CSS entry mapping design-language tokens; no per-component stylesheets (co-located `<style>` for component-scoped tweaks only).
- `src/pages/` — file-based routing; every page has a `layout`, `title`, `description`, `canonical`, and `og` from `lib/seo.ts`.
- `public/` — unprocessed static assets (favicons, OG images, robots, feeds meta).
- `content.config.ts` — Zod schemas; type-safe content everywhere (matches `astro check`).

---

## 15. SEO Strategy

- **Semantic HTML:** one `h1` per page; section `h2`s in order; landmarks `header/nav/main/footer`; nav labeled; footer `nav` labeled.
- **Metadata:** every page: `title` (≤60 chars), `description` (≤160), `canonical`, `og:type/title/description/image/url/site_name`, `twitter:card`. Built by `lib/seo.ts` — a single function, no duplication.
- **Open Graph:** one OG image per page type — server-rendered or static template: 1200×630, violet canvas, logo, headline, mono metadata. Reused as social banner.
- **Structured data (JSON-LD):** `Organization` (sitewide, with logo, URL, sameAs), `SoftwareApplication` (homepage, applicationCategory: DeveloperApplication), `FAQPage` (from faqs collection), `BreadcrumbList` (blog/legal), `BlogPosting` (posts), `WebSite` with `SearchAction` omitted (no site search yet).
- **Core Web Vitals:** SSG HTML, no render-blocking JS (islands are `load` or `visible` deferred), fonts `font-display: swap` with `preload` on the two weights used above the fold, images `srcset` + `width/height` (zero CLS), code windows are text (no images of code).
- **Robots/sitemap:** `sitemap-index.xml` from `@astrojs/sitemap`, `robots.txt` referencing it, `llms.txt` (developer-facing summary + key URLs — new standard, cheap win for an AI-era dev audience).
- **Internal linking:** every section links to the matching docs page; footer links to all surfaces; breadcrumbs on blog/legal; no orphan pages.
- **Readable URLs:** kebab-case, no query strings for content, trailingSlash never, redirects (`astro.config` redirects) for legacy names.

---

## 16. Accessibility Strategy

Design language §17 applies wholesale. Landing-specific commitments:

- **Skip link** first element, visible on focus.
- **Mega menus:** hover-intent on pointer, full keyboard (arrow keys across columns, Escape closes, focus returns to trigger), `aria-expanded` + `aria-controls`.
- **Hero flow diagram:** `role="img"` + `aria-label` summarizing the flow ("A single API request fans out through a queue to email, SMS, and push, all delivered."); animated state is decorative — never the only message.
- **Code windows:** `aria-label="Example: send a notification with JavaScript"`; copy button has a visible label + `aria-live` confirmation.
- **Animated counters:** text nodes hold final values; animation is decorative (screen readers and reduced-motion users get the final number instantly).
- **Contrast:** all text AA; the violet gradient headline accent checked at 4.5:1 on both themes; `prefers-contrast: more` raises border tokens.
- **Touch:** all CTAs ≥ 44px; card targets have ≥ 8px separation; mega menus degrade to full-screen drawer.
- **Forms:** labels visible, `aria-describedby` for helper/error, success state announced (`role="status"`).
- **Focus order** matches visual order on every responsive breakpoint (test the drawer on mobile).
- **Language:** `lang="en"` on `<html>`; theme script inline in `<head>` prevents theme flash without blocking render.

---

## 17. Conversion Optimization Strategy

| Goal | Mechanism | Placement |
|---|---|---|
| Sign up (primary) | "Start building free" primary CTA + "No credit card · 10k free/mo" meta | Header, hero, CTA section, pricing (all tiers) |
| Docs engagement | "Read the docs" secondary CTA; every section links to relevant docs page | Hero, features, API example, SDKs |
| Dashboard demo | "Explore dashboard" — analytics preview links to a demo | Analytics section |
| API reference | Tabs in API example link to the full reference | API example section |
| Guides | Blog + changelog cards link to docs guides | Blog, changelog |
| Pricing | Popular tier highlighted; comparison table on /pricing; FAQ kills objections before pricing | Pricing section + page |
| Enterprise | "Contact sales" secondary in enterprise + security sections; /contact-sales page | Enterprise, security |
| Status trust | LiveStatusPill links status; performance numbers link status page | Header, performance section |
| Newsletter | Email capture at CTA section + footer | CTA, footer |

Rules: exactly **one primary CTA per viewport**; all others secondary/ghost. Every CTA's destination is a real page (no dead ends). All conversion points work without JS (forms fall back to `mailto`/static confirmation).

---

## 18. Content Strategy

### Hierarchy

| Layer | Message | Voice |
|---|---|---|
| Hero | One API, every channel, every platform | Direct, declarative |
| Feature | Delivery you can count on — retries, backoff, analytics | Concrete, benefit-first |
| Developer | A quickstart that fits your stack; SDKs for your language | Terse, code-first |
| Enterprise | SSO, RBAC, audit logs, SLA — the checklist | Factual, checklist-shaped |
| Trust | Numbers, architecture, security, status, people | Evidenced, calm |
| Technical | Copy is accompanied by real API shapes, never screenshots of docs | Precise |
| CTA | Result-oriented, risk-reversing | Imperative, short |
| Footer | Wayfinding + proof links (status) | Minimal |

### Copy rules (design language §15)
- Eyebrows (mono, uppercase): `ONE API · EVERY CHANNEL`, `DELIVERY PIPELINE`, `ENTERPRISE-READY`.
- Headlines ≤ 8 words. Sentences ≤ 20 words.
- Forbidden words list enforced in copy review (see §15.2).
- Honesty: performance metrics are placeholder-labeled until M6 wires real data; "Coming soon" chips used for Slack/Discord/Telegram/WhatsApp/Teams and SOC 2.

### Draft key copy (final candidates)

| Slot | Copy |
|---|---|
| Hero H1 | One API. Every notification. Every platform. |
| Hero sub | Send email, SMS, push, and desktop notifications from a single endpoint — with automatic retries, delivery analytics, and production-grade reliability. |
| Architecture H2 | Built like the infrastructure you already run |
| Architecture sub | Notifi is not a wrapper. It is a pipeline: ingest, outbox, queue, workers, providers — each stage observable. |
| Channels H2 | One integration. Every channel. |
| API example H2 | Sending is one request. |
| Analytics H2 | Every send, measured. |
| Performance H2 | Numbers we stand behind. |
| CTA H2 | Ship your first notification in minutes. |
| CTA sub | Free for 10,000 notifications a month. No credit card. Your API key is ready in under a minute. |
| CTA button | Start building free |

---

## 19. Illustration & Visualization Recommendations

Design language §16 applies. Landing-specific executions:

1. **Hero delivery flow (signature motif):** SVG — a `POST /v1/notifications` node → outbox → queue (3 stacked bars) → workers → 4 channel glyphs (envelope, message, bell, monitor) → green "Delivered" flag. A violet packet travels the path on loop. Captions under nodes in mono 11px.
2. **Floating code window:** real, copy-pasteable JS snippet in a `CodeWindow` (filename `send.js`), tilted 2°, `--shadow-lg`; it contains the working quickstart (idempotency key + channel array).
3. **Architecture diagram:** wide SVG with 5 stages, per-stage hover highlights a caption column (sticky); arrows show fan-out; a failed-provider edge shows a violet retry loop.
4. **Analytics mock:** dashboard-style panel (server-rendered): 4 counters (Delivered today, Open rate, Channel split, p95 latency), horizontal channel bars, latency sparkline — all SVG/CSS, tabular figures, skeleton-free (it is a static illustration, marked "Sample data").
5. **Schedule visual:** 7×24 grid with violet delivery windows across three timezones (UTC, EST, IST) — communicates timezone-aware scheduling without a real widget.
6. **Channel icons:** consistent 24px line glyphs (design language §16), tinted violet-200/400 in dark, 500/600 in light.
7. **OG images:** template per page type — violet radial, logo mark, headline in Geist 700, mono footer line.
8. **No stock photography, no 3D renders, no clipart.** When a real capability exists (dashboard), show a faithful mock; never a fake screenshot.

---

## 20. Milestone-Based Implementation Plan

| Milestone | Scope | Exit criteria |
|---|---|---|
| **M0 — Scaffold & tokens** | `app/landing/` Astro project (config, content.config.ts, layouts, global.css with design-language tokens, BaseLayout + MarketingLayout, ThemeToggle, fonts, public/ favicon + robots) | `npm run dev` renders a themed shell; `astro check` + `build` green |
| **M1 — Navigation & hero** | AnnouncementBanner, Header, MegaMenu, MobileDrawer, Hero (flow diagram, code window, background layers), TrustBar, Footer skeleton, LiveStatusPill | Hero meets CWV budget (LCP < 2.0s, CLS 0); keyboard nav passes |
| **M2 — Capability sections** | Channels (8 + coming-soon), Why developers love it, SDK chips, Templates, Scheduling | All content from collections; responsive tables §13 verified |
| **M3 — Technical trust** | ArchitectureDiagram, Interactive API example (tabs + send/delivered sequence), Analytics mock, Performance counters | Counter + flow animations reduced-motion safe; tabs keyboard accessible |
| **M4 — Enterprise & proof** | Enterprise features, Security, Provider ecosystem, Testimonials, /security, /enterprise, /contact-sales | All pages canonical + OG complete; content collections populated |
| **M5 — Conversion** | Pricing preview + /pricing, FAQ (JSON-LD FAQPage), CTA section + newsletter, footer complete, 404, sitemap/rss/llms.txt | Every CRO goal (§17) has a destination; Lighthouse ≥ 95 |
| **M6 — SEO, a11y & data wiring** | Blog + changelog collections and pages, OG image templates, structured data audit, contrast/axe pass, swap placeholder performance numbers to status API snapshot, design-language convergence check | Full Lighthouse suite (desktop + mobile) ≥ 95; `prefers-reduced-motion` verified; axe 0 violations; sitemap valid |

Dependencies: M0 → M1 → M2/M3 (parallel) → M4 → M5 → M6. Each milestone ends with `npm run build && npm run check` green and a Lighthouse screenshot recorded.
