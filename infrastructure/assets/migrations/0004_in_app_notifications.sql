-- In-app notifications: personal only (per user), no project scope.
-- origin: 'system' (auto-emitted on events) | 'admin' (manual, admin dashboard — future).
-- content is markdown/HTML rendered by the dashboard detail panel.

CREATE TABLE platform_in_app_notifications (
    id         VARCHAR(26) PRIMARY KEY,
    user_id    VARCHAR(26) NOT NULL REFERENCES auth_users(id) ON DELETE CASCADE,
    type       TEXT NOT NULL,
    origin     TEXT NOT NULL DEFAULT 'system'
               CHECK (origin IN ('system', 'admin')),
    title      TEXT NOT NULL,
    content    TEXT NOT NULL,
    read_at    TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    deleted_at TIMESTAMPTZ
);

CREATE INDEX idx_inapp_notifications_user_time ON platform_in_app_notifications(user_id, created_at DESC);
CREATE INDEX idx_inapp_notifications_unread ON platform_in_app_notifications(user_id) WHERE read_at IS NULL AND deleted_at IS NULL;
