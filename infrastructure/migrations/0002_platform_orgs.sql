-- 0002_platform_orgs.sql
-- Organizations, members, and DB-driven RBAC (roles + permission catalog).
--
-- RBAC model:
--   * auth_permissions        = the catalog of every checkable capability
--                               (e.g. 'project.create'), keyed by code.
--   * auth_roles              = named role per org; org_id NULL means the
--                               role is a system role seeded below and shared
--                               by every org (not deletable per-org).
--   * auth_role_permissions   = which permissions a role grants.
--   * platform_members        = which user holds which role in which org.

CREATE TABLE platform_organizations (
    id         VARCHAR(26) PRIMARY KEY,
    name       TEXT        NOT NULL,
    slug       TEXT        NOT NULL UNIQUE,
    logo_url   TEXT,
    region     TEXT,
    timezone   TEXT,
    plan       TEXT        NOT NULL DEFAULT 'free'
               CHECK (plan IN ('free', 'starter', 'pro', 'enterprise')),
    status     TEXT        NOT NULL DEFAULT 'active'
               CHECK (status IN ('active', 'suspended')),
    created_by VARCHAR(26) REFERENCES auth_users(id) ON DELETE SET NULL,
    version    INT         NOT NULL DEFAULT 1,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    deleted_at TIMESTAMPTZ
);

CREATE TABLE auth_roles (
    id          VARCHAR(26) PRIMARY KEY,
    org_id      VARCHAR(26) REFERENCES platform_organizations(id) ON DELETE CASCADE,
    name        TEXT        NOT NULL,
    description TEXT,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE (org_id, name)
);

