-- 0001_initial_schema.sql
-- Clean-origin schema for the Notifi server (dev-stage squash of all
-- migrations into a single file — from production onward migrations
-- are append-only, see ARCHITECTURE.md §19).
-- Re-squashed 2026-09-10: folds in admin governance (super-admin +
-- status; the dropped approval-request flow is omitted entirely),
-- audit actor attribution, billing catalog + subscriptions, and user
-- TOTP + project 2FA requirement.
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
    totp_secret       TEXT, -- base32 TOTP secret; set once 2FA setup starts
    totp_enabled      BOOLEAN NOT NULL DEFAULT FALSE,
    status            TEXT        NOT NULL DEFAULT 'active'
                      CHECK (status IN ('active', 'suspended')),
    last_login_at     TIMESTAMPTZ,
    created_at        TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at        TIMESTAMPTZ NOT NULL DEFAULT now(),
    deleted_at        TIMESTAMPTZ,
    UNIQUE (oauth_provider, oauth_subject)
);

CREATE INDEX idx_auth_users_status ON auth_users(status) WHERE deleted_at IS NULL;

-- Platform administrators: separate from customers (`auth_users`). Admins
-- manage the Notifi platform itself; 2FA (TOTP) is enforced for all admins.
CREATE TABLE admin_users (
    id            VARCHAR(26) PRIMARY KEY,
    name          TEXT        NOT NULL,
    email         TEXT        NOT NULL UNIQUE,
    password_hash TEXT        NOT NULL,
    totp_secret   TEXT,
    totp_enabled  BOOLEAN     NOT NULL DEFAULT FALSE,
    is_super_admin BOOLEAN    NOT NULL DEFAULT FALSE,
    status        TEXT        NOT NULL DEFAULT 'active'
                  CHECK (status IN ('pending', 'active', 'suspended')),
    last_login_at TIMESTAMPTZ,
    created_at    TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at    TIMESTAMPTZ NOT NULL DEFAULT now(),
    deleted_at    TIMESTAMPTZ
);

CREATE INDEX idx_admin_users_email ON admin_users(email) WHERE deleted_at IS NULL;

