# `src/api` — HTTP presentation layer

Shared code stays here; no `shared/` folder.

```
src/api/
├── routes/      # audience-specific route surfaces (one folder per caller)
│   ├── app/       → `/app/*`   user-dashboard backend
│   ├── external/  → `/v1/*`    product API for customers' apps
│   └── admin/     → `/admin/*` platform console
├── middleware/  # cross-cutting HTTP middleware (cors, request_id)
├── state.rs     # AppState
├── catalog.rs   # GET /routes registry
├── error.rs     # RFC 9457 problem documents
└── handlers.rs  # ops routes: /, /healthz, /readyz
```

| Surface | Serves | Caller | Auth |
|---------|--------|--------|------|
| `routes/app/` | `/app/*` — dashboard backend (auth, projects, templates, sends, schedules…) | user-dashboard | `CurrentUser` cookie |
| `routes/external/` | `/v1/*` — product API: `POST /v1/notifications` | customers' apps | session now, API keys M6 |
| `routes/admin/` | `/admin/*` — platform console (admins, users, all projects, broadcasts…) | admin-dashboard | `CurrentAdmin` + 2FA |

Rule: new surface → new folder under `routes/` named after its caller; middleware
stays in `middleware/`, never mixed into a surface. Detail:
`assets/docs/ARCHITECTURE.md` §8.
