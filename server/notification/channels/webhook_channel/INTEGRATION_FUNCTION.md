# Webhook Channel: Function Trigger Integration

The Function mode allows you to trigger custom code hosted on the Notifi platform.

## How it Works
1. **Code Upload**: You upload a single-file script to your Notifi account.
2. **Review**: Notifi reviews the code snippet.
3. **Execution**: When Notifi sends a notification, it triggers the registered function instance by its ID.

## Configuration Example
```json
{
  "mode": "function",
  "function_id": "custom-func-12345"
}
```
*Note: This feature requires Function hosting enabled on your Notifi account.*
