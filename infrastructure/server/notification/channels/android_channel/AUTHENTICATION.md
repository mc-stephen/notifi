# Android Channel Authentication (FCM v1)

To enable Android notifications via Firebase Cloud Messaging (FCM):

1.  **Firebase Console**: Create a project and add an Android app.
2.  **Service Account**:
    *   Go to **Project Settings** -> **Service Accounts**.
    *   Click **Generate new private key** to download the `service-account.json` file.
3.  **Configuration**:
    *   Place the downloaded `service-account.json` in `configs/{brand}/fcm/service-account.json`.
4.  **Config File**: Your `configs/{brand}/android/config.json` should point to this location:
    ```json
    {
      "service_account_path": "configs/{brand}/fcm/service-account.json"
    }
    ```
