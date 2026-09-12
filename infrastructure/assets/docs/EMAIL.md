# System email

Transactional platform mail (account verification, password resets) sends
through SMTP configured **by environment only** — never in code, never in
git. Local development uses a fake inbox; production swaps the same keys.

## Development (Ethereal fake inbox)

1. Create a free account at [ethereal.email](https://ethereal.email) (no
   real delivery; messages land in its web inbox).
2. Put the credentials in gitignored `infrastructure/.env` (names only —
   values never enter the repo):

   ```sh
   NOTIFI_SMTP_HOST=smtp.ethereal.email
   NOTIFI_SMTP_PORT=587
   NOTIFI_SMTP_USERNAME=<ethereal username>
   NOTIFI_SMTP_PASSWORD=<ethereal password>
   NOTIFI_MAIL_DOMAIN=notifi.dev
   NOTIFI_SMTP_TLS=true
   ```

3. Restart the server (`system mail enabled` in the boot log confirms).
4. Sign up / request a reset / sign in / invite a teammate → the message
   appears in the Ethereal inbox (log in at ethereal.email/messages; the
   server log carries the SMTP reference per send).

Without `NOTIFI_SMTP_HOST`, flows keep their dev behavior (raw tokens in
responses/logs) — nothing breaks.

## Production swap

Same keys, live values (e.g. SendGrid/AWS SES SMTP relay or any
STARTTLS/465 relay), `NOTIFI_MAIL_DOMAIN` set to the verified sender
domain, `NOTIFI_EXPOSE_DEV_TOKENS` off. No code or template changes needed.

## Senders

Who mail is sent as is code-owned (`SystemSender` in
`src/ports/mailer.rs`), never a free-form env string — automated mail
can never accidentally wear a human address:

| Identity | Address | Used for |
|---|---|---|
| `NoReply` | `no-reply@{domain}` | all automated mail today: verification, password resets (user + admin), password-changed confirmations, login notices, team invites |
| `Support` | `support@{domain}` | human mail from the admin dashboard (account managers emailing users) — reserved, no sender uses it yet |
| `Custom` | admin-configured | future admin-dashboard custom senders (stored + managed there); validated as a bare email |

Only the domain comes from config (`NOTIFI_MAIL_DOMAIN`, default
`notifi.dev`). To add a new identity, extend the enum — call sites pass
it explicitly per send, so the mapping stays visible.

## Templates

Single reader: `src/infra/system_templates.rs` (dir defaults to
`assets/templates/`, override with `NOTIFI_TEMPLATES_DIR`). Email-only:
per template `<name>`, `<name>.subject.txt` (subject line) plus
`<name>.html` (the body) with `{{var}}` substitution — no `.txt`
bodies; the plaintext part is auto-derived by stripping tags, so
multipart sending keeps working with HTML as the single source.
Missing files fall back to built-ins so mail still flows in
development. To move or rename templates, only that module changes.
Current templates: `verify-email`, `password-reset`, `password-changed`
(sent after every completed reset — compromise signal when the reset
wasn't yours), `login-notice`
(sent on every password/OAuth sign-in), `member-invite` (project team
invites; vars `name`/`project`/`role`/`link`, 7-day accept link to
`/invite/{token}` on the dashboard).

## Architecture notes

- `email_channel` owns SMTP transport (lettre); `tls=true` selects a
  STARTTLS upgrade (port-587 style), `false` stays plaintext for local
  relays. Only the `smtp` provider sends directly so far.
- Domain code depends on the `SmtpMailer` / `SystemTemplates` ports, with
  `FakeMailer` / `FakeTemplates` in `src/testing.rs` keeping CI hermetic.
- Delivery failures are logged, never fatal to the flow that triggered
  them (signup still succeeds; tokens still issue).
