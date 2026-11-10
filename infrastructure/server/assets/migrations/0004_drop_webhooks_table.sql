-- The Webhooks page is gone: its ownership rows are dead weight. Webhook
-- triggers are definition-born now — every webhook trigger node carries
-- its token, the `flow_trigger_tokens` index syncs from definitions on
-- saves, and the builder panel manages each door's secrets + IP rules.
-- The page's rows (one per page-born webhook) drop; their flows and
-- tokens survive untouched.
DROP TABLE IF EXISTS webhooks;
