-- Ticket conversation thread. Each reply is a row.
-- author_type: 'customer' (dashboard user) | 'support' (admin dashboard, future).

CREATE TABLE platform_support_ticket_messages (
    id          VARCHAR(26) PRIMARY KEY,
    ticket_id   VARCHAR(26) NOT NULL REFERENCES platform_support_tickets(id) ON DELETE CASCADE,
    author_type TEXT NOT NULL,
    author_id   VARCHAR(26),
    body        TEXT NOT NULL,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX idx_ticket_messages_ticket ON platform_support_ticket_messages(ticket_id);
