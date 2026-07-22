# Windows Channel: Webhook Authentication

For non-native Windows applications, use the generic Webhook approach.

1. **Endpoint**: Your Windows app must implement a local HTTP listener (e.g., using a simple C# or C++ library) that receives POST requests.
2. **Config File**: Your `configs/{brand}/window/config.json`:
   ```json
   {
     "type": "webhook"
   }
   ```
3. **Usage**: When calling `WindowProvider::send_notification`, provide the local listener's URL in the `target` field.
