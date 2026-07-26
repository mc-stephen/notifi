11. Implementation Milestones
Phase 0: Foundation (~1 day)
- Install all dependencies (shadcn init, zustand, recharts, react-hook-form, zod, framer-motion, tanstack-table, lucide-react, next-themes, etc.)
- Set up shadcn/v4 with Base UI primitives
- Configure full theme in globals.css (all CSS variables)
- Create folder structure: src/components/ui/, src/components/custom/, src/store/, src/hooks/, src/lib/, src/app/(dashboard)/
- Define TypeScript types in src/lib/types.ts
- Define constants (nav items, channel labels, status labels) in src/lib/constants.ts
- Set up Zustand stores (org, project, environment, UI)
Phase 1: Design System (~1 day)
- All shadcn/ui components installed and customized
- Custom components: MetricCard, StatusBadge, ChannelIcon, HealthIndicator, EnvBadge
- Custom components: EmptyState, ErrorState, LoadingState
- Custom components: JsonViewer, CodeBlock
Phase 2: Layout Shell (~1 day)
- (dashboard)/layout.tsx with sidebar + topbar + content area
- Sidebar with all nav items, icons, collapsible sections
- Sidebar collapse/expand toggle
- Topbar with org/project/environment switchers, user avatar
- Mobile responsive (sheet overlay sidebar)
- Breadcrumbs
- next-themes dark/light toggle
Phase 3: Dashboard Page (~1-2 days)
- Overview metric cards (8 KPIs)
- Charts: notification timeline, delivery rate, channel distribution, country distribution, platform distribution
- Recent activity feeds (failures, webhooks, API requests, deployments)
- Quick actions bar
- Recent notifications table (DataTable)
- Health indicators section
Phase 4: Notifications (~1-2 days)
- DataTable with TanStack Table (search, filters, sorting, pagination, bulk actions, column visibility, export)
- Filter bar (status, channel, date range, priority, recipient)
- Notification detail page (/notifications/[id])
- Detail tabs: overview, payload, metadata, delivery events, logs, provider response
- Timeline component for notification lifecycle
- Actions: retry, cancel, duplicate
Phase 5: Recipients (~1-2 days)
- DataTable with virtual scrolling support (@tanstack/react-virtual for millions)
- Recipient profile page (/recipients/[id])
- Profile sections: contact info, devices, subscriptions, preferences, segments, tags, custom attributes
- Device management table
- Notification history for recipient
Phase 6: Templates (~1-2 days)
- Template list with folder/category organization
- Template editor (/templates/[id])
- Split panel: editor (left) + preview (right)
- Variable management
- Version history
- Localization support
- Draft/Publish workflow
- Testing interface
Phase 7: Channels & Providers (~1 day)
- Channels page: channel cards showing enable/disable status
- Channel detail: configuration form per channel type
- Providers page: provider list grouped by channel
- Provider detail: credentials form, health stats, latency, success rate, fallback config
- Priority/quota management
Phase 8: Analytics (~1-2 days)
- Analytics overview dashboard with multiple chart widgets
- Delivery trends (area chart, date range selector)
- Channel analytics (bar chart comparison)
- Country analytics (table + map placeholder)
- Device analytics
- Audience growth (line chart)
- API usage stats
- Download/export reports
Phase 9: Events & Logs (~1 day)
- Event stream with real-time feel (polling)
- Event detail page with full metadata, JSON viewer
- Logs page: filterable log viewer
- Log detail: syntax-highlighted, searchable
Phase 10: Webhooks & API Keys (~1 day)
- Webhook CRUD with event selection
- Webhook history with payload preview
- Replay functionality
- API key list with environment tabs
- Key creation dialog with permissions/scopes
- Usage tracking, regeneration
Phase 11: Team, Settings, Billing (~1-2 days)
- Team member list with role badges
- Invite dialog with role selection
- Settings pages: org, project, branding, domains, security, preferences
- Billing: plan display, usage meters, invoices list, upgrade CTA
Phase 12: Polish (~1 day)
- Framer Motion subtle animations (page transitions, card hover, sidebar)
- Keyboard shortcuts (Cmd+K search, navigation)
- Loading skeletons for all pages
- Empty states with illustrations + CTAs
- Error boundaries
- Accessibility audit (ARIA labels, focus management, screen reader)
- Final responsive audit
