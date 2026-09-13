-- Per-key IP allowlists: only these addresses/CIDRs may use the key.
-- An empty list means unrestricted (enforcement begins with the first entry).

CREATE TABLE key_ip_allowlist (
    id          VARCHAR(26) PRIMARY KEY,
    key_id      VARCHAR(26) NOT NULL REFERENCES auth_api_keys(id) ON DELETE CASCADE,
    cidr        TEXT        NOT NULL,
    label       TEXT        NOT NULL DEFAULT '',
    created_at  TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX idx_ip_allowlist_key ON key_ip_allowlist(key_id);
CREATE UNIQUE INDEX uq_ip_allowlist_key_cidr ON key_ip_allowlist(key_id, cidr);
