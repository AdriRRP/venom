# W83. Governance Decision Lifecycle

Wave: `W83-governance-decision-lifecycle`
Status: `done`
BDD impact: `extend`
Agentic impact: `none`
Infra profile: `db`

## Goal

Complete the governed-finding lifecycle by letting operators reopen governed
findings back to the canonical `open` state, both individually and across one
closed release collection cohort.

## Feature paths

- `features/reopen-finding.feature`
- `apps/web/e2e/operator-flow.spec.ts`

## Execution lanes

- `unit`
- `integration`
- `web`
- `e2e`

## Owned paths

- `crates/venom-domain/src/findings/**`
- `crates/venom-domain/src/durable_state.rs`
- `crates/venom-domain/examples/acceptance.rs`
- `apps/api/src/app/service.rs`
- `apps/api/src/http/mod.rs`
- `apps/api/src/infra/postgres_backend.rs`
- `apps/web/src/lib/api.ts`
- `apps/web/src/lib/api.test.ts`
- `apps/web/src/routes/findings.tsx`
- `apps/web/src/routes/findings.test.tsx`
- `apps/web/e2e/operator-flow.spec.ts`
- `docs/product-direction.md`
- `docs/ubiquitous-language.md`

## Slices

| Slice | Status | Goal | Verification |
|---|---|---|---|
| `W83-S01` | done | add one durable reopen lifecycle in domain governance and rebuildable read models | `unit` |
| `W83-S02` | done | expose one reopen lifecycle through API and Postgres durability | `integration` |
| `W83-S03` | done | let the operator reopen governed findings in the console and cover it in browser smoke | `web`, `e2e` |

## Language impact

`add`

## Invariant impact

`none`

## ADR impact

`none`
