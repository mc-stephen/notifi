# Auth API Contract

The contract between the dashboard auth pages and the Rust API (`infrastructure/crates/domains/auth`, mounted under `/v1/auth` in `crates/api/src/http/routes.rs`). **Implemented on both sides** — keep this file updated when either side changes (see root `AGENTS.md`).

Sources: `app/auth/`, `hooks/use-auth.ts`, `store/auth-store.ts`, `lib/auth-types.ts`, `lib/api.ts`; backend: `domains/auth/src/presentation/{routes,handlers,dto}.rs`.

## Conventions

- Base URL: `NEXT_PUBLIC_API_URL` (`.env.local`: `http://localhost:8080`). All paths below are relative to it.
- Wire format is camelCase; backend DTOs use `#[serde(rename_all = "camelCase")]`.
- Auth is **cookie-based**: the server sets/clears an httpOnly `session_token` cookie (`SameSite=Lax`, 30 days with `rememberMe`, else 1 day). The frontend never touches `document.cookie`.
- CORS: only browser origins in `NOTIFI_CORS_ORIGINS` may call with credentials (defaults to `http://localhost:3000`). Methods GET/POST only.
- Errors are RFC 9457 problem documents (`application/problem+json`):
  `{ type, title, status, detail, correlation_id }` — the frontend surfaces `detail ?? title`.
- Dev tokens: when `expose_dev_tokens=true` (`NOTIFI_AUTH_EXPOSE_DEV_TOKENS`), one-time verification/reset tokens are returned in responses. Local development only until real email delivery lands (M4).

## Endpoints

### 1. Login — `POST /v1/auth/login`

Request:

```json
{ "email": "string", "password": "string", "rememberMe": "boolean" }
```

Response `200`:

```json
{
  "user": { "...UserDto": "see Shared Types" },
  "session": {
    "user": {},
    "token": "raw cookie value (shown once)",
    "expiresAt": "ISO datetime",
    "onboardingCompleted": false
  }
}
```

`session.onboardingCompleted` is server-derived: true when the account already belongs to an organization that owns at least one project (e.g. an invited member). Sets the httpOnly `session_token` cookie. Errors: `401` "Invalid email or password." Page redirects to `/`.

### 2. Signup — `POST /v1/auth/signup`

Request:

```json
{ "name": "string", "email": "string", "password": "string" }
```

(The page also validates `confirmPassword` + `agreedToTerms` locally — not sent.)

Response `201`:

```json
{
  "user": {},
  "session": { "user": {}, "token": "raw cookie value (shown once)", "expiresAt": "ISO datetime" },
  "verificationToken": "raw token (dev mode only)"
}
```

Signup **starts a session immediately** (`rememberMe = false` semantics → 1-day httpOnly `session_token` cookie) so the browser continues into onboarding and the dashboard without logging in again. `session.onboardingCompleted` is included (server-derived, see Login). Errors: `409` "An account with this email already exists.", `400` validation.

Page behavior: redirects to `/onboarding/welcome`. The backend sends a **welcome email plus a verification email** whose link points to `/auth/verify-email?token={token}` (email delivery pending M4; in dev mode the raw token is returned instead for manual testing of that page). Until verified, the dashboard shows an amber banner warning that unverified accounts are deleted after 48 hours (deletion enforcement pending).

### 3. OAuth — `GET /v1/auth/oauth/{provider}` (`github` | `google`)

Browser-redirect flow (popup-first on the dashboard):

1. **Start** `GET /v1/auth/oauth/github?popup=1` → `302` to the provider's consent screen; parks short-lived httpOnly cookies (`oauth_state_*`, `oauth_verifier_*`, `oauth_mode_*`) scoped to the callback path. Unconfigured provider (no client credentials) → `503` problem document; unknown provider → `400`.
2. **Callback** `GET /v1/auth/oauth/{provider}/callback?code=…&state=…` → validates CSRF state, exchanges the code server-side, upserts the user, sets the standard 1-day `session_token` cookie, then:
   - `popup` mode → `200 text/html` page that postMessages `{ "type": "oauth:success" }` (or `{ "type": "oauth:error" }`) to the dashboard origin and closes the window;
   - `redirect` mode → `302` to the dashboard root.
   - Any failure → popup gets `oauth:error`; redirect mode lands on `/auth/login?error=oauth_failed`.
