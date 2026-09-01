-- 0001_initial_schema.sql
-- Clean-origin schema for the Notifi server (dev-stage squash of the
-- project's earliest migration history — from production onward migrations
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

-- The user account IS the workspace: projects are owned by their creator
-- and shared via per-project membership (platform_project_members).
-- `environment` is the project-level gate: development → only dev-scoped
-- credentials work; production → dev and prod credentials both work.
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

-- API keys: the raw key is shown once at creation and never stored. The
-- server stores the key's public `prefix` (for identification in listings
-- and logs) plus `key_hash` (SHA-256 of the full key). `permissions` is a
-- subset of the permission catalog; `scopes` restricts the key to specific
-- channels (e.g. {'email','sms'}) or entities (e.g. {'project:abc'}).
--
-- Environment model (Stripe-style test/live):
--   * every env-scoped resource carries its own `environment` column
--     (`development` | `production`) — dev and prod rows coexist;
--   * `platform_projects.environment` gates which environments are allowed
--     right now: development → only dev-scoped credentials work (prod keys
--     get a "switch to live" error); production → both work.
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
    rate_limit     INT, -- requests per minute; NULL = plan default
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

-- User-level "folders" for organizing projects in the UI. No auth/billing
-- semantics — purely a grouping convenience.
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

-- Producers INSERT an envelope in the same transaction as their aggregate
-- write; a dispatcher polls `pending` rows and publishes them to the message
-- bus (pgmq at M2+), then marks them `published`. `attempts` counts retries
-- for crash/serialization failures; poison rows stay `failed` for inspection.

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
