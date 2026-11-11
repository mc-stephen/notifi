# Recipient
- Name: Chukwuemeka Michael
- User ID: 0123456789
- Email: stephenugo939@gmail.com
- Phone: 09026560079
- Username: mc-stephen
- Whatsapp: 09026560079
- Android push token: 
- iOS push token: 

# Template
- Name: Welcome
- Desc: Welcome message
- channel (if configure)
    - Email: "Hello {{recipient.username}}, trust you are doing good"
    - Default: "Hello {{recipient.username}}, trust you are doing good"
    [
        - Attachment: `just put a random img you find online`
        - Dynamic Data:
    ]

# Flow
- 

# Campaign
-

# Schedule
- Name: Weekly Newsletter
- Flow: `create new one for it`
- Template: `none, we will use the ai stuff in the flow stuff to generate this by a set of instruction`
- Timing: Weekly
- Run on: Sat<00:00am>

# channel
- SMTP (Generic)
    - smtp config (use the one from my rust env and if invalid, create a new one from Ethereal Email).

# API Key
- Create one.

# Team
- If more than one user exist, add one to a project

# Support
- Just create any support ticket/messagge

# Accounts (User and Admin)
1. Super Admin:
   - full-name: Chukwuemeka Michael
   - email: stephenugo939@gmail.com
   - password: Emeka123!
2. Other Admins:
   - full-name: `use random names`
   - email: `use random mails`
   - password: Emeka123!
3. Normal User:
   - full-name: Michael Stephen
   - email: stephenugo939@gmail.com
   - password: Emeka123!
4. Other Normal Users:
   - full-name: `use random names`
   - email: `use random mails`
   - password: Emeka123!

# Providers
1. Email: 
    - SMTP: create one from Ethereal mail and input the data on the SMTP mail stuff (always try to remember this, so you can always reuse them, and create new ones if only the remembered one doesn't work anymore, as to not abuse Ethereal server).
2. Android: 
    - FCM (Firebase): 
3. IOS:
    - APNS (IOS): 

