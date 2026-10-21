1. The back-end has 4 API surfaces (see root `note.md`): ops at `/`, external product API at `/api/v1/*`, user dashboard backend at `/app/*`, administration at `/admin/*` — each evolves independently.

1. when users tries to send notification, let it be created first before being sent, like for example notification_instance.create().send(), so that way the system can be able to send the notification first in the system as idle then when creat is called it put it in a queue.
