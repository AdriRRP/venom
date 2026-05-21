# W101. Postgres Governance Event Atomicity

Wave: `W101-postgres-governance-event-atomicity`
Status: `done`
BDD impact: `none`
Agentic impact: `none`
Infra profile: `none`

## Goal

Keep governance decision rows and their operator-facing `system_events` inside
the same durable Postgres transaction so observability cannot lag behind the
business write.

## Feature paths

- none

## Execution lanes

- `integration`

## Owned paths

- `apps/api/src/infra/postgres_backend.rs`

## Slices

| Slice | Status | Goal | Verification |
|---|---|---|---|
| `W101-S01` | done | persist single and bulk governance `system_events` in the same Postgres transaction as the decision rows | `integration` |

## Language impact

- none

## Invariant impact

`I2`, `I3`, `I9`

## ADR impact

`none`
