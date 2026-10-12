# `src/api` — HTTP presentation layer

Folders match URL prefixes. Shared code stays here; no `shared/` folder.

| Folder | Serves | Caller | Auth |
|--------|--------|--------|------|
| `app/` | `/app/*` — dashboard backend (auth, projects, templates, sends, schedules…) | user-dashboard | `CurrentUser` cookie |
| `admin/` | `/admin/*` — platform console (admins, users, all projects, broadcasts…) | admin-dashboard | `CurrentAdmin` + 2FA |
| `project/` | `/v1/*` — product API: `POST /notifications`, `POST /hooks/{token}` | customers' apps | session now, API keys M6 |

Shared: `state.rs` (`AppState`), `catalog.rs` (`GET /routes`), `middleware/`, `error.rs` (RFC 9457), `handlers.rs` (ops: `/`, `/healthz`, `/readyz`).

Rule: new surface → new folder named after its URL prefix. Detail lives in `assets/docs/ARCHITECTURE.md` §8.
