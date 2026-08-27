# WhatsApp Channel Authentication

To enable WhatsApp notifications via the WhatsApp Business Platform:

1.  Create a **Meta Developer Account** and create a new App.
2.  Add the **WhatsApp product** to your App.
3.  Verify your business in the Business Manager.
4.  Generate a **Permanent Access Token** in your App dashboard.
5.  Retrieve the **Phone Number ID** (available in the WhatsApp -> API Setup section).
6.  Submit and get your **Message Templates** approved in the WhatsApp Manager.
7.  Use these credentials in your `configs/{brand}/whatsapp/config.json`:
    ```json
    {
      "access_token": "YOUR_PERMANENT_ACCESS_TOKEN",
      "phone_number_id": "YOUR_PHONE_NUMBER_ID"
    }
    ```
