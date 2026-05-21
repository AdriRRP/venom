# W102. API Read Snapshot Arc Sharing

Wave: `W102-api-read-snapshot-arc-sharing`
Status: `done`
BDD impact: `none`
Agentic impact: `none`
Infra profile: `none`

## Goal

Reduce avoidable churn inside the HTTP read snapshot by sharing unchanged lanes
through `Arc` when one lane refreshes, instead of rebuilding the whole snapshot
value graph every time.

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
| `W102-S01` | done | refresh HTTP read-snapshot lanes by `Arc` and reuse the unchanged lanes across inventory, read-model, system-event, and command-status refreshes | `unit`, `integration` |

## Language impact

- none

## Invariant impact

`I8`, `I11`

## ADR impact

`none`
