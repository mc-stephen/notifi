-- 0003: project deletion lifecycle. Requests set deleted_at + purge_after;
-- the worker purges (bucket folder, then rows — cascades take children)
-- when purge_after has passed. project_exports records every data-export
-- download for the per-project daily rate limit (and later, the audit).

ALTER TABLE platform_projects
    ADD COLUMN purge_after TIMESTAMPTZ;

CREATE TABLE project_exports (
    id           VARCHAR(26) PRIMARY KEY,
    project_id   VARCHAR(26) NOT NULL REFERENCES platform_projects(id) ON DELETE CASCADE,
    requested_by VARCHAR(26) REFERENCES auth_users(id) ON DELETE SET NULL,
    created_at   TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX idx_project_exports_project ON project_exports(project_id, created_at);
