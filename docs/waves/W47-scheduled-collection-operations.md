# W47. Scheduled Collection Operations

Wave: `W47-scheduled-collection-operations`
Status: `active`
BDD impact: `create`
Agentic impact: `none`
Infra profile: `db`

## Goal

Give operators one compact, durable, and efficiently queryable view of scheduled release collections, including due-now state and next-due ordering, so daily release scanning work can be driven from a clear operations surface rather than ad hoc detail lookups.

## Feature paths

- `features/view-collection-schedules.feature`

## Execution lanes

- `unit`
- `integration`
- `infra`
- `acceptance`
- `e2e`

## Owned paths

- `crates/venom-domain/src/inventory/**`
- `crates/venom-domain/examples/acceptance.rs`
- `features/view-collection-schedules.feature`
- `apps/api/src/app/service.rs`
- `apps/api/src/http/mod.rs`
- `apps/web/src/lib/api.ts`
- `apps/web/src/lib/api.test.ts`
- `apps/web/src/routes/operations.tsx`
- `apps/web/src/routes/operations.test.tsx`
- `apps/web/e2e/**`

## Slices

| Slice | Status | Goal | Verification |
|---|---|---|---|
| `W47-S01` | done | define one canonical operator-facing scheduled collection view in the domain | `scripts/check-slice.sh --wave W47-scheduled-collection-operations --slice W47-S01 --lane acceptance --path crates/venom-domain/src/inventory --path crates/venom-domain/examples/acceptance.rs --path features/view-collection-schedules.feature` |
| `W47-S02` | done | expose scheduled collection operator summaries through the API and durable snapshots | `scripts/check-slice.sh --wave W47-scheduled-collection-operations --slice W47-S02 --lane integration --path apps/api/src/app --path apps/api/src/http` |
| `W47-S03` | in_progress | surface the scheduled collection operations board in the UI and browser flow | `scripts/check-slice.sh --wave W47-scheduled-collection-operations --slice W47-S03 --lane e2e --path apps/web/src --path apps/web/e2e` |
| `W47-S04` | planned | close the wave with docs and full gate alignment | `scripts/check-wave.sh --wave W47-scheduled-collection-operations` |

## Language impact

`none`

## Invariant impact

`none`

## ADR impact

`none`

## Notes

- list semantics must stay compact and deterministic
- ordering should prioritize scheduled collections by next due time, then unscheduled collections by key
- UI should not need one detail request per collection to render the operations board
