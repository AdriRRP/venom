# VENOM Legacy: critical backend review and target architecture

## Scope

This document reviews the VENOM legacy backend with focus on:

- reliability of the command -> event -> projection path;
- weaknesses of the Actix + actor runtime;
- performance bottlenecks in data and process design;
- CQRS, Event Sourcing, and persistence debt;
- current Rust best practices for reliable event-driven systems;
- a target architecture and a realistic refactor path.

Analyzed base:

- legacy: `/Volumes/media/Backup Stratio/cyber-venom/cyber-venom`
- current repo: `/Users/adrianramos/Cybersecurity/Venom`

## Executive summary

The core problem is not just missing Rust optimizations. The legacy backend mixes runtime, queueing, persistence, and projection decisions in ways that weaken reliability.

The worst flaw is semantic: a command is treated as completed when a handler accepts it into a mailbox, not when the business operation actually finishes. That breaks trust at the control-plane level.

The code also proves that event and projection loss under pressure was already happening. Projection-gap repair logic explicitly references mailbox overflow.

The recommended target shape for the new VENOM is:

- modular monolith first;
- Tokio + `axum` + `tower`;
- PostgreSQL as source of truth;
- append-only event store;
- real transactional outbox;
- durable command runtime with leases and idempotency;
- durable projection workers with checkpoints;
- external broker only for real integration boundaries.

## Hard truth about "100% reliability"

Absolute 100% reliability is not real. The useful target is a strict contract:

- no command is marked `completed` before the business transaction commits;
- no durably accepted command is lost on restart;
- persisted domain events are delivered at least once to durable consumers;
- idempotent consumers turn that into exactly-once business effect;
- projections rebuild deterministically from durable offsets;
- pressure becomes latency or rejection, never silent loss.

## Critical findings

### 1. `Accepted` is treated as `Succeeded`

Evidence:

- [`apps/api/src/command_queue.rs:1121`](</Volumes/media/Backup Stratio/cyber-venom/cyber-venom/apps/api/src/command_queue.rs:1121>)
- [`contexts/shared/src/application/command_bus.rs:139`](</Volumes/media/Backup Stratio/cyber-venom/cyber-venom/contexts/shared/src/application/command_bus.rs:139>)

`Ack::Accepted` only means mailbox acceptance, but the distributed command executor records it as success.

Result:

- false success states;
- weak retry semantics;
- control-plane ambiguity.

### 2. The event bus can drop by design

Evidence:

- [`contexts/shared/src/application/event_bus.rs:149`](</Volumes/media/Backup Stratio/cyber-venom/cyber-venom/contexts/shared/src/application/event_bus.rs:149>)

The event bus queues until a limit and then drops. That is incompatible with reliable projections and workflows.

### 3. Projection repairs prove real loss

Evidence:

- [`contexts/vulnerability-management/src/infrastructure/repository/component/postgres.rs:575`](</Volumes/media/Backup Stratio/cyber-venom/cyber-venom/contexts/vulnerability-management/src/infrastructure/repository/component/postgres.rs:575>)
- [`contexts/vulnerability-management/src/infrastructure/repository/collection/postgres.rs:65`](</Volumes/media/Backup Stratio/cyber-venom/cyber-venom/contexts/vulnerability-management/src/infrastructure/repository/collection/postgres.rs:65>)

The code explicitly documents mailbox overflow as the cause of missing projection rows or collection updates.

### 4. Projection replay is tied to actor mailboxes

Evidence:

- [`apps/api/src/bootstrap/mod.rs:1708`](</Volumes/media/Backup Stratio/cyber-venom/cyber-venom/apps/api/src/bootstrap/mod.rs:1708>)

Replay loads every event and pushes each one to projection actors with `do_send`. That couples rebuild throughput and safety to mailbox behavior.

### 5. Projection actors serialize too much work

Evidence:

- [`contexts/vulnerability-management/src/application/projection/component/actor.rs:33`](</Volumes/media/Backup Stratio/cyber-venom/cyber-venom/contexts/vulnerability-management/src/application/projection/component/actor.rs:33>)

The component projection actor mixes deserialization, lookups, upserts, retries, sleeps, and `ctx.wait`, which hurts throughput and increases mailbox pressure.

### 6. The outbox is not fully closed transactionally

Evidence:

- [`design/adr/0001-postgres-event-store-rabbitmq-outbox-inbox.md:30`](</Volumes/media/Backup Stratio/cyber-venom/cyber-venom/design/adr/0001-postgres-event-store-rabbitmq-outbox-inbox.md:30>)
- [`contexts/shared/src/infrastructure/event_store/postgres/mod.rs:90`](</Volumes/media/Backup Stratio/cyber-venom/cyber-venom/contexts/shared/src/infrastructure/event_store/postgres/mod.rs:90>)

