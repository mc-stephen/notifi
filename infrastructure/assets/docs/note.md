1. The back-end is to have 2 main api route, and to each it own, so when i change that of End-users api logic it should not affect that of my project api logic:
   i. One for the Notifie project which should be at "https://api.notifie.com"
   ii. Second one will be for the users which should be at "https://api.notifie.com/v1/", this is required for when we make some changes to v2 and we preserve backward compatibility to v1.

2. While the Infrastructure folder is looking good and all that, i belive it is being over-engineered, so we need to look into that, and make it as simple but modern as it should.

The current Infrastructure

```
Infrastructure
└── crates/
    ├── domain/           # Feature domains (Auth, Onboarding, Dashboard, etc.)
    ├── api/              # Web endpoints & entrypoints (v1, internal core APIs)
    ├── adapter/          # Integration handlers (Email, SMS, third-party services)
    ├── core/             # Core business models, rules, & domain logic
    ├── domain-ports/     # Trait definitions & abstract contracts
    └── infra/            # Concrete database, cache, & network drivers
```

Seems like we need to make it somthing simpler (Keep everything in a single crate and enforce architectural boundaries using Rust's mod system.)

```
src/
├── main.rs           # App setup, dependency injection, route registration
├── domain/           # Auth, Onboarding, Dashboard logic & models
├── ports/            # Traits/Interfaces (e.g., trait UserRepository)
├── infra/            # Postgres, Redis, HTTP clients
├── adapters/         # Email, SMS, third-party services
└── api/              # Actix/Axum handlers, routes, DTOs
```

3. Also our migrations sql code, we need to also review it as the tables naming convention is not what i like, keep that in mind.

4.
