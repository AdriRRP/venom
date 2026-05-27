# W180. System Event Category Window Truth

Wave: `W180-system-event-category-window-truth`
Status: `done`
BDD impact: `none`
Agentic impact: `none`
Infra profile: `db`

## Goal

Restore truthful category-scoped recent `system events` pages without going
back to duplicating full event payloads per category window.

## Feature paths

- `crates/venom-domain/src/operations/system_event_trace.rs`
- `apps/api/src/infra/postgres_backend.rs`

## Execution lanes

- `unit`
- `integration`

## Owned paths

- `crates/venom-domain/src/operations/system_event_trace.rs`
- `apps/api/src/infra/postgres_backend.rs`

## Slices

| Slice | Status | Goal | Verification |
|---|---|---|---|
| `W180-S01` | done | keep one retained event store while rebuilding truthful category-recent pages through shared retained slots and category-specific links | `cargo test -p venom-domain system_event_trace --all-features --offline` |

## Language impact

`none`

## Invariant impact

`I2`, `I8`, `I9`, `I11`

## ADR impact

`none`
