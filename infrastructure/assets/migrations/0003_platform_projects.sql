-- 0003_platform_projects.sql
-- Projects (a tenant's send destination set), environments, and API keys.
--
-- API keys: the raw key is shown once at creation and never stored. The
-- server stores the key's public `prefix` (for identification in listings
-- and logs) plus `key_hash` (SHA-256 of the full key). `permissions` is a
-- subset of the auth_permissions catalog; `scopes` restricts the key to
-- specific channels (e.g. {'email','sms'}) or entities (e.g. {'project:abc'}).

CREATE TABLE platform_projects (
    id          VARCHAR(26) PRIMARY KEY,
    org_id      VARCHAR(26) NOT NULL REFERENCES platform_organizations(id) ON DELETE CASCADE,
    name        TEXT        NOT NULL,
    slug        TEXT        NOT NULL,
    description TEXT,
    created_by  VARCHAR(26) REFERENCES auth_users(id) ON DELETE SET NULL,
    version     INT         NOT NULL DEFAULT 1,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at  TIMESTAMPTZ NOT NULL DEFAULT now(),
    deleted_at  TIMESTAMPTZ,
    UNIQUE (org_id, slug)
);

CREATE INDEX idx_projects_org ON platform_projects(org_id);

CREATE TABLE platform_environments (
    id         VARCHAR(26) PRIMARY KEY,
    project_id VARCHAR(26) NOT NULL REFERENCES platform_projects(id) ON DELETE CASCADE,
    name       TEXT        NOT NULL
               CHECK (name IN ('development', 'staging', 'production')),
    slug       TEXT        NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    deleted_at TIMESTAMPTZ,
    UNIQUE (project_id, slug)
);

CREATE INDEX idx_environments_project ON platform_environments(project_id);

CREATE TABLE auth_api_keys (
    id             VARCHAR(26) PRIMARY KEY,
    project_id     VARCHAR(26) NOT NULL REFERENCES platform_projects(id) ON DELETE CASCADE,
    environment_id VARCHAR(26) REFERENCES platform_environments(id) ON DELETE SET NULL,
    name           TEXT        NOT NULL,
    prefix         TEXT        NOT NULL,
    key_hash       TEXT        NOT NULL UNIQUE,
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
    deleted_at     TIMESTAMPTZ
);

CREATE INDEX idx_api_keys_project ON auth_api_keys(project_id);
CREATE INDEX idx_api_keys_env     ON auth_api_keys(environment_id);
