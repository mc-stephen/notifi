### How to start both server (Infrastructure) lib
Run this in terms `bash -c 'trap "kill 0" EXIT; cargo run -p server & cargo run -p worker; wait'`

### API path for health data on server:
GET / → "who am I" (name, version)
GET /healthz → "am I alive?" (always 200)
GET /readyz → "are my helpers up?" (200 only when its database AND cache answer)
