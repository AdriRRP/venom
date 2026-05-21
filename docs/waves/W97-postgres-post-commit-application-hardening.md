# W97. Postgres Post-Commit Application Hardening

Wave: `W97-postgres-post-commit-application-hardening`
Status: `done`
BDD impact: `none`
Agentic impact: `none`
Infra profile: `db`

## Goal

Remove the `commit-first, apply-later` gaps from the Postgres scheduler and
scan-completion paths so durable truth and operator observability advance in one
transaction, with only in-memory refresh left for the second phase.

## Feature paths

- `features/request-collection-scan.feature`
- `features/request-scan.feature`

## Execution lanes

- `integration`

## Owned paths

- `apps/api/src/infra/postgres_backend.rs`
- `docs/reliability-hardening-plan.md`

## Slices

| Slice | Status | Goal | Verification |
|---|---|---|---|
| `W97-S01` | done | persist scheduler and scan-completion system events in the same Postgres transaction as their coordinated business writes | `integration` |

## Language impact

- none

## Invariant impact

`I2`, `I3`, `I9`

## ADR impact

`none`