3. Dashboard: opens the start URL in a popup from the click handler (falls back to full-page redirect when blocked), listens for one validated-origin message, then calls `/v1/auth/me` to hydrate user + onboarding flag.

**Account linking:** only provider-verified emails are accepted. Resolution order: existing `(provider, subject)` account → sign in; else an account with the same email → auto-link the OAuth identity onto it (a provider-verified address proves inbox control); else create a fresh verified account.

**Config (backend env):** `NOTIFI_OAUTH_GITHUB_CLIENT_ID/_SECRET`, `NOTIFI_OAUTH_GOOGLE_CLIENT_ID/_SECRET`, `NOTIFI_DASHBOARD_URL` (default `http://localhost:3000`), `NOTIFI_API_BASE_URL` (default `http://localhost:8080`). Callback URLs to register with providers: `{api_base_url}/v1/auth/oauth/{provider}/callback`.

### 4. Logout — `POST /v1/auth/logout`

Revokes the session behind the cookie and clears it. Idempotent — always `200 { "status": "ok" }`, even without a cookie.

### 5. Current user — `GET /v1/auth/me`

Requires a valid session cookie. Response `200 { "user": {}, "onboardingCompleted": false }`. Errors: `401` "Authentication required."

Called by the dashboard on first load (`fetchMe`) to restore session state, including the onboarding flag.

### 5b. Complete onboarding — `POST /v1/auth/onboarding/complete`

Requires a valid session cookie.

Request:

```json
{
  "organization": { "name": "string", "logoUrl": "string|null", "region": "string|null", "timezone": "string|null" },
  "project": { "name": "string", "description": "string|null", "environment": "development | staging | production" }
}
```

Response `200 { "status": "ok" }` — or `{ "status": "ok", "alreadyCompleted": true }` when the flag was already set (idempotent). Creates the organization + owner membership + project + default environment in one transaction. Errors: `400` validation (names 1–100 chars, environment allowlist).

Called by the onboarding success page; flips the server-derived flag that unlocks the dashboard.

### 6. Forgot password — `POST /v1/auth/password/forgot`

Request: `{ "email": "string" }`

Always `200 { "status": "ok"[, "resetToken": "dev mode"] }` — never reveals whether the account exists (no enumeration). Emails a reset link. Page shows its "check your email" state regardless.

### 7. Reset password — `POST /v1/auth/password/reset`

Request: `{ "token": "string", "password": "string" }` (token from `?token=` query param of the emailed link)

Response `200 { "status": "ok" }`. Consumes the one-time token, rotates the password, and revokes all existing sessions for the account. Errors: `400` invalid/used token, `410` expired token. Page shows success then redirects to `/auth/login` after 2s.

### 8. Verify email — `POST /v1/auth/verify-email`

Request: `{ "token": "string" }` (from `?token=` query param)

Response `200 { "status": "ok" }`. Errors: `410` expired (detail contains the word "expired" — **load-bearing**: the page branches on it to pick its "Link expired" vs "Invalid link" state; do not change that wording), `400` unknown/used token. Success → page redirects to `/onboarding/welcome` after 3s.

### 9. Resend verification — `POST /v1/auth/verify-email/resend`

Request: `{ "email": "string" }`

Always `200 { "status": "ok" }[, "verificationToken": dev]` — no enumeration. Wired to the verify-email page's "Resend" button; requires `&email=` in the page URL (signup includes it). Without it the page shows a "sign in and request a new link" hint instead of calling.

## Shared Types (`lib/auth-types.ts` ⇄ `dto.rs`)

```ts
User    = { id, name, email, avatar: string | null, emailVerified, createdAt, lastLoginAt: string | null }
Session = { user, token, expiresAt }   // SessionDto from login/OAuth
```

- Password policy (frontend-enforced): min 8 chars + uppercase, lowercase, number, special char.
- `lastLoginAt` and `avatar` are nullable (server may omit/null them).

## Middleware Interaction

`proxy.ts` treats any `session_token` cookie as authenticated; `/auth/*` and `/onboarding/*` are public. Authenticated users hitting `/auth/*` are redirected to `/` except: `/auth/signup`, `/auth/password/forgot`, `/auth/password/reset`, `/auth/verify-email`. Because cookies ignore ports, the localhost-scoped API cookie flows to Next.js navigations in local dev.