The ADR describes the right contract, but the current Postgres event append path does not insert outbox rows in the same transaction as `event_stream` writes.

### 7. Postgres connections are opened per call

Evidence:

- [`contexts/shared/src/infrastructure/event_store/postgres/mod.rs:31`](</Volumes/media/Backup Stratio/cyber-venom/cyber-venom/contexts/shared/src/infrastructure/event_store/postgres/mod.rs:31>)
- [`contexts/vulnerability-management/src/infrastructure/repository/component/postgres.rs:27`](</Volumes/media/Backup Stratio/cyber-venom/cyber-venom/contexts/vulnerability-management/src/infrastructure/repository/component/postgres.rs:27>)

This raises latency and operational cost and should be replaced with a real pool.

### 8. Read APIs overuse `find_all()`

Evidence:

- [`apps/api/src/http/routes/dashboard.rs:22`](</Volumes/media/Backup Stratio/cyber-venom/cyber-venom/apps/api/src/http/routes/dashboard.rs:22>)

The dashboard loads all components, findings, and collections, then aggregates in memory. That does not scale and wastes the read side.

### 9. Projection schemas are too text-heavy

Evidence:

- [`contexts/vulnerability-management/src/infrastructure/repository/component/postgres.rs:47`](</Volumes/media/Backup Stratio/cyber-venom/cyber-venom/contexts/vulnerability-management/src/infrastructure/repository/component/postgres.rs:47>)
- [`contexts/vulnerability-management/src/infrastructure/repository/finding/postgres.rs:101`](</Volumes/media/Backup Stratio/cyber-venom/cyber-venom/contexts/vulnerability-management/src/infrastructure/repository/finding/postgres.rs:101>)

Too many timestamps and structured values are stored as `TEXT`, which hurts indexing, filtering, and semantic integrity.

### 10. `redb` rewrites full streams on append

Evidence:

- [`contexts/shared/src/infrastructure/event_store/redb.rs:63`](</Volumes/media/Backup Stratio/cyber-venom/cyber-venom/contexts/shared/src/infrastructure/event_store/redb.rs:63>)

This makes append cost grow with stream size and is not a good long-term event-store strategy.

## What is worth preserving

- the domain model around `Component`, `Finding`, `Vulnerability`, `Collection`, and `ContextProfile`;
- the split between canonical advisories and component-specific findings;
- the contextual CVSS thesis;
- the instinct toward CQRS and Event Sourcing;
- the operational concern for retries, backlog, and readiness.

## Current Rust best-practice direction

Official references:

- [Tokio graceful shutdown](https://tokio.rs/tokio/topics/shutdown)
- [Tokio `mpsc`](https://docs.rs/tokio/latest/tokio/sync/mpsc/index.html)
- [Tokio `TaskTracker`](https://docs.rs/tokio-util/latest/tokio_util/task/task_tracker/struct.TaskTracker.html)
- [axum](https://docs.rs/axum/latest/axum/)
- [tower `ServiceBuilder`](https://docs.rs/tower/latest/tower/builder/struct.ServiceBuilder.html)
- [SQLx `Pool`](https://docs.rs/sqlx/latest/sqlx/struct.Pool.html)

Direction:

- task-oriented Tokio runtime instead of actor-centric consistency;
- explicit backpressure;
- pooled Postgres access;
- durable offsets for projection workers;
- structured observability from the beginning.

## Recommended target architecture

### Shape

- event-driven modular monolith;
- `tokio` runtime;
- `axum` + `tower` HTTP stack;
- PostgreSQL source of truth;
- append-only `event_stream`;
- real transactional outbox;
- durable `command_inbox` with leases;
- query-oriented projections with durable checkpoints.

### Command runtime

Recommended states:

- `queued`
- `leased`
- `running`
- `completed`
- `failed_retryable`
- `failed_terminal`
- `timed_out`
- `cancelled`

### Projection model

Each projection should have:

- a dedicated worker;
- batch reads by offset;
- deterministic transforms;
- set-based writes;
- durable checkpoint commit after each successful batch.

### Broker policy

Do not put an external broker at the center of consistency in the first refactor wave. Use it later for external integration or real decoupling boundaries.

## Refactor methodology

1. Freeze semantics and identify invariants.
2. Write characterization tests for the domain.
3. Build a new domain core without Actix.
4. Build the new durable write path.
5. Rebuild critical projections.
6. Add external integrations later.
7. Use shadow verification and incremental cutover when feasible.

## Final recommendation

VENOM should be rebuilt around durability, idempotency, observable backpressure, deterministic projections, and real SQL-oriented read models.

The domain idea is strong. The part that must change decisively is the operational substrate.
