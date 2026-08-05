# Webhook Channel: Relay Integration

The Relay mode acts as a flexible HTTP forwarder, allowing you to trigger custom webhooks on your own infrastructure.

## How it Works
1. **HTTP Listener**: You run a lightweight HTTP listener on your server.
2. **Configuration**: Configure Notifi with your endpoint details.
3. **Templating**: Define the JSON/XML payload structure using Handlebars.
4. **Trigger**: When Notifi sends a notification, it POSTs the rendered payload to your endpoint.

## Configuration Example
```json
{
  "mode": "relay",
  "target_url": "https://your-app.com/notifi-hook",
  "method": "POST",
  "headers": {
    "Content-Type": "application/json",
    "X-Notifi-Signature": "your-secret-key"
  },
  "payload_template": "{\"title\": \"{{title}}\", \"message\": \"{{body}}\", \"source\": \"{{brand}}\"}"
}
```
Available placeholders: `{{title}}`, `{{body}}`, `{{brand}}`, `{{timestamp}}`.
