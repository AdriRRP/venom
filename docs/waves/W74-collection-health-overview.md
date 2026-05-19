# W74. Collection health overview

Wave: `W74-collection-health-overview`
Status: `done`
BDD impact: `create`
Agentic impact: `none`
Infra profile: `db`

## Goal

Let operators see one compact health summary for every managed release
collection, derived from active findings, contextual risk, and governance state
without widening the write model.

## Feature paths

- `features/view-collection-health.feature`

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
- `apps/web/src/lib/api.ts`
- `apps/web/src/routes/operations.tsx`
- `apps/web/src/routes/operations.test.tsx`
- `apps/web/e2e/operator-flow.spec.ts`
- `features/view-collection-health.feature`
- `docs/product-direction.md`
- `docs/ubiquitous-language.md`

## Slices

| Slice | Status | Goal | Verification |
|---|---|---|---|
| `W74-S01` | done | derive one compact collection health summary from scoped active findings | unit and acceptance checks |
| `W74-S02` | done | expose collection health through list and detail API projections | integration checks |
| `W74-S03` | done | show collection health in the operator console and browser smoke | web and e2e checks |

## Language impact

`add`

## Invariant impact

`none`

## ADR impact

`none`
