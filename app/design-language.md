# Notifi — Master Design Language

**Status:** Canonical design DNA for the entire Notifi platform. Every product prompt (landing, dashboard, docs, status, auth, onboarding) begins by referencing this document. No app may define its own brand tokens, fonts, radii, or motion rules; any deviation must be proposed as a revision to this file.

**Canonical brand anchor:** violet `#5B3CD9` (`oklch(0.515 0.235 280)`), Geist + Geist Mono, dark-first.

**Convergence note:** The dashboard (cyan-steel, hue 202.8) and status (blue `#2563eb`, Inter) currently deviate from this language. They are not wrong today — this document is the target they migrate toward. New work must ship violet-first.

---

## 1. Purpose & Usage

This document is written to be consumed two ways:

1. **As a prompt preamble** — any AI agent asked to build or modify a Notifi surface (marketing site, dashboard, docs, status, auth flows, emails) begins with this file, then follows the product-specific spec.
2. **As an engineering spec** — the token reference in §18 is the single source of truth for CSS custom properties. Apps copy tokens into their own theme files; they do not invent new ones.

Hard rules:

- One accent color: violet. No per-app accent hues.
- One type family: Geist, plus Geist Mono for code. No other fonts.
- Dark mode is the default, light mode is a first-class theme — never an afterthought.
- Motion serves comprehension. If a motion has no communicative job, remove it.
- Every state a user can see (empty, loading, error, success) is designed, not discovered.
- Accessibility is not a layer — AA contrast, keyboard operability, and reduced motion are baked into the tokens and components in §17.

---

## 2. Brand Personality

Notifi is a notification platform developers trust with production traffic. The personality is:

| Trait | What it means in design decisions |
|---|---|
| Reliability | Numbers are truthful or omitted. No marketing math. Calm surfaces, no drama. |
| Performance | Fast by construction: static-first, minimal JS, no hero images over 200KB. |
| Engineering excellence | Precision typography, consistent 4px rhythm, documented systems. |
| Developer happiness | Copy is direct and useful; APIs are shown, not described; nothing is hidden behind jargon. |
| Security | SSO, encryption, audit logs are first-class sections, not badges. |
| Innovation | The product is different (one API, every channel); the site should feel quietly confident, not loud. |
| Simplicity | One idea per section. One primary action per view. |
| Scalability | Architecture is drawn honestly — queues, workers, providers — never hidden behind abstraction. |
| Enterprise confidence | Restraint. Dense, precise UI. No gradients for decoration. |
| Modern infrastructure | Violet-on-dark, schematic diagrams, mono type for data — the look of modern dev infrastructure. |

The brand voice in one sentence: **a senior engineer explaining the system to a peer — precise, brief, and confident.**

---

## 3. Design Principles

1. **Clarity first** — every screen answers "what am I looking at?" in under 5 seconds. Hierarchy, not decoration.
2. **Calm confidence** — strong structure, generous whitespace, restrained motion. No noise, no glitter.
3. **Craftsmanship in every pixel** — alignment on a visible grid, consistent radii, no 1px offsets, no orphan headings.
4. **Motion with intent** — animations explain causality (request → delivery) or direct attention. 100ms of purpose beats 1s of flash.
5. **Truthful data** — every metric displayed is real or clearly a placeholder. Never fabricate performance claims.
6. **Accessible by default** — AA contrast, full keyboard paths, word+glyph states, reduced-motion respected.
7. **Fast by construction** — SSG, zero-JS-by-default, SVG over canvas, system fonts as fallback, images sized and lazy.
8. **Consistent, not uniform** — marketing may be more expressive than the dashboard, but both use the same tokens, fonts, and radii. Personality changes; DNA does not.

---

## 4. Color System

All colors are OKLCh. The canonical anchor values come from `shared/identity/colors.md`; the scale below extends it.

### 4.1 Primary scale

