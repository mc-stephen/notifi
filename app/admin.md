
# ADMIN DASHBOARD — IMPLEMENTATION PROMPT

## ROLE

You are a Senior Astro Engineer, Product Designer, UX Engineer, Security Engineer, and SaaS Admin Platform Architect.

You are working on an existing Notification Platform as a Service (NPaaS) project.

Your job is to inspect the existing project and implement the **internal Admin Dashboard** in Astro.

This is a production-oriented internal administration system.

Do not treat this as a generic admin template.

The admin dashboard must feel like a carefully designed internal control plane for a serious SaaS infrastructure company.

---

# FIRST STEP — INSPECT THE EXISTING PROJECT

Before writing or modifying code:

1. Scan the entire project structure.
2. Identify the existing Astro application.
3. Identify the existing dashboard implementation.
4. Identify the existing design system.
5. Identify existing layouts.
6. Identify existing UI components.
7. Identify existing typography.
8. Identify existing color tokens.
9. Identify existing spacing and radius conventions.
10. Identify existing icons.
11. Identify existing navigation patterns.
12. Identify existing authentication patterns.
13. Identify existing API clients or data-fetching utilities.
14. Identify existing TypeScript types.
15. Identify existing environment/configuration handling.
16. Identify existing reusable components.
17. Identify existing loading, error, empty, and success states.

Do not replace existing architecture unnecessarily.

Do not introduce a second design system.

Do not create duplicate components when suitable components already exist.

Reuse the existing project's visual language wherever possible.

The existing product's design language is the source of truth.

---

# IMPORTANT — DESIGN CONSISTENCY

The Admin Dashboard must follow the same overall design pattern already established for:

* Landing website
* Documentation
* Customer dashboard
* Authentication

It should feel like the same company made every product.

However, do NOT simply copy the customer dashboard.

The admin dashboard should have its own information hierarchy appropriate for internal operations.

The visual language should remain consistent while the functionality becomes more operational and information-dense.

---

# TECHNOLOGY

Use:

* Astro
* TypeScript
* Existing CSS/design system where available
* Existing component library where available
* Lucide or the project's existing icon system
* Existing API/data-fetching architecture
* Existing authentication utilities where applicable

Do not introduce React, Next.js, or another frontend framework unless the existing project already requires it.

Prefer Astro components and progressive enhancement.

Use client-side JavaScript only where genuinely necessary.

---

# PURPOSE OF THE ADMIN DASHBOARD

The Admin Dashboard is an internal control center for platform operators.

The admin system should allow authorized administrators to:

* Manage users
* Manage organizations
* Manage customer accounts
* Handle support tickets
* Communicate with users
* Send general notifications
* Send personal notifications
* Investigate notification activity
* Handle account restrictions
* Ban/suspend users
* Restore accounts
* Investigate billing issues
* View subscriptions
* Handle failed payments
* Review invoices
* Manage customer plans
* Review platform usage
* Review abuse reports
* Review account activity
* Inspect audit logs
* Monitor platform-wide activity

Design the system so that more administrative functionality can be added later.

---

# SECURITY MODEL

This is an ADMIN-ONLY application.

There must be:

## NO public admin registration.

Do not create:

* Admin Sign Up
* Create Admin Account
* Public Admin Registration
* Admin Invitation accessible without authentication

The initial administrator is provisioned during the first application launch/deployment.

After initial provisioning:

* Existing administrators can manage other administrators.
* Normal customers cannot create administrator accounts.
* Exposing the admin URL must not expose account creation functionality.

---

# ADMIN AUTHENTICATION

The authentication flow should contain only:

```text
Admin Login
    ↓
Admin Authentication
    ↓
Admin Dashboard

Forgot Password
    ↓
Password Reset
    ↓
Admin Login
```

Do not implement customer registration inside the admin application.

---

# ADMIN LOGIN

Create a professional login experience.

Include:

* Email
* Password
* Show/hide password
* Remember session where appropriate
* Login
* Forgot password
* Loading state
* Invalid credentials state
* Account disabled state
* Session expired state
* Rate limited state
* Server unavailable state

The page must clearly communicate that this is an administrative system.

Do not expose unnecessary information in authentication errors.

For example, avoid revealing whether a particular email belongs to an administrator.

---

# FORGOT PASSWORD

Create:

```text
Forgot Password
    ↓
Enter Admin Email
    ↓
Password Reset Email
    ↓
Reset Password
    ↓
Success
    ↓
Login
```

Include:

