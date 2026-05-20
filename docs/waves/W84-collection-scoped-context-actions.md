# W84. Collection-Scoped Context Actions

Wave: `W84-collection-scoped-context-actions`
Status: `done`
BDD impact: `extend`
Agentic impact: `none`
Infra profile: `db`

## Goal

Attach one managed default context profile to one closed release scope and
merge it with component-specific context field by field, letting the component
override win where it is explicitly defined.

## Feature paths

- `features/manage-context-profiles.feature`
- `apps/web/e2e/operator-flow.spec.ts`

## Execution lanes

- `unit`
- `integration`
- `acceptance`
- `web`
- `e2e`

## Owned paths

- `crates/venom-domain/src/inventory/**`
- `crates/venom-domain/src/durable_state.rs`
- `crates/venom-domain/examples/acceptance.rs`
- `apps/api/src/app/service.rs`
- `apps/api/src/http/mod.rs`
- `apps/api/src/infra/postgres_backend.rs`
- `apps/web/src/lib/api.ts`
- `apps/web/src/lib/api.test.ts`
- `apps/web/src/routes/operations.tsx`
- `apps/web/src/routes/operations.test.tsx`
- `apps/web/e2e/operator-flow.spec.ts`
- `features/manage-context-profiles.feature`
- `docs/debt-closure-plan.md`
- `docs/product-direction.md`
- `docs/ubiquitous-language.md`

## Slices

| Slice | Status | Goal | Verification |
|---|---|---|---|
| `W84-S01` | done | add one durable collection default-context assignment in domain inventory and replayable local durability | `unit`, `acceptance` |
| `W84-S02` | done | expose one collection default-context assignment through API and Postgres durability | `integration` |
| `W84-S03` | done | let the operator set one collection default context in the console and cover the scoped merge behavior in browser smoke | `web`, `e2e` |

## Language impact

`add`

## Invariant impact

`none`

## ADR impact

`none`