| Token | Light | Dark | Usage |
|---|---|---|---|
| `--color-primary-50` | `oklch(0.965 0.02 280)` | `oklch(0.30 0.12 280)` | Tinted backgrounds, hover fills |
| `--color-primary-100` | `oklch(0.925 0.05 280)` | `oklch(0.34 0.15 280)` | Selected backgrounds, chips |
| `--color-primary-200` | `oklch(0.855 0.10 280)` | `oklch(0.40 0.18 280)` | Hover borders, chart fills |
| `--color-primary-300` | `oklch(0.775 0.15 280)` | `oklch(0.48 0.21 280)` | Icons on dark, focus rings (dark) |
| `--color-primary-400` | `oklch(0.68 0.19 280)` | `oklch(0.65 0.22 280)` ≈ `#8B6DF0` | Accent text on dark, focus rings |
| `--color-primary-500` | `oklch(0.515 0.235 280)` ≈ `#5B3CD9` | `oklch(0.68 0.19 280)` | Primary buttons, links (light) |
| `--color-primary-600` | `oklch(0.455 0.215 280)` | `oklch(0.72 0.17 280)` | Hover: primary buttons (light) |
| `--color-primary-700` | `oklch(0.395 0.19 280)` | `oklch(0.78 0.13 280)` | Active, pressed states (light) |
| `--color-primary-800` | `oklch(0.34 0.16 280)` | `oklch(0.85 0.08 280)` | Text on tinted backgrounds |
| `--color-primary-900` | `oklch(0.28 0.12 280)` | `oklch(0.92 0.04 280)` | Headings on tinted backgrounds |
| `--color-primary-foreground` | `oklch(0.985 0 0)` ≈ `#FAFAFA` | `oklch(0.985 0 0)` | Text/icon on primary fills |

**Rules:**
- Primary fills (buttons, active nav, selected rows): use 500 in light, 400 in dark.
- Primary text on light backgrounds: never below 600.
- Brand gradient (marketing only): `oklch(0.515 0.235 280)` → `oklch(0.42 0.20 310)` (violet → indigo). One gradient direction, subtle use only.

### 4.2 Neutral scale

| Token | Light | Dark | Usage |
|---|---|---|---|
| `--color-canvas` | `oklch(0.985 0.002 280)` | `oklch(0.14 0.008 280)` | Page background |
| `--color-surface` | `oklch(0.998 0.001 280)` | `oklch(0.17 0.009 280)` | Cards, nav, dialogs |
| `--color-surface-muted` | `oklch(0.965 0.002 280)` | `oklch(0.205 0.010 280)` | Slightly raised panels, code headers |
| `--color-surface-sunken` | `oklch(0.955 0.002 280)` | `oklch(0.125 0.008 280)` | Code blocks, terminal windows |
| `--color-border-subtle` | `oklch(0.915 0.003 280)` | `oklch(0.235 0.010 280)` | Default card/input borders |
| `--color-border` | `oklch(0.875 0.004 280)` | `oklch(0.275 0.010 280)` | Stronger separators, hover borders |
| `--color-foreground` | `oklch(0.16 0.006 280)` | `oklch(0.94 0.004 280)` | Primary text |
| `--color-foreground-muted` | `oklch(0.45 0.008 280)` | `oklch(0.68 0.006 280)` | Secondary text, descriptions |
| `--color-foreground-faint` | `oklch(0.58 0.006 280)` | `oklch(0.55 0.005 280)` | Placeholders, captions, disabled |

**Rules:**
- Elevation in dark mode comes from surface lightness, not shadows.
- Never use pure black or pure white for surfaces; always the tinted tokens above.

### 4.3 Semantic scale

| Token | Light / Dark (same) | Usage |
|---|---|---|
| `--color-info` | `oklch(0.623 0.214 259.815)` | Informational status, tips |
| `--color-success` | `oklch(0.723 0.219 149.579)` | Delivered, success states |
| `--color-warning` | `oklch(0.769 0.188 70.08)` | Retries, degraded, warnings |
| `--color-error` | `oklch(0.637 0.237 25.331)` | Failures, destructive actions |

- Semantic colors are for **states, not decoration**. Never use success green for branding.
- On dark surfaces, use the 400/500-equivalent tints of each hue for text (approx `oklch(0.80 …)`) and 12–15% alpha fills for backgrounds.

