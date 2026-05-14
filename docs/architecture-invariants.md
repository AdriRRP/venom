# Architecture Invariants

## Purpose

These are the current architectural rules that must remain true.

This file is a living contract.

It is not an ADR log:

- ADRs explain why a decision was made.
- invariants define what must currently hold.

## Current invariants

### I1. Provider-agnostic domain

The core domain model must not depend on a concrete finding provider schema, naming convention, or payload shape.

### I2. No fake completion

A command must never be represented as completed before the business operation has actually completed successfully.

### I3. No silent drop in business paths

Business-critical command, event, and projection paths must fail explicitly, backpressure explicitly, or queue durably. They must not silently drop work.

### I4. Durable publication boundary

If the system publishes external integration events, the durable write path and the publication boundary must be coordinated through an outbox-style contract rather than an unsafe dual write.

### I5. Rebuildable read side

Read models must be rebuildable from durable history or another durable checkpointed source of truth.

### I6. Local-first verification path

The default development and verification workflow must work with local fixtures and real local infrastructure, without requiring paid external services.

### I7. Language is part of the architecture

When the domain model changes in a meaningful way, `docs/ubiquitous-language.md` must be checked and updated if needed.

### I8. Near embedded-grade efficiency

Default implementation choices for hot paths must favor low allocation, bounded copying, predictable latency, and compact data movement over convenience abstractions that materially erode performance.

### I9. Near nation-grade reliability

Business-critical paths must be designed so that loss, duplication, reordering, restart, backpressure, and partial infrastructure failure are treated as normal design inputs, not exceptional afterthoughts.

## Update rule

Change this file only when:

- a rule is added;
- a rule is removed;
- a rule is materially changed.

When that happens, decide whether an ADR is also required.
