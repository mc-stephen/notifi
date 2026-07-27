---

# ROLE

You are a Principal UX Designer, Identity Architect, Security Engineer, and Senior SaaS Product Designer with more than 20 years of experience designing authentication systems for enterprise cloud platforms.

Your task is to design a complete authentication and onboarding experience for a Notification Platform as a Service (NPaaS).

The authentication experience should match the quality of:

* Stripe
* GitHub
* Vercel
* Clerk
* Supabase
* Cloudflare
* Linear
* Google Cloud
* AWS Console

The goal is not simply allowing users to sign in.

The goal is to build trust, security, scalability, and an exceptional first impression.

The UI must be clean, minimal, accessible, modern, and enterprise-ready.

Avoid unnecessary complexity.

Every screen must have a clear purpose.

---

# PRODUCT CONTEXT

This platform allows organizations and developers to manage notification infrastructure.

Users can create organizations, projects, API keys, providers, recipients, templates, and notifications.

Authentication must support individual developers, startups, teams, and enterprise organizations.

The design must scale from one-person accounts to organizations with hundreds of users.

---

# AUTHENTICATION PHILOSOPHY

Authentication is not a single page.

It is a complete user lifecycle.

Design the entire lifecycle.

---

# USER FLOW

Design every screen needed for the complete authentication experience.

Include:

Landing Page

↓

Sign Up

↓

Email Verification

↓

Welcome

↓

Create Organization

↓

Create First Project

↓

Choose Use Case

↓

Generate API Key

↓

Invite Team (Optional)

↓

Connect First Provider (Optional)

↓

Dashboard

---

# SIGN UP

Support:

Email & Password

Google

GitHub

Microsoft

Apple

Enterprise SSO (future)

Requirements:

Name

Email

Password

Terms Agreement

Newsletter Opt-In (optional)

Password Strength Meter

Real-time Validation

Bot Protection Placeholder

Progressive Form Validation

Loading States

Error States

Success States

Accessibility

---

# LOGIN

Support:

Email & Password

Google

GitHub

Microsoft

Apple

Remember Me

Forgot Password

Magic Link (future)

Passkeys (future)

SSO (future)

Display:

Recent login info

Session warning

Loading states

Error handling

Rate limiting feedback

---

# EMAIL VERIFICATION

Design:

Verification Success

Verification Failed

Expired Link

Resend Verification

Change Email

Loading State

Automatic Redirect

---

# PASSWORD RESET

Include:

Forgot Password

Email Sent

Reset Password

Expired Token

Invalid Token

Success Confirmation

Password Requirements

Password Confirmation

---

# FIRST-TIME ONBOARDING

The onboarding should not feel like setup.

It should feel exciting.

Step 1

Welcome

Step 2

What are you building?

Options:

Web App

Mobile App

SaaS

Enterprise

E-commerce

Gaming

Other

Step 3

Organization Name

Logo Upload

Region

Timezone

Step 4

Create First Project

Project Name

Environment

Description

Step 5

Generate First API Key

Development

Production

Step 6

Choose Notification Channels

Email

SMS

Push

Web Push

APNS

FCM

Webhook

Step 7

Invite Team

(Optional)

Step 8

Success Screen

---

# ORGANIZATION MANAGEMENT

Users may belong to multiple organizations.

Support:

Organization Switcher

Organization Creation

Organization Invitations

Organization Members

Organization Roles

Organization Settings

Organization Billing

Organization Branding

Organization Security

---

# PROJECT MANAGEMENT

Organizations contain projects.

Support:

Create Project

Delete Project

Archive Project

Rename Project

Environment Selection

Production

Development

Testing

Staging

---

# TEAM INVITATIONS

Invite via email.

Support:

Pending Invites

Accepted

Declined

Expired

Resend

Cancel

Bulk Invite

Role Assignment

---

# USER PROFILE

Profile Picture

Display Name

Username

Email

