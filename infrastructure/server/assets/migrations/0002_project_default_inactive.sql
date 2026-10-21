-- 0002: projects start inactive by default.
--
-- Doctrine change after the squash landed in 0001: a new project is
-- dormant until its owner turns it on deliberately (topbar switch,
-- Settings → Project state, or the dashboard banner's "Resume
-- delivery"). This only changes the DEFAULT for fresh inserts —
-- owners' existing switches stay as they are. `create_project` and
-- `complete_onboarding` never pass `active`, so they pick this up.
ALTER TABLE platform_projects
    ALTER COLUMN active SET DEFAULT FALSE;
