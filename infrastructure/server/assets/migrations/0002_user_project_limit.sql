-- 0002 — per-user project limit (§24 team doctrine).
--
-- Every user may own at most `project_limit` projects; 5 is the default
-- for all users and an admin raises it per account from the admin
-- console (`PATCH /admin/users/{id}/project-limit`). The count keeps
-- tombstones (soft-deleted, pre-purge) occupying the slot, so the cap
-- cannot be dodged by deleting and remaking. Appended on
-- 2026-11-07 — dev stage, trivial to reformat later.

ALTER TABLE auth_users
    ADD COLUMN project_limit INT NOT NULL DEFAULT 5;
