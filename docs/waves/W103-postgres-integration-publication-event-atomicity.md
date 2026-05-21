# W103. Postgres Integration Publication Event Atomicity

Wave: `W103-postgres-integration-publication-event-atomicity`
Status: `done`
BDD impact: `none`
Agentic impact: `none`
Infra profile: `db`

## Goal

Keep integration publication observability atomic in Postgres by persisting the
outbox publication update and its operator-facing `SystemEvent` inside the same
transaction.

## Feature paths

- `features/request-scan.feature`
- `features/report-finding.feature`

## Execution lanes

- `integration`

## Owned paths

- `apps/api/src/infra/postgres_backend.rs`
- `docs/reliability-hardening-plan.md`

## Slices

| Slice | Status | Goal | Verification |
|---|---|---|---|
| `W103-S01` | done | persist integration publication success/failure updates and their system events in one Postgres transaction and lock that with reload coverage | `integration` |

## Language impact

- none

## Invariant impact

`I2`, `I3`, `I9`, `I11`

## ADR impact

`none`
