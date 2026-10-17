# Slack Webhook Authentication

To enable Slack notifications via Incoming Webhooks, follow these steps:

1. Create a Slack App in your workspace.
2. Enable **Incoming Webhooks**.
3. Create a new Webhook and select the channel you want to post to.
4. Copy the resulting **Webhook URL**.
5. Use this URL in your `configs/{brand}/slack/config.json`:
   ```json
   {
     "type": "webhook",
     "webhook_url": "YOUR_WEBHOOK_URL"
   }
   ```
