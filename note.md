### How to start both server (Infrastructure) lib
Run this in terms `bash -c 'trap "kill 0" EXIT; cargo run -p server & cargo run -p worker; wait'`

### API path for health data on server:
GET / → "who am I" (name, version)
GET /healthz → "am I alive?" (always 200)
GET /readyz → "are my helpers up?" (200 only when its database AND cache answer)

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