### 4.4 Chart palette

`chart-1` primary violet `oklch(0.62 0.21 280)` (dark) / `oklch(0.515 0.235 280)` (light) · `chart-2` success green · `chart-3` info blue · `chart-4` warning amber · `chart-5` error red. Series order fixed; never swap.

### 4.5 Usage ratios

- Violet should be roughly **5–10% of any viewport** on the dashboard, **10–20%** on marketing surfaces (freedom to be expressive). If a screen feels "too violet", it is.
- Neutrals carry 80%+ of the visual weight. Violet marks interactivity and identity; it does not paint backgrounds.

---

## 5. Typography

### 5.1 Families

| Role | Family | Source |
|---|---|---|
| Display / body | **Geist** | `geist` npm package (Vercel) |
| Code / data | **Geist Mono** | `geist` npm package |

Weights: 400 (body), 500 (nav, buttons), 600 (subheads, emphasis), 700 (headings, display). No variable-weight gymnastics; no italics except for legal or inline citations.

### 5.2 Scale (marketing + product)

| Level | Size (px) | Weight | Line-height | Letter-spacing | Use |
|---|---|---|---|---|---|
| Display | 64–88 (clamped `clamp(40px, 6vw, 88px)`) | 700 | 1.05 | `-0.03em` | Hero headlines only |
| H1 | 36–48 | 700 | 1.1 | `-0.025em` | Page titles |
| H2 | 28–32 | 700 | 1.15 | `-0.02em` | Section headings |
| H3 | 22–24 | 600 | 1.25 | `-0.015em` | Card titles, subsections |
| H4 | 18–20 | 600 | 1.3 | `-0.01em` | Small card titles |
| Body | 16 | 400 | 1.6 | `0` | Default |
| Body small | 14 | 400 | 1.55 | `0` | Secondary text |
| Caption | 12 | 500 | 1.4 | `0.01em` | Labels, timestamps, badges |
| Code | 13.5–14 | 400 | 1.6 | `-0.005em` | Inline code, blocks, metrics |
| Button | 14 | 500 | 1 | `0` | Button labels |

### 5.3 Rules

- Reading measure: **68ch max** for prose; code blocks up to 110ch.
- Headings: semibold/bold, tighter tracking, no uppercase headings except micro-labels (12px, `letter-spacing 0.08em`, uppercase) — used sparingly for eyebrow text.
- Body: never below 14px in product UI, never below 16px on marketing surfaces.
- Numbers and metrics use Geist Mono with tabular figures where precision matters.
- Text alignment: left-aligned everywhere except short centered hero lines; never justified.

---

## 6. Spacing System

4px base scale, applied everywhere:

`0, 4, 8, 12, 16, 20, 24, 32, 40, 48, 64, 80, 96, 128`

Semantic slots:

| Token | Value | Use |
|---|---|---|
| `--space-1` | 4px | Icon-to-label gap, tight insets |
| `--space-2` | 8px | Nested spacing, chip padding |
| `--space-3` | 12px | Compact component padding |
| `--space-4` | 16px | Default padding, gap |
| `--space-5` | 20px | Card padding (small) |
| `--space-6` | 24px | Card padding (default) |
| `--space-8` | 32px | Section internal gaps |
| `--space-12` | 48px | Sub-section rhythm |
| `--space-16` | 64px | Section spacing (mobile) |
| `--space-20` | 80px | Section spacing (desktop) |
| `--space-24` | 96px | Major section separation |
| `--space-32` | 128px | Landing page section breaks |

Rules: section padding uses the 64/80/96 rhythm; never mix 32 and 40 in the same column; group related elements at ≤24px and separate unrelated groups at ≥48px.

---

## 7. Radius System

| Token | Value | Use |
|---|---|---|
| `--radius-sm` | 6px | Badges, chips, inputs (compact) |
| `--radius-md` | 8px | Buttons, small cards, inputs |
| `--radius-lg` | 12px | Default cards, popovers |
| `--radius-xl` | 16px | Feature cards, marketing panels |
| `--radius-2xl` | 20px | Hero visuals, large dialogs |
| `--radius-3xl` | 24px | Only on marketing showcase surfaces |
| `--radius-full` | 9999px | Pills, dots, toggles |

