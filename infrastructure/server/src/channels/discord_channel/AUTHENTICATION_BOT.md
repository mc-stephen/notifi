# Discord Bot API Authentication

To enable Discord notifications via the Bot API:

1. Create an Application in the [Discord Developer Portal](https://discord.com/developers/applications).
2. Go to the **Bot** tab to retrieve your **Bot Token**.
3. Under **Privileged Gateway Intents**, ensure *Message Content Intent* is enabled if needed.
4. Invite the bot to your server using the OAuth2 URL generator (scopes: `bot`, `send_messages`).
5. Retrieve the **Channel ID** where the bot should post (enable Developer Mode in Discord, right-click the channel, "Copy Channel ID").
6. Use these in your `configs/{brand}/discord/config.json`:
   ```json
   {
     "type": "bot",
     "bot_token": "YOUR_BOT_TOKEN",
     "channel_id": "YOUR_CHANNEL_ID"
   }
   ```
