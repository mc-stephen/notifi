# Linux Channel Architecture: Generic Push-to-Endpoint

Linux does not provide a unified, vendor-managed push notification service (like Apple's APNs or Google's FCM). Therefore, for remote Linux applications, Notifi uses a **Generic Push-to-Endpoint** approach.

## How it Works
1.  **End-User Application as Listener**: The end-user application (the Linux app installed on the user's machine) must implement a lightweight HTTP listener (e.g., using a web framework like `axum` or `actix-web`) to act as a local agent.
2.  **Registration**: When the user installs or runs the Linux app, it registers its HTTP endpoint URL with your system.
3.  **Server-Side Delivery**: When your backend needs to send a notification to a specific user device, the `LinuxProvider` sends a POST request containing the notification payload (title, body) to the registered `endpoint` URL.
4.  **Local Display**: The local listener on the Linux machine receives the POST request and triggers a native notification (e.g., using `notify-send` or the desktop environment's D-Bus API).

## Benefits
- **Platform Agnostic**: Notifi doesn't need to know the specific Linux desktop environment (GNOME, KDE, XFCE).
- **Flexibility**: Brands can build any local agent they want, provided it can listen for an HTTP POST request.
- **Secure**: Brands can secure their local agent using standard HTTP headers (e.g., `Authorization`) which Notifi can be configured to pass in the request.