Rules: internal radii are always `radius - 4px` from the container (nesting rule). Never mix more than two radii within one component.

---

## 8. Shadows & Elevation

Three levels; elevation is expressed as **border + shadow**, never shadow alone (keeps light mode honest and dark mode clean).

| Level | Light | Dark |
|---|---|---|
| `--shadow-sm` | `0 1px 2px oklch(0 0 0 / 0.05)`, border subtle | `0 1px 2px oklch(0 0 0 / 0.3)`, border subtle |
| `--shadow-md` | `0 1px 2px oklch(0 0 0 / 0.05), 0 8px 24px oklch(0 0 0 / 0.08)`, border | `0 4px 16px oklch(0 0 0 / 0.4)`, border |
| `--shadow-lg` | `0 2px 4px oklch(0 0 0 / 0.05), 0 16px 48px oklch(0 0 0 / 0.12)`, border | `0 12px 40px oklch(0 0 0 / 0.5)`, border |
| `--shadow-glow` | — (light mode uses shadow-md) | `0 0 0 1px oklch(0.65 0.22 280 / 0.35), 0 8px 40px oklch(0.515 0.235 280 / 0.25)` |

Rules: shadows denote floatable layers (modals, dropdowns, cards at rest on light surfaces). In dark mode, raise surface lightness before reaching for shadow. Glow is reserved for the primary CTA and hero highlight, once per viewport.

---

## 9. Motion Philosophy

### 9.1 Durations

| Token | Value | Use |
|---|---|---|
| `--dur-instant` | 100ms | Press feedback, checkmarks |
| `--dur-fast` | 150ms | Hover, color transitions |
| `--dur-base` | 250ms | Standard appear/disappear, tooltips |
| `--dur-slow` | 400ms | Section reveals, cards entering |
| `--dur-emphasis` | 600ms | Hero entrance, full-flow animations |

### 9.2 Easing

- Default: `cubic-bezier(0.16, 1, 0.3, 1)` — "expo-out" style, fast start, long settle. Use for all entrances and layout motion.
- Hover states and colors: `ease-out` (150ms).
- Never use `ease-in-out` for entrances; never bounce; never spring for production UI.

### 9.3 Motion rules

- Entrance: fade + rise `8–16px`, 250–400ms, staggered children at 60–80ms intervals, max 6 children.
- Causality animation (notification flows, queue diagrams): moving a token/particle along a path is always 400–600ms, easing linear for flow, with a 300ms pause at each node.
- Loading: skeleton shimmer loops at 1.4s, opacity 0.5→1.0, never scale.
- Counter/stat animations: 600–900ms with expo-out, using `requestAnimationFrame`-free techniques (CSS or deferred hydrate), numbers tabular to avoid jitter.
- Hover: elevation `+4px` shadow step and border-color shift at 150ms. Transform scale is only for icon buttons (1.04 max) and only on pointer devices.
- Page transitions (if used): 150ms opacity crossfade, no layout shift.

### 9.4 Reduced motion

Respect `prefers-reduced-motion: reduce`: zero transform-based animation; opacity crossfades ≤150ms allowed; counters render final value instantly; flow diagrams render statically with arrow markers. This is a requirement, not a courtesy.

---

## 10. Iconography

- **Style:** lucide-style line icons — 24px grid, 1.75px stroke (16px icons: 1.75px; 20/24px: 1.5–2px), rounded caps and joins.
- **Source:** `lucide-react`/`lucide` (dashboard) and inline SVG copies (static sites). No icon libraries beyond lucide.
- **Sizes:** 16 inline (inline code/actions), 20 compact UI, 24 default, 32/48 feature illustrations (stroke 1.5).
- **Color:** inherits text color; violet only to signal interactive/active; semantic colors for status icons only.
- **Rules:** an icon must be understandable in isolation; never use emoji as icons in product UI; decorative icons are `aria-hidden`.

---

## 11. Grid & Layout

