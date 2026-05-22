# W118. Bounded System-Event Snapshots With Truthful Querying

Wave: `W118-bounded-system-event-snapshots-with-truthful-querying`
Status: `done`
BDD impact: `none`
Agentic impact: `none`
Infra profile: `none`

## Goal

Keep HTTP snapshot lanes for `system events` bounded and cheap while preserving
truthful event queries over the full durable history.

## Owned paths

- `crates/venom-domain/src/durable_state.rs`
- `crates/venom-domain/src/scanning/durable_scan_runtime.rs`
- `apps/api/src/app/service.rs`
- `apps/api/src/http/mod.rs`
- `apps/api/src/infra/postgres_backend.rs`

## Slices

| Slice | Status | Goal | Verification |
|---|---|---|---|
| `W118-S01` | done | bound cached snapshot lanes for local and Postgres system events, and route operator event queries through full-history iterators instead of the bounded HTTP snapshot | `unit`, `integration` |

## Invariant impact

`I2`, `I8`, `I11`
