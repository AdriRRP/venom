# W86. Tag-Scoped Context Overlays

Wave: `W86-tag-scoped-context-overlays`
Status: `done`
BDD impact: `extend`
Agentic impact: `none`
Infra profile: `db`

## Goal

Attach one managed context profile to one component tag as a reusable partial
overlay, merge it with collection defaults and explicit component context, and
reject conflicting tag overlays instead of choosing one silently.

## Feature paths

- `features/manage-component-tags.feature`
- `features/classify-finding.feature`

## Execution lanes

- `unit`
- `integration`
- `acceptance`
- `web`

## Owned paths

- `crates/venom-domain/src/inventory/**`
- `crates/venom-domain/src/findings/contextual_risk.rs`
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
- `features/classify-finding.feature`
- `docs/product-direction.md`
- `docs/ubiquitous-language.md`

## Slices

| Slice | Status | Goal | Verification |
|---|---|---|---|
| `W86-S01` | done | add tag overlay assignment and conflict-free effective-context merge in the domain | `unit`, `acceptance` |
| `W86-S02` | done | persist and expose tag context overlays through API and Postgres durability | `integration` |
| `W86-S03` | done | let the operator assign one context profile to one tag and observe effective context behavior in the console | `web` |

## Language impact

`add`

## Invariant impact

`none`

## ADR impact

`none`
