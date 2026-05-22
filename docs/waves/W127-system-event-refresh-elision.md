# W127. System Event Refresh Elision

Wave: `W127-system-event-refresh-elision`
Status: `done`
BDD impact: `none`
Agentic impact: `none`
Infra profile: `none`

## Goal

Stop deep read-side cache rebuilds when one mutation only appends operator-facing
system events.

## Owned paths

- `crates/venom-domain/src/durable_state.rs`
- `apps/api/src/infra/postgres_backend.rs`

## Slices

| Slice | Status | Goal | Verification |
|---|---|---|---|
| `W127-S01` | done | refresh only the bounded system-event snapshot lane when pushing system events instead of rebuilding inventory, findings, and release-board caches | `unit`, `integration` |

## Language impact

`none`

## Invariant impact

`I8`, `I11`
