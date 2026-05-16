# 0003. Durable outbox and integration events

## Status

`Accepted`

## Context

VENOM needs external integration events, but the legacy project already showed how quickly event-driven semantics become untrustworthy when durability, retries, and completion are ambiguous.

If VENOM publishes directly from transient in-memory state or treats the broker as the source of truth, three risks appear early:

- business state and published events can diverge through unsafe dual writes
- completion semantics become unclear under retries, restarts, and partial failure
- performance erodes through unbounded fan-out, background buffering, or per-event task spawning

## Decision

Use a durable outbox boundary with these rules:

- the durable state change and the integration event append must happen in one coordinated durable write path
- integration events are canonical VENOM events, not provider payloads or broker-native envelopes
- the outbox is the source of truth for unpublished vs published integration events
- publication is externally at-least-once and must therefore be idempotent by event identity
- the first publisher path must read and publish in explicit bounded batches
- publication success and failure must be persisted explicitly; no hidden completion and no silent drop
- the broker is an integration transport, not the core system of record

## Consequences

- VENOM can introduce real event-driven integration early without delegating correctness to the broker
- replay, restart, duplicate publication, and partial failure become first-class verification stories
- the publication path stays aligned with near embedded-grade efficiency by avoiding unbounded buffering and unreviewable per-event fan-out
- external consumers must treat VENOM integration events as at-least-once and idempotent by durable event identity