- 12-column grid, gutters 24px (marketing) / 16px (product), outer margin 24px mobile → fluid to container.
- Containers: product `72rem`, marketing `80rem`, prose `40rem` (68ch).
- Breakpoints: `640 / 768 / 1024 / 1280 / 1536`. Design at 1440; verify at 390, 768, 1024, 1536+.
- Density: product UI one-click-per-44px-row minimum; marketing whitespace is a feature — do not compress sections to fit above the fold.
- Alignment: everything on the 4px grid; optical centering overrides pixel centering for icons inside shapes.

---

## 12. Component Behavior

### 12.1 Buttons

| Variant | Style | Use |
|---|---|---|
| Primary | Violet fill (`primary-500` light / `primary-400` dark), white text, `radius-md`, hover → 600, active → 700, focus ring | The one action per view |
| Secondary | `surface` bg, border, fg text; hover → `surface-muted`, border strongens | Neutral actions |
| Ghost | Transparent, fg-muted text; hover → `surface-muted` | Inline/toolbar actions |
| Danger | Transparent, error text; hover → error-tinted fill (12% alpha) | Destructive, confirmations |
| Link | Violet text, underline-offset 4px, no fill | In-prose actions |

Sizes: `sm` 32px / `md` 40px / `lg` 48px height. Radius: `radius-md` all sizes. Padding: `sm` 12×24, `md` 16×24, `lg` 20×32.

States: hover, active (translate-y-px), focus-visible ring (`2px` violet, offset `2px`), disabled (60% opacity, no hover), loading (spinner 14px + label retained, `aria-busy`). Minimum touch target 44px on touch devices.

### 12.2 Cards

- Surface bg, border, `radius-lg` default. Padding `space-6` (24px); compact `space-4`.
- Interactive cards (clickable): hover raises border + one shadow step at 150ms; whole card is one link; focus-visible ring on card.
- Card headers: icon 20px + title 16px/600 + optional action; body fg-muted 14px.

### 12.3 Inputs & forms

- 40px height, `radius-md`, border, `space-3` horizontal padding; focus ring (not border color change alone); placeholder = fg-faint; labels 14px/500 with 8px gap; error = error border + message with icon, 12px below; helper text fg-faint.
- Never hide the label; never disable autofill.

### 12.4 Badges & chips

- `radius-full` or `radius-sm`, 12px/500 label, 4px horizontal padding (pill), tinted background (10% alpha of the semantic/primary color) + colored text. Word + glyph for states, never color alone.

### 12.5 Code blocks & terminal windows

- Surface: `surface-sunken` in both themes (dark-first: it is dark in dark mode; in light mode a very light violet-tinted gray). Border, `radius-lg`.
- Header chrome: no traffic lights — a subtle title bar with filename left (mono, 12px, fg-faint) and optional copy button right.
- Code: Geist Mono 13.5px, `line-height 1.6`, horizontal scroll, 16px padding. Line numbers optional at fg-faint.
- Syntax palette: violet keywords, green strings, blue functions, amber numbers — consistent across all code in all apps.
- `aria-label` on the window, `role="group"` when it has a title.

### 12.6 Accordion, tabs, tooltips, dropdowns, dialogs, toasts

- Accordion: disclosure pattern, 250ms height animation, chevron rotates 90°, one section open at a time in FAQ (optional elsewhere).
- Tabs: underline/pill style, 150ms indicator slide, `role="tablist"` semantics, arrow-key navigation.
- Tooltip: appears at 150ms, `radius-md`, 12px text, `role="tooltip"` for meaningful-only info; never required to understand the UI.
- Dropdown/mega menu: disclosure, 150ms fade + 4px rise, close on Escape/outside click, full keyboard nav, `aria-expanded`.
- Dialog: 250ms, overlay 40% black, surface panel, focus moves in, trap, Escape closes, return focus on close.
- Toast: 250ms enter, 4s auto-dismiss + manual close, semantic left-border color, `role="status"` / `role="alert"`.

---

## 13. States (empty, loading, error, success)

All four states are designed for every data surface. Common copy pattern: **title + one-line explanation + optional action**.

