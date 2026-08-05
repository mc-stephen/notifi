# Web Channel Authentication (VAPID)

To enable Web Push notifications:

1. **Generate VAPID Keys**: You need a public/private VAPID key pair for secure identification.
   - You can generate them using the `web-push` CLI or OpenSSL.
2. **Config File**: Your `configs/{brand}/web/config.json` should look like this:
   ```json
   {
     "vapid_public_key": "YOUR_PUBLIC_KEY",
     "vapid_private_key": "YOUR_PRIVATE_KEY",
     "contact_email": "admin@yourbrand.com"
   }
   ```
3. **Client-Side Requirements**:
   - Your frontend must request Push Notification permission from the browser.
   - The browser returns a `Subscription` object containing the `endpoint`, `p256dh`, and `auth` keys.
   - Your backend must store these subscription details in your database to send messages later.
