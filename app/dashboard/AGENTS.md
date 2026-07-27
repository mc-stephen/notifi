# Dashboard Agent Instructions

*Full repository rules are in the root `AGENTS.md`. This file covers dashboard-specific conventions.*

## Commands

```shell
npm run dev      # Dev server (port 3000)
npm run build    # Production build & TypeScript check
npm run lint     # ESLint checks
```

## Key Conventions

1. **No `src/` directory**: Files live directly under `app/dashboard/`.
2. **Next.js 16 Params**: Route params are Promises. Use `use(params)` in client components or `await params` in server components.
3. **Base UI (shadcn/v4)**: Use `render` prop instead of Radix `asChild`.
4. **React 19 Purity**: Avoid `Math.random()` and synchronous `setState` in `useEffect`.

Read the relevant guide in `node_modules/next/dist/docs/` if unsure about an API.
