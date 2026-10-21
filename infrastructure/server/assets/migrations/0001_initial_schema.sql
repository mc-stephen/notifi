-- 0001_initial_schema.sql
-- Clean-origin schema for the Notifi server (dev-stage squash of all
-- migrations into a single file — from production onward migrations
-- are append-only, see ARCHITECTURE.md §19).
-- Re-squashed 2026-09-10: folds in admin governance (super-admin +
-- status; the dropped approval-request flow is omitted entirely),
-- audit actor attribution, billing catalog + subscriptions, and user
-- TOTP + project 2FA requirement.
-- Re-squashed 2026-10-12: folds in 0009 (project_invites), 0011
-- (key_ip_allowlist), 0012 (notification_sends + flows, minus the
-- kind column dropped by 0013), 0014 + 0015 (schedules incl. weekly
-- timing + template columns), 0016 (flow_trigger_tokens). 0010 is
-- omitted on purpose (data-only OAuth backfill — nothing to backfill
-- on a fresh database).
-- Re-squashed 2026-10-20: folds in 0002 (environment gate → project
-- `active` flag), 0003 (webhook ownership), and 0004+0005 (campaigns
-- incl. the explicit provider pick). Applied migrations are append-only
-- from here on: new schema changes ride new files, never edits.
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
    -- Traffic switch: one boolean that turns all of the project's
    -- sends on or off (replaces the historic dev/prod environment gate).
    active      BOOLEAN     NOT NULL DEFAULT TRUE,
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
    UNIQUE (project_id, name)
);

CREATE INDEX idx_api_keys_project ON auth_api_keys(project_id);