Verified Status

Timezone

Language

Notification Preferences

Appearance

Connected Accounts

Sessions

Security

Danger Zone

---

# SECURITY CENTER

Create a dedicated security page.

Include:

Password

Change Password

Email

Phone

Recovery Codes

Authenticator App

Passkeys

2FA

Backup Methods

Active Sessions

Device History

Recent Login History

Suspicious Activity

API Tokens

Connected Applications

Account Recovery

Delete Account

---

# ACTIVE SESSIONS

Display:

Current Device

Browser

Operating System

Location (approximate)

IP Address

Login Time

Last Activity

Terminate Session

Terminate All Sessions

---

# MULTI-FACTOR AUTHENTICATION

Support:

Authenticator Apps

SMS (optional)

Email Codes

Backup Codes

Recovery Flow

Disable MFA

Verification Flow

Setup Wizard

---

# API TOKEN AUTHENTICATION

Users should be able to manage personal access tokens.

Support:

Create Token

Expiration

Scopes

Regenerate

Delete

Usage

Last Used

IP History

---

# OAUTH CONNECTIONS

Allow linking:

Google

GitHub

Microsoft

Apple

Disconnect

Reconnect

Primary Provider

---

# PERMISSIONS

Design role management.

Owner

Admin

Developer

Viewer

Billing

Support future custom roles.

---

# INVITATION FLOW

Invitation Email

↓

Accept Invitation

↓

Create Account (if needed)

↓

Join Organization

↓

Dashboard

---

# EMPTY STATES

Design beautiful empty states for:

No Organizations

No Projects

No Teams

No API Keys

No Providers

No Sessions

No Security Devices

---

# ERROR STATES

Design screens for:

401 Unauthorized

403 Forbidden

404

Account Suspended

Email Unverified

Organization Disabled

Project Archived

Session Expired

Maintenance Mode

Rate Limited

---

# LOADING STATES

Skeletons

Progress Indicators

Inline Loading

Full Screen Loading

Async Verification

Optimistic UI

---

# DESIGN SYSTEM

Authentication pages should reuse a common design language.

Include:

Buttons

Cards

Forms

Validation

Stepper

Progress

Illustrations

Icons

Alerts

Banners

Dialogs

Inputs

Password Fields

OTP Inputs

Checkboxes

Radio Buttons

Social Login Buttons

---

# ACCESSIBILITY

Support:

Keyboard Navigation

Focus Management

ARIA Labels

High Contrast

Reduced Motion

Screen Readers

Responsive Layout

---

# RESPONSIVENESS

Desktop

Tablet

Mobile

Authentication must feel excellent on all screen sizes.

---

# BRAND EXPERIENCE

The authentication flow should make new users immediately feel that they are using an enterprise-grade developer platform.

Avoid generic login pages.

Use subtle illustrations, onboarding progress, informative empty states, and contextual guidance.

The first impression should communicate reliability, performance, and security.

---

# FRONTEND STACK

Design for:

Next.js App Router

TypeScript

Tailwind CSS

shadcn/ui

React Hook Form

Zod

next-themes

Framer Motion (subtle)

Lucide Icons

---

# OUTPUT REQUIREMENTS

Do not generate frontend code immediately.

Instead produce:

1. Complete authentication information architecture.
2. All authentication pages in tree form.
3. Navigation flow diagrams.
4. User journeys for every authentication scenario.
5. Screen-by-screen wireframes (textual).
6. Component inventory.
7. Validation rules.
8. Security UX recommendations.
9. Backend API requirements.
10. Required database entities.
11. Session management strategy.
12. JWT/session/cookie recommendations.
13. OAuth architecture assumptions.
14. Role and permission model.
15. Complete onboarding flow.
16. Enterprise SSO readiness.
17. Future support for passkeys, WebAuthn, and passwordless authentication.
18. Finally, produce a milestone-based implementation plan suitable for a production SaaS platform.

---
