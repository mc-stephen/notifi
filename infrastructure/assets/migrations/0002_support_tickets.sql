-- Support tickets: personal (project_id NULL) or project-scoped.
-- Personal tickets are visible only to the creator.
-- Project tickets are visible to all active project members.

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
