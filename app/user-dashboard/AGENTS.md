# Dashboard Agent Instructions

*Full repository rules are in the root `AGENTS.md`. This file covers dashboard-specific conventions.*

## Commands

```shell
npm run dev      # Dev server (port 3000)
npm run build    # Production build & TypeScript check
npm run lint     # ESLint checks
```

## Key Conventions

1. **No `src/` directory**: Files live directly under `app/user-dashboard/`.
2. **Next.js 16 Params**: Route params are Promises. Use `use(params)` in client components or `await params` in server components.
3. **Base UI (shadcn/v4)**: Use `render` prop instead of Radix `asChild`.
4. **React 19 Purity**: Avoid `Math.random()` and synchronous `setState` in `useEffect`.

## Performance Notes

5. **Charts must use measured dimensions**: size Recharts via the shared
   `hooks/use-debounced-size.ts` hook (explicit pixel `width`/`height` on
   `ResponsiveContainer`), never `width="100%"`. The built-in observer
   re-renders full SVGs on every animation frame during layout transitions
   (e.g. the 150ms sidebar collapse), dropping frames on chart-heavy pages.
   Accepted tradeoff: charts may sit ~120ms stale mid-resize rather than
   jank the chrome.
6. **`.cv-auto` for heavy below-fold cards**: `content-visibility: auto`
   plus a 320px intrinsic placeholder (see `app/globals.css`) skips
   off-screen layout/paint — currently applied to the analytics chart
   cards. Do not use inside `h-full` grids or above the fold.
7. **Sidebar animation budget**: the collapse toggle animates
   `transition-[width] duration-150`. Keep per-frame work (observers,
   shadows, blurs) out of the collapse path.

Read the relevant guide in `node_modules/next/dist/docs/` if unsure about an API.
