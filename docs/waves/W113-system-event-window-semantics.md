# W113. System Event Window Semantics

Wave: `W113-system-event-window-semantics`
Status: `done`
BDD impact: `none`
Agentic impact: `none`
Infra profile: `none`

## Goal

Make recent system-event totals and filters operate over the actual retained
timeline rather than a pre-truncated window.

## Owned paths

- `crates/venom-domain/src/durable_state.rs`
- `crates/venom-domain/src/scanning/durable_scan_runtime.rs`
- `apps/api/src/app/service.rs`
- `apps/api/src/infra/postgres_backend.rs`

## Slices

| Slice | Status | Goal | Verification |
|---|---|---|---|
| `W113-S01` | done | stop truncating local and Postgres event timelines before query filtering | `unit`, `integration` |

## Invariant impact

`I2`, `I9`, `I11`
