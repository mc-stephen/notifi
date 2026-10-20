# Onboarding API Contract

Contract for the onboarding flow (`/onboarding/*`, 6 steps). Sources: `app/onboarding/`, `store/onboarding-store.ts`, `lib/env.ts`.

## Current State

The flow is client-side (Zustand, `store/onboarding-store.ts`) **except for completion**: the success page `POST`s the collected project to the API (`/app/auth/onboarding/complete`), which persists it and flips the server-derived `onboardingCompleted` flag returned by login/signup/`me`. The dashboard layout gates on that flag — accounts that own/belong to no project are redirected back into onboarding on every visit. Skip buttons have been removed; all steps are required.

Data collected per step below; the project persists via the complete endpoint and starts **inactive** (the project-level kill switch — turn it on via the topbar switch, Settings → Project state, or the dashboard banner's Resume delivery), and **team invites send from the success step** once the project exists (per-invite `POST /app/projects/{id}/members`). Channel selection still needs a backend endpoint. The project state is **persisted on the server**: the topbar Active/Off switch calls `PATCH /app/projects/{id}/active` (see the auth contract §5b for the full endpoint shapes). **API keys are not part of onboarding** — they are managed in the dashboard (`/api-keys`); server-side key generation (secret shown exactly once) is live at `POST /app/projects/{id}/api-keys`.

## Flow

`welcome → use-case → project → setup-channels → invite-team → success`

## Data Collected Per Step

### `/onboarding/welcome`
No data. Reads `user.name` from the auth store for a personalized greeting.

### `/onboarding/use-case`
```json
{ "useCase": "transactional | marketing | collaboration | alerts | marketplace | enterprise" }
```

### `/onboarding/project`
```json
{ "projectName": "string", "projectDescription": "string" }
```
No environment concept — new projects start **inactive** (the project-level kill switch, see the auth contract §5b). To activate a project, use the topbar switch, Settings → Project state, or the dashboard banner's Resume delivery; to pause it, flip any of the same switches off.

### `/onboarding/setup-channels`
```json
{ "selectedChannels": ["email", "sms", "push", "in_app", "webhook"] }
```
Subset selection; `email` and `in_app` are marked pre-configured.

### `/onboarding/invite-team`
```json
{ "invites": [{ "email": "string", "role": "admin | developer | viewer" }] }
```
Roles match the backend's assignable set (`owner` excluded — only the creator holds it). The store keeps the full `{email, role}` pairs (`teamInvites`; `teamEmails` mirrors the addresses for compat). Nothing sends here — the project does not exist yet.

### `/onboarding/success`
Reads back the collected project data, calls the complete endpoint below, then sends each pending invite (`POST /app/projects/{id}/members` against the just-created project, resolved via `GET /app/projects` name match), then marks onboarding complete locally and redirects to `/`. Per-invite failures (unknown account → `404`, already on team → `409`) are reported inline with a "resend from the Team page" hint and never block entering the dashboard.

## Endpoints

### `POST /app/auth/onboarding/complete` (implemented)

Single atomic call from the success page: `{ project: { name, description? } }` → `{ status: "ok" }`; creates the project (owned by the session user, starting in development mode); idempotent via `alreadyCompleted`. Full shapes in the auth contract (`../auth/API_CONTRACT.md` §5b).

## Project model (schema-ready, no endpoints yet)

- **Ownership**: `platform_projects.created_by` = the owner (the account is the workspace)
- **Membership**: `platform_project_members` — `(project_id, user_id, role ∈ owner|admin|editor|viewer)`
- **Folders**: `platform_user_groups` + `platform_user_group_projects` — user-level UI grouping of projects, no auth/billing semantics

### Still pending backend work

| Endpoint | Purpose |
|---|---|
| API keys | Server-generated keys (`prefix` + hash per `platform_projects`/`auth_api_keys` schema), shown once, managed at `/api-keys` |
| channel selection persistence | Enable selected channels for the project |
| groups CRUD | Folder create/rename/assign endpoints for the projects page |

The welcome/use-case data is not persisted.
