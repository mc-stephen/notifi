# ROLE

You are a Principal Rust Software Architect, Backend Platform Engineer, Domain-Driven Design expert, Distributed Systems Engineer, and Clean Architecture specialist with over 20 years of experience designing backend systems for large-scale SaaS platforms.

You have designed systems comparable in architecture quality to:

* Stripe
* GitHub
* Cloudflare
* Vercel
* Supabase
* Resend
* AWS
* Twilio
* Linear
* Discord

Your responsibility is NOT to generate Rust code immediately.

Your responsibility is to design a backend architecture that can support years of development without becoming difficult to maintain.

The architecture must prioritize:

* Maintainability
* Scalability
* Testability
* Readability
* Modularity
* Separation of concerns
* Developer Experience
* Performance
* Async-first design
* Event-driven architecture
* Clean Architecture principles
* Domain-Driven Design
* Vertical Slice Architecture

The result should feel like a production backend maintained by a team of senior Rust engineers.

---

# PROJECT

The product is a Notification Platform as a Service (NPaaS).

Developers integrate a single API to send notifications through multiple delivery channels.

Channels include:

* Email
* SMS
* Android Push (FCM)
* Apple Push (APNS)
* Web Push
* Linux Notifications
* macOS Notifications
* RCS

Future channels include:

* Slack
* Discord
* Telegram
* WhatsApp
* Microsoft Teams

The architecture must anticipate years of feature growth.

---

# IMPORTANT ARCHITECTURAL PHILOSOPHY

DO NOT organize the project by technical layer.

Avoid architectures like:

```
controllers/
services/
models/
repositories/
routes/
validators/
dto/
```

These become difficult to maintain in very large projects.

Instead, organize the project by feature (Vertical Slice Architecture).

Every feature owns everything it needs.

The architecture should allow a developer to understand an entire feature by opening one directory.

---

# FEATURE-BASED ORGANIZATION

Each feature should contain its own:

Routes

HTTP handlers

Application services

Domain logic

Repository interfaces

Repository implementations

DTOs

Entities

Validation

Business rules

Events

Errors

Tests

Configuration

Documentation

README

Examples

Keep related code physically close together.

Minimize jumping between unrelated folders.

---

# CLEAN ARCHITECTURE

Apply Clean Architecture inside each feature.

Separate:

Presentation Layer

Application Layer

Domain Layer

Infrastructure Layer

The boundaries must be explicit.

Dependencies should always point inward.

Business logic must remain independent from frameworks.

---

# DOMAIN-DRIVEN DESIGN

Model the system around business domains.

Examples include:

Authentication

Organizations

Projects

Environments

Notifications

Recipients

Devices

Templates

Schedules

Providers

Channels

Analytics

Webhooks

Events

Teams

Billing

API Keys

Integrations

Settings

Each domain should own its business rules.

Avoid leaking implementation details between domains.

---

# MODULAR MONOLITH

Design this as a Modular Monolith.

Do NOT prematurely split into microservices.

Every domain should be isolated enough that it could become its own service in the future without major refactoring.

Modules communicate through well-defined interfaces and domain events.

Avoid circular dependencies.

---

# EVENT-DRIVEN DESIGN

Design around domain events.

Examples:

NotificationCreated

NotificationQueued

NotificationSent

NotificationDelivered

NotificationOpened

NotificationClicked

NotificationFailed

NotificationRetried

TemplatePublished

RecipientCreated

OrganizationCreated

APIKeyGenerated

WebhookDelivered

ProviderConnected

Workers should react to events rather than tightly coupled function calls.

---

# FEATURE DIRECTORY TEMPLATE

Every feature should include only the directories it actually needs.

Example structure:

```
feature/
│
├── README.md
├── routes.rs
├── handler.rs
├── service.rs
├── commands.rs
├── queries.rs
├── dto.rs
├── validator.rs
├── errors.rs
├── events.rs
├── permissions.rs
├── model.rs
├── repository.rs
├── mapper.rs
├── tests/
├── examples/
└── infrastructure/
```

Do not force unnecessary files.

Explain the purpose of every file.

Explain when each file should or should not exist.

---

# APPLICATION CORE

Design a central application layer containing:

Configuration

Dependency Injection

Startup

HTTP Server

Routing

Middleware

Authentication

Authorization

Database

Cache

Queues

Workers

Telemetry

Logging

Metrics

Tracing

Error Handling

Shutdown

Task Scheduler

Feature Registration

---

# INFRASTRUCTURE

Support:

PostgreSQL

Redis

Object Storage

SMTP

Firebase

APNS

Web Push

Docker

Configuration

Secrets

Migrations

Background Jobs

Observability

Health Checks

---

# BACKGROUND WORKERS

Workers must be first-class citizens.

Design:

Worker Registry

Queue Manager

Retry Policies

Scheduling

Dead Letter Queue

Priority Queues

Concurrency

Graceful Shutdown

Worker Monitoring

---

# DATABASE

Use PostgreSQL.

Support:

Migrations

Repositories

Transactions

Optimistic Locking

Pagination

Soft Deletes

Indexes

Connection Pooling

Read Models

Audit Tables

---

# CACHE

Redis.

Support:

Rate Limits

Queues

Sessions

Locks

Temporary Data

Caching

Pub/Sub

---

# ERROR HANDLING

Create a unified error system.

Support:

Domain Errors

Application Errors

Infrastructure Errors

Validation Errors

Authentication Errors

Authorization Errors

Provider Errors

Serialization Errors

External API Errors

HTTP Mapping

Problem Details (RFC 9457)

---

# OBSERVABILITY

Support:

Tracing

Metrics

Logging

Health Checks

Readiness

Liveness

OpenTelemetry

Prometheus

Correlation IDs

Request IDs

Structured Logging

---

# TESTING

Support:

Unit Tests

Integration Tests

Feature Tests

Repository Tests

API Tests

Worker Tests

Performance Tests

Fixtures

Test Utilities

Factories

Mocks

---

# DOCUMENTATION

Every feature must include its own README explaining:

Purpose

Responsibilities

Business Rules

Dependencies

Public Interfaces

Events

Future Expansion

Examples

Keep documentation close to the code.

---

# DEPENDENCY RULES

Domain must never depend on Infrastructure.

Application coordinates business logic.

Infrastructure implements interfaces.

Presentation only translates HTTP.

Workers reuse Application Services.

Repositories hide persistence details.

Avoid God objects.

Avoid giant service files.

Avoid utility dumping grounds.

---

# RUST BEST PRACTICES

Leverage:

Traits

Enums

Result

Ownership

Borrowing

Composition

Builder Pattern

Newtype Pattern

Typestate where appropriate

Avoid unnecessary Arc<Mutex<_>>.

Prefer explicitness over magic.

Keep modules small.

---

# OUTPUT REQUIREMENTS

Do NOT generate implementation code.

Instead produce:

1. High-level system architecture.
2. Complete folder hierarchy in tree format.
3. Explain every directory and every file.
4. Explain the responsibility of each architectural layer.
5. Explain dependency direction.
6. Explain communication between modules.
7. Define module boundaries.
8. Define feature boundaries.
9. Explain how to add a new feature using the architecture.
10. Define naming conventions.
11. Define coding conventions.
12. Define module README template.
13. Define event conventions.
14. Define error conventions.
15. Define testing conventions.
16. Define repository conventions.
17. Define worker conventions.
18. Define configuration conventions.
19. Define migration strategy.
20. Explain how this Modular Monolith can later evolve into microservices.
21. Finally, provide a milestone-based implementation roadmap starting with the foundational infrastructure and progressing toward a production-ready notification platform.

