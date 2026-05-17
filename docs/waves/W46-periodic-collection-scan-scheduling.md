# W46. Periodic Collection Scan Scheduling

Wave: `W46-periodic-collection-scan-scheduling`
Status: `active`
BDD impact: `create`
Agentic impact: `none`
Infra profile: `db`

## Goal

Let operators attach one periodic scan schedule to a closed release collection and materialize due collection scans into durable scan commands through an explicit bounded worker loop.

## Feature paths

- `features/schedule-collection-scan.feature`

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
- `features/schedule-collection-scan.feature`
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
| `W46-S01` | done | define periodic collection scan schedules and deterministic due-materialization in the domain | `scripts/check-slice.sh --wave W46-periodic-collection-scan-scheduling --slice W46-S01 --lane acceptance --path crates/venom-domain/src/inventory --path crates/venom-domain/src/scanning --path crates/venom-domain/src/durable_state.rs --path features/schedule-collection-scan.feature` |
| `W46-S02` | done | expose schedule configuration and due worker loops through the API and durable backends | `scripts/check-slice.sh --wave W46-periodic-collection-scan-scheduling --slice W46-S02 --lane integration --path apps/api/src/app --path apps/api/src/http --path apps/api/src/infra/postgres_backend.rs` |
| `W46-S03` | done | let operators configure and run collection schedules from the UI | `scripts/check-slice.sh --wave W46-periodic-collection-scan-scheduling --slice W46-S03 --lane e2e --path apps/web/src --path apps/web/e2e` |
| `W46-S04` | in_progress | close the wave with full docs and gate alignment | `scripts/check-wave.sh --wave W46-periodic-collection-scan-scheduling` |

## Language impact

`add`

## Invariant impact

`none`

## ADR impact

`none`

## Notes

- one schedule belongs to one managed collection
- missed periods coalesce into one due materialization per collection worker pass
- the schedule worker only enqueues scan commands; scan execution remains a separate explicit loop
