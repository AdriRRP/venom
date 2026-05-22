# W106. Postgres Read Snapshot Caches

Wave: `W106-postgres-read-snapshot-caches`
Status: `done`
BDD impact: `none`
Agentic impact: `none`
Infra profile: `none`

## Goal

Stop rebuilding the Postgres-backed `system events` and `command statuses`
snapshot lanes on every getter call by maintaining explicit cached `Arc`
snapshots refreshed only when those lanes mutate.

## Feature paths

- `features/request-scan.feature`
- `features/request-collection-scan.feature`
- `features/view-system-events.feature`

## Execution lanes

- `unit`
- `integration`

## Owned paths

- `apps/api/src/infra/postgres_backend.rs`
- `docs/reliability-hardening-plan.md`

## Slices

| Slice | Status | Goal | Verification |
|---|---|---|---|
| `W106-S01` | done | cache Postgres `system events` and `command statuses` snapshot lanes and refresh them only on real mutation paths or rebuild | `unit`, `integration` |

## Language impact

- none

## Invariant impact

`I8`, `I11`

## ADR impact

`none`
