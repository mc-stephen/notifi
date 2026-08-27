# Onboarding API Contract

Contract for the onboarding flow (`/onboarding/*`, 7 steps). Sources: `app/onboarding/`, `store/onboarding-store.ts`, `lib/env.ts`.

## Current State

The flow is client-side (Zustand, `store/onboarding-store.ts`) **except for completion**: the success page `POST`s the collected org + project to the API (`/v1/auth/onboarding/complete`), which persists them and flips the server-derived `onboardingCompleted` flag returned by login/signup/`me`. The dashboard layout gates on that flag — accounts without an org + project are redirected back into onboarding on every visit. Skip buttons have been removed; all steps are required.

Data collected per step below; only organization + project (+ default environment) persist today — channel selection and team invites still need backend endpoints. **API keys are not part of onboarding** — they are managed in the dashboard (`/api-keys`, Developers page); server-side key generation (shown exactly once) remains pending backend work.

## Flow

`welcome → use-case → organization → project → setup-channels → invite-team → success`

## Data Collected Per Step

### `/onboarding/welcome`
No data. Reads `user.name` from the auth store for a personalized greeting.

### `/onboarding/use-case`
```json
{ "useCase": "transactional | marketing | collaboration | alerts | marketplace | enterprise" }
```

### `/onboarding/organization`
```json
{ "orgName": "string", "orgLogo": "string | null" }
```
Notes: logo upload button is currently non-functional. (Region/timezone were removed from the UI; the API still accepts them as optional fields and stores NULL — reserved for future data-residency use.)

### `/onboarding/project`
```json
{ "projectName": "string", "projectDescription": "string" }
```
Environment is not collected — new projects always start in `development` (sent fixed at completion; staging/production managed later).

### `/onboarding/setup-channels`
```json
{ "selectedChannels": ["email", "sms", "push", "in_app", "webhook"] }
```
Subset selection; `email` and `in_app` are marked pre-configured.

### `/onboarding/invite-team`
```json
{ "invites": [{ "email": "string", "role": "admin | editor | viewer" }] }
```
**Known gap:** UI collects roles but the store only persists emails (`teamEmails`) — roles are dropped. Backend contract should accept `{ email, role }` pairs.

### `/onboarding/success`
Reads back the collected org/project data, calls the complete endpoint below, then marks onboarding complete locally and redirects to `/`.

## Endpoints

### `POST /v1/auth/onboarding/complete` (implemented)

Single atomic call from the success page: `{ organization: { name, logoUrl? }, project: { name, description?, environment: "development" } }` → `{ status: "ok" }`; creates org + owner membership + project + default environment; idempotent via `alreadyCompleted`. Full shapes in the auth contract (`../auth/API_CONTRACT.md` §5b).

### Still pending backend work

| Endpoint | Purpose |
|---|---|
| API keys | Server-generated keys (`prefix` + hash per `platform_projects`/`auth_api_keys` schema), shown once, managed at `/api-keys` |
| channel selection persistence | Enable selected channels for the project |
| team invites | Send invites from the onboarding invite step |

The welcome/use-case data is not persisted.
