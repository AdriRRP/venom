# W128. Collection Scan Scheduler Borrowed Inventory

Wave: `W128-collection-scan-scheduler-borrowed-inventory`
Status: `done`
BDD impact: `none`
Agentic impact: `none`
Infra profile: `none`

## Goal

Materialize due collection scan batches from borrowed inventory state instead of
cloning whole inventory shapes before planning.

## Owned paths

- `crates/venom-domain/src/scanning/collection_scan_scheduler.rs`
- `apps/api/src/app/service.rs`
- `apps/api/src/infra/postgres_backend.rs`

## Slices

| Slice | Status | Goal | Verification |
|---|---|---|---|
| `W128-S01` | done | make the collection scan scheduler read-only and remove unnecessary inventory clones from local and Postgres scheduling paths | `unit`, `integration` |

## Language impact

`none`

## Invariant impact

`I8`, `I11`
