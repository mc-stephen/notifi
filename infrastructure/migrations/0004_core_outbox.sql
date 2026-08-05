-- 0004_core_outbox.sql
-- Transactional outbox for domain events (see notifi_core::event).
--
-- Producers INSERT an envelope in the same transaction as their aggregate
-- write; a dispatcher polls `pending` rows and publishes them to the message
-- bus (pgmq at M2+), then marks them `published`. `attempts` counts retries
-- for crash/serialization failures; poison rows stay `failed` for inspection.

CREATE TABLE event_outbox (
    id           BIGSERIAL PRIMARY KEY,
    envelope     JSONB       NOT NULL,
    event_type   TEXT        NOT NULL,
    aggregate_id VARCHAR(26) NOT NULL,
    status       TEXT        NOT NULL DEFAULT 'pending'
                 CHECK (status IN ('pending', 'published', 'failed')),
    attempts     INT         NOT NULL DEFAULT 0,
    created_at   TIMESTAMPTZ NOT NULL DEFAULT now(),
    published_at TIMESTAMPTZ
);

CREATE INDEX idx_outbox_pending
    ON event_outbox (created_at)
    WHERE status = 'pending';