| State | Visual | Copy pattern | Example |
|---|---|---|---|
| Empty | 48px inline SVG line icon, centered, muted | Verb-object title, "here's how to start" line, one primary action | "No events yet — your first notification will appear here." + [Send a test notification] |
| Loading | Skeleton blocks matching final layout (shimmer 1.4s), `aria-busy` | — | Table → 6 skeleton rows |
| Error | Error icon, no color-only signaling; optional `correlation_id` in mono | "Something went wrong" + specific cause + retry/action | "We couldn't load channels. Retry" |
| Success | Success icon + concise confirmation | Verb + result, no exclamation marks | "Notification sent to 12,847 devices." |

Rules: never show "No data" (it is an error-coded empty); never show a spinner without a label; success is confirmed visually and, where meaningful, by message ID.

---

## 14. Dark / Light Theme Rules

- **Dark first.** Every layout is designed dark, then adapted to light. Dark is the default for marketing; product follows system preference with a manual toggle.
- Tokens are the same variable names in both themes (swap values only). No theme-specific component classes.
- Dark surfaces: tinted neutrals, elevation via lightness, borders `border-subtle` → `border` on hover, glow only for primary CTA.
- Light surfaces: near-white canvas, elevation via shadow + border, violet at full 500 strength, glow never.
- `color-scheme: dark` on the root in dark mode; form controls and scrollbars follow.
- Theme toggle must not flash — apply theme in a head inline script before paint (no flicker, no hydration dependency).

---

## 15. Tone of Voice & Copywriting

### 15.1 Voice

Confident, precise, calm. The author is a senior engineer at Notifi writing to another engineer. Facts do the selling.

### 15.2 Rules

- Lead with the benefit or result, then the mechanism.
- Active voice, present tense, imperative for CTAs ("Start building", "Send your first notification").
- Headings ≤ 8 words; sentences ≤ 20 words; paragraphs ≤ 3 sentences.
- Numbers: use figures, mono type for metrics, "12,847 devices" — never "over 10k".
- Microcopy: state what happens, not what the user does ("Keys rotate every 90 days" not "You can rotate your keys").
- Forbidden: exclamation marks in UI, "revolutionary", "disruptive", "seamless", "ultra-", "world-class", "powered by AI" (unless it is), "simply", "just", empty superlatives, fake urgency ("limited time", "act now"), emoji in product copy.
- Honesty: if a feature is upcoming, say "Coming soon" — never imply it exists.
- Error copy never blames the user.

### 15.3 Examples

- ❌ "Our platform allows you to easily send notifications to your users in a simple and seamless way."
- ✅ "One API request delivers to email, SMS, and push — with retries and delivery analytics built in."
- ❌ "Get started today!" → ✅ "Start building free — no credit card required."

---

## 16. Illustration Style

- **Schematic > literal.** Draw systems (queues, flows, topology) with precision; never generic 3D or flat-design marketing illustrations.
- Style: 1.5px line art in fg/fg-muted, filled nodes in violet-100–500 tints, rounded 4px corners on shapes, soft violet radial background washes (alpha ≤ 0.12) for marketing panels.
- The signature motif: **the delivery flow** — a packet/dot traveling a path through nodes (API → queue → workers → channels → delivered). It is the visual shorthand for Notifi everywhere.
- Diagrams are server-rendered SVG (static) with CSS-animated markers only in hero contexts; no JS animation libraries.
- Channel icons: abstract line glyphs per channel (envelope, chat bubble, bell, phone, monitor), 24px stroke 1.75, consistent corner geometry.

---

## 17. Accessibility Rules

- WCAG 2.2 **AA** minimum: 4.5:1 text, 3:1 UI components and icons; AAA on large marketing text where cheap.
- Focus: every interactive element shows `focus-visible` — 2px violet ring, 2px offset, never removed silently.
- Keyboard: full tab order, visible focus, no keyboard traps, disclosure/dialog/tablist ARIA patterns as specified in §12.
- Screen readers: landmark structure (`header/nav/main/footer`), one `h1` per page, heading order, `aria-label` on icon-only controls, word + glyph status.
- Reduced motion: §9.4 mandatory. Test with OS setting on.
- Touch: 44px minimum targets, 8px gaps between adjacent targets.
- High contrast: `prefers-contrast: more` raises border colors one step (the status app already does this — carry it everywhere).

