-- Flow run history: one row per completed execution of a flow (webhook
-- fire, schedule run, event-triggered send). The `steps` JSON trace
-- carries the per-node outcomes (trigger, template render, provider
-- attempts, http steps) the builder's History tab replays. Rows are
-- written once, at run end — no dangling "running" rows — and
-- recording is best-effort: a failed history write never fails the
-- delivery it describes.
CREATE TABLE flow_runs (
  id          TEXT PRIMARY KEY,
  project_id  TEXT NOT NULL,
  flow_id     TEXT NOT NULL,
  mode        TEXT NOT NULL,
  status      TEXT NOT NULL,
  error       TEXT,
  started_at  TIMESTAMPTZ NOT NULL DEFAULT now(),
  finished_at TIMESTAMPTZ,
  send_id     TEXT,
  steps       JSONB NOT NULL DEFAULT '[]'
);
CREATE INDEX idx_flow_runs_flow ON flow_runs(project_id, flow_id, started_at DESC);
