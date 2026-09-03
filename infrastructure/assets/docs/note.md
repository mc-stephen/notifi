1. The back-end is to have 2 main api route, and to each it own, so when i change that of End-users api logic it should not affect that of my project api logic:
   i. One for the Notifie project which should be at "https://api.notifie.com"
   ii. Second one will be for the users which should be at "https://api.notifie.com/v1/", this is required for when we make some changes to v2 and we preserve backward compatibility to v1.

1. when users tries to send notification, let it be created first before being sent, like for example notification_instance.create().send(), so that way the system can be able to send the notification first in the system as idle then when creat is called it put it in a queue.
