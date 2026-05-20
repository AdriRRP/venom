# W81. Source-Driven Collections

Wave: `W81-source-driven-collections`
Status: `done`
BDD impact: `extend`
Agentic impact: `none`
Infra profile: `db`

## Goal

Let one managed release collection derive membership from one declared source
with explicit `replace` or `reconcile` semantics and one deterministic
materialization flow.

## Feature paths

- `features/manage-collections.feature`

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
- `apps/web/src/routes/operations.tsx`
- `apps/web/e2e/operator-flow.spec.ts`
- `features/manage-collections.feature`
- `docs/product-direction.md`
- `docs/ubiquitous-language.md`

## Slices

| Slice | Status | Goal | Verification |
|---|---|---|---|
| `W81-S01` | done | add one declared collection source model with deterministic replace or reconcile materialization in domain and durable replay | `unit`, `acceptance` |
| `W81-S02` | done | expose collection source configuration and materialization through API and Postgres durability | `integration` |
| `W81-S03` | done | operate one source-driven collection from the UI and cover the flow with browser smoke | `web`, `e2e` |

## Language impact

`add`

## Invariant impact

`none`

## ADR impact

`none`
