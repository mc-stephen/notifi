-- 0003_audit_actor.sql
-- Actor attribution for audit logs (append-only: 0001 is applied to
-- existing databases, so this ships as a new file — never edit 0001).
--
-- Problem: admin actions stored admin_users ULIDs in audit_logs.user_id,
-- which references auth_users(id) — Postgres rejects those rows, so admin
-- audit writes were silently dropped. The new actor_type + admin_id columns
-- carry admin attribution explicitly; user_id stays for platform users.

ALTER TABLE audit_logs
    ADD COLUMN actor_type TEXT NOT NULL DEFAULT 'user'
    CHECK (actor_type IN ('user', 'admin', 'system'));

ALTER TABLE audit_logs
    ADD COLUMN admin_id VARCHAR(26) NULL REFERENCES admin_users(id) ON DELETE SET NULL;

CREATE INDEX idx_audit_logs_actor_time
    ON audit_logs(actor_type, occurred_at DESC);
CREATE INDEX idx_audit_logs_admin_time
    ON audit_logs(admin_id, occurred_at DESC)
    WHERE admin_id IS NOT NULL;
