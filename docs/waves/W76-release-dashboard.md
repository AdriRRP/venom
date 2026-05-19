# W76. Release Dashboard

Wave: `W76-release-dashboard`
Status: `done`
BDD impact: `create`
Agentic impact: `docs`
Infra profile: `none`

## Goal

Expose one executive release dashboard that compresses managed collection health,
governance, contextual risk, and schedule state into one compact operator view
without widening the write model.

## Feature paths

- `features/view-release-dashboard.feature`

## Execution lanes

- `unit`
- `integration`
- `acceptance`
- `e2e`

## Owned paths

- `crates/venom-domain/src/findings/**`
- `crates/venom-domain/examples/acceptance.rs`
- `apps/api/src/app/service.rs`
- `apps/api/src/http/mod.rs`
- `apps/web/src/app/**`
- `apps/web/src/routes/**`
- `apps/web/src/lib/api.ts`
- `apps/web/e2e/operator-flow.spec.ts`
- `features/view-release-dashboard.feature`

## Slices

| Slice | Status | Goal | Verification |
|---|---|---|---|
| `W76-S01` | done | project one release dashboard from collection summaries and collection health | `acceptance`, `unit` |
| `W76-S02` | done | expose the release dashboard through the API | `integration` |
| `W76-S03` | done | add one dashboard route to the operator console and cover it end to end | `web`, `e2e` |

## Language impact

`add`

## Invariant impact

`none`

## ADR impact

`none`

## Notes

- The dashboard is a read-side projection only.
- It must not reuse the write model directly as a view payload.
