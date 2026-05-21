# W94. Local Collection Scan Materialization Atomicity

Wave: `W94-local-collection-scan-materialization-atomicity`
Status: `done`
BDD impact: `none`
Agentic impact: `none`
Infra profile: `none`

## Goal

Make the local collection scheduler safe across retries by ensuring one due
collection batch is durably keyed and not duplicated when collection
materialization metadata is recorded after queue enqueue.

## Feature paths

- `features/schedule-collection-scan.feature`

## Execution lanes

- `unit`
- `integration`

## Owned paths

- `crates/venom-domain/src/scanning/collection_scan_scheduler.rs`
- `crates/venom-domain/src/scanning/durable_scan_runtime.rs`
- `apps/api/src/app/service.rs`
- `docs/reliability-hardening-plan.md`

## Slices

| Slice | Status | Goal | Verification |
|---|---|---|---|
| `W94-S01` | done | key one local scheduled batch by `collection + due_at` and reuse it on retry | `unit` |
| `W94-S02` | done | make the local collection scheduler enqueue durably before recording materialization and reuse existing batches on retry | `integration` |

## Language impact

`none`

## Invariant impact

`I2`, `I3`, `I9`

## ADR impact

`none`