* Email input
* Validation
* Loading state
* Success state
* Invalid/expired reset token
* Password requirements
* Password confirmation
* Reset success

Do not expose whether an administrator account exists through the forgot-password response.

---

# ADMIN DASHBOARD

Create a high-level operational overview.

The dashboard should answer:

"What is happening across my platform right now?"

Include:

## Platform Metrics

* Total Users
* Active Users
* New Users
* Suspended Users
* Total Organizations
* Active Organizations
* Notifications Sent
* Notifications Delivered
* Failed Notifications
* Revenue
* Monthly Recurring Revenue
* Failed Payments
* Open Support Tickets

## Activity

Show:

* Recent user registrations
* Recent organizations
* Recent support tickets
* Recent billing failures
* Recent account restrictions
* Recent notifications
* Recent suspicious activity

## System Overview

Show high-level status for:

* API
* Notification Engine
* Queue
* Workers
* Email Providers
* SMS Providers
* Push Providers
* Webhooks
* Database
* Redis

The admin dashboard should provide links into detailed operational pages.

---

# NAVIGATION

Create a dedicated admin sidebar.

Suggested structure:

```text
Overview

Customers
├── Users
├── Organizations
├── Projects
└── Accounts

Support
├── Tickets
├── Conversations
└── Knowledge Base

Notifications
├── Send Notification
├── Campaigns
├── Delivery Logs
└── Notification History

Billing
├── Overview
├── Subscriptions
├── Payments
├── Failed Payments
├── Invoices
└── Refunds

Platform
├── Providers
├── Channels
├── Queues
├── Workers
└── System Health

Security
├── Abuse Reports
├── Suspensions
├── Bans
├── Audit Logs
└── Security Events

Administration
├── Admins
├── Roles
├── Permissions
└── Settings
```

Keep the navigation expandable.

The sidebar should clearly distinguish administrative functionality from customer-facing functionality.

---

# USER MANAGEMENT

This is one of the most important sections.

Create a complete user-management experience.

Users page:

```text
Search
Filter
Sort
Pagination

Users Table
```

Display useful information such as:

* User
* Email
* Account status
* Organization
* Plan
* Registration date
* Last activity
* Notification usage
* Billing status

Do not overload the table.

Use a detailed user page for deeper information.

---

# USER DETAIL PAGE

Create a complete user profile/control page.

Include:

## Overview

* Name
* Email
* Avatar
* Account status
* Created date
* Last active
* Organization membership
* Current plan

## Activity

* Login history
* API activity
* Notification activity
* Support activity
* Billing activity

## Organizations

Display organizations associated with the user.

## Projects

Display projects the user can access.

## Notifications

Show notification activity relevant to the account.

## Billing

Show:

* Subscription
* Payment status
* Invoice history
* Usage

## Support

Show support tickets and conversations.

## Administrative Actions

Support:

* Suspend
* Ban
* Unban
* Restore
* Force password reset
* Revoke sessions
* Revoke API credentials where appropriate
* Add internal note

Destructive actions must require confirmation.

---

# ACCOUNT SUSPENSION

Design explicit account states:

```text
Active
Restricted
Suspended
Banned
Deleted
```

A suspension should allow an administrator to specify:

* Reason
* Duration
* Internal notes
* Customer-visible message where appropriate

Do not permanently delete customer data merely because an account is suspended.

---

# BAN USER

Create a clear administrative workflow.

Example:

```text
Ban User
    ↓
Confirmation Dialog
    ↓
Reason
    ↓
Optional Internal Note
    ↓
Confirm
    ↓
Account Banned
```

Display the consequences before confirmation.

Support unbanning/restoring accounts.

Every action must be audited.

---

# SUPPORT TICKETS

Create a complete support ticket system.

Tickets should support:

* Open
* Pending
* Waiting for Customer
* Resolved
* Closed

Each ticket should contain:

* Ticket ID
* Customer
* Organization
* Subject
* Priority
* Status
* Assigned Admin
* Created date
* Last updated

---

# SUPPORT CONVERSATION

Create a professional support conversation interface.

Include:

* Message timeline
* Customer messages
* Admin responses
* Internal notes
* Attachments where supported
* Assignment
* Priority
* Status
* Tags

Clearly distinguish:

```text
Customer-visible reply

vs

Internal admin note
```

Never allow an internal note to accidentally become customer-visible.

---

# GENERAL NOTIFICATIONS

Administrators should be able to send platform-wide communications.

Examples:

* Maintenance announcements
* Service announcements
* Product announcements
* Marketing messages
* Important security notifications

