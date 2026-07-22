# macOS Channel Authentication (APNs JWT)

To enable macOS push notifications via Apple Push Notification service (APNs):

1. **Apple Developer Portal**: 
   - Go to **Certificates, Identifiers & Profiles** -> **Keys**.
   - Create a new Key and enable **Apple Push Notifications service (APNs)**.
   - Download the `.p8` key file. Note the **Key ID** and your **Team ID**.
2. **Configuration**:
   - Place the `.p8` key file in `configs/{brand}/macos/AuthKey_{ID}.p8`.
   - Ensure you have your macOS App's **Bundle ID**.
3. **Config File**: Your `configs/{brand}/macos/config.json` should look like this:
   ```json
   {
     "key_id": "YOUR_KEY_ID",
     "team_id": "YOUR_TEAM_ID",
     "bundle_id": "com.yourcompany.app",
     "p8_key_path": "configs/{brand}/macos/AuthKey_{ID}.p8"
   }
   ```
