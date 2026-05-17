# W52. Command Status Snapshot Elision

Wave: `W52-command-status-snapshot-elision`
Status: `done`
BDD impact: `none`
Agentic impact: `none`
Infra profile: `db`

## Goal

Reduce API-layer copy churn by removing scan command statuses from the shared read snapshot. Command status is only queried by `command_id`, so it should be served directly from `AppService` instead of rebuilding a full status map after each write.

## Feature paths

- `features/request-scan.feature`

## Execution lanes

- `unit`
- `integration`

## Owned paths

- `apps/api/src/app/service.rs`
- `apps/api/src/http/mod.rs`
- `docs/waves/W52-command-status-snapshot-elision.md`

## Slices

| Slice | Status | Goal | Verification |
|---|---|---|---|
| `W52-S01` | done | remove command status snapshots from API read state and serve them directly from `AppService` | `scripts/check-slice.sh --wave W52-command-status-snapshot-elision --slice W52-S01 --lane integration --path apps/api/src/app/service.rs --path apps/api/src/http/mod.rs --path apps/api/src/infra/postgres_backend.rs` |
| `W52-S02` | done | close the wave with docs and full gate alignment | `scripts/check-wave.sh --wave W52-command-status-snapshot-elision` |

## Language impact

`none`

## Invariant impact

`none`

## ADR impact

`none`

## Notes

- prefer direct command-status queries over carrying a whole status map in the operator read snapshot