Allow targeting:

* All users
* Specific organizations
* Specific plans
* User segments
* Custom filters

---

# PERSONAL NOTIFICATIONS

Allow administrators to send a notification to an individual customer.

The UI should allow:

```text
Select User

↓

Select Channel

↓

Compose Message

↓

Preview

↓

Confirm

↓

Send
```

Support channels appropriate to the platform.

Do not expose provider credentials.

---

# MARKETING / BROADCAST NOTIFICATIONS

Design a campaign-style interface for general notifications.

Include:

* Campaign name
* Audience
* Channels
* Message
* Template
* Scheduling
* Preview
* Estimated recipients
* Delivery tracking

Clearly distinguish marketing communications from operational/system notifications.

---

# BILLING MANAGEMENT

Create a complete billing administration section.

Admins should be able to investigate:

* Failed payments
* Past-due accounts
* Cancelled subscriptions
* Invoices
* Refunds
* Credits
* Usage
* Plan changes

The interface should allow an admin to inspect a customer's billing history.

---

# BILLING FAILURE WORKFLOW

Design:

```text
Failed Payment
    ↓
Customer Account
    ↓
Billing Details
    ↓
Payment History
    ↓
Failure Reason
    ↓
Administrative Action
```

Possible actions:

* Retry payment
* Contact customer
* Apply account credit
* Adjust subscription where authorized
* Mark issue for follow-up

Do not expose sensitive payment information such as full card numbers.

---

# SUBSCRIPTIONS

Display:

* Plan
* Status
* Start date
* Renewal date
* Usage
* Limits
* Payment state

Allow authorized administrators to perform controlled administrative actions.

Every billing modification must be audited.

---

# PLATFORM-WIDE NOTIFICATION HISTORY

Admins need a global view of notifications.

Include:

* Notification ID
* User
* Organization
* Channel
* Status
* Provider
* Created time
* Delivery time
* Failure reason

Support:

Search

Filtering

Sorting

Date ranges

Export where appropriate

---

# DELIVERY LOGS

Create detailed diagnostic views.

Allow admins to inspect:

* Notification lifecycle
* Provider
* Provider response
* Retry attempts
* Delivery timestamps
* Failure reason
* Correlation ID
* Request ID
* Relevant metadata

This should be optimized for debugging customer issues.

---

# PROVIDERS

Allow administrators to inspect platform-level notification providers.

Display:

* Provider
* Channel
* Status
* Health
* Success rate
* Failure rate
* Latency
* Region
* Last health check

Provider credentials must never be displayed in plaintext.

---

# AUDIT LOG

Every sensitive administrative operation must be auditable.

Examples:

```text
Admin logged in

Admin suspended user

Admin banned user

Admin unbanned user

Admin changed subscription

Admin issued refund

Admin sent notification

Admin modified provider

Admin changed permissions
```

Display:

* Actor
* Action
* Target
* Timestamp
* IP information where appropriate
* Request/correlation ID
* Result

Audit records should be immutable from the normal admin UI.

---

# ADMIN MANAGEMENT

Create an administration section.

Support:

* Admin list
* Admin creation by authorized admin
* Admin status
* Roles
* Permissions
* Last login
* Sessions
* Account disable

Suggested roles:

```text
Super Admin
Admin
Support Admin
Billing Admin
Operations Admin
Read Only
```

Design the system so custom roles can be added later.

---

# PERMISSIONS

Use permission-based authorization rather than relying only on role names.

Examples:

```text
users.read
users.update
users.suspend
users.ban

tickets.read
tickets.reply
tickets.assign

notifications.read
notifications.send
notifications.broadcast

billing.read
billing.update
billing.refund

providers.read
providers.manage

admins.read
admins.manage

audit.read
```

The frontend must not be treated as the security boundary.

Permissions must ultimately be enforced by the backend.

---

# SEARCH

The admin dashboard should have powerful global search.

Allow administrators to search for:

* Users
* Organizations
* Projects
* Tickets
* Notification IDs
* Invoice IDs
* Subscription IDs
* API request IDs
* Correlation IDs

Provide keyboard-friendly search.

---

# DATA TABLES

All major administrative tables should support:

* Search
* Filtering
* Sorting
* Pagination
* Column selection
* Bulk actions where appropriate
* Empty state
* Loading state
* Error state

Tables should remain usable with large datasets.

Do not attempt to load thousands of rows into the browser at once.

Assume the backend provides server-side pagination and filtering.

