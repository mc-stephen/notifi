# Migrations

Single-file dev-stage squash, applied automatically at server boot (`src/infra/db.rs`).

- `0001_initial_schema.sql` — the complete schema in one file (dev-stage squash of
  the pre-production history; from production onward migrations are
  **append-only** — never edit or delete an applied file, only add the next
  number, per `docs/ARCHITECTURE.md` §19).
- Tables are domain-prefixed: `auth_*` (identity), `platform_*` (projects,
  members, recipients, templates, provider configs), `billing_plans` +
  `project_subscriptions` (pricing catalog + per-project subscriptions),
  `audit_logs` (append-only audit trail), `event_outbox` (transactional outbox).
- Plus: `project_invites` (team invites), `key_ip_allowlist` (per-key IP
  allowlists), `notification_sends` + `flows` (record-first sends; no `kind`
  column — "default" is emergent, not stored), `schedules` (once/recurring/
  weekly + template columns folded in), `flow_trigger_tokens` (inbound
  webhook tokens). Folded into `0001` on 2026-10-12; the 0010 OAuth backfill
  was data-only and is omitted on fresh databases.
