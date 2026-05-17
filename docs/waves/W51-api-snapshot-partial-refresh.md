# W51. API Snapshot Partial Refresh

Wave: `W51-api-snapshot-partial-refresh`
Status: `done`
BDD impact: `none`
Agentic impact: `none`
Infra profile: `db`

## Goal

Reduce memory churn in the API layer by refreshing only the changed parts of the operator read snapshot after each mutation, instead of rebuilding full inventory, read model, and command-status snapshots every time.

## Feature paths

- `features/register-component.feature`
- `features/manage-collections.feature`
- `features/request-scan.feature`
- `features/report-finding.feature`
- `features/view-active-findings.feature`
- `features/view-collection-schedules.feature`

## Execution lanes

- `unit`
- `integration`
- `infra`
- `acceptance`
- `e2e`

## Owned paths

- `apps/api/src/app/service.rs`
- `apps/api/src/http/mod.rs`
- `docs/waves/W51-api-snapshot-partial-refresh.md`

## Slices

| Slice | Status | Goal | Verification |
|---|---|---|---|
| `W51-S01` | done | split API read snapshots into reusable parts and refresh only the changed lanes after each mutation | `scripts/check-slice.sh --wave W51-api-snapshot-partial-refresh --slice W51-S01 --lane integration --path apps/api/src/app/service.rs --path apps/api/src/http/mod.rs` |
| `W51-S02` | done | close the wave with docs and full gate alignment | `scripts/check-wave.sh --wave W51-api-snapshot-partial-refresh` |

## Language impact

`none`

## Invariant impact

`none`

## ADR impact

`none`

## Notes

- prefer explicit partial refresh helpers over hidden cache invalidation
- skip snapshot refresh entirely when a mutation does not affect current operator reads
- keep read paths simple; optimize writes and refresh strategy, not API semantics
