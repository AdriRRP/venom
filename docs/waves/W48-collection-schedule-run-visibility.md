# W48. Collection Schedule Run Visibility

Wave: `W48-collection-schedule-run-visibility`
Status: `done`
BDD impact: `extend`
Agentic impact: `none`
Infra profile: `db`

## Goal

Give operators durable, explicit visibility of the last periodic collection schedule materialization, including when it ran and how many scan commands it enqueued, so release scanning can be governed like daily work rather than inferred from scattered state.

## Feature paths

- `features/view-collection-schedules.feature`

## Execution lanes

- `unit`
- `integration`
- `infra`
- `acceptance`
- `e2e`

## Owned paths

- `crates/venom-domain/src/inventory/**`
- `crates/venom-domain/src/scanning/**`
- `crates/venom-domain/src/durable_state.rs`
- `crates/venom-domain/examples/acceptance.rs`
- `features/view-collection-schedules.feature`
- `apps/api/src/app/service.rs`
- `apps/api/src/http/mod.rs`
- `apps/api/src/infra/postgres_backend.rs`
- `apps/web/src/lib/api.ts`
- `apps/web/src/routes/operations.tsx`
- `apps/web/src/routes/operations.test.tsx`
- `apps/web/e2e/**`

## Slices

| Slice | Status | Goal | Verification |
|---|---|---|---|
| `W48-S01` | done | add durable last-run collection schedule metadata in the domain | `scripts/check-slice.sh --wave W48-collection-schedule-run-visibility --slice W48-S01 --lane acceptance --path crates/venom-domain/src/inventory --path crates/venom-domain/src/scanning --path crates/venom-domain/src/durable_state.rs --path crates/venom-domain/examples/acceptance.rs --path features/view-collection-schedules.feature` |
| `W48-S02` | done | expose last-run schedule metadata through API and durable Postgres state | `scripts/check-slice.sh --wave W48-collection-schedule-run-visibility --slice W48-S02 --lane integration --path apps/api/src/app --path apps/api/src/http --path apps/api/src/infra/postgres_backend.rs` |
| `W48-S03` | done | surface last-run schedule visibility in the UI and browser flow | `scripts/check-slice.sh --wave W48-collection-schedule-run-visibility --slice W48-S03 --lane e2e --path apps/web/src --path apps/web/e2e` |
| `W48-S04` | done | close the wave with docs and full gate alignment | `scripts/check-wave.sh --wave W48-collection-schedule-run-visibility` |

## Language impact

`none`

## Invariant impact

`none`

## ADR impact

`none`

## Notes

- last-run metadata must be explicit, durable, and bounded
- scheduler passes must not invent hidden retries or inferred success
- operator views must still be served from one compact collection board response