CREATE TABLE auth_permissions (
    code        TEXT        PRIMARY KEY,
    description TEXT        NOT NULL,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE auth_role_permissions (
    role_id       VARCHAR(26) NOT NULL REFERENCES auth_roles(id) ON DELETE CASCADE,
    permission_id TEXT        NOT NULL REFERENCES auth_permissions(code) ON DELETE CASCADE,
    PRIMARY KEY (role_id, permission_id)
);

CREATE TABLE platform_members (
    id             VARCHAR(26) PRIMARY KEY,
    org_id         VARCHAR(26) NOT NULL REFERENCES platform_organizations(id) ON DELETE CASCADE,
    user_id        VARCHAR(26) NOT NULL REFERENCES auth_users(id) ON DELETE CASCADE,
    role_id        VARCHAR(26) NOT NULL REFERENCES auth_roles(id),
    status         TEXT        NOT NULL DEFAULT 'active'
                   CHECK (status IN ('invited', 'active', 'suspended')),
    invited_by     VARCHAR(26) REFERENCES auth_users(id) ON DELETE SET NULL,
    joined_at      TIMESTAMPTZ,
    last_active_at TIMESTAMPTZ,
    created_at     TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at     TIMESTAMPTZ NOT NULL DEFAULT now(),
    deleted_at     TIMESTAMPTZ,
    UNIQUE (org_id, user_id)
);

CREATE INDEX idx_members_user ON platform_members(user_id);

CREATE TABLE platform_invitations (
    id          VARCHAR(26) PRIMARY KEY,
    org_id      VARCHAR(26) NOT NULL REFERENCES platform_organizations(id) ON DELETE CASCADE,
    email       TEXT        NOT NULL,
    role_id     VARCHAR(26) NOT NULL REFERENCES auth_roles(id),
    token_hash  TEXT        NOT NULL UNIQUE,
    invited_by  VARCHAR(26) REFERENCES auth_users(id) ON DELETE SET NULL,
    expires_at  TIMESTAMPTZ NOT NULL,
    accepted_at TIMESTAMPTZ,
    revoked_at  TIMESTAMPTZ,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX idx_invitations_org ON platform_invitations(org_id);

-- ---------------------------------------------------------------------------
-- Seed: permission catalog.
-- Codes are grouped by domain; the dashboard and the API read these from the
-- DB at runtime, so adding a capability is an INSERT, not a code change.
-- ---------------------------------------------------------------------------

INSERT INTO auth_permissions (code, description) VALUES
  ('org.read',        'View organization settings'),
  ('org.update',      'Update organization settings'),
  ('org.delete',      'Delete the organization'),
  ('project.create',  'Create projects'),
  ('project.read',    'View projects'),
  ('project.update',  'Update projects'),
  ('project.delete',  'Delete projects'),
  ('environment.read',   'View environments'),
  ('environment.update', 'Create/update environments'),
  ('api_key.read',    'View API keys'),
  ('api_key.create',  'Create API keys'),
  ('api_key.revoke',  'Revoke API keys'),
  ('team.read',       'View team members'),
  ('team.invite',     'Invite team members'),
  ('team.update_role','Change member roles'),
  ('team.remove',     'Remove team members'),
  ('notification.read',   'View notifications'),
  ('notification.send',   'Send notifications'),
  ('template.read',   'View templates'),
  ('template.create', 'Create/update templates'),
  ('billing.read',    'View billing information'),
  ('billing.update',  'Change plan and payment methods'),
  ('analytics.read',  'View analytics');

-- ---------------------------------------------------------------------------
-- Seed: system roles (org_id NULL = shared by every org).
-- Role ids are stable ULIDs; membership references them by id.
-- ---------------------------------------------------------------------------

INSERT INTO auth_roles (id, org_id, name, description) VALUES
  ('01KZ8KAFYKD2RFQB4481AR5EV3', NULL, 'owner',
   'Full control, including deleting the organization'),
  ('01KZ8KAFYKS36VYF9QA86RZS7C', NULL, 'admin',
   'Everything except deleting the organization'),
  ('01KZ8KAFYKPTECPY298XVQYD3A', NULL, 'developer',
   'Builds and ships notifications: projects, keys, templates, sends'),
  ('01KZ8KAFYKFMZA5NVC9QC2C404', NULL, 'viewer',
   'Read-only access to everything in the organization'),
  ('01KZ8KAFYKC8DC5302KAQ0XXAK', NULL, 'billing',
   'Manages plan and payments only');

-- owner: all permissions
INSERT INTO auth_role_permissions (role_id, permission_id)
SELECT '01KZ8KAFYKD2RFQB4481AR5EV3', code FROM auth_permissions;

-- admin: all except org.delete
INSERT INTO auth_role_permissions (role_id, permission_id)
SELECT '01KZ8KAFYKS36VYF9QA86RZS7C', code FROM auth_permissions
WHERE code <> 'org.delete';

-- developer: org read + project/environment/api_key/notification/template/analytics
INSERT INTO auth_role_permissions (role_id, permission_id) VALUES
  ('01KZ8KAFYKPTECPY298XVQYD3A', 'org.read'),
  ('01KZ8KAFYKPTECPY298XVQYD3A', 'project.create'),
  ('01KZ8KAFYKPTECPY298XVQYD3A', 'project.read'),
  ('01KZ8KAFYKPTECPY298XVQYD3A', 'project.update'),
  ('01KZ8KAFYKPTECPY298XVQYD3A', 'environment.read'),
  ('01KZ8KAFYKPTECPY298XVQYD3A', 'environment.update'),
  ('01KZ8KAFYKPTECPY298XVQYD3A', 'api_key.read'),
  ('01KZ8KAFYKPTECPY298XVQYD3A', 'api_key.create'),
  ('01KZ8KAFYKPTECPY298XVQYD3A', 'api_key.revoke'),
  ('01KZ8KAFYKPTECPY298XVQYD3A', 'team.read'),
  ('01KZ8KAFYKPTECPY298XVQYD3A', 'notification.read'),
  ('01KZ8KAFYKPTECPY298XVQYD3A', 'notification.send'),
  ('01KZ8KAFYKPTECPY298XVQYD3A', 'template.read'),
  ('01KZ8KAFYKPTECPY298XVQYD3A', 'template.create'),
  ('01KZ8KAFYKPTECPY298XVQYD3A', 'analytics.read');

-- viewer: read-only
INSERT INTO auth_role_permissions (role_id, permission_id) VALUES
  ('01KZ8KAFYKFMZA5NVC9QC2C404', 'org.read'),
  ('01KZ8KAFYKFMZA5NVC9QC2C404', 'project.read'),
  ('01KZ8KAFYKFMZA5NVC9QC2C404', 'environment.read'),
  ('01KZ8KAFYKFMZA5NVC9QC2C404', 'api_key.read'),
  ('01KZ8KAFYKFMZA5NVC9QC2C404', 'team.read'),
  ('01KZ8KAFYKFMZA5NVC9QC2C404', 'notification.read'),
  ('01KZ8KAFYKFMZA5NVC9QC2C404', 'template.read'),
  ('01KZ8KAFYKFMZA5NVC9QC2C404', 'analytics.read');

-- billing: billing + org read
INSERT INTO auth_role_permissions (role_id, permission_id) VALUES
  ('01KZ8KAFYKC8DC5302KAQ0XXAK', 'org.read'),
  ('01KZ8KAFYKC8DC5302KAQ0XXAK', 'billing.read'),
  ('01KZ8KAFYKC8DC5302KAQ0XXAK', 'billing.update');
