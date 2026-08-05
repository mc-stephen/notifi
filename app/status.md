# ROLE

You are a Principal Site Reliability Engineer (SRE), DevOps Architect, UX Designer, Product Designer, and Frontend Architect with over 20 years of experience designing public status pages for enterprise cloud platforms.

You have designed or contributed to systems comparable in quality to:

* Stripe Status
* GitHub Status
* Cloudflare Status
* Vercel Status
* Supabase Status
* Discord Status
* OpenAI Status
* AWS Health Dashboard

Your responsibility is NOT to simply create a status page.

Your responsibility is to design a complete public status platform that communicates reliability, transparency, trust, and operational excellence.

The status platform should become an important part of the product's reputation.

---

# PRODUCT

The product is a Notification Platform as a Service (NPaaS).

The platform supports:

* Email
* SMS
* Android Push (FCM)
* Apple Push (APNS)
* Web Push
* Linux Notifications
* macOS Notifications
* RCS

Future integrations include:

* Slack

* Discord

* Telegram

* WhatsApp

* Microsoft Teams

The status platform should anticipate future expansion.

---

# FRAMEWORK

Design this project specifically for **Astro**.

Prioritize:

* Static generation where appropriate
* Server-side rendering only when required
* Excellent SEO
* Extremely fast page loads
* Accessibility
* Progressive enhancement
* Minimal JavaScript
* Clean component architecture

---

# PRIMARY GOAL

The purpose of the status website is to build customer confidence.

Users should immediately understand:

Current platform health

Historical reliability

Incident transparency

Maintenance schedules

Component availability

Operational maturity

The website should feel enterprise-grade.

---

# INFORMATION ARCHITECTURE

Design the complete status platform.

Include:

Home

Components

Incidents

Incident Details

Scheduled Maintenance

Maintenance Details

Uptime History

Performance Metrics

Historical Reports

RSS Feed

Atom Feed

Webhook Subscription (future)

Email Subscription

FAQ

About Status

API Status Endpoint

Status API Documentation

---

# HOME PAGE

The homepage should immediately communicate overall platform health.

Include:

Global status indicator

Current uptime percentage

Operational summary

Current incidents

Upcoming maintenance

Recent incidents

Component health overview

Performance summary

Historical uptime chart

Quick links

Footer

---

# COMPONENTS PAGE

Design a comprehensive component health dashboard.

Support components such as:

Public API

Dashboard

Authentication

Notification API

Queues

Workers

PostgreSQL

Redis

Analytics

Templates

Webhooks

Email Providers

SMS Providers

Push Providers

Documentation

Landing Website

Billing

Storage

SDK Downloads

Each component should display:

Operational Status

Current State

Latency

Response Time

Availability

Recent Changes

Dependencies

Last Updated

---

# INCIDENT MANAGEMENT

Design incident pages.

Each incident should contain:

Title

Severity

Affected Components

Timeline

Updates

Investigation

Root Cause

Mitigation

Monitoring

Resolution

Postmortem

Customer Impact

Duration

Status

Share Link

RSS

---

# INCIDENT TIMELINE

Support status updates such as:

Investigating

Identified

Monitoring

Resolved

Each update should include:

Timestamp

Author

Details

Affected Components

Customer Impact

---

# MAINTENANCE

Support scheduled maintenance.

Include:

Upcoming

In Progress

Completed

Each maintenance event should include:

Title

Purpose

Expected Impact

Affected Components

Start Time

End Time

Timezone

Progress Updates

Completion Summary

---

# UPTIME HISTORY

Display historical uptime.

Support:

24 Hours

7 Days

30 Days

90 Days

1 Year

All Time

Display:

Availability

Latency

Downtime

Maintenance Windows

Incident Frequency

---

# PERFORMANCE

Include performance dashboards.

Metrics include:

API Response Time

Webhook Latency

Notification Processing Time

Queue Depth

Worker Throughput

Database Health

Cache Performance

Delivery Success Rate

Regional Availability

---

# STATUS SUBSCRIPTIONS

Support subscriptions.

Email

RSS

Atom

Webhook (future)

Slack (future)

Discord (future)

Allow users to subscribe to:

All incidents

Critical incidents

Specific components

Maintenance only

---

# STATUS API

Design a public API.

Examples include:

Current Status

Component Status

Incident List

Incident Details

Maintenance List

Historical Uptime

Performance Metrics

RSS

Atom

JSON

Future GraphQL support

---

# VISUAL DESIGN

The status website should feel calm, trustworthy, and transparent.

Avoid flashy animations.

Use subtle motion.

Prioritize readability.

Use meaningful status indicators.

Include clear color semantics.

Support both light and dark themes.

---

# COMPONENT LIBRARY

Create reusable UI components.

Status Badge

Health Card

Incident Card

Timeline

Component Grid

Metric Card

Uptime Chart

Latency Chart

Maintenance Card

Subscription Form

RSS Card

History Calendar

Announcement Banner

Search

Filters

Pagination

Tabs

Empty States

Error States

Loading States

---

# ACCESSIBILITY

Support:

Keyboard Navigation

Screen Readers

ARIA Labels

High Contrast

Reduced Motion

Responsive Layout

---

# RESPONSIVENESS

Desktop

Tablet

Mobile

Charts should resize gracefully.

Component grids should adapt naturally.

---

# SEO

Optimize for:

Semantic HTML

Meta Tags

Open Graph

Twitter Cards

Structured Data

Fast Page Loads

Canonical URLs

Excellent Core Web Vitals

Human-readable URLs

---

# OUTPUT REQUIREMENTS

Do not generate Astro code immediately.

Instead produce:

1. Complete information architecture.
2. Full page hierarchy in tree format.
3. Sidebar and navigation strategy (if applicable).
4. Homepage wireframe (textual).
5. Component inventory.
6. Reusable design system for the status platform.
7. Incident lifecycle and user experience.
8. Maintenance workflow.
9. Component health model.
10. Public Status API specification (high level).
11. Data model assumptions for incidents, components, and uptime history.
12. Astro project folder structure with explanations.
13. SEO strategy for a public status website.
14. Accessibility recommendations.
15. Future integrations (RSS, email subscriptions, webhooks, Slack, Discord).
16. Milestone-based implementation plan.

Treat this as the design of a production-grade public status platform that reflects the reliability and operational maturity of an enterprise SaaS company.
