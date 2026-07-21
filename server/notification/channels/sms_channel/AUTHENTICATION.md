# SMS Channel Authentication

This channel supports multiple providers to give you flexibility. Configure your choice in `configs/{brand}/sms/config.json`.

## 1. Twilio
- **Config**:
  ```json
  {
    "provider": "twilio",
    "account_sid": "YOUR_ACCOUNT_SID",
    "auth_token": "YOUR_AUTH_TOKEN",
    "from_number": "+1234567890"
  }
  ```

## 2. Local Nigeria Provider
- **Config**:
  ```json
  {
    "provider": "local_nigeria",
    "api_key": "YOUR_API_KEY",
    "base_url": "https://api.yourprovider.ng",
    "sender_id": "YourBrand"
  }
  ```
