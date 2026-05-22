# W123. Bounded Event Retention And Indexed Querying

Wave: `W123-bounded-event-retention-and-indexed-querying`
Status: `done`
BDD impact: `none`
Agentic impact: `none`
Infra profile: `none`

## Goal

Replace unbounded in-memory `system events` queues with a bounded query index
that keeps truthful totals and recent-category windows across local and
Postgres-backed reads.

## Feature paths

- `apps/api/src/infra/postgres_backend.rs`

## Execution lanes

- `unit`
- `integration`

## Owned paths

- `crates/venom-domain/src/operations/system_event_trace.rs`
- `crates/venom-domain/src/durable_state.rs`
- `crates/venom-domain/src/scanning/durable_scan_runtime.rs`
- `apps/api/src/app/service.rs`
- `apps/api/src/infra/postgres_backend.rs`
- `docs/reliability-hardening-plan.md`

## Slices

| Slice | Status | Goal | Verification |
|---|---|---|---|
| `W123-S01` | done | index `system events` by bounded recent windows and truthful category totals instead of cloning full queues into snapshots | `unit`, `integration` |

## Language impact

- none

## Invariant impact

`I2`, `I8`, `I11`

## ADR impact

`none`
