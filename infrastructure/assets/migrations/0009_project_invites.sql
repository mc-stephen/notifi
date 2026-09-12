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
