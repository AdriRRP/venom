# W229. Durable Granularity Closure

Wave: `W229-durable-granularity-closure`
Status: `done`
BDD impact: `none`
Agentic impact: `docs`
Infra profile: `db`

## Goal

Close the remaining recurring structural findings by moving Postgres read-side refreshes onto durable per-entity or per-finding deltas, and by finishing the operator-facing `system events` edge on shared or borrowed shapes instead of repeated public materialization.

## Feature paths

- `none`

## Execution lanes

- `unit`
- `integration`
- `infra`

## Owned paths

- `apps/api/src/infra/postgres_backend.rs`
- `apps/api/src/app/service.rs`
- `crates/venom-domain/src/findings/finding_read_model.rs`
- `crates/venom-domain/src/operations/system_event_trace.rs`
- `docs/reliability-hardening-plan.md`
- `docs/waves/W229-durable-granularity-closure.md`

## Slices

| Slice | Status | Goal | Verification |
|---|---|---|---|
| `W229-S01` | done | close the remaining recurring structural findings through finding-level read-model deltas, latest-entity collection refresh, and shared immutable `system events` edge shapes | targeted `cargo test` checks plus full package tests for `venom-domain` and `venom-api` |

## Language impact

`none`

## Invariant impact

`none`

## ADR impact

`none`

## Notes

- Prefer one reusable durable delta substrate over more lane-specific refresh helpers.
- No glossary, invariant, or BDD changes were required; the closure stayed inside durable refresh topology and operator-event projection shape.
