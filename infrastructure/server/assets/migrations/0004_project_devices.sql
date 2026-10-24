-- 0004: project devices — per-device destination records (M5).
-- One row per registered device token: the project, the device
-- destination (android | apple | linux | web | window), the vendor whose
-- credentials deliver to it, and that vendor's registration token or
-- endpoint URI. Re-registering the same (project, destination, token)
-- refreshes the row instead of duplicating it (store-level upsert on the
-- unique index below). Revocation is a status flip, never a delete —
-- the row stays as delivery history. recipient_id may stay unset at
-- registration time (the dashboard assigns it later); the FK guards that
-- an assigned recipient exists within the same project.

CREATE TABLE project_devices (
    id            VARCHAR(26) PRIMARY KEY,
    project_id    VARCHAR(26) NOT NULL REFERENCES platform_projects(id) ON DELETE CASCADE,
    recipient_id  VARCHAR(26) REFERENCES platform_recipients(id) ON DELETE SET NULL,
    destination   TEXT NOT NULL,
    vendor        TEXT NOT NULL,
    token         TEXT NOT NULL,
    label         TEXT,
    status        TEXT NOT NULL DEFAULT 'active',
    last_seen_at  TIMESTAMPTZ,
    created_at    TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at    TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE UNIQUE INDEX uq_project_devices_token
    ON project_devices(project_id, destination, token);
CREATE INDEX idx_project_devices_recipient ON project_devices(recipient_id);
