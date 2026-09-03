# Migrations

Single-file dev-stage squash, applied automatically at server boot (`src/infra/db.rs`).

- `0001_initial_schema.sql` — the complete schema in one file (dev-stage squash of
  the pre-production history; from production onward migrations are
  **append-only** — never edit or delete an applied file, only add the next
  number, per `docs/ARCHITECTURE.md` §19).
- Tables are domain-prefixed: `auth_*` (identity), `platform_*` (projects,
  members, recipients, templates, provider configs), `audit_logs` (append-only
  audit trail), `event_outbox` (transactional outbox).
