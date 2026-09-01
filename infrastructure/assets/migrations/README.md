# Migrations

Numbered sqlx migrations, applied automatically at server boot (`src/infra/db.rs`).

- `0001_initial_schema.sql` — the clean-origin schema (dev-stage squash of
  the pre-production history; from production onward, migrations are
  **append-only** — never edit or delete an applied file, only add the next
  number, per `docs/ARCHITECTURE.md` §19).
- Tables are domain-prefixed: `auth_*` (identity), `platform_*` (projects,
  members, user folders), `event_outbox` (transactional outbox).
