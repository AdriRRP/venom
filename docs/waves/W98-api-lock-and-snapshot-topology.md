# W98. API Lock And Snapshot Topology

Wave: `W98-api-lock-and-snapshot-topology`
Status: `done`
BDD impact: `none`
Agentic impact: `none`
Infra profile: `none`

## Goal

Remove the operator scan-command status read path from the mutable application
lock by projecting command statuses into a compact read-side snapshot and
refreshing that lane only where command state actually changes.

## Feature paths

- `features/request-scan.feature`
- `features/request-collection-scan.feature`

## Execution lanes

- `integration`
- `unit`

## Owned paths

- `apps/api/src/app/service.rs`
- `apps/api/src/http/mod.rs`
- `apps/api/src/infra/postgres_backend.rs`
- `docs/reliability-hardening-plan.md`

## Slices

| Slice | Status | Goal | Verification |
|---|---|---|---|
| `W98-S01` | done | project compact scan command statuses into the read snapshot and serve `GET /scan-commands/{id}` from that lane | `integration` |

## Language impact

- none

## Invariant impact

`I8`, `I11`

## ADR impact

`none`
