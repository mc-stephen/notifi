-- Backfill email verification for OAuth-created rows.
--
-- Provider-unverified addresses are refused upstream, so every row with an
-- OAuth identity is verified by construction — but `create_user` did not
-- persist `email_verified_at` until this fix, leaving them NULL (and the
-- dashboard banner stuck on). `created_at` preserves an honest timestamp.

UPDATE auth_users
SET email_verified_at = created_at,
    updated_at = now()
WHERE oauth_provider IS NOT NULL
  AND email_verified_at IS NULL;
