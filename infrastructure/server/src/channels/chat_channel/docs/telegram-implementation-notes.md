# Telegram Channel Implementation Notes

## Architecture
The Telegram channel uses the **Telegram Bot API**.

## Critical Constraint: User Subscription Required
Due to Telegram's strict privacy and anti-spam policies, **bots cannot initiate conversations** with users. 

### Why we implemented it this way:
1.  **Bot API Restrictions:** Telegram bots have no way to "find" a user by phone number or username. A user **must** explicitly start a chat with the bot (`t.me/your_bot_name`) before the bot can send them any messages.
2.  **Privacy:** Telegram protects users from unsolicited automated contact.
3.  **Risk of Banning:** While "Client API" (MTProto) methods exist that allow messaging by phone number, they are intended for human-use only. Automated use (bots acting as users) violates Telegram's Terms of Service and will result in your service being permanently banned.

### Recommended Implementation Flow
To allow users to receive notifications:
1.  **Handshake:** In the dashboard, provide a "Connect Telegram" button linking to `t.me/your_bot_name?start=unique_code`.
2.  **Capture:** When the user clicks "Start" in Telegram, the bot receives the `unique_code` and the user's `chat_id`.
3.  **Storage:** Your backend must match the `unique_code` to the user's account and save their `chat_id` in your database.
4.  **Delivery:** Notifications are delivered by looking up the stored `chat_id` and calling the Telegram `sendMessage` API.
