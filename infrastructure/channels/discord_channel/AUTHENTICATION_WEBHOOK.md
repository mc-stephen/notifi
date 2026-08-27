# Discord Webhook Authentication

To enable Discord notifications via Webhooks:

1. Open your Discord server settings.
2. Navigate to **Integrations** -> **Webhooks**.
3. Click **New Webhook**.
4. Select the channel and name your webhook.
5. Copy the **Webhook URL**.
6. Use this URL in your `configs/{brand}/discord/config.json`:
   ```json
   {
     "type": "webhook",
     "webhook_url": "YOUR_WEBHOOK_URL"
   }
   ```
