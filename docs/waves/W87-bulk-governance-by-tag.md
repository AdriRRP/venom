# W87. Bulk Governance by Tag

Wave: `W87-bulk-governance-by-tag`
Status: `done`
BDD impact: `create`
Agentic impact: `none`
Infra profile: `db`

## Goal

Apply explicit bulk governance actions over one reusable tag-scoped open
cohort without deriving scope from page size, release membership, or ad hoc
client loops.

## Feature paths

- `features/bulk-governance-by-tag.feature`

## Execution lanes

- `unit`
- `integration`
- `acceptance`
- `web`
- `e2e`

## Owned paths

- `crates/venom-domain/src/findings/**`
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
- `features/bulk-governance-by-tag.feature`
- `docs/product-direction.md`
- `docs/ubiquitous-language.md`

## Slices

| Slice | Status | Goal | Verification |
|---|---|---|---|
| `W87-S01` | done | add one tag-scoped governed cohort and bulk governance path in the domain | `unit`, `acceptance` |
| `W87-S02` | done | expose tag-scoped bulk governance through API and Postgres durability | `integration` |
| `W87-S03` | done | let the operator run bulk governance by tag in the console and cover the flow in browser smoke | `web`, `e2e` |

## Language impact

`add`

## Invariant impact

`none`

## ADR impact

`none`
