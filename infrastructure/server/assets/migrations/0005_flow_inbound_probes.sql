-- The builder's webhook trigger "Incoming data" tab: the last ping an
-- authorized caller left at the door. One row per trigger token — the
-- most recent fire's request details (caller IP, whitelisted headers,
-- capped body). Captured best-effort after the door's checks pass
-- (recording failures never fail the fire); pings that fail the door's
-- checks (401/403) never reach the flow, so they are never probed.
CREATE TABLE flow_inbound_probes (
  token          TEXT PRIMARY KEY,
  flow_id        TEXT NOT NULL,
  project_id     TEXT NOT NULL,
  client_ip      TEXT,
  content_type   TEXT,
  user_agent     TEXT,
  forwarded_for  TEXT,
  body           TEXT,
  fired_at       TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE INDEX idx_inbound_probes_flow ON flow_inbound_probes(flow_id, project_id);
