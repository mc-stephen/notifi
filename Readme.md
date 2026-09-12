# NOTE:

1. This app consite of 3 users type.
   - Admin (this users mange the platform and it Users)
   - Normal Users (this is the Users of my platform [Notifie])
   - Project Users (this is when a normal users created a project and add his own end users to it)
2. The API should also have 3 mail path
   - https://api.base_url/* (This is for the ops/root)
   - https://api.base_url/v1/* (This is for external API)
   - https://api.base_url/app/* (This is for the User dashboard api)
   - https://api.base_url/admin/* (This is for the Admin dashboard api)
3.

---

# AI NOTE FOR EVERY PROMPT

0. When creating dummy or random users / admin account always use "Emeka123!" as their password, so i can always access the relevant account.
1. If you run any proccesses internally either for testing or whatever, kindly also kill that your running process after you are done, DON'T FORGET THIS!.

---

# TODO:

1. Billing should be par project and not par account
2. Users can't create more than 10 orgs without emailing support. this is to avoid abuse of the system.
3. in the "Channel" section, the "provider" page will be for adding providers like adding 3 provider for emailing, and then the "channel" page will be for configuring a channel (eg email) and then selecting one of the pre-config providers
