# W149 HTTP Write Plane Real Partitioning

## Why

Fresh Postgres-backed reads still contended on the mutable `ApiApplication`
slot whenever another instance advanced durable state. That kept part of the
operator read plane serialized behind the live write service.

## What changed

- Added one detached Postgres read-snapshot loader for stale fresh reads.
- Fresh HTTP reads now rebuild one remote snapshot without taking the live
  mutable application slot.
- The write path keeps its own durable refresh before mutation, so correctness
  still lives on the write side.

## Verification

- `cargo check -p venom-api --all-features`

