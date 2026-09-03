-- 0001_initial_schema.sql
-- Clean-origin schema for the Notifi server (dev-stage squash of all
-- migrations into a single file — from production onward migrations
-- are append-only, see ARCHITECTURE.md §19).
--
-- Conventions:
--   * All ids are ULIDs stored as 26-char strings (see notifi_core::id).
--   * timestamps are TIMESTAMPTZ; created_at/updated_at everywhere;
--     deleted_at enables soft deletes; version enables optimistic locking.
--   * Secrets are never stored in plaintext: token_hash/key_hash are
--     SHA-256 hex digests of the raw token.

-- ---------------------------------------------------------------------------
-- auth: users, sessions, one-time tokens
-- ---------------------------------------------------------------------------

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

-- ---------------------------------------------------------------------------
-- projects and API keys
-- ---------------------------------------------------------------------------

CREATE TABLE platform_projects (
    id          VARCHAR(26) PRIMARY KEY,
    name        TEXT        NOT NULL,
    slug        TEXT        NOT NULL UNIQUE,
    description TEXT,
    created_by  VARCHAR(26) REFERENCES auth_users(id) ON DELETE SET NULL,
    environment TEXT        NOT NULL DEFAULT 'development'
               CHECK (environment IN ('development', 'production')),
    version     INT         NOT NULL DEFAULT 1,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at  TIMESTAMPTZ NOT NULL DEFAULT now(),
    deleted_at  TIMESTAMPTZ
);

CREATE INDEX idx_projects_created_by ON platform_projects(created_by);

CREATE TABLE auth_api_keys (
    id             VARCHAR(26) PRIMARY KEY,
    project_id     VARCHAR(26) NOT NULL REFERENCES platform_projects(id) ON DELETE CASCADE,
    name           TEXT        NOT NULL,
    prefix         TEXT        NOT NULL,
    key_hash       TEXT        NOT NULL UNIQUE,
    environment    TEXT        NOT NULL DEFAULT 'development'
                   CHECK (environment IN ('development', 'production')),
    permissions    TEXT[]      NOT NULL DEFAULT '{}',
    scopes         TEXT[]      NOT NULL DEFAULT '{}',
    rate_limit     INT,
    expires_at     TIMESTAMPTZ,
    last_used_at   TIMESTAMPTZ,
    enabled        BOOLEAN     NOT NULL DEFAULT TRUE,
    revoked_at     TIMESTAMPTZ,
    version        INT         NOT NULL DEFAULT 1,
    created_at     TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at     TIMESTAMPTZ NOT NULL DEFAULT now(),
    deleted_at     TIMESTAMPTZ,
    UNIQUE (project_id, environment, name)
);

CREATE INDEX idx_api_keys_project ON auth_api_keys(project_id);

-- ---------------------------------------------------------------------------
-- per-project membership and user-level project folders
-- ---------------------------------------------------------------------------

CREATE TABLE platform_project_members (
    id         VARCHAR(26) PRIMARY KEY,
    project_id VARCHAR(26) NOT NULL REFERENCES platform_projects(id) ON DELETE CASCADE,
    user_id    VARCHAR(26) NOT NULL REFERENCES auth_users(id) ON DELETE CASCADE,
    role       TEXT        NOT NULL
               CHECK (role IN ('owner', 'admin', 'editor', 'viewer')),
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    deleted_at TIMESTAMPTZ,
    UNIQUE (project_id, user_id)
);

CREATE INDEX idx_project_members_user ON platform_project_members(user_id);

CREATE TABLE platform_user_groups (
    id         VARCHAR(26) PRIMARY KEY,
    user_id    VARCHAR(26) NOT NULL REFERENCES auth_users(id) ON DELETE CASCADE,
    name       TEXT        NOT NULL,
    position   INT         NOT NULL DEFAULT 0,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    deleted_at TIMESTAMPTZ,
    UNIQUE (user_id, name)
);

CREATE INDEX idx_user_groups_user ON platform_user_groups(user_id);

CREATE TABLE platform_user_group_projects (
    group_id   VARCHAR(26) NOT NULL REFERENCES platform_user_groups(id) ON DELETE CASCADE,
    project_id VARCHAR(26) NOT NULL REFERENCES platform_projects(id) ON DELETE CASCADE,
    PRIMARY KEY (group_id, project_id)
);

-- ---------------------------------------------------------------------------
-- transactional outbox for domain events
-- ---------------------------------------------------------------------------

