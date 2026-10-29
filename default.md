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