---

# UI / VISUAL DESIGN

This is extremely important.

Do NOT create a generic Bootstrap-style admin dashboard.

Do NOT make every section a collection of cards.

Do NOT fill the screen with unnecessary charts.

The interface should feel like a professional internal product from:

* Stripe
* Vercel
* Linear
* GitHub
* Cloudflare
* AWS
* Supabase

Follow the existing project's design language.

The admin interface should be:

* Clean
* Dense where useful
* Spacious where needed
* Highly readable
* Professional
* Calm
* Fast
* Information-rich
* Consistent

Use visual hierarchy to distinguish:

Primary information

Secondary information

Warnings

Critical actions

Operational states

---

# ADMIN-SPECIFIC VISUAL LANGUAGE

The admin dashboard can be slightly more information-dense than the customer dashboard.

Use:

* Compact tables
* Status badges
* Timeline components
* Metric cards
* Split views
* Detail drawers
* Command/search interface
* Context menus
* Confirmation dialogs
* Activity feeds
* Inline actions

Do not turn every interaction into a modal.

Prefer dedicated detail pages when the information is complex.

---

# RESPONSIVE DESIGN

Support:

Desktop

Tablet

Mobile

However, prioritize desktop because this is an operational tool.

On smaller screens:

* Sidebar becomes collapsible.
* Tables become horizontally scrollable or transform into useful card/list views.
* Detail panels stack.
* Actions remain accessible.

---

# LOADING STATES

Every data-driven page must have:

* Skeleton loading
* Initial loading
* Pagination loading
* Action loading
* Refresh state

Never leave users staring at a blank page.

---

# EMPTY STATES

Design useful empty states.

For example:

No users

No tickets

No billing issues

No incidents

No notifications

No audit events

Every empty state should explain what it means and what the administrator can do next.

---

# ERROR STATES

Create professional error handling.

Include:

* Network failure
* API failure
* Permission denied
* Session expired
* Resource not found
* Validation failure
* Server error

Provide useful recovery actions.

---

# CONFIRMATION UX

Require confirmation for destructive or high-impact actions:

* Ban user
* Delete data
* Refund payment
* Disable admin
* Revoke credentials
* Change sensitive configuration
* Send large notification campaigns

Confirmation dialogs should explain the action and its consequences.

---

# NOTIFICATION COMPOSER

Create a polished notification composer for administrators.

The interface should support:

```text
Audience
    ↓
Channel
    ↓
Template / Message
    ↓
Variables
    ↓
Preview
    ↓
Schedule
    ↓
Review
    ↓
Send
```

For large broadcasts, show:

* Estimated audience
* Estimated cost where available
* Selected channels
* Scheduled time
* Confirmation

Prevent accidental broadcasts.

---

# ARCHITECTURE

Use a feature-oriented Astro architecture.

Do not create an architecture where all pages live in one giant components folder.

Prefer organization around administrative domains.

Example direction:

```text
src/
├── pages/
│   └── admin/
│
├── features/
│   └── admin/
│       ├── overview/
│       ├── users/
│       ├── organizations/
│       ├── support/
│       ├── notifications/
│       ├── billing/
│       ├── providers/
│       ├── security/
│       ├── admins/
│       └── settings/
│
├── components/
│   ├── admin/
│   └── shared/
│
├── layouts/
├── lib/
├── services/
├── types/
├── utils/
└── styles/
```

However, inspect the existing project first.

If it already has an established feature-based architecture, follow it rather than imposing this exact structure.

---

# FEATURE ORGANIZATION

Each major admin feature should own its related:

* Components
* Types
* API functions
* Validation
* Utilities
* UI states
* Documentation

For example:

```text
users/
├── components/
├── api.ts
├── types.ts
├── schemas.ts
├── utils.ts
└── README.md
```

Do not create unnecessary files.

Do not force a file structure simply because it appears in this prompt.

---

# API CONTRACT

Before implementing mock functionality, inspect the existing API contracts.

If the backend is not yet implemented:

Create typed service interfaces and clearly defined mock data adapters.

Do NOT scatter hardcoded fake data throughout UI components.

The eventual Rust backend should be able to replace the mock implementation without requiring major UI rewrites.

---

# DATA REQUIREMENTS

For every admin page, identify:

* Data required
* API endpoint required
* Query parameters
* Filters
* Pagination
* Sorting
* Mutations
* Permissions
* Loading states
* Error states
* Empty states

Keep this information documented close to the feature.

---

# SECURITY REQUIREMENTS