CREATE TABLE event_outbox (
    id           BIGSERIAL PRIMARY KEY,
    envelope     JSONB       NOT NULL,
    event_type   TEXT        NOT NULL,
    aggregate_id VARCHAR(26) NOT NULL,
    status       TEXT        NOT NULL DEFAULT 'pending'
                 CHECK (status IN ('pending', 'published', 'failed')),
    attempts     INT         NOT NULL DEFAULT 0,
    created_at   TIMESTAMPTZ NOT NULL DEFAULT now(),
    published_at TIMESTAMPTZ
);

CREATE INDEX idx_outbox_pending
    ON event_outbox (created_at)
    WHERE status = 'pending';

-- ---------------------------------------------------------------------------
-- audit logs (append-only)
-- ---------------------------------------------------------------------------

CREATE TABLE audit_logs (
    id           VARCHAR(26) PRIMARY KEY,
    user_id      VARCHAR(26) REFERENCES auth_users(id) ON DELETE SET NULL,
    actor_name   TEXT,
    event_type   TEXT        NOT NULL,
    message      TEXT        NOT NULL,
    project_id   VARCHAR(26),
    metadata     JSONB,
    occurred_at  TIMESTAMPTZ NOT NULL
);

CREATE INDEX idx_audit_logs_user_time
    ON audit_logs (user_id, occurred_at DESC);

CREATE INDEX idx_audit_logs_time
    ON audit_logs (occurred_at DESC);

-- ---------------------------------------------------------------------------
-- recipients (brand end-users)
-- ---------------------------------------------------------------------------

CREATE TABLE platform_recipients (
    id           VARCHAR(26) PRIMARY KEY,
    project_id   VARCHAR(26) NOT NULL REFERENCES platform_projects(id) ON DELETE CASCADE,
    user_id      TEXT        NOT NULL,
    name         TEXT        NOT NULL,
    contacts     JSONB       NOT NULL DEFAULT '{}',
    created_by   VARCHAR(26) REFERENCES auth_users(id) ON DELETE SET NULL,
    created_at   TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at   TIMESTAMPTZ NOT NULL DEFAULT now(),
    deleted_at   TIMESTAMPTZ,
    UNIQUE (project_id, user_id)
);

CREATE INDEX idx_recipients_project ON platform_recipients(project_id);

-- ---------------------------------------------------------------------------
-- message templates
-- ---------------------------------------------------------------------------

CREATE TABLE platform_templates (
    id           VARCHAR(26) PRIMARY KEY,
    project_id   VARCHAR(26) NOT NULL REFERENCES platform_projects(id) ON DELETE CASCADE,
    name         TEXT        NOT NULL,
    description  TEXT,
    channel      TEXT        NOT NULL DEFAULT 'email',
    content      JSONB       NOT NULL DEFAULT '{}',
    version      INT         NOT NULL DEFAULT 1,
    created_by   VARCHAR(26) REFERENCES auth_users(id) ON DELETE SET NULL,
    created_at   TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at   TIMESTAMPTZ NOT NULL DEFAULT now(),
    deleted_at   TIMESTAMPTZ
);

CREATE INDEX idx_templates_project ON platform_templates(project_id);

CREATE TABLE platform_template_attachments (
    id           VARCHAR(26) PRIMARY KEY,
    template_id  VARCHAR(26) NOT NULL REFERENCES platform_templates(id) ON DELETE CASCADE,
    name         TEXT        NOT NULL,
    mime_type    TEXT        NOT NULL,
    size_bytes   BIGINT      NOT NULL DEFAULT 0,
    url          TEXT        NOT NULL,
    created_at   TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX idx_template_attachments_template ON platform_template_attachments(template_id);

-- ---------------------------------------------------------------------------
-- provider configs (per-project credentials)
-- ---------------------------------------------------------------------------

CREATE TABLE platform_project_provider_configs (
    id               TEXT PRIMARY KEY,
    project_id       TEXT NOT NULL REFERENCES platform_projects(id) ON DELETE CASCADE,
    channel_id       TEXT NOT NULL,
    provider_id      TEXT NOT NULL,
    config           JSONB NOT NULL DEFAULT '{}',
    smtp_fallback    JSONB,
    enabled          BOOLEAN NOT NULL DEFAULT TRUE,
    created_at       TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at       TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE (project_id, channel_id, provider_id)
);

CREATE INDEX idx_provider_configs_project ON platform_project_provider_configs(project_id);
CREATE INDEX idx_provider_configs_channel ON platform_project_provider_configs(channel_id);
