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

# TEST ACCOUNT (AI AGENTs keep this in mind at all times)
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

# NOTE:
1. This app consite of 2 users type.
   - Admin (this users manage the platform and it Users)
   - Normal Users (this is the Users of my platform [Notifie])
2. Other kind of users are "Recipient"
   - Project Users (this is when a normal users created a project and add his own end users to it)
3. The API should also have 3 mail path
   - https://api.base_url/* (This is for the ops/root)
   - https://api.base_url/v1/* (This is for external API)
   - https://api.base_url/app/* (This is for the User dashboard api)
   - https://api.base_url/admin/* (This is for the Admin dashboard api)
   - https://api.base_url/gateway/* (This is for device routes — apps poll their secret route here)

