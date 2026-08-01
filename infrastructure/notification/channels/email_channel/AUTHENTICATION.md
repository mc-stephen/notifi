# Email Channel Authentication

To enable Email notifications via SMTP, you need to configure your SMTP provider credentials.

## Requirements
- SMTP Host
- SMTP Port
- Username
- Password

## How to get your Credentials
1. **Choose your provider** (e.g., SendGrid, AWS SES, Gmail, Outlook).
2. **Retrieve settings** from your provider's dashboard:
   - **Host:** (e.g., `smtp.sendgrid.net`)
   - **Port:** (Usually `587` for STARTTLS or `465` for SSL/TLS)
   - **Username:** (Often an API key or email address)
   - **Password:** (If using Gmail, generate an "App Password"; do not use your primary login password).
3. Use these credentials in your configuration file (`configs/{brand}/email/config.json`).
