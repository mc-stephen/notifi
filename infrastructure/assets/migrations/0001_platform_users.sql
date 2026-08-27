-- 0001_platform_users.sql
-- Platform users: people who sign in to the Notifi dashboard.
--
-- Conventions:
--   * All ids are ULIDs stored as 26-char strings (see notifi_core::id).
--   * timestamps are TIMESTAMPTZ; created_at/updated_at everywhere;
--     deleted_at enables soft deletes; version enables optimistic locking.
--   * Secrets are never stored in plaintext: token_hash/key_hash are
--     SHA-256 hex digests of the raw token.

CREATE TABLE auth_users (
    id                VARCHAR(26) PRIMARY KEY,
    name              TEXT        NOT NULL,
    email             TEXT        NOT NULL UNIQUE,
    password_hash     TEXT        NOT NULL,
    avatar_url        TEXT,
    email_verified_at TIMESTAMPTZ,
    oauth_provider    TEXT, -- 'github' | 'google'; NULL = email/password
    oauth_subject     TEXT, -- provider-side user id
    last_login_at     TIMESTAMPTZ,
    created_at        TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at        TIMESTAMPTZ NOT NULL DEFAULT now(),
    deleted_at        TIMESTAMPTZ,
    UNIQUE (oauth_provider, oauth_subject)
);

-- Active sign-in sessions (dashboard cookie `session_token`; only the hash
-- is stored server-side).
CREATE TABLE auth_sessions (
    id           VARCHAR(26) PRIMARY KEY,
    user_id      VARCHAR(26) NOT NULL REFERENCES auth_users(id) ON DELETE CASCADE,
    token_hash   TEXT        NOT NULL UNIQUE,
    expires_at   TIMESTAMPTZ NOT NULL,
    revoked_at   TIMESTAMPTZ,
    last_used_at TIMESTAMPTZ,
    created_at   TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX idx_auth_sessions_user ON auth_sessions(user_id);

-- One-time tokens for email verification and password reset flows.
CREATE TABLE auth_tokens (
    id          VARCHAR(26) PRIMARY KEY,
    user_id     VARCHAR(26) NOT NULL REFERENCES auth_users(id) ON DELETE CASCADE,
    purpose     TEXT        NOT NULL
                CHECK (purpose IN ('email_verification', 'password_reset')),
    token_hash  TEXT        NOT NULL UNIQUE,
    expires_at  TIMESTAMPTZ NOT NULL,
    consumed_at TIMESTAMPTZ,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX idx_auth_tokens_user ON auth_tokens(user_id);