-- Admin sign-in sessions (admin dashboard cookie `admin_session`; only the
-- hash is stored server-side).
CREATE TABLE admin_sessions (
    id         VARCHAR(26) PRIMARY KEY,
    admin_id   VARCHAR(26) NOT NULL REFERENCES admin_users(id) ON DELETE CASCADE,
    token_hash TEXT        NOT NULL UNIQUE,
    expires_at TIMESTAMPTZ NOT NULL,
    revoked_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX idx_admin_sessions_admin ON admin_sessions(admin_id);

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
    require_2fa BOOLEAN     NOT NULL DEFAULT FALSE,
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
               -- Team roles follow the dashboard union; 'editor' predates it.
               CHECK (role IN ('owner', 'admin', 'developer', 'viewer', 'billing', 'editor')),
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
    actor_type   TEXT        NOT NULL DEFAULT 'user'
                 CHECK (actor_type IN ('user', 'admin', 'system')),
    admin_id     VARCHAR(26) NULL REFERENCES admin_users(id) ON DELETE SET NULL,
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

CREATE INDEX idx_audit_logs_actor_time
    ON audit_logs (actor_type, occurred_at DESC);

CREATE INDEX idx_audit_logs_admin_time
    ON audit_logs (admin_id, occurred_at DESC)
    WHERE admin_id IS NOT NULL;

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

-- ---------------------------------------------------------------------------
-- support tickets
-- ---------------------------------------------------------------------------

-- Personal (project_id NULL, visible only to creator) or project-scoped
-- (visible to all active project members).
CREATE TABLE platform_support_tickets (
    id          VARCHAR(26) PRIMARY KEY,
    project_id  VARCHAR(26) REFERENCES platform_projects(id) ON DELETE CASCADE,
    created_by  VARCHAR(26) NOT NULL REFERENCES auth_users(id) ON DELETE CASCADE,
    subject     TEXT NOT NULL,
    category    TEXT NOT NULL,
    priority    TEXT NOT NULL,
    description TEXT NOT NULL,
    status      TEXT NOT NULL DEFAULT 'open',
    created_at  TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at  TIMESTAMPTZ NOT NULL DEFAULT now(),
    deleted_at  TIMESTAMPTZ
);

CREATE INDEX idx_support_tickets_creator ON platform_support_tickets(created_by);
CREATE INDEX idx_support_tickets_project ON platform_support_tickets(project_id);

-- Ticket conversation thread. Each reply is a row.
-- author_type: 'customer' (dashboard user) | 'support' (admin dashboard).
CREATE TABLE platform_support_ticket_messages (
    id          VARCHAR(26) PRIMARY KEY,
    ticket_id   VARCHAR(26) NOT NULL REFERENCES platform_support_tickets(id) ON DELETE CASCADE,
    author_type TEXT NOT NULL,
    author_id   VARCHAR(26),
    body        TEXT NOT NULL,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX idx_ticket_messages_ticket ON platform_support_ticket_messages(ticket_id);

-- ---------------------------------------------------------------------------
-- admin notification broadcasts (must precede in-app: FK target)
-- ---------------------------------------------------------------------------

-- A broadcast is the admin-side record of one compose action; recipient rows
-- in `platform_in_app_notifications` link back via `broadcast_id` and are
-- only materialized when the broadcast is sent — scheduled broadcasts stay
-- invisible to users until the worker sends them.
CREATE TABLE platform_notification_broadcasts (
    id              VARCHAR(26) PRIMARY KEY,
    admin_id        VARCHAR(26) NOT NULL REFERENCES admin_users(id) ON DELETE CASCADE,
    admin_email     TEXT        NOT NULL,
    type            TEXT        NOT NULL,
    title           TEXT        NOT NULL,
    content         TEXT        NOT NULL,
    audience        JSONB       NOT NULL DEFAULT '{}',
    channels        TEXT[]      NOT NULL DEFAULT '{in_app}',
    status          TEXT        NOT NULL DEFAULT 'sent'
                    CHECK (status IN ('scheduled', 'sending', 'sent', 'cancelled')),
    scheduled_for   TIMESTAMPTZ,
    sent_at         TIMESTAMPTZ,
    recipient_count BIGINT      NOT NULL DEFAULT 0,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT scheduled_needs_time CHECK (
        status <> 'scheduled' OR scheduled_for IS NOT NULL
    )
);

CREATE INDEX idx_broadcasts_admin_time
    ON platform_notification_broadcasts(admin_id, created_at DESC);
CREATE INDEX idx_broadcasts_due
    ON platform_notification_broadcasts(scheduled_for)
    WHERE status = 'scheduled';

-- ---------------------------------------------------------------------------
-- in-app notifications
-- ---------------------------------------------------------------------------

-- Personal only (per user), no project scope.
-- origin: 'system' (auto-emitted on events) | 'admin' (manual, admin dashboard).
-- content is markdown/HTML rendered by the dashboard detail panel.
-- broadcast_id links rows materialized by an admin broadcast (NULL otherwise).
CREATE TABLE platform_in_app_notifications (
    id           VARCHAR(26) PRIMARY KEY,
    user_id      VARCHAR(26) NOT NULL REFERENCES auth_users(id) ON DELETE CASCADE,
    type         TEXT NOT NULL,
    origin       TEXT NOT NULL DEFAULT 'system'
                 CHECK (origin IN ('system', 'admin')),
    title        TEXT NOT NULL,
    content      TEXT NOT NULL,
    read_at      TIMESTAMPTZ,
    broadcast_id VARCHAR(26) NULL REFERENCES platform_notification_broadcasts(id) ON DELETE SET NULL,
    created_at   TIMESTAMPTZ NOT NULL DEFAULT now(),
    deleted_at   TIMESTAMPTZ
);

CREATE INDEX idx_inapp_notifications_user_time ON platform_in_app_notifications(user_id, created_at DESC);
CREATE INDEX idx_inapp_notifications_unread ON platform_in_app_notifications(user_id) WHERE read_at IS NULL AND deleted_at IS NULL;
CREATE INDEX idx_inapp_broadcast ON platform_in_app_notifications(broadcast_id);

-- ---------------------------------------------------------------------------
-- billing: plan catalog + per-project subscriptions
-- ---------------------------------------------------------------------------

-- Billing is per project, not per account. Subscriptions are created
-- lazily: the first read for a project without a row inserts the free
-- plan, so new projects land on free by default.
CREATE TABLE billing_plans (
    id          VARCHAR(64) PRIMARY KEY,
    name        TEXT        NOT NULL UNIQUE,
    price_cents BIGINT CHECK (price_cents IS NULL OR price_cents >= 0),
    currency    TEXT        NOT NULL DEFAULT 'USD',
    interval    TEXT        NOT NULL DEFAULT 'month'
                CHECK (interval IN ('month', 'year', 'custom')),
    capabilities JSONB      NOT NULL DEFAULT '{}',
    -- NULL = monthly only; otherwise {"kind": "percent"|"fixed", "value": N}
    -- where percent is (0,100) and fixed is cents off the 12-month total.
    yearly_discount JSONB,
    is_active   BOOLEAN     NOT NULL DEFAULT TRUE,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at  TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE project_subscriptions (
    id                   VARCHAR(26) PRIMARY KEY,
    project_id           VARCHAR(26) NOT NULL REFERENCES platform_projects(id) ON DELETE CASCADE,
    plan_id              VARCHAR(64) NOT NULL REFERENCES billing_plans(id),
    status               TEXT        NOT NULL DEFAULT 'active'
                         CHECK (status IN ('active', 'past_due', 'cancelled')),
    billing_cycle        TEXT        NOT NULL DEFAULT 'monthly'
                         CHECK (billing_cycle IN ('monthly', 'yearly')),
    -- Paid plan ends at period end, then reverts to free.
    cancel_at_period_end BOOLEAN     NOT NULL DEFAULT FALSE,
    current_period_start TIMESTAMPTZ NOT NULL DEFAULT now(),
    current_period_end   TIMESTAMPTZ NOT NULL DEFAULT now() + INTERVAL '30 days',
    created_at           TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at           TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE (project_id)
);

CREATE INDEX idx_subscriptions_plan ON project_subscriptions(plan_id);

-- Seed catalog mirrors the admin console defaults.
INSERT INTO billing_plans (id, name, price_cents, currency, interval, capabilities, yearly_discount, is_active) VALUES
    ('free', 'Free', 0, 'USD', 'month',
     '{"notificationsPerMonth": 1000, "channels": ["email"], "teamMembers": 1, "retentionDays": 7, "support": "Community", "branding": false, "apiCalls": 10000}',
     NULL, TRUE),
    ('starter', 'Starter', 1900, 'USD', 'month',
     '{"notificationsPerMonth": 10000, "channels": ["email", "sms", "push"], "teamMembers": 3, "retentionDays": 30, "support": "Email", "branding": false, "apiCalls": 100000}',
     '{"kind": "percent", "value": 10}', TRUE),
    ('pro', 'Pro', 9900, 'USD', 'month',
     '{"notificationsPerMonth": 100000, "channels": ["all"], "teamMembers": 10, "retentionDays": 90, "support": "Priority", "branding": true, "apiCalls": null}',
     '{"kind": "percent", "value": 20}', TRUE),
    ('enterprise', 'Enterprise', NULL, 'USD', 'custom',
     '{"notificationsPerMonth": null, "channels": ["all"], "teamMembers": null, "retentionDays": null, "support": "Dedicated", "branding": true, "apiCalls": null}',
     NULL, TRUE);
