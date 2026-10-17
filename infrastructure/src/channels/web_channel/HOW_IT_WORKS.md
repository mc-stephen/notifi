# Web Push Channel Implementation: How it Works

Web Push notifications rely on the **VAPID** protocol and the browser's **Service Worker API**. Your server does not need to know the specific browser implementation (Chrome, Firefox, etc.)—it only needs to know where to send the message.

## The Subscription Object
Every browser installation on every device generates a unique `Subscription` object when a user grants notification permission. This object must be sent to your backend and stored in your database. It contains:

1.  **`endpoint`**: A unique URL provided by the browser vendor's push service (e.g., Google's FCM endpoint for Chrome, Mozilla's push service for Firefox).
2.  **`keys.p256dh`**: An encryption key used to ensure only the browser can decrypt your message.
3.  **`keys.auth`**: A secret key used to verify the request.

## Handling Multi-Device/Multi-Browser
Your system treats every browser/device combination as a separate, independent endpoint.

1.  **Frontend Registration**: 
    - When a user visits your site on a new device or browser, the browser triggers the Service Worker registration.
    - Your frontend receives the unique `Subscription` object for that specific browser.
    - Your frontend sends this object to your backend.
2.  **Server-Side Storage**: 
    - Your database stores all subscriptions linked to a specific User ID (e.g., a user might have one subscription for Chrome on PC and another for Firefox on Laptop).
3.  **Server-Side Delivery**: 
    - When you need to send a notification to a user, your backend queries the database for **all** stored subscriptions associated with that user.
    - Your `WebProvider` iterates through the list and sends the notification to **each endpoint URL individually**.

The browser vendor's push service (the `endpoint`) receives the request, verifies the VAPID signature, decrypts the message using the keys, and delivers it to the specific browser instance.
