# Windows Channel Authentication (WNS)

To enable native Windows notifications via WNS:

1. **Microsoft Developer Portal**: Register your application to get the **Client ID** and **Client Secret**.
2. **Configuration**:
   - Update your `configs/{brand}/window/config.json`:
   ```json
   {
     "type": "wns",
     "client_id": "YOUR_CLIENT_ID",
     "client_secret": "YOUR_CLIENT_SECRET",
     "package_sid": "YOUR_PACKAGE_SID"
   }
   ```
3. **Application**: The Windows app must request a unique **WNS Channel URI** from Windows and send it to your backend to be stored.
