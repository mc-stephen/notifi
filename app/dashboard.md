# ROLE

You are a Senior Product Designer, UX Designer, SaaS Architect, and Senior Frontend Engineer with over 20 years of experience designing enterprise SaaS products.

Your responsibility is NOT to simply create attractive pages.

Your responsibility is to design an industry-leading developer platform dashboard comparable to products like Stripe, Clerk, Vercel, Supabase, Neon, GitHub, Resend, Cloudflare, Upstash, PostHog, and Linear.

The final design must feel like software built by one of these companies.

The dashboard should prioritize:

* Developer Experience (DX)
* Simplicity
* Scalability
* Enterprise usability
* Beautiful information hierarchy
* Fast navigation
* Excellent spacing
* Modern SaaS aesthetics
* Accessibility
* Mobile responsiveness
* Dark mode first
* Light mode support

Avoid unnecessary decorations.

Avoid excessive gradients.

Avoid glassmorphism.

Avoid flashy effects.

The dashboard should feel professional enough for large enterprises while remaining approachable for indie developers.

---

# PRODUCT

The product is a Notification Platform as a Service (NPaaS).

Developers integrate one API into their applications to send notifications across multiple delivery channels.

Supported channels include:

* Email
* SMS
* Android Push (FCM)
* Apple Push (APNS)
* Web Push
* Linux Notifications
* macOS Notifications
* RCS
* WhatsApp (future)
* Slack (future)
* Discord (future)
* Microsoft Teams (future)
* Telegram (future)
* Custom Webhooks
* Additional providers may be added in the future.

The dashboard must be designed with future expansion in mind.

---

# IMPORTANT

Never design pages around providers.

Design pages around notifications.

Providers are merely implementations.

The product revolves around:

Organizations

Projects

Environments

Recipients

Templates

Notifications

Analytics

Events

API Keys

Webhooks

Settings

Providers

Billing

Team Members

---

# DESIGN PHILOSOPHY

Every page should answer these questions immediately:

Where am I?

What can I do?

What is the current state?

What actions are available?

How do I navigate elsewhere?

The UI should minimize cognitive load.

Every table should support:

Search

Filters

Sorting

Pagination

Bulk Actions

Column visibility

Export

Responsive layout

---

# SIDEBAR STRUCTURE

Design the navigation around the following hierarchy.

Dashboard

Notifications

Recipients

Templates

Campaigns

Schedules

Channels

Providers

Analytics

Events

Webhooks

API Keys

SDKs

Developers

Logs

Team

Projects

Billing

Integrations

Settings

Support

Each menu item should include a meaningful icon.

Support nested navigation.

Support collapsible sections.

---

# DASHBOARD PAGE

The dashboard should immediately communicate system health.

Include:

Overview cards

Notifications sent today

Successful deliveries

Failures

Queued notifications

Delivery rate

Open rate

Click rate

Average latency

Charts

Notification timeline

Delivery rate graph

Channel distribution

Country distribution

Platform distribution

Recent activity

Recent failures

Recent webhook deliveries

Recent API requests

Recent deployments

Quick actions

Create Notification

Create Template

Invite Team Member

Generate API Key

Add Provider

Recent notifications table

Health indicators

System status

Queue health

Worker health

Provider status

API latency

---

# NOTIFICATIONS

Design a comprehensive notification management interface.

Include:

Search

Advanced filters

Status filter

Channel filter

Date filter

Priority filter

Recipient filter

Notification list

Timeline

Retry

Cancel

Duplicate

View payload

View metadata

View delivery events

View logs

View webhook history

View provider response

---

# RECIPIENTS

Support millions of recipients.

Display:

Recipient profile

Email

Phone

Devices

Subscriptions

Preferences

Language

Timezone

Segments

Notification history

Delivery history

Events

Tags

Custom attributes

---

# DEVICE MANAGEMENT

Recipients may own multiple devices.

Support:

Android

iPhone

iPad

macOS

Linux

Windows

Browser

Each device should display:

Token

Provider

App version

Last active

Platform version

Status

Expiration

---

# TEMPLATES

Design a powerful template management experience.

Include:

Folder organization

Search

Categories

Preview

Variables

Version history

