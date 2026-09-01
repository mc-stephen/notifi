# Onboarding API Contract

Contract for the onboarding flow (`/onboarding/*`, 6 steps). Sources: `app/onboarding/`, `store/onboarding-store.ts`, `lib/env.ts`.

## Current State

The flow is client-side (Zustand, `store/onboarding-store.ts`) **except for completion**: the success page `POST`s the collected project to the API (`/v1/auth/onboarding/complete`), which persists it and flips the server-derived `onboardingCompleted` flag returned by login/signup/`me`. The dashboard layout gates on that flag — accounts that own/belong to no project are redirected back into onboarding on every visit. Skip buttons have been removed; all steps are required.

Data collected per step below; only the project (+ its default `development` environment) persists today — channel selection and team invites still need backend endpoints. The environment gate is now **persisted on the server**: the topbar segmented control calls `PATCH /v1/projects/{id}/environment` to switch between development and production (see the auth contract §5b for the full endpoint shapes). **API keys are not part of onboarding** — they are managed in the dashboard (`/api-keys`, Developers page); server-side key generation (shown exactly once) remains pending backend work.

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
Environment is not collected — new projects always start in **development mode** (the project-level environment gate, see the auth contract §5b). Switching to live happens later in the dashboard.

### `/onboarding/setup-channels`
```json
{ "selectedChannels": ["email", "sms", "push", "in_app", "webhook"] }
```
Subset selection; `email` and `in_app` are marked pre-configured.

### `/onboarding/invite-team`
```json
{ "invites": [{ "email": "string", "role": "admin | editor | viewer" }] }
```
**Known gap:** UI collects roles but the store only persists emails (`teamEmails`) — roles are dropped. Invites will attach to the just-created project (per-project membership model, see the auth contract §5b).

### `/onboarding/success`
Reads back the collected project data, calls the complete endpoint below, then marks onboarding complete locally and redirects to `/`.

## Endpoints

### `POST /v1/auth/onboarding/complete` (implemented)

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
| team invites | Send per-project invites from the onboarding invite step |
| groups CRUD | Folder create/rename/assign endpoints for the projects page |

The welcome/use-case data is not persisted.