-- Per-key IP allowlists: only these addresses/CIDRs may use the key.
-- An empty list means unrestricted (enforcement begins with the first entry).
CREATE TABLE key_ip_allowlist (
    id          VARCHAR(26) PRIMARY KEY,
    key_id      VARCHAR(26) NOT NULL REFERENCES auth_api_keys(id) ON DELETE CASCADE,
    cidr        TEXT        NOT NULL,
    label       TEXT        NOT NULL DEFAULT '',
    created_at  TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX idx_ip_allowlist_key ON key_ip_allowlist(key_id);
CREATE UNIQUE INDEX uq_ip_allowlist_key_cidr ON key_ip_allowlist(key_id, cidr);

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

-- Project team invites (GitHub-style): inviting creates a pending
-- row + email; the invitee accepts or declines. Only acceptance
-- creates the membership row.
CREATE TABLE project_invites (
    id          VARCHAR(26) PRIMARY KEY,
    project_id  VARCHAR(26) NOT NULL REFERENCES platform_projects(id) ON DELETE CASCADE,
    email       TEXT        NOT NULL,
    role        TEXT        NOT NULL
                CHECK (role IN ('owner', 'admin', 'developer', 'viewer', 'billing', 'editor')),
    token_hash  TEXT        NOT NULL UNIQUE,
    status      TEXT        NOT NULL DEFAULT 'pending'
                CHECK (status IN ('pending', 'accepted', 'declined')),
    expires_at  TIMESTAMPTZ NOT NULL,
    created_by  VARCHAR(26) REFERENCES auth_users(id) ON DELETE SET NULL,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT now(),
    decided_at  TIMESTAMPTZ
);

CREATE INDEX idx_invites_project ON project_invites(project_id) WHERE status = 'pending';
CREATE INDEX idx_invites_email ON project_invites(email) WHERE status = 'pending';

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
-- product sends + flow definitions (record-first API sends)
-- ---------------------------------------------------------------------------

-- notification_sends: one row per POST /v1/notifications call, and one
-- row per recipient expanded from a campaign fan-out (`campaign_id`
-- set). Flows execute direct sends inline (instant sends); campaigns
-- deliver their rows directly through connected providers; completion
-- is marked on the row.
CREATE TABLE notification_sends (
    id          VARCHAR(26) PRIMARY KEY,
    project_id  VARCHAR(26) NOT NULL REFERENCES platform_projects(id) ON DELETE CASCADE,
    channel     TEXT        NOT NULL,
    recipients  JSONB       NOT NULL DEFAULT '[]',
    title       TEXT        NOT NULL,
    body        TEXT        NOT NULL DEFAULT '',
    send_at     TIMESTAMPTZ NOT NULL DEFAULT now(),
    status      TEXT        NOT NULL DEFAULT 'pending'
                CHECK (status IN ('pending', 'sending', 'completed', 'failed')),
    attempts    INT         NOT NULL DEFAULT 0,
    provider    TEXT,
    external_id TEXT,
    -- Set for campaign fan-out rows; NULL for direct API sends.
    campaign_id VARCHAR(26),
    last_error  TEXT,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at  TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX idx_sends_project ON notification_sends(project_id);
CREATE INDEX idx_sends_campaign ON notification_sends(campaign_id);

-- flows: notification routing graphs. "Default" is an emergent property (a
-- flow carrying a notification.created trigger), not a stored flag.
CREATE TABLE flows (
    id          VARCHAR(26) PRIMARY KEY,
    project_id  VARCHAR(26) NOT NULL REFERENCES platform_projects(id) ON DELETE CASCADE,
    name        TEXT        NOT NULL,
    definition  JSONB       NOT NULL DEFAULT '{}',
    version     INT         NOT NULL DEFAULT 1,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at  TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX idx_flows_project ON flows(project_id);

-- Webhooks: inbound event receivers. Doctrine: an owner (schedule or
-- webhook) creates its flow — a flow can never be assigned to one from
-- the outside. The owner names the flow, seeds its locked trigger node,
-- and deletes it when the owner dies. Both kinds of owned flows stay
-- off the `/flows` list; the only door in is the owner's page.
-- `archived_at` is the non-destructive alternative to deletion: archived
-- owners stop running but keep their flow bound (still openable via the
-- owner page) and can be restored.
CREATE TABLE webhooks (
    id          VARCHAR(26) PRIMARY KEY,
    project_id  VARCHAR(26) NOT NULL REFERENCES platform_projects(id) ON DELETE CASCADE,
    name        TEXT        NOT NULL,
    description TEXT        NOT NULL DEFAULT '',
    -- 1:1: the flow this webhook created and owns. Unique — one flow back
    -- at most one owner. Deleting the flow deletes the webhook row.
    flow_id     VARCHAR(26) NOT NULL UNIQUE REFERENCES flows(id) ON DELETE CASCADE,
    -- Event names this webhook is interested in (display metadata for the
    -- trigger node; inbound firing still keys on the trigger token).
    events      JSONB       NOT NULL DEFAULT '[]',
    active      BOOLEAN     NOT NULL DEFAULT TRUE,
    created_by  VARCHAR(26) NOT NULL REFERENCES auth_users(id),
    archived_at TIMESTAMPTZ,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at  TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX idx_webhooks_project ON webhooks(project_id);
CREATE INDEX idx_webhooks_flow ON webhooks(flow_id);

-- ---------------------------------------------------------------------------
-- flow schedules: run a flow at a time (once), on an interval (recurring),
-- or weekly. A schedule owns its execution payload (channel, title, body,
-- recipients) — the worker needs them to deliver. Template schedules render
-- their template at fire time; legacy rows use the stored payload. Flows
-- referenced by any schedule are hidden from the /flows list (managed via
-- schedules instead).
-- ---------------------------------------------------------------------------

CREATE TABLE schedules (
    id          VARCHAR(26) PRIMARY KEY,
    project_id  VARCHAR(26) NOT NULL REFERENCES platform_projects(id) ON DELETE CASCADE,
    name        TEXT        NOT NULL,
    flow_id     VARCHAR(26) NOT NULL REFERENCES flows(id) ON DELETE CASCADE,
    kind        TEXT        NOT NULL CHECK (kind IN ('once', 'recurring', 'weekly')),
    run_at      TIMESTAMPTZ,
    interval_secs BIGINT,
    days        JSONB,
    run_time    TEXT,
    timezone    TEXT        NOT NULL DEFAULT 'UTC',
    template_id VARCHAR(26) REFERENCES platform_templates(id) ON DELETE SET NULL,
    channel     TEXT        NOT NULL,
    title       TEXT        NOT NULL,
    body        TEXT        NOT NULL DEFAULT '',
    recipients  JSONB       NOT NULL DEFAULT '[]',
    active      BOOLEAN     NOT NULL DEFAULT TRUE,
    next_run_at TIMESTAMPTZ,
    last_run_at TIMESTAMPTZ,
    last_status TEXT        CHECK (last_status IN ('completed', 'failed')),
    last_error  TEXT,
    run_count   INT         NOT NULL DEFAULT 0,
    -- Non-destructive alternative to deletion (see webhooks): archived
    -- schedules stop running but keep their flow bound and can be
    -- restored.
    archived_at TIMESTAMPTZ,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at  TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX idx_schedules_project ON schedules(project_id);
CREATE INDEX idx_schedules_flow ON schedules(flow_id);
CREATE INDEX idx_schedules_due ON schedules(active, next_run_at);
CREATE INDEX idx_schedules_template ON schedules(template_id);

-- Inbound webhook trigger tokens: unguessable per-trigger-node tokens
-- mapping to their flow. Synced from flow definitions on every flow
-- save (the definition is the source of truth); flow deletes cascade.
CREATE TABLE flow_trigger_tokens (
    token      TEXT        PRIMARY KEY,
    flow_id    VARCHAR(26) NOT NULL REFERENCES flows(id) ON DELETE CASCADE,
    node_id    TEXT        NOT NULL,
    project_id VARCHAR(26) NOT NULL REFERENCES platform_projects(id) ON DELETE CASCADE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX idx_trigger_tokens_flow ON flow_trigger_tokens(flow_id);
CREATE INDEX idx_trigger_tokens_project ON flow_trigger_tokens(project_id);

-- ---------------------------------------------------------------------------
-- campaigns: one-shot messages — one channel × one audience × one message
-- × one timing.
-- ---------------------------------------------------------------------------
--
-- Flow-less doctrine: campaigns deliver directly through the project's
-- connected providers for their channel (the Providers page is the only
-- delivery prerequisite; flows stay the door for API sends, schedules,
-- and webhooks). Campaign rows keep the composed copy of the message
-- (template renders are snapshotted at compose time), and fan-out lives
-- in `notification_sends`: one pending row per recipient with
-- `campaign_id` set. Campaign stats derive from those rows — nothing is
-- ever stored twice.
CREATE TABLE campaigns (
    id          VARCHAR(26) PRIMARY KEY,
    project_id  VARCHAR(26) NOT NULL REFERENCES platform_projects(id) ON DELETE CASCADE,
    name        TEXT        NOT NULL,
    description TEXT        NOT NULL DEFAULT '',
    created_by  VARCHAR(26) NOT NULL REFERENCES auth_users(id),
    -- Exactly one channel per campaign: content and delivery semantics
    -- (HTML bodies vs push payloads) do not mix.
    channel     TEXT        NOT NULL,
    -- When composed from a template the rendered content snapshot lives
    -- in title/body; the reference is kept for provenance.
    template_id VARCHAR(26) REFERENCES platform_templates(id) ON DELETE SET NULL,
    title       TEXT        NOT NULL DEFAULT '',
    body        TEXT        NOT NULL DEFAULT '',
    -- Who gets it: 'all' = every recipient with a contact for the
    -- channel (resolved live at fire time); 'custom' = only the
    -- selected recipients (their keys, contacts still resolved live).
    audience_mode     TEXT NOT NULL DEFAULT 'all',
    selected_user_ids JSONB NOT NULL DEFAULT '[]',
    -- Explicit primary provider pick (empty = no explicit pick: the
    -- connected providers for the channel try in store order with
    -- fallback; a named pick is tried first).
    provider_id TEXT        NOT NULL DEFAULT '',
    status      TEXT        NOT NULL DEFAULT 'draft',
    send_at     TIMESTAMPTZ,
    started_at  TIMESTAMPTZ,
    completed_at TIMESTAMPTZ,
    last_error  TEXT,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at  TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX idx_campaigns_project ON campaigns(project_id);
CREATE INDEX idx_campaigns_status ON campaigns(status);

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
