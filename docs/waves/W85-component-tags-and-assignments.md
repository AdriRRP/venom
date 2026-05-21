# W85. Component Tags and Assignments

Wave: `W85-component-tags-and-assignments`
Status: `done`
BDD impact: `create`
Agentic impact: `none`
Infra profile: `db`

## Goal

Add one managed tag vocabulary for reusable transversal component cohorts and
make tag membership durable, queryable, and operator-facing without changing
the semantics of closed release collections.

## Feature paths

- `features/manage-component-tags.feature`

## Execution lanes

- `unit`
- `integration`
- `acceptance`
- `web`

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
- `features/manage-component-tags.feature`
- `docs/product-direction.md`
- `docs/ubiquitous-language.md`

## Slices

| Slice | Status | Goal | Verification |
|---|---|---|---|
| `W85-S01` | done | define one managed tag entity and durable component membership in domain inventory | `unit`, `acceptance` |
| `W85-S02` | done | expose managed tags and component-tag assignment through API and Postgres durability | `integration` |
| `W85-S03` | done | let the operator create tags and assign components to them in the console | `web` |

## Language impact

`add`

## Invariant impact

`none`

## ADR impact

`none`
