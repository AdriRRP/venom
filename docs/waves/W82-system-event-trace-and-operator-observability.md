# W82. System Event Trace And Operator Observability

Wave: `W82-system-event-trace-and-operator-observability`
Status: `done`
BDD impact: `extend`
Agentic impact: `none`
Infra profile: `db`

## Goal

Expose command, scheduler, governance, and publication traceability as one
operator-facing event timeline.

## Feature paths

- `apps/web/e2e/operator-flow.spec.ts`

## Execution lanes

- `unit`
- `integration`
- `web`
- `e2e`

## Owned paths

- `crates/venom-domain/src/operations/**`
- `crates/venom-domain/src/durable_state.rs`
- `crates/venom-domain/src/scanning/durable_scan_runtime.rs`
- `apps/api/src/app/service.rs`
- `apps/api/src/http/mod.rs`
- `apps/api/src/infra/postgres_backend.rs`
- `apps/web/src/app/app-shell.tsx`
- `apps/web/src/app/router.tsx`
- `apps/web/src/lib/api.ts`
- `apps/web/src/routes/events.tsx`
- `apps/web/e2e/operator-flow.spec.ts`
- `docs/product-direction.md`
- `docs/ubiquitous-language.md`

## Slices

| Slice | Status | Goal | Verification |
|---|---|---|---|
| `W82-S01` | done | add one durable rebuildable system-event projection in domain state and durable scan runtime | `unit` |
| `W82-S02` | done | expose one operator-facing timeline through API snapshots and Postgres durability | `integration` |
| `W82-S03` | done | add one system-events route in the console and cover the operator flow in browser smoke | `web`, `e2e` |

## Language impact

`add`

## Invariant impact

`none`

## ADR impact

`none`
