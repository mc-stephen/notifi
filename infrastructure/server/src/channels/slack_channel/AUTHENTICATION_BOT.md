# Slack Bot API Authentication

To enable Slack notifications via the Slack Bot API (more robust):

1. Create a Slack App in your workspace.
2. Add the `chat:write` OAuth Scope to your app.
3. Install the app to your workspace to generate a **Bot User OAuth Token** (starting with `xoxb-`).
4. Identify the **Channel ID** where the bot should post.
5. Use these in your `configs/{brand}/slack/config.json`:
   ```json
   {
     "type": "bot",
     "bot_token": "xoxb-YOUR_TOKEN",
     "channel_id": "YOUR_CHANNEL_ID"
   }
   ```
