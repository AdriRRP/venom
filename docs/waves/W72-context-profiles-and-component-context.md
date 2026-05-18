# W72. Context profiles and component context

Wave: `W72-context-profiles-and-component-context`
Status: `done`
BDD impact: `create`
Agentic impact: `none`
Infra profile: `db`

## Goal

Let operators define reusable execution-context profiles and attach one profile
to one managed component so later contextual risk can derive meaning from
durable, explicit runtime context rather than ad hoc input.

## Feature paths

- `features/manage-context-profiles.feature`

## Execution lanes

- `unit`
- `integration`
- `acceptance`
- `e2e`

## Owned paths

- `crates/venom-domain/src/inventory/**`
- `crates/venom-domain/src/durable_state.rs`
- `crates/venom-domain/examples/acceptance.rs`
- `apps/api/src/app/service.rs`
- `apps/api/src/http/mod.rs`
- `apps/api/src/infra/postgres_backend.rs`
- `apps/web/src/lib/api.test.ts`
- `apps/web/src/lib/api.ts`
- `apps/web/src/routes/operations.tsx`
- `apps/web/src/routes/operations.test.tsx`
- `apps/web/e2e/operator-flow.spec.ts`
- `features/manage-context-profiles.feature`

## Slices

| Slice | Status | Goal | Verification |
|---|---|---|---|
| `W72-S01` | done | add durable context profiles and component assignment to the domain inventory | unit and acceptance checks |
| `W72-S02` | done | expose context profile creation, listing, and component assignment through API and Postgres | integration checks |
| `W72-S03` | done | let operators manage context profiles from the console and prove the flow through browser-driven smoke | web and e2e checks |
| `W72-S04` | done | normalize Rust formatting and close the wave on a clean committed tree | full wave gate |
| `W72-S05` | done | remove ambiguous browser selectors so the operator smoke stays stable as the UI grows | full wave gate |
| `W72-S06` | done | normalize the browser smoke after selector hardening so the frontend gate stays deterministic | full wave gate |

## Language impact

`none`

## Invariant impact

`none`

## ADR impact

`none`
