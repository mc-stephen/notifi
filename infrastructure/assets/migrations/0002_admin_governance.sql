-- 0002_admin_governance.sql
-- Super-admin governance for platform admins (append-only: 0001 is applied
-- to existing databases, so this ships as a new file — never edit 0001).
--
-- Model:
--   * the earliest-created admin is the super admin (backfilled below);
--     only a super admin approves things and only a super admin acts
--     unilaterally. The super admin cannot be removed or suspended.
--   * new admins start `pending` and cannot log in until approved, unless
--     created by the super admin (active immediately).
--   * removal is soft-delete (`deleted_at`) via an approval request, except
--     super-admin removals which apply immediately.

ALTER TABLE admin_users
    ADD COLUMN is_super_admin BOOLEAN NOT NULL DEFAULT FALSE;

-- The first admin ever bootstrapped becomes the super admin.
UPDATE admin_users
SET is_super_admin = TRUE
WHERE id = (SELECT id FROM admin_users ORDER BY created_at ASC LIMIT 1);

ALTER TABLE admin_users
    ADD COLUMN status TEXT NOT NULL DEFAULT 'active'
    CHECK (status IN ('pending', 'active', 'suspended'));

CREATE TABLE admin_approval_requests (
    id             VARCHAR(26) PRIMARY KEY,
    kind           TEXT        NOT NULL CHECK (kind IN ('create', 'remove')),
    target_admin_id VARCHAR(26) NOT NULL REFERENCES admin_users(id) ON DELETE CASCADE,
    requested_by   VARCHAR(26) NOT NULL REFERENCES admin_users(id) ON DELETE CASCADE,
    status         TEXT        NOT NULL DEFAULT 'pending'
                   CHECK (status IN ('pending', 'approved', 'rejected')),
    decided_by     VARCHAR(26) NULL REFERENCES admin_users(id) ON DELETE SET NULL,
    decided_at     TIMESTAMPTZ,
    created_at     TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX idx_approval_requests_status ON admin_approval_requests(status, created_at DESC);
CREATE INDEX idx_approval_requests_target ON admin_approval_requests(target_admin_id);