---

## 18. Canonical Token Reference

Apps map this to their own format (Tailwind v4 `@theme`, CSS custom properties, or plain vars). Names are final.

```css
/* Brand */
--color-primary-*:          /* §4.1 table */
--color-primary-foreground: oklch(0.985 0 0);

/* Neutrals */
--color-canvas:            oklch(0.985 0.002 280) /* light */ oklch(0.14 0.008 280)  /* dark */
--color-surface:           oklch(0.998 0.001 280) /* light */ oklch(0.17 0.009 280)  /* dark */
--color-surface-muted:     oklch(0.965 0.002 280) /* light */ oklch(0.205 0.010 280) /* dark */
--color-surface-sunken:    oklch(0.955 0.002 280) /* light */ oklch(0.125 0.008 280) /* dark */
--color-border-subtle:     oklch(0.915 0.003 280) /* light */ oklch(0.235 0.010 280) /* dark */
--color-border:            oklch(0.875 0.004 280) /* light */ oklch(0.275 0.010 280) /* dark */
--color-foreground:        oklch(0.16 0.006 280)  /* light */ oklch(0.94 0.004 280)  /* dark */
--color-foreground-muted:  oklch(0.45 0.008 280)  /* light */ oklch(0.68 0.006 280)  /* dark */
--color-foreground-faint:  oklch(0.58 0.006 280)  /* light */ oklch(0.55 0.005 280)  /* dark */

/* Semantic */
--color-info:    oklch(0.623 0.214 259.815);
--color-success: oklch(0.723 0.219 149.579);
--color-warning: oklch(0.769 0.188 70.08);
--color-error:   oklch(0.637 0.237 25.331);

/* Type */
--font-sans: "Geist", system-ui, -apple-system, "Segoe UI", Roboto, sans-serif;
--font-mono: "Geist Mono", ui-monospace, SFMono-Regular, Menlo, monospace;

/* Spacing */
--space-1: 4px;  --space-2: 8px;   --space-3: 12px; --space-4: 16px;
--space-5: 20px; --space-6: 24px;  --space-8: 32px; --space-12: 48px;
--space-16: 64px; --space-20: 80px; --space-24: 96px; --space-32: 128px;

/* Radius */
--radius-sm: 6px; --radius-md: 8px; --radius-lg: 12px; --radius-xl: 16px;
--radius-2xl: 20px; --radius-3xl: 24px; --radius-full: 9999px;

/* Motion */
--dur-instant: 100ms; --dur-fast: 150ms; --dur-base: 250ms;
--dur-slow: 400ms; --dur-emphasis: 600ms;
--ease-out: cubic-bezier(0.16, 1, 0.3, 1);

/* Layout */
--container-product: 72rem; --container-marketing: 80rem; --measure: 68ch;
```

---

## 19. Migration Notes (dashboard, status, docs)

These are the only existing deviations; converge without breaking current builds:

| App | Current | Migrate to | Effort |
|---|---|---|---|
| Dashboard (`app/dashboard/app/globals.css`) | Primary hue `202.8` (cyan-steel), chart-1 same | Hue `280`; keep structure (oklch values swap; dark primary becomes `oklch(0.65 0.22 280)`) | Token swap only |
| Status (`app/status/src/styles/global.css`) | Accent `#2563eb`, Inter/JetBrains Mono | Violet `oklch(0.515 0.235 280)` light / `oklch(0.65 0.22 280)` dark; Geist + Geist Mono; keep the status-specific semantic palette (operational/degraded/… is domain data, not brand) | Token + font swap |
| Docs (Fumadocs) | Already violet per `shared/identity` | Align neutral scale + radii to this document | Minor |

Migration is incremental: ship new surfaces violet-first; convert existing surfaces in dedicated cleanup milestones, verifying with `astro check` / `npm run build` after each swap.
