# Linux Channel Architecture: Hosted Notifi Gateway

Linux does not provide a unified, vendor-managed push notification service
(like Apple's APNs or Google's FCM). So instead of asking every brand to run
their own push infrastructure, Notifi hosts the listening path itself: the
**Notifi Gateway**.

## How it Works

1.  **Route recipe**: From the dashboard's provider page, the user connects
    the **Notifi Gateway** card — they pick a route topic, Notifi generates
    the secret key, and they choose a delivery mode (`polling` = long-poll
    holds the request for instant landings; `https` = the app checks on its
    own cadence).
2.  **Server-side delivery**: When a send targets the route, the
    `GatewayAdapter` queues the message into the gateway queue (see
    `ports::gateway` and the `gateway_messages` table).
3.  **App-side fetch**: The user's Linux app polls
    `GET /gateway/linux/{topic}?key=...` — with `?wait=N` it long-polls (the
    reply lands the moment something queues). Reads are poll-and-delete: a
    message leaves its route at fetch time.
4.  **Local display**: The app turns the message (title/body) into a native
    notification (`notify-send` or the desktop environment's D-Bus API).

## Benefits

- **No listener to run**: The user's app is a client, not a server — no
  inbound ports to expose, no registration URLs to share.
- **Platform agnostic**: Any Linux app that can make an HTTPS GET can
  receive its route's messages.
- **Outage friendly**: Queued messages wait in the route until the app's
  next poll catches up (messages are read away at fetch time).

The gateway's poll surface is documented beside its code:
`src/api/routes/gateway.rs` (route) and `src/ports/gateway.rs` (queue
contract), proven by `tests/gateway_api.rs`.
