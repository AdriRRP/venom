# W104. Postgres Scan Request Event Atomicity

Wave: `W104-postgres-scan-request-event-atomicity`
Status: `done`
BDD impact: `none`
Agentic impact: `none`
Infra profile: `db`

## Goal

Make Postgres scan-request enqueue paths atomic from the operator point of
view by persisting pending commands and their `scan-command-enqueued`
`SystemEvent`s in one transaction.

## Feature paths

- `features/request-scan.feature`
- `features/request-collection-scan.feature`

## Execution lanes

- `integration`

## Owned paths

- `apps/api/src/infra/postgres_backend.rs`
- `docs/reliability-hardening-plan.md`

## Slices

| Slice | Status | Goal | Verification |
|---|---|---|---|
| `W104-S01` | done | persist Postgres single and collection scan request enqueue operations together with their system events in one transaction and keep reload coverage explicit | `integration` |

## Language impact

- none

## Invariant impact

`I2`, `I3`, `I9`, `I11`

## ADR impact

`none`
