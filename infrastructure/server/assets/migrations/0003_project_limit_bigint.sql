-- 0003 — `auth_users.project_limit` was created in 0002 as INT (INT4), but
-- the code reads and binds it as `i64` (INT8). On real Postgres every read
-- refuses to decode — project creation's ceiling check, the shared
-- `project_limit` port read, and the admin console's user view all fail
-- with "Rust type `i64` (as SQL type `INT8`) is not compatible with SQL
-- type `INT4`" — a mismatch the store fakes (and therefore CI) never trip.
-- Follows the repo doctrine: keep `INT8`/`BIGINT` in sync with `i64`.
--
-- 0002 is already applied and stays byte-frozen (append-only rule), so the
-- fix rides in the next number rather than editing the past.

ALTER TABLE auth_users
    ALTER COLUMN project_limit TYPE BIGINT;
