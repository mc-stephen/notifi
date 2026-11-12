# Migrations

Single-file dev-stage squash, applied automatically at server boot (`src/infra/db.rs`).

- `0001_initial_schema.sql` — the complete schema in one file (dev-stage squash of
  the pre-production history; from production onward migrations are
  **append-only** — never edit or delete an applied file, only add the next
  number, per `docs/ARCHITECTURE.md` §19).
- Tables are domain-prefixed: `auth_*` (identity), `platform_*` (projects,
  members, recipients, templates, provider configs), `billing_plans` +
  `project_subscriptions` (pricing catalog + per-project subscriptions),
  `gateway_messages` (device-route poll queue — poll-and-delete),
  `audit_logs` (append-only audit trail), `event_outbox` (transactional outbox).
- `0002_user_project_limit.sql` — per-user project ceiling (`auth_users.project_limit`,
  default 5, admin-raised; tombstones occupy the slot until purge).
- `0003_project_limit_bigint.sql` — widens `auth_users.project_limit` to `BIGINT`
- `0004_drop_webhooks_table.sql` — drops the Webhooks page's ownership rows (webhook triggers are definition-born now; flows + `flow_trigger_tokens` survive)
- `0005_flow_inbound_probes.sql` — adds `flow_inbound_probes`: the last authorized ping per webhook door (IP, whitelisted headers, capped body) for the builder's "Incoming data" tab
- `0006_flow_runs.sql` — adds `flow_runs`: one row per completed flow execution (mode, status, terminal error, timestamps, send link, `steps` JSON trace) for the builder's History tab
  (INT8) to match the repository's `i64` reads/binds; the INT4 it was created as
  refused to decode at runtime (ceiling, project creation, admin user view).
- Plus: `project_invites` (team invites), `key_ip_allowlist` (per-key IP
  allowlists), `notification_sends` + `flows` (record-first sends; no `kind`
  column — "default" is emergent, not stored), `schedules` (once/recurring/
  weekly + template columns folded in), `flow_trigger_tokens` (inbound
  webhook tokens), `project_exports` (deletion-window download ledger).
  Folded into `0001` on 2026-10-12 (governance/billing/TOTP) and again on
  2026-10-27 (default-inactive + deletion lifecycle; the 0010 OAuth backfill
  was data-only and is omitted on fresh databases). The never-shipped
  `project_devices` table was dropped the same day — device tokens live in
  the recipient contacts blob (§25).
