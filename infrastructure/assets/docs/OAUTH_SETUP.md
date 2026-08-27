# OAuth Provider Setup

How to obtain GitHub and Google OAuth credentials for local development.
Both providers are free; the server only needs the client id/secret pairs
from its environment (see `infrastructure/.env`, which is gitignored).

Complements §3 of the auth contract (`app/dashboard/app/auth/API_CONTRACT.md`)
and the `OAuthConfig` section in `ARCHITECTURE.md` §18.

## Env keys

```env
NOTIFI_OAUTH_GITHUB_CLIENT_ID=...
NOTIFI_OAUTH_GITHUB_CLIENT_SECRET=...
NOTIFI_OAUTH_GOOGLE_CLIENT_ID=...
NOTIFI_OAUTH_GOOGLE_CLIENT_SECRET=...
# Optional (defaults shown):
NOTIFI_DASHBOARD_URL=http://localhost:3000
NOTIFI_API_BASE_URL=http://localhost:8080
```

A provider is enabled only when **both** its id and secret are set. The only
values that must match the code exactly are the callback URLs.

## GitHub

1. GitHub → avatar → **Settings** → **Developer settings** → **OAuth Apps**
   → **New OAuth App** (do not use a GitHub App — different flow).
2. Fill in:
   - **Application name**: `Notifi (dev)`
   - **Homepage URL**: `http://localhost:3000`
   - **Authorization callback URL**:
     `http://localhost:8080/v1/auth/oauth/github/callback`
     (must equal `NOTIFI_API_BASE_URL + /v1/auth/oauth/github/callback`)
3. **Register application** → copy the **Client ID**. Click
   **Generate a new client secret** — copy it immediately (GitHub shows it
   once; it can be regenerated later).
4. Put both values in `infrastructure/.env`.

## Google

1. [console.cloud.google.com](https://console.cloud.google.com) → create or
   select a project (e.g. `notifi-dev`).
2. **APIs & Services** → **OAuth consent screen**:
   - User type: **External**
   - App name, support email, developer contact — anything sensible for dev
   - Scopes: nothing extra needed — the app already requests
     `openid`, `email`, `profile`
   - Add yourself as a **Test user** (in Testing mode, unlisted accounts get
     "access blocked")
3. **APIs & Services** → **Credentials** → **+ Create Credentials** →
   **OAuth client ID** → Application type: **Web application**.
4. Fill in:
   - **Authorized JavaScript origins**: `http://localhost:3000`
   - **Authorized redirect URIs**:
     `http://localhost:8080/v1/auth/oauth/google/callback`
5. Create → copy **Client ID** and **Client secret** into
   `infrastructure/.env`.

## Wire-up & test

1. Restart the API (`cargo run` from `infrastructure/`). The boot log should
   no longer print `oauth disabled (no provider credentials)`.
2. Smoke test:
   ```shell
   curl -I "http://localhost:8080/v1/auth/oauth/github"
   ```
   Expect `302` to `github.com/login/oauth/authorize...` — a `503` means the
   env vars weren't picked up.
3. Full test: dashboard → Sign in → GitHub/Google → consent → popup closes →
   you're signed in.

## Gotchas

- **Google "access blocked / invalid client"** — you're not on the consent
  screen's test-user list, or the redirect URI has a typo/trailing-slash
  mismatch.
- **Google profile fetch** — the server reads the profile from the
  `id_token` in the token response; the `openidconnect.googleapis.com`
  userinfo endpoint is only a fallback (some networks block that host).
- **GitHub: no verified email** — the account's GitHub emails are set to
  private (or only `@users.noreply.github.com` addresses exist, which never
  count as public). The server requires a provider-verified email to
  create/link an account. Fix: GitHub → Settings → Emails → make one email
  public.
- **Ports must match** — dashboard `3000`, API `8080`. If the API runs
  elsewhere, update the registered redirect URI **and** `NOTIFI_API_BASE_URL`.
- **Production** — register separate apps/URIs for the real domains and set
  `NOTIFI_DASHBOARD_URL` accordingly; never ship dev credentials.
