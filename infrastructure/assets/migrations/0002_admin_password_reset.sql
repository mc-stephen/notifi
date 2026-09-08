-- 0002_admin_password_reset.sql
-- One-time password-reset tokens for platform admins (separate from the
-- customer `auth_tokens` table, which references `auth_users`).
-- Email delivery is not wired; raw tokens are surfaced via logs and, when
-- explicitly enabled, in API responses (dev mode only).

CREATE TABLE admin_password_reset_tokens (
    id          VARCHAR(26) PRIMARY KEY,
    admin_id    VARCHAR(26) NOT NULL REFERENCES admin_users(id) ON DELETE CASCADE,
    token_hash  TEXT        NOT NULL UNIQUE,
    expires_at  TIMESTAMPTZ NOT NULL,
    consumed_at TIMESTAMPTZ,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX idx_admin_reset_tokens_admin ON admin_password_reset_tokens(admin_id);