Localization

Testing

Drafts

Publishing

Rich editor

Code editor

Preview pane

---

# ANALYTICS

Enterprise-grade analytics.

Support:

Delivery trends

Open rate

Click rate

Bounce rate

Failure reasons

Provider comparison

Country analytics

Device analytics

Channel analytics

Recipient growth

API usage

Webhook performance

Latency

Heatmaps

Funnels

Time series

Download reports

---

# EVENTS

Every notification has a timeline.

Example:

Queued

Worker Assigned

Provider Selected

Sent

Delivered

Opened

Clicked

Failed

Retried

Cancelled

Each event should contain timestamps, metadata, provider information, request IDs, correlation IDs, and diagnostic details.

---

# PROVIDERS

Provider management.

Email providers

SMS providers

Push providers

Webhook providers

Each provider should display:

Health

Latency

Success rate

Credentials

Limits

Quota

Region

Fallback provider

Priority

---

# WEBHOOKS

Webhook management.

Create endpoints

Signing secrets

Retries

History

Payload preview

Replay

Failures

Testing

Filtering

Events selection

---

# API KEYS

Support multiple keys.

Development

Production

Testing

Staging

Each key should display:

Permissions

Scopes

Expiration

Usage

Last used

Rate limits

Regenerate

Disable

Audit logs

---

# TEAM MANAGEMENT

Support organizations.

Owner

Admin

Developer

Viewer

Billing

Invite members

Permissions

Activity logs

Audit history

SSO readiness

---

# BILLING

Subscription overview

Invoices

Usage

Notification volume

Channel usage

Current plan

Upcoming invoice

Payment methods

Limits

Overages

Upgrade

---

# SETTINGS

Organization

Project

Branding

Domains

Security

MFA

OAuth

Preferences

Default timezone

Localization

Notification defaults

Danger Zone

---

# DESIGN SYSTEM

Create a reusable design system.

Buttons

Inputs

Selects

Dropdowns

Dialogs

Drawers

Cards

Tabs

Badges

Breadcrumbs

Pagination

Data Tables

Charts

Date Pickers

Toasts

Loading States

Skeletons

Empty States

Error States

Permission States

Context Menus

Tooltips

Modals

Accordions

Tree Views

Split Panels

Resizable Layouts

Code Blocks

JSON Viewers

Log Viewers

Timeline Components

Metric Cards

Status Indicators

---

# UX

Every action should have:

Loading state

Success state

Error state

Confirmation dialog (when destructive)

Undo (when possible)

Keyboard shortcuts

Accessibility

ARIA labels

Focus states

Screen reader support

---

# RESPONSIVENESS

Desktop-first.

Fully responsive.

Tablet optimized.

Mobile supported.

Sidebar becomes collapsible.

Tables adapt intelligently.

Charts resize gracefully.

---

# VISUAL STYLE

Modern enterprise SaaS.

Inspired by:

Stripe

Vercel

Linear

Clerk

Cloudflare

Supabase

GitHub

Resend

Neon

PostHog

Do NOT copy their designs.

Instead, capture the same level of polish, spacing, consistency, typography, and usability.

---

# FRONTEND STACK

Generate the UI using:

* Next.js (App Router)
* TypeScript
* Tailwind CSS
* shadcn/ui
* Lucide Icons
* TanStack Table
* Recharts
* Framer Motion (subtle animations only)
* React Hook Form
* Zod
* next-themes

Use reusable components.

Use Server Components where appropriate.

Organize the project using scalable feature-based architecture.

---

# IMPORTANT OUTPUT REQUIREMENTS

Do not generate code immediately.

First, produce:

1. Complete information architecture.
2. Complete page hierarchy.
3. Navigation flow.
4. User journeys.
5. Dashboard wireframes (textual).
6. Layout specifications.
7. Component inventory.
8. Reusable design system.
9. Data model assumptions for every page.
10. API requirements inferred from the UI.
11. Required backend endpoints grouped by feature.
12. Suggested database entities needed to power the dashboard.
13. State management strategy.
14. Finally, generate a complete implementation plan broken into milestones.

Treat this as the design of a production-ready SaaS platform intended to support millions of users and enterprise customers.

---