Never trust client-side authorization.

Never store sensitive credentials unnecessarily.

Never display:

* Passwords
* Password hashes
* Full payment card numbers
* Provider secrets
* API secrets

Use secure session handling.

Protect all admin routes.

Handle session expiration.

Prevent unauthorized access.

Use CSRF protection where applicable.

Use secure cookies where applicable.

Do not expose administrative APIs through publicly accessible client-side logic without proper authorization.

---

# PERFORMANCE

Assume:

* Millions of users
* Millions of notifications
* Large audit logs
* Large ticket history
* Large billing datasets

Do not fetch massive datasets into the browser.

Use:

* Server-side pagination
* Filtering
* Sorting
* Incremental loading
* Caching where appropriate

---

# SEO

The authenticated admin dashboard is NOT a public SEO surface.

Do not optimize private dashboard pages for search engines.

Explicitly prevent indexing of authenticated admin pages.

The public landing page and documentation are responsible for SEO.

---

# ACCESSIBILITY

Follow WCAG principles.

Support:

* Keyboard navigation
* Screen readers
* Proper focus management
* ARIA labels
* Accessible dialogs
* Accessible tables
* Visible focus states
* Reduced motion

---

# AUDITABILITY

Any action that changes customer state should produce an audit event.

The UI should make it clear when an action is:

* Informational
* Reversible
* Destructive
* Customer-visible
* Platform-wide

---

# IMPLEMENTATION PROCESS

Do not immediately generate the entire application.

First:

### PHASE 1 — INSPECTION

Inspect the repository and explain:

* Existing architecture
* Existing design system
* Existing dashboard
* Existing authentication
* Existing reusable components
* Existing API structure
* Existing Astro configuration

Then identify what can be reused.

### PHASE 2 — ADMIN INFORMATION ARCHITECTURE

Produce:

* Route tree
* Sidebar tree
* Page hierarchy
* Feature hierarchy
* User journeys

### PHASE 3 — DESIGN SYSTEM

Map the existing design system to the admin dashboard.

Identify any missing components.

### PHASE 4 — IMPLEMENTATION

Build the admin dashboard incrementally.

Start with:

```text
Admin Authentication
        ↓
Admin Layout
        ↓
Admin Navigation
        ↓
Overview
        ↓
Users
        ↓
User Details
        ↓
Support
        ↓
Notifications
        ↓
Billing
        ↓
Security
        ↓
Administration
```

Do not attempt to implement everything as one giant change.

---

# CODE QUALITY

Write production-quality TypeScript.

Avoid:

* Any-type abuse
* Giant components
* Giant page files
* Duplicated API logic
* Duplicated UI
* Hardcoded business rules
* Inline authentication logic
* Scattered fetch calls
* Unnecessary client-side state

Use strong typing.

Use reusable components.

Keep features isolated.

---

# DOCUMENTATION

Create a README for the admin dashboard explaining:

* Purpose
* Architecture
* Route structure
* Authentication flow
* Permission model
* Feature structure
* API integration
* Environment variables
* Development commands
* Production build
* Security considerations

Each major feature should also have a concise README explaining its responsibility.

---

# FINAL DELIVERABLE

After implementation, provide:

1. Final admin route tree.
2. Final folder structure.
3. Explanation of each major directory.
4. Explanation of authentication flow.
5. Explanation of authorization flow.
6. Admin role/permission model.
7. List of reusable components created.
8. API contracts required by the frontend.
9. Mock APIs/data adapters created, if backend endpoints do not yet exist.
10. Security considerations.
11. Performance considerations.
12. Testing strategy.
13. Remaining TODOs.
14. Any assumptions made because backend functionality is not yet available.

Do not hide unfinished functionality.

Clearly mark anything that requires backend implementation.

---

# MOST IMPORTANT RULE

Build this as an **internal operations platform**, not as a pretty CRUD dashboard.

The administrator should be able to answer questions such as:

"Why can't this customer send notifications?"

"Why did this payment fail?"

"Why are this user's notifications failing?"

"Is this account abusing the platform?"

"What notifications did this customer send?"

"What happened to this notification?"

"Which provider is failing?"

"How many users are affected?"

"Who changed this customer's account?"

"Can I communicate with this customer?"

"Can I safely suspend this account?"

"Can I investigate this issue without touching production data manually?"

The UI should make these workflows obvious.

Prioritize **operational clarity, safety, auditability, and speed** over visual decoration.

The final product should feel like a serious internal control plane for a production notification infrastructure company.
