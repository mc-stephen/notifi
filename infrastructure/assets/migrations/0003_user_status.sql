-- 0003_user_status.sql
-- Account state for platform users (admin suspend/restore). Soft-delete via
-- `deleted_at` is unchanged; `status` only tracks active/suspended.

ALTER TABLE auth_users
    ADD COLUMN status TEXT NOT NULL DEFAULT 'active'
    CHECK (status IN ('active', 'suspended'));

CREATE INDEX idx_auth_users_status ON auth_users(status) WHERE deleted_at IS NULL;
